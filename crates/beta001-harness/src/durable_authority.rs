//! EXPERIMENTAL: restart-safe wrapper around the agent-runtime capability registry.
//!
//! Every change to authority state is written to an isolated `sovereign-ledger`
//! directory before the caller receives a result, and the registry is rebuilt on
//! open by re-driving its public API from that log.
//!
//! Boundaries:
//! - This is not the C01 canonical governance-event milestone. It persists six
//!   agent-runtime record kinds in a separate log with no state-root, snapshot,
//!   or registry-graph integration.
//! - It performs no effects and confers no execution authority. Callers in this
//!   experiment use simulated actions only.
//! - The log directory must not be shared with an engine ledger. Records use the
//!   `PolicyEnforcement` event type, which no existing mapper consumes, and carry
//!   their own magic header so a foreign reader cannot mistake them.
//! - Removal of the newest records cannot be detected from the log alone. A
//!   separately retained checkpoint ([`CheckpointStore`]) detects loss of any
//!   acknowledged record, provided the checkpoint itself is intact and current.
//!   It does not detect a rollback of the log and the checkpoint together, loss
//!   after an outdated checkpoint was put back, or deliberate rewriting: the
//!   checkpoint is unkeyed. Where a checkpoint should live is an open decision.
//! - Durability claims are those of `sovereign-ledger`. On Windows the ledger
//!   cannot synchronize the directory, and nothing here has been tested against
//!   power loss.

use chrono::{DateTime, FixedOffset};
use sovereign_agent_runtime::capability::{
    issue_grant, CapabilityRequest, CapabilityScope, CapabilityType, EvaluationResult, GrantId,
    RequestId, RequestStatus, SignedEvaluation,
};
use sovereign_agent_runtime::execution::{
    verify_receipt, AttemptId, AuthorizedExecution, CapabilityRegistry, ExecutionAttempt,
    ExecutionAuthorizationError, ExecutionReceipt, ExecutionResult, FixedClock, ReceiptId,
    TrustedClock,
};
use sovereign_agent_runtime::identity::{
    AgentClass, AgentIdentity, AgentIdentityId, AgentIdentityStatus, Digest, PolicyId,
    ReplayTimestamp, SchemaVersion,
};
use sovereign_audit::RecordId;
use sovereign_ledger::{
    AppendCommitStage, EventType, LedgerAppendEngine, LedgerConfig, LedgerError, Lsn,
    ReplayIterator,
};
use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;

/// Ledger event type used for every record in the isolated authority log.
pub const AUTHORITY_LOG_EVENT_TYPE: EventType = EventType::PolicyEnforcement;

const MAGIC: &[u8; 8] = b"SOVAUTH1";
const CHECKPOINT_MAGIC: &[u8; 8] = b"SOVAUTHC";
const CHECKPOINT_LEN: usize = 8 + 8 + 32 + 32;
const ZERO_DIGEST: [u8; 32] = [0u8; 32];
const AUTHORITY_LOG_MAX_PAYLOAD: usize = 16 * 1024;
const AUTHORITY_LOG_MAX_SEGMENT: usize = 32 * 1024;

/// Ledger configuration for an authority log rooted at `storage_root`.
pub fn authority_log_config(storage_root: PathBuf) -> LedgerConfig {
    let mut config = LedgerConfig::new(storage_root);
    config.max_record_payload_size = AUTHORITY_LOG_MAX_PAYLOAD;
    config.max_segment_size = AUTHORITY_LOG_MAX_SEGMENT;
    config
}

/// The keys and policy identity the registry trusts. Stored as the first record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustConfig {
    pub policy_id: String,
    pub policy_verifying_key: [u8; 32],
    pub execution_verifying_key: [u8; 32],
}

/// Position and chain digest of the newest record, for external anchoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadAnchor {
    pub records: u64,
    pub digest: [u8; 32],
}

/// What is known about the result of an authorized attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptOutcome {
    /// Authorized in this process; no receipt recorded yet.
    InFlight,
    /// A verified receipt is recorded.
    Confirmed(ExecutionResult),
    /// Authorized before a restart with no recorded receipt. The action may or
    /// may not have happened. Never retried or released automatically.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityLogError {
    /// The underlying ledger refused to open or read the log.
    Ledger(LedgerError),
    /// `open` found no records, or `create` found existing records.
    EmptyLog,
    LogNotEmpty,
    /// The number of readable records differs from the validated ledger tail.
    RecordCountMismatch,
    WrongEventType,
    Malformed,
    SequenceMismatch,
    ChainMismatch,
    MissingGenesis,
    UnexpectedGenesis,
    TrustMismatch,
    AnchorMismatch,
    InvalidTimestamp,
    ClockRegression,
    RecordTooLarge,
    GrantIdentityMismatch,
    InvalidReceipt,
    ReceiptWithoutAuthorization,
    ReceiptBindingMismatch,
    ReceiptPredatesAuthorization,
    DuplicateReceipt,
    /// The attempt's outcome is unknown after a restart and needs reconciliation.
    OutcomeUnknown,
    Authorization(ExecutionAuthorizationError),
    /// A durable save failed or was uncertain after memory changed. Nothing more
    /// is authorized until the store is reopened from the log.
    DurabilityUncertain,
    Poisoned,
    /// `open_with_checkpoint` found no checkpoint. Never treated as "no history".
    CheckpointMissing,
    /// The checkpoint exists but is damaged or not a checkpoint.
    CheckpointCorrupt,
    /// The checkpoint could not be read or written.
    CheckpointUnavailable,
    /// `create_with_checkpoint` found an existing checkpoint.
    CheckpointAlreadyExists,
}

/// Separately retained record of the newest acknowledged log position.
///
/// The store is updated after a record is durable and before the caller is
/// answered, so every acknowledged change is at or below the checkpoint.
pub trait CheckpointStore {
    /// `Ok(None)` means no checkpoint exists.
    fn load(&self) -> Result<Option<HeadAnchor>, AuthorityLogError>;
    fn save(&mut self, head: &HeadAnchor) -> Result<(), AuthorityLogError>;
}

/// Checkpoint kept in one small file, replaced atomically on every save.
///
/// The file carries a digest of its own contents to detect damage. It is not
/// signed: anyone who can write the file can forge it.
pub struct FileCheckpoint {
    path: PathBuf,
}

impl FileCheckpoint {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl CheckpointStore for FileCheckpoint {
    fn load(&self) -> Result<Option<HeadAnchor>, AuthorityLogError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(AuthorityLogError::CheckpointUnavailable),
        };
        if bytes.len() != CHECKPOINT_LEN
            || &bytes[..8] != CHECKPOINT_MAGIC
            || digest(&bytes[..48]) != bytes[48..]
        {
            return Err(AuthorityLogError::CheckpointCorrupt);
        }
        let records: [u8; 8] = bytes[8..16]
            .try_into()
            .map_err(|_| AuthorityLogError::CheckpointCorrupt)?;
        let head: [u8; 32] = bytes[16..48]
            .try_into()
            .map_err(|_| AuthorityLogError::CheckpointCorrupt)?;
        Ok(Some(HeadAnchor {
            records: u64::from_be_bytes(records),
            digest: head,
        }))
    }

    fn save(&mut self, head: &HeadAnchor) -> Result<(), AuthorityLogError> {
        let mut bytes = Vec::with_capacity(CHECKPOINT_LEN);
        bytes.extend_from_slice(CHECKPOINT_MAGIC);
        bytes.extend_from_slice(&head.records.to_be_bytes());
        bytes.extend_from_slice(&head.digest);
        let self_digest = digest(&bytes);
        bytes.extend_from_slice(&self_digest);

        let unavailable = |_| AuthorityLogError::CheckpointUnavailable;
        let pending = self.path.with_extension("pending");
        let mut file = std::fs::File::create(&pending).map_err(unavailable)?;
        file.write_all(&bytes).map_err(unavailable)?;
        file.sync_all().map_err(unavailable)?;
        drop(file);
        std::fs::rename(&pending, &self.path).map_err(unavailable)?;
        #[cfg(not(windows))]
        if let Some(parent) = self.path.parent() {
            std::fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(unavailable)?;
        }
        Ok(())
    }
}

/// Minimal durable append interface, so tests can inject save failures.
pub trait AuthorityJournal {
    /// Returns only after the payload is durable. Any error means the caller
    /// cannot know whether the record was saved.
    fn append(&mut self, payload: &[u8]) -> Result<(), AuthorityLogError>;
    fn max_payload(&self) -> usize;
}

type StageObserver = Box<dyn FnMut(AppendCommitStage)>;

/// Journal backed by the existing crash-atomic `sovereign-ledger` append engine.
pub struct LedgerJournal {
    engine: LedgerAppendEngine,
    max_payload: usize,
    stage_observer: Option<StageObserver>,
}

impl LedgerJournal {
    /// Installs an observer for the ledger's commit stages (crash-point tests).
    pub fn set_stage_observer(&mut self, observer: StageObserver) {
        self.stage_observer = Some(observer);
    }
}

impl AuthorityJournal for LedgerJournal {
    fn append(&mut self, payload: &[u8]) -> Result<(), AuthorityLogError> {
        let result = match self.stage_observer.as_mut() {
            Some(observer) => {
                self.engine
                    .append_observed(AUTHORITY_LOG_EVENT_TYPE, payload, observer)
            }
            None => self.engine.append(AUTHORITY_LOG_EVENT_TYPE, payload),
        };
        result.map(|_| ()).map_err(AuthorityLogError::Ledger)
    }

    fn max_payload(&self) -> usize {
        self.max_payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthorityRecord {
    Genesis(TrustConfig),
    IdentityRegistered(AgentIdentity),
    IdentityStatusChanged {
        identity_id: AgentIdentityId,
        status: AgentIdentityStatus,
    },
    GrantAdmitted {
        request: CapabilityRequest,
        evaluation: SignedEvaluation,
    },
    GrantRevoked(GrantId),
    ExecutionAuthorized(ExecutionAttempt),
    ReceiptRecorded(ExecutionReceipt),
}

/// One log entry: chain position, the decision time, and the record itself.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Envelope {
    sequence: u64,
    previous_digest: [u8; 32],
    /// The trusted-clock time at which the decision was taken.
    decided_at: String,
    record: AuthorityRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttemptState {
    grant_id: GrantId,
    executor_identity: AgentIdentityId,
    authorized_at: DateTime<FixedOffset>,
    outcome: AttemptOutcome,
}

/// In-memory authority state. Identical code applies a record live and on rebuild.
struct State {
    trust: TrustConfig,
    registry: CapabilityRegistry,
    attempts: HashMap<AttemptId, AttemptState>,
    last_decision: Option<DateTime<FixedOffset>>,
}

enum Applied {
    None,
    Grant(GrantId),
    Authorized(AuthorizedExecution),
}

impl State {
    fn new(trust: TrustConfig) -> Self {
        let registry = CapabilityRegistry::new(trust.policy_id.clone(), trust.policy_verifying_key);
        Self {
            trust,
            registry,
            attempts: HashMap::new(),
            last_decision: None,
        }
    }

    /// Validates and applies one record. On error, no state has changed.
    fn apply(&mut self, envelope: &Envelope) -> Result<Applied, AuthorityLogError> {
        let decided = parse_timestamp(&envelope.decided_at)
            .map_err(|_| AuthorityLogError::InvalidTimestamp)?;
        if self.last_decision.is_some_and(|last| decided < last) {
            return Err(AuthorityLogError::ClockRegression);
        }

        let applied = match &envelope.record {
            AuthorityRecord::Genesis(_) => return Err(AuthorityLogError::UnexpectedGenesis),
            AuthorityRecord::IdentityRegistered(identity) => {
                // The registry refuses an identity that is already registered.
                self.registry
                    .register_identity(identity.clone())
                    .map_err(AuthorityLogError::Authorization)?;
                Applied::None
            }
            AuthorityRecord::IdentityStatusChanged {
                identity_id,
                status,
            } => {
                self.registry
                    .set_identity_status(identity_id, status.clone())
                    .map_err(AuthorityLogError::Authorization)?;
                Applied::None
            }
            AuthorityRecord::GrantAdmitted {
                request,
                evaluation,
            } => {
                // The decision time is the issue time. The grant identifier is
                // derived from the request, the evaluation and that time, so it is
                // recomputed here rather than trusted from the log.
                let expected = issue_grant(
                    request,
                    evaluation,
                    &self.trust.policy_verifying_key,
                    &envelope.decided_at,
                )
                .map_err(|error| {
                    AuthorityLogError::Authorization(ExecutionAuthorizationError::GrantAdmission(
                        error,
                    ))
                })?;
                // The registry refuses a request or grant it has already admitted.
                let grant_id = self
                    .registry
                    .admit(request, evaluation, &envelope.decided_at)
                    .map_err(AuthorityLogError::Authorization)?;
                if grant_id != expected.grant_id {
                    return Err(AuthorityLogError::GrantIdentityMismatch);
                }
                Applied::Grant(grant_id)
            }
            AuthorityRecord::GrantRevoked(grant_id) => {
                self.registry
                    .revoke(grant_id)
                    .map_err(AuthorityLogError::Authorization)?;
                Applied::None
            }
            AuthorityRecord::ExecutionAuthorized(attempt) => {
                let authorized = self
                    .registry
                    .authorize_and_consume(attempt, &FixedClock(envelope.decided_at.clone()))
                    .map_err(AuthorityLogError::Authorization)?;
                self.attempts.insert(
                    attempt.attempt_id.clone(),
                    AttemptState {
                        grant_id: authorized.grant_id.clone(),
                        executor_identity: authorized.executor_identity.clone(),
                        authorized_at: decided,
                        outcome: AttemptOutcome::InFlight,
                    },
                );
                Applied::Authorized(authorized)
            }
            AuthorityRecord::ReceiptRecorded(receipt) => {
                if !verify_receipt(receipt, &self.trust.execution_verifying_key) {
                    return Err(AuthorityLogError::InvalidReceipt);
                }
                let completed = parse_timestamp(&receipt.completed_at)
                    .map_err(|_| AuthorityLogError::InvalidTimestamp)?;
                let state = self
                    .attempts
                    .get_mut(&receipt.attempt_reference)
                    .ok_or(AuthorityLogError::ReceiptWithoutAuthorization)?;
                if state.grant_id != receipt.grant_reference
                    || state.executor_identity != receipt.executor_identity
                {
                    return Err(AuthorityLogError::ReceiptBindingMismatch);
                }
                match state.outcome {
                    AttemptOutcome::InFlight => {}
                    AttemptOutcome::Confirmed(_) => {
                        return Err(AuthorityLogError::DuplicateReceipt)
                    }
                    AttemptOutcome::Unknown => return Err(AuthorityLogError::OutcomeUnknown),
                }
                if completed < state.authorized_at {
                    return Err(AuthorityLogError::ReceiptPredatesAuthorization);
                }
                state.outcome = AttemptOutcome::Confirmed(receipt.result.clone());
                Applied::None
            }
        };

        self.last_decision = Some(decided);
        Ok(applied)
    }
}

/// Restart-safe capability registry. See the module documentation for limits.
pub struct DurableAuthority<J: AuthorityJournal> {
    state: State,
    journal: J,
    next_sequence: u64,
    head_digest: [u8; 32],
    poisoned: bool,
    checkpoint: Option<Box<dyn CheckpointStore>>,
}

impl DurableAuthority<LedgerJournal> {
    /// Starts a new authority log. Fails if the directory already holds records.
    pub fn create(config: LedgerConfig, trust: TrustConfig) -> Result<Self, AuthorityLogError> {
        Self::create_inner(config, trust, None)
    }

    /// Starts a new authority log with a separately retained checkpoint. Fails
    /// if either the log or the checkpoint already exists.
    pub fn create_with_checkpoint(
        config: LedgerConfig,
        trust: TrustConfig,
        checkpoint: Box<dyn CheckpointStore>,
    ) -> Result<Self, AuthorityLogError> {
        if checkpoint.load()?.is_some() {
            return Err(AuthorityLogError::CheckpointAlreadyExists);
        }
        Self::create_inner(config, trust, Some(checkpoint))
    }

    fn create_inner(
        config: LedgerConfig,
        trust: TrustConfig,
        checkpoint: Option<Box<dyn CheckpointStore>>,
    ) -> Result<Self, AuthorityLogError> {
        let (journal, payloads) = open_ledger(&config)?;
        if !payloads.is_empty() {
            return Err(AuthorityLogError::LogNotEmpty);
        }
        let mut authority = Self {
            state: State::new(trust.clone()),
            journal,
            next_sequence: 0,
            head_digest: ZERO_DIGEST,
            poisoned: false,
            checkpoint,
        };
        let envelope = Envelope {
            sequence: 0,
            previous_digest: ZERO_DIGEST,
            decided_at: String::new(),
            record: AuthorityRecord::Genesis(trust),
        };
        let payload = authority.encode_checked(&envelope)?;
        authority.persist(&payload)?;
        Ok(authority)
    }

    /// Rebuilds authority state from an existing log. Fails closed on an empty
    /// log, any ledger error, any undecodable or out-of-chain record, a trust
    /// configuration that differs from `expected_trust`, any record the registry
    /// refuses, and an `anchor` the log does not contain.
    pub fn open(
        config: LedgerConfig,
        expected_trust: &TrustConfig,
        anchor: Option<&HeadAnchor>,
    ) -> Result<Self, AuthorityLogError> {
        Self::open_inner(config, expected_trust, anchor, None)
    }

    /// Rebuilds authority state and requires the log to contain the checkpointed
    /// head. A missing or damaged checkpoint fails closed. Records newer than
    /// the checkpoint are kept: they were durable but not yet acknowledged.
    pub fn open_with_checkpoint(
        config: LedgerConfig,
        expected_trust: &TrustConfig,
        mut checkpoint: Box<dyn CheckpointStore>,
    ) -> Result<Self, AuthorityLogError> {
        let anchor = checkpoint
            .load()?
            .ok_or(AuthorityLogError::CheckpointMissing)?;
        let mut authority = Self::open_inner(config, expected_trust, Some(&anchor), None)?;
        let head = authority.head();
        if head != anchor {
            checkpoint.save(&head)?;
        }
        authority.checkpoint = Some(checkpoint);
        Ok(authority)
    }

    fn open_inner(
        config: LedgerConfig,
        expected_trust: &TrustConfig,
        anchor: Option<&HeadAnchor>,
        checkpoint: Option<Box<dyn CheckpointStore>>,
    ) -> Result<Self, AuthorityLogError> {
        let (journal, payloads) = open_ledger(&config)?;
        if payloads.is_empty() {
            return Err(AuthorityLogError::EmptyLog);
        }

        let mut state: Option<State> = None;
        let mut head_digest = ZERO_DIGEST;
        let mut anchor_satisfied = anchor.is_none();
        for (index, payload) in payloads.iter().enumerate() {
            let envelope = decode_envelope(payload)?;
            if envelope.sequence != index as u64 {
                return Err(AuthorityLogError::SequenceMismatch);
            }
            if envelope.previous_digest != head_digest {
                return Err(AuthorityLogError::ChainMismatch);
            }
            match (&mut state, &envelope.record) {
                (None, AuthorityRecord::Genesis(trust)) => {
                    if trust != expected_trust {
                        return Err(AuthorityLogError::TrustMismatch);
                    }
                    state = Some(State::new(trust.clone()));
                }
                (None, _) => return Err(AuthorityLogError::MissingGenesis),
                (Some(existing), _) => {
                    existing.apply(&envelope)?;
                }
            }
            head_digest = digest(payload);
            if let Some(anchor) = anchor {
                if anchor.records == index as u64 + 1 {
                    if anchor.digest != head_digest {
                        return Err(AuthorityLogError::AnchorMismatch);
                    }
                    anchor_satisfied = true;
                }
            }
        }
        if !anchor_satisfied {
            return Err(AuthorityLogError::AnchorMismatch);
        }

        let mut state = state.ok_or(AuthorityLogError::MissingGenesis)?;
        // Anything authorized before this restart without a recorded receipt may
        // or may not have happened. It stays consumed and is never retried here.
        for attempt in state.attempts.values_mut() {
            if attempt.outcome == AttemptOutcome::InFlight {
                attempt.outcome = AttemptOutcome::Unknown;
            }
        }

        Ok(Self {
            state,
            journal,
            next_sequence: payloads.len() as u64,
            head_digest,
            poisoned: false,
            checkpoint,
        })
    }
}

impl<J: AuthorityJournal> DurableAuthority<J> {
    /// Swaps the journal, keeping state. For fault-injection tests only.
    pub fn with_journal<K: AuthorityJournal>(
        self,
        map: impl FnOnce(J) -> K,
    ) -> DurableAuthority<K> {
        DurableAuthority {
            state: self.state,
            journal: map(self.journal),
            next_sequence: self.next_sequence,
            head_digest: self.head_digest,
            poisoned: self.poisoned,
            checkpoint: self.checkpoint,
        }
    }

    pub fn journal_mut(&mut self) -> &mut J {
        &mut self.journal
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn trust(&self) -> &TrustConfig {
        &self.state.trust
    }

    pub fn head(&self) -> HeadAnchor {
        HeadAnchor {
            records: self.next_sequence,
            digest: self.head_digest,
        }
    }

    pub fn registry(&self) -> &CapabilityRegistry {
        &self.state.registry
    }

    pub fn attempt_outcome(&self, attempt_id: &AttemptId) -> Option<&AttemptOutcome> {
        self.state
            .attempts
            .get(attempt_id)
            .map(|state| &state.outcome)
    }

    /// Attempts whose outcome is unknown after a restart.
    pub fn unknown_outcomes(&self) -> Vec<AttemptId> {
        let mut unknown: Vec<AttemptId> = self
            .state
            .attempts
            .iter()
            .filter(|(_, state)| state.outcome == AttemptOutcome::Unknown)
            .map(|(attempt_id, _)| attempt_id.clone())
            .collect();
        unknown.sort_by(|left, right| left.0.cmp(&right.0));
        unknown
    }

    pub fn register_identity(
        &mut self,
        identity: AgentIdentity,
        clock: &impl TrustedClock,
    ) -> Result<(), AuthorityLogError> {
        self.commit(AuthorityRecord::IdentityRegistered(identity), clock)
            .map(|_| ())
    }

    pub fn set_identity_status(
        &mut self,
        identity_id: &AgentIdentityId,
        status: AgentIdentityStatus,
        clock: &impl TrustedClock,
    ) -> Result<(), AuthorityLogError> {
        self.commit(
            AuthorityRecord::IdentityStatusChanged {
                identity_id: identity_id.clone(),
                status,
            },
            clock,
        )
        .map(|_| ())
    }

    /// Admits a grant. The trusted-clock time is recorded as the issue time.
    pub fn admit(
        &mut self,
        request: &CapabilityRequest,
        evaluation: &SignedEvaluation,
        clock: &impl TrustedClock,
    ) -> Result<GrantId, AuthorityLogError> {
        match self.commit(
            AuthorityRecord::GrantAdmitted {
                request: request.clone(),
                evaluation: evaluation.clone(),
            },
            clock,
        )? {
            Applied::Grant(grant_id) => Ok(grant_id),
            _ => Err(AuthorityLogError::GrantIdentityMismatch),
        }
    }

    pub fn revoke(
        &mut self,
        grant_id: &GrantId,
        clock: &impl TrustedClock,
    ) -> Result<(), AuthorityLogError> {
        self.commit(AuthorityRecord::GrantRevoked(grant_id.clone()), clock)
            .map(|_| ())
    }

    /// Authorizes and consumes a grant. The returned token exists only after the
    /// consumption record is durable; a simulated action must not run without it.
    pub fn authorize(
        &mut self,
        attempt: &ExecutionAttempt,
        clock: &impl TrustedClock,
    ) -> Result<AuthorizedExecution, AuthorityLogError> {
        match self.commit(AuthorityRecord::ExecutionAuthorized(attempt.clone()), clock)? {
            Applied::Authorized(authorized) => Ok(authorized),
            _ => Err(AuthorityLogError::Malformed),
        }
    }

    /// Records a verified receipt for an attempt authorized in this process.
    pub fn record_receipt(
        &mut self,
        receipt: &ExecutionReceipt,
        clock: &impl TrustedClock,
    ) -> Result<(), AuthorityLogError> {
        self.commit(AuthorityRecord::ReceiptRecorded(receipt.clone()), clock)
            .map(|_| ())
    }

    fn encode_checked(&self, envelope: &Envelope) -> Result<Vec<u8>, AuthorityLogError> {
        let payload = encode_envelope(envelope);
        if payload.len() > self.journal.max_payload() {
            return Err(AuthorityLogError::RecordTooLarge);
        }
        Ok(payload)
    }

    fn persist(&mut self, payload: &[u8]) -> Result<(), AuthorityLogError> {
        if self.journal.append(payload).is_err() {
            // Memory may now be ahead of, or behind, the durable log.
            self.poisoned = true;
            return Err(AuthorityLogError::DurabilityUncertain);
        }
        self.head_digest = digest(payload);
        self.next_sequence += 1;
        let head = self.head();
        if let Some(checkpoint) = self.checkpoint.as_mut() {
            if checkpoint.save(&head).is_err() {
                // The record is durable but the caller has not been answered, and
                // the checkpoint may be behind it. Stop until reconstructed.
                self.poisoned = true;
                return Err(AuthorityLogError::DurabilityUncertain);
            }
        }
        Ok(())
    }

    fn commit(
        &mut self,
        record: AuthorityRecord,
        clock: &impl TrustedClock,
    ) -> Result<Applied, AuthorityLogError> {
        if self.poisoned {
            return Err(AuthorityLogError::Poisoned);
        }
        let envelope = Envelope {
            sequence: self.next_sequence,
            previous_digest: self.head_digest,
            decided_at: clock.now(),
            record,
        };
        // Encode and size-check first, so an oversized record changes nothing.
        let payload = self.encode_checked(&envelope)?;
        let applied = self.state.apply(&envelope)?;
        self.persist(&payload)?;
        Ok(applied)
    }
}

fn open_ledger(config: &LedgerConfig) -> Result<(LedgerJournal, Vec<Vec<u8>>), AuthorityLogError> {
    // Bootstrap runs the ledger's strict tail discovery: a damaged or missing
    // canonical record anywhere, including the newest, is an error. Canonical
    // records are published atomically, so damage is never an interrupted save.
    // An interrupted save leaves only a non-canonical pending file.
    let engine =
        LedgerAppendEngine::bootstrap(config.clone()).map_err(AuthorityLogError::Ledger)?;
    let mut iterator = ReplayIterator::bootstrap(config.clone(), Lsn::GENESIS)
        .map_err(AuthorityLogError::Ledger)?;
    let mut payloads = Vec::new();
    while let Some(record) = iterator.next_record() {
        let record = record.map_err(AuthorityLogError::Ledger)?;
        if record.event_type != AUTHORITY_LOG_EVENT_TYPE {
            return Err(AuthorityLogError::WrongEventType);
        }
        payloads.push(record.payload.to_vec());
    }
    if payloads.len() as u64 != engine.next_lsn().get() {
        return Err(AuthorityLogError::RecordCountMismatch);
    }
    Ok((
        LedgerJournal {
            engine,
            max_payload: config.max_record_payload_size,
            stage_observer: None,
        },
        payloads,
    ))
}

/// Same RFC 3339 parsing the agent runtime applies to grant and attempt times.
fn parse_timestamp(value: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(value)
}

fn digest(payload: &[u8]) -> [u8; 32] {
    *blake3::hash(payload).as_bytes()
}

// ---------------------------------------------------------------------------
// Length-prefixed binary codec. Decoding rejects trailing bytes and unknown tags.
// ---------------------------------------------------------------------------

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
    fn fixed(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
    fn bytes(&mut self, bytes: &[u8]) {
        self.0
            .extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        self.0.extend_from_slice(bytes);
    }
    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
}

struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

type Decoded<T> = Result<T, AuthorityLogError>;

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Decoded<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(AuthorityLogError::Malformed)?;
        if end > self.data.len() {
            return Err(AuthorityLogError::Malformed);
        }
        let slice = &self.data[self.offset..end];
        self.offset = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Decoded<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Decoded<u32> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| AuthorityLogError::Malformed)?;
        Ok(u32::from_be_bytes(bytes))
    }
    fn u64(&mut self) -> Decoded<u64> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| AuthorityLogError::Malformed)?;
        Ok(u64::from_be_bytes(bytes))
    }
    fn array<const N: usize>(&mut self) -> Decoded<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| AuthorityLogError::Malformed)
    }
    fn string(&mut self) -> Decoded<String> {
        let len = self.u32()? as usize;
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| AuthorityLogError::Malformed)
    }
    fn finish(&self) -> Decoded<()> {
        if self.offset == self.data.len() {
            Ok(())
        } else {
            Err(AuthorityLogError::Malformed)
        }
    }
}

fn encode_envelope(envelope: &Envelope) -> Vec<u8> {
    let mut writer = Writer(Vec::new());
    writer.fixed(MAGIC);
    writer.u64(envelope.sequence);
    writer.fixed(&envelope.previous_digest);
    writer.string(&envelope.decided_at);
    match &envelope.record {
        AuthorityRecord::Genesis(trust) => {
            writer.u8(0);
            writer.string(&trust.policy_id);
            writer.fixed(&trust.policy_verifying_key);
            writer.fixed(&trust.execution_verifying_key);
        }
        AuthorityRecord::IdentityRegistered(identity) => {
            writer.u8(1);
            writer.string(&identity.schema_version.0);
            writer.string(&identity.identity_id.0);
            writer.string(&identity.public_key_digest.0);
            writer.u8(match identity.agent_class {
                AgentClass::LocalModel => 0,
                AgentClass::ExternalModel => 1,
                AgentClass::HumanOperator => 2,
            });
            writer.string(&identity.created_at.0);
            writer.string(&identity.governing_policy.0);
            writer.u8(identity_status_tag(&identity.status));
        }
        AuthorityRecord::IdentityStatusChanged {
            identity_id,
            status,
        } => {
            writer.u8(2);
            writer.string(&identity_id.0);
            writer.u8(identity_status_tag(status));
        }
        AuthorityRecord::GrantAdmitted {
            request,
            evaluation,
        } => {
            writer.u8(3);
            writer.string(&request.schema_version);
            writer.string(&request.request_id.0);
            writer.string(&request.requester_identity.0);
            writer.fixed(request.proposal_reference.as_bytes());
            writer.u8(capability_type_tag(&request.capability_type));
            encode_scope(&mut writer, &request.requested_scope);
            writer.string(&request.created_at);
            writer.u8(match request.status {
                RequestStatus::Pending => 0,
                RequestStatus::Approved => 1,
                RequestStatus::Denied => 2,
            });
            writer.string(&evaluation.request_reference.0);
            writer.string(&evaluation.policy_id);
            match &evaluation.result {
                EvaluationResult::Approved { scope, expires_at } => {
                    writer.u8(0);
                    encode_scope(&mut writer, scope);
                    writer.string(expires_at);
                }
                EvaluationResult::Denied { reason } => {
                    writer.u8(1);
                    writer.string(reason);
                }
            }
            writer.string(&evaluation.evaluated_at);
            writer.fixed(&evaluation.signature);
        }
        AuthorityRecord::GrantRevoked(grant_id) => {
            writer.u8(4);
            writer.string(&grant_id.0);
        }
        AuthorityRecord::ExecutionAuthorized(attempt) => {
            writer.u8(5);
            writer.string(&attempt.schema_version);
            writer.string(&attempt.attempt_id.0);
            writer.string(&attempt.grant_reference.0);
            writer.string(&attempt.executor_identity.0);
            writer.string(&attempt.operation_type);
            writer.string(&attempt.target_resource);
            writer.fixed(attempt.target_reference.as_bytes());
            writer.string(&attempt.created_at);
        }
        AuthorityRecord::ReceiptRecorded(receipt) => {
            writer.u8(6);
            writer.string(&receipt.schema_version);
            writer.string(&receipt.receipt_id.0);
            writer.string(&receipt.attempt_reference.0);
            writer.string(&receipt.grant_reference.0);
            writer.string(&receipt.executor_identity.0);
            writer.u8(match receipt.result {
                ExecutionResult::Success => 0,
                ExecutionResult::Failure => 1,
            });
            writer.string(&receipt.output_digest);
            writer.string(&receipt.completed_at);
            writer.string(&receipt.receipt_digest);
            writer.fixed(&receipt.signature);
        }
    }
    writer.0
}

fn decode_envelope(payload: &[u8]) -> Decoded<Envelope> {
    let mut reader = Reader {
        data: payload,
        offset: 0,
    };
    if reader.take(MAGIC.len())? != MAGIC {
        return Err(AuthorityLogError::Malformed);
    }
    let sequence = reader.u64()?;
    let previous_digest = reader.array::<32>()?;
    let decided_at = reader.string()?;
    let record = match reader.u8()? {
        0 => AuthorityRecord::Genesis(TrustConfig {
            policy_id: reader.string()?,
            policy_verifying_key: reader.array::<32>()?,
            execution_verifying_key: reader.array::<32>()?,
        }),
        1 => AuthorityRecord::IdentityRegistered(AgentIdentity {
            schema_version: SchemaVersion(reader.string()?),
            identity_id: AgentIdentityId(reader.string()?),
            public_key_digest: Digest(reader.string()?),
            agent_class: match reader.u8()? {
                0 => AgentClass::LocalModel,
                1 => AgentClass::ExternalModel,
                2 => AgentClass::HumanOperator,
                _ => return Err(AuthorityLogError::Malformed),
            },
            created_at: ReplayTimestamp(reader.string()?),
            governing_policy: PolicyId(reader.string()?),
            status: decode_identity_status(reader.u8()?)?,
        }),
        2 => AuthorityRecord::IdentityStatusChanged {
            identity_id: AgentIdentityId(reader.string()?),
            status: decode_identity_status(reader.u8()?)?,
        },
        3 => {
            let request = CapabilityRequest {
                schema_version: reader.string()?,
                request_id: RequestId(reader.string()?),
                requester_identity: AgentIdentityId(reader.string()?),
                proposal_reference: RecordId::from_bytes(reader.array::<32>()?),
                capability_type: decode_capability_type(reader.u8()?)?,
                requested_scope: decode_scope(&mut reader)?,
                created_at: reader.string()?,
                status: match reader.u8()? {
                    0 => RequestStatus::Pending,
                    1 => RequestStatus::Approved,
                    2 => RequestStatus::Denied,
                    _ => return Err(AuthorityLogError::Malformed),
                },
            };
            let evaluation = SignedEvaluation {
                request_reference: RequestId(reader.string()?),
                policy_id: reader.string()?,
                result: match reader.u8()? {
                    0 => EvaluationResult::Approved {
                        scope: decode_scope(&mut reader)?,
                        expires_at: reader.string()?,
                    },
                    1 => EvaluationResult::Denied {
                        reason: reader.string()?,
                    },
                    _ => return Err(AuthorityLogError::Malformed),
                },
                evaluated_at: reader.string()?,
                signature: reader.array::<64>()?,
            };
            AuthorityRecord::GrantAdmitted {
                request,
                evaluation,
            }
        }
        4 => AuthorityRecord::GrantRevoked(GrantId(reader.string()?)),
        5 => AuthorityRecord::ExecutionAuthorized(ExecutionAttempt {
            schema_version: reader.string()?,
            attempt_id: AttemptId(reader.string()?),
            grant_reference: GrantId(reader.string()?),
            executor_identity: AgentIdentityId(reader.string()?),
            operation_type: reader.string()?,
            target_resource: reader.string()?,
            target_reference: RecordId::from_bytes(reader.array::<32>()?),
            created_at: reader.string()?,
        }),
        6 => AuthorityRecord::ReceiptRecorded(ExecutionReceipt {
            schema_version: reader.string()?,
            receipt_id: ReceiptId(reader.string()?),
            attempt_reference: AttemptId(reader.string()?),
            grant_reference: GrantId(reader.string()?),
            executor_identity: AgentIdentityId(reader.string()?),
            result: match reader.u8()? {
                0 => ExecutionResult::Success,
                1 => ExecutionResult::Failure,
                _ => return Err(AuthorityLogError::Malformed),
            },
            output_digest: reader.string()?,
            completed_at: reader.string()?,
            receipt_digest: reader.string()?,
            signature: reader.array::<64>()?,
        }),
        _ => return Err(AuthorityLogError::Malformed),
    };
    reader.finish()?;
    Ok(Envelope {
        sequence,
        previous_digest,
        decided_at,
        record,
    })
}

fn identity_status_tag(status: &AgentIdentityStatus) -> u8 {
    match status {
        AgentIdentityStatus::Pending => 0,
        AgentIdentityStatus::Active => 1,
        AgentIdentityStatus::Suspended => 2,
        AgentIdentityStatus::Revoked => 3,
    }
}

fn decode_identity_status(tag: u8) -> Decoded<AgentIdentityStatus> {
    match tag {
        0 => Ok(AgentIdentityStatus::Pending),
        1 => Ok(AgentIdentityStatus::Active),
        2 => Ok(AgentIdentityStatus::Suspended),
        3 => Ok(AgentIdentityStatus::Revoked),
        _ => Err(AuthorityLogError::Malformed),
    }
}

fn capability_type_tag(capability_type: &CapabilityType) -> u8 {
    match capability_type {
        CapabilityType::Read => 0,
        CapabilityType::Write => 1,
        CapabilityType::Execute => 2,
    }
}

fn decode_capability_type(tag: u8) -> Decoded<CapabilityType> {
    match tag {
        0 => Ok(CapabilityType::Read),
        1 => Ok(CapabilityType::Write),
        2 => Ok(CapabilityType::Execute),
        _ => Err(AuthorityLogError::Malformed),
    }
}

fn encode_scope(writer: &mut Writer, scope: &CapabilityScope) {
    writer.string(&scope.target_resource);
    writer.fixed(scope.target_reference.as_bytes());
    writer
        .0
        .extend_from_slice(&(scope.permissions.len() as u32).to_be_bytes());
    for permission in &scope.permissions {
        writer.string(permission);
    }
}

fn decode_scope(reader: &mut Reader<'_>) -> Decoded<CapabilityScope> {
    let target_resource = reader.string()?;
    let target_reference = RecordId::from_bytes(reader.array::<32>()?);
    let count = reader.u32()? as usize;
    // Each permission needs at least its four-byte length prefix.
    if count > (reader.data.len() - reader.offset) / 4 {
        return Err(AuthorityLogError::Malformed);
    }
    let mut permissions = Vec::with_capacity(count);
    for _ in 0..count {
        permissions.push(reader.string()?);
    }
    Ok(CapabilityScope {
        target_resource,
        target_reference,
        permissions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_envelopes() -> Vec<Envelope> {
        let scope = CapabilityScope {
            target_resource: "repo/src/lib.rs".into(),
            target_reference: RecordId::from_bytes([1u8; 32]),
            permissions: vec!["write".into(), "read".into()],
        };
        let records = vec![
            AuthorityRecord::Genesis(TrustConfig {
                policy_id: "policy".into(),
                policy_verifying_key: [3u8; 32],
                execution_verifying_key: [4u8; 32],
            }),
            AuthorityRecord::IdentityRegistered(AgentIdentity {
                schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
                identity_id: AgentIdentityId("id".into()),
                public_key_digest: Digest("11".repeat(32)),
                agent_class: AgentClass::HumanOperator,
                created_at: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
                governing_policy: PolicyId("policy".into()),
                status: AgentIdentityStatus::Active,
            }),
            AuthorityRecord::IdentityStatusChanged {
                identity_id: AgentIdentityId("id".into()),
                status: AgentIdentityStatus::Suspended,
            },
            AuthorityRecord::GrantAdmitted {
                request: CapabilityRequest {
                    schema_version: "CAPABILITY_REQUEST-v1".into(),
                    request_id: RequestId("request".into()),
                    requester_identity: AgentIdentityId("id".into()),
                    proposal_reference: RecordId::from_bytes([9u8; 32]),
                    capability_type: CapabilityType::Write,
                    requested_scope: scope.clone(),
                    created_at: "2026-08-20T09:55:00Z".into(),
                    status: RequestStatus::Pending,
                },
                evaluation: SignedEvaluation {
                    request_reference: RequestId("request".into()),
                    policy_id: "policy".into(),
                    result: EvaluationResult::Approved {
                        scope,
                        expires_at: "2026-08-20T11:00:00Z".into(),
                    },
                    evaluated_at: "2026-08-20T09:59:00Z".into(),
                    signature: [5u8; 64],
                },
            },
            AuthorityRecord::GrantRevoked(GrantId("grant".into())),
            AuthorityRecord::ExecutionAuthorized(ExecutionAttempt {
                schema_version: "EXECUTION_ATTEMPT-v1".into(),
                attempt_id: AttemptId("attempt".into()),
                grant_reference: GrantId("grant".into()),
                executor_identity: AgentIdentityId("id".into()),
                operation_type: "write".into(),
                target_resource: "repo/src/lib.rs".into(),
                target_reference: RecordId::from_bytes([1u8; 32]),
                created_at: "2026-08-20T10:15:00Z".into(),
            }),
            AuthorityRecord::ReceiptRecorded(ExecutionReceipt {
                schema_version: "EXECUTION_RECEIPT-v1".into(),
                receipt_id: ReceiptId("receipt".into()),
                attempt_reference: AttemptId("attempt".into()),
                grant_reference: GrantId("grant".into()),
                executor_identity: AgentIdentityId("id".into()),
                result: ExecutionResult::Failure,
                output_digest: "digest".into(),
                completed_at: "2026-08-20T10:16:00Z".into(),
                receipt_digest: "receipt-digest".into(),
                signature: [6u8; 64],
            }),
        ];
        records
            .into_iter()
            .enumerate()
            .map(|(index, record)| Envelope {
                sequence: index as u64,
                previous_digest: [index as u8; 32],
                decided_at: "2026-08-20T10:00:00Z".into(),
                record,
            })
            .collect()
    }

    #[test]
    fn every_record_kind_round_trips() {
        for envelope in sample_envelopes() {
            let encoded = encode_envelope(&envelope);
            assert_eq!(decode_envelope(&encoded), Ok(envelope));
        }
    }

    #[test]
    fn every_truncation_is_rejected() {
        for envelope in sample_envelopes() {
            let encoded = encode_envelope(&envelope);
            for len in 0..encoded.len() {
                assert_eq!(
                    decode_envelope(&encoded[..len]),
                    Err(AuthorityLogError::Malformed),
                    "truncation to {len} bytes was accepted"
                );
            }
        }
    }

    #[test]
    fn trailing_bytes_and_unknown_tags_are_rejected() {
        for envelope in sample_envelopes() {
            let mut encoded = encode_envelope(&envelope);
            encoded.push(0);
            assert_eq!(decode_envelope(&encoded), Err(AuthorityLogError::Malformed));
        }
        let mut wrong_magic = encode_envelope(&sample_envelopes()[0]);
        wrong_magic[0] ^= 0xff;
        assert_eq!(
            decode_envelope(&wrong_magic),
            Err(AuthorityLogError::Malformed)
        );

        let genesis = encode_envelope(&sample_envelopes()[0]);
        // The record tag follows magic, sequence, digest and the decided_at string.
        let tag_offset = MAGIC.len() + 8 + 32 + 4 + "2026-08-20T10:00:00Z".len();
        let mut unknown_tag = genesis;
        unknown_tag[tag_offset] = 0x7f;
        assert_eq!(
            decode_envelope(&unknown_tag),
            Err(AuthorityLogError::Malformed)
        );
    }

    #[test]
    fn oversized_permission_count_is_rejected_without_allocation() {
        let envelope = sample_envelopes().remove(3);
        let encoded = encode_envelope(&envelope);
        // Find the permission count (2) that follows the first scope's reference.
        let needle = [&[1u8; 32][..], &2u32.to_be_bytes()[..]].concat();
        let position = encoded
            .windows(needle.len())
            .position(|window| window == needle.as_slice())
            .expect("scope present");
        let mut corrupted = encoded;
        corrupted[position + 32..position + 36].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            decode_envelope(&corrupted),
            Err(AuthorityLogError::Malformed)
        );
    }
}
