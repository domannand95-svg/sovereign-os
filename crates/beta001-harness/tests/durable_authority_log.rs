//! EXPERIMENTAL restart-safety tests for the durable authority log.
//!
//! All actions are simulated: the "effect" is a counter that can only be advanced
//! with an authorization token. Nothing here touches real files or tools outside a
//! per-test temporary directory, and nothing here is the C01 milestone.

use beta001_harness::durable_authority::{
    authority_log_config, AttemptOutcome, AuthorityJournal, AuthorityLogError, CheckpointStore,
    DurableAuthority, FileCheckpoint, HeadAnchor, LedgerJournal, TrustConfig,
    AUTHORITY_LOG_EVENT_TYPE,
};
use sovereign_agent_runtime::capability::{
    derive_grant_id, derive_request_id, CapabilityRequest, CapabilityScope, CapabilityType,
    EvaluationResult, GrantId, GrantStatus, PolicyAuthority, RequestId, RequestStatus,
    SignedEvaluation,
};
use sovereign_agent_runtime::execution::{
    derive_attempt_id, AttemptId, AuthorizedExecution, ExecutionAttempt, ExecutionAuthority,
    ExecutionAuthorizationError, ExecutionReceipt, ExecutionResult, FixedClock,
};
use sovereign_agent_runtime::identity::{
    derive_identity_id, AgentClass, AgentIdentity, AgentIdentityId, AgentIdentityStatus, Digest,
    IdentityValidationError, PolicyId, ReplayTimestamp, SchemaVersion,
};
use sovereign_audit::RecordId;
use sovereign_ledger::{AppendCommitStage, EventType, LedgerAppendEngine, LedgerError};
use std::cell::Cell;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tempfile::TempDir;

const T_REGISTER: &str = "2026-08-20T09:30:00Z";
const T_ADMIT: &str = "2026-08-20T10:00:00Z";
const T_ATTEMPT: &str = "2026-08-20T10:15:00Z";
const T_RECEIPT: &str = "2026-08-20T10:16:00Z";
const T_LATER: &str = "2026-08-20T10:20:00Z";

type Authority = DurableAuthority<LedgerJournal>;

fn clock(time: &str) -> FixedClock {
    FixedClock(time.into())
}

fn policy_authority() -> PolicyAuthority {
    PolicyAuthority::from_seed("policy_engine_v1", [7u8; 32])
}

fn execution_authority() -> ExecutionAuthority {
    ExecutionAuthority::from_seed([8u8; 32])
}

fn trust() -> TrustConfig {
    TrustConfig {
        policy_id: "policy_engine_v1".into(),
        policy_verifying_key: policy_authority().verifying_key(),
        execution_verifying_key: execution_authority().verifying_key(),
    }
}

fn identity() -> AgentIdentity {
    let key = Digest("11".repeat(32));
    let policy = PolicyId("policy_engine_v1".into());
    let class = AgentClass::LocalModel;
    AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: derive_identity_id(&key, &class, &policy),
        public_key_digest: key,
        agent_class: class,
        created_at: ReplayTimestamp("2026-08-20T09:00:00Z".into()),
        governing_policy: policy,
        status: AgentIdentityStatus::Active,
    }
}

fn request_for(target_resource: &str) -> CapabilityRequest {
    let mut request = CapabilityRequest {
        schema_version: "CAPABILITY_REQUEST-v1".into(),
        request_id: RequestId(String::new()),
        requester_identity: identity().identity_id,
        proposal_reference: RecordId::from_bytes([9u8; 32]),
        capability_type: CapabilityType::Write,
        requested_scope: CapabilityScope {
            target_resource: target_resource.into(),
            target_reference: RecordId::from_bytes([1u8; 32]),
            permissions: vec!["write".into()],
        },
        created_at: "2026-08-20T09:55:00Z".into(),
        status: RequestStatus::Pending,
    };
    request.request_id = derive_request_id(
        &request.requester_identity,
        &request.proposal_reference,
        &request.capability_type,
        &request.requested_scope,
        &request.created_at,
    );
    request
}

fn request() -> CapabilityRequest {
    request_for("repo/src/lib.rs")
}

fn approval(request: &CapabilityRequest) -> SignedEvaluation {
    policy_authority().evaluate(
        request,
        EvaluationResult::Approved {
            scope: request.requested_scope.clone(),
            expires_at: "2026-08-20T11:00:00Z".into(),
        },
        "2026-08-20T09:59:00Z",
    )
}

fn attempt_at(grant_id: &GrantId, created_at: &str) -> ExecutionAttempt {
    let mut attempt = ExecutionAttempt {
        schema_version: "EXECUTION_ATTEMPT-v1".into(),
        attempt_id: AttemptId(String::new()),
        grant_reference: grant_id.clone(),
        executor_identity: identity().identity_id,
        operation_type: "write".into(),
        target_resource: "repo/src/lib.rs".into(),
        target_reference: RecordId::from_bytes([1u8; 32]),
        created_at: created_at.into(),
    };
    attempt.attempt_id = derive_attempt_id(
        &attempt.grant_reference,
        &attempt.executor_identity,
        &attempt.operation_type,
        &attempt.target_resource,
        &attempt.target_reference,
        &attempt.created_at,
    );
    attempt
}

fn attempt(grant_id: &GrantId) -> ExecutionAttempt {
    attempt_at(grant_id, T_ATTEMPT)
}

fn receipt(authorized: &AuthorizedExecution) -> ExecutionReceipt {
    execution_authority().issue_receipt(authorized, ExecutionResult::Success, "output", T_RECEIPT)
}

/// Stand-in for a real action. It can only run with an authorization token.
#[derive(Default)]
struct SimulatedAction {
    runs: HashMap<String, u32>,
}

impl SimulatedAction {
    fn run(&mut self, token: &AuthorizedExecution) {
        *self.runs.entry(token.attempt_id.0.clone()).or_insert(0) += 1;
    }
    fn count(&self, attempt: &ExecutionAttempt) -> u32 {
        self.runs.get(&attempt.attempt_id.0).copied().unwrap_or(0)
    }
}

fn log_dir(temp: &TempDir) -> PathBuf {
    temp.path().join("authority-log")
}

fn create(temp: &TempDir) -> Authority {
    DurableAuthority::create(authority_log_config(log_dir(temp)), trust()).expect("create")
}

fn reopen(temp: &TempDir) -> Authority {
    DurableAuthority::open(authority_log_config(log_dir(temp)), &trust(), None).expect("reopen")
}

fn try_reopen(temp: &TempDir) -> Result<Authority, AuthorityLogError> {
    DurableAuthority::open(authority_log_config(log_dir(temp)), &trust(), None)
}

/// A store with one registered identity and one admitted, unconsumed grant.
fn with_grant(temp: &TempDir) -> (Authority, GrantId) {
    let mut authority = create(temp);
    authority
        .register_identity(identity(), &clock(T_REGISTER))
        .expect("register");
    let request = request();
    let grant_id = authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .expect("admit");
    (authority, grant_id)
}

fn record_path(temp: &TempDir, lsn: u64) -> PathBuf {
    log_dir(temp).join(format!("{lsn:016x}.seg"))
}

fn pending_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .expect("read dir")
        .filter(|entry| {
            entry
                .as_ref()
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".pending")
        })
        .count()
}

fn consumed(error: AuthorityLogError) -> bool {
    error == AuthorityLogError::Authorization(ExecutionAuthorizationError::AlreadyConsumed)
}

#[derive(Clone, Copy)]
enum Fault {
    None,
    BeforeWrite,
    AfterWrite,
}

/// Journal that can fail before writing, or after the record is already durable.
struct FaultyJournal {
    inner: LedgerJournal,
    fault: Fault,
}

impl AuthorityJournal for FaultyJournal {
    fn append(&mut self, payload: &[u8]) -> Result<(), AuthorityLogError> {
        match self.fault {
            Fault::None => self.inner.append(payload),
            Fault::BeforeWrite => Err(AuthorityLogError::Ledger(LedgerError::WriteViolation)),
            Fault::AfterWrite => {
                self.inner.append(payload)?;
                Err(AuthorityLogError::Ledger(LedgerError::CommitAmbiguous))
            }
        }
    }
    fn max_payload(&self) -> usize {
        self.inner.max_payload()
    }
}

fn faulty(authority: Authority, fault: Fault) -> DurableAuthority<FaultyJournal> {
    authority.with_journal(|inner| FaultyJournal { inner, fault })
}

// --- Restart -----------------------------------------------------------------

#[test]
fn dal_01_consumption_and_confirmed_result_survive_restart() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let attempt = attempt(&grant_id);
    let mut action = SimulatedAction::default();

    let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    action.run(&token);
    authority
        .record_receipt(&receipt(&token), &clock(T_RECEIPT))
        .unwrap();
    let head_before = authority.head();
    drop(authority);

    let mut authority = reopen(&temp);
    assert_eq!(authority.head(), head_before);
    assert_eq!(
        authority.attempt_outcome(&attempt.attempt_id),
        Some(&AttemptOutcome::Confirmed(ExecutionResult::Success))
    );
    assert!(authority.unknown_outcomes().is_empty());
    assert!(consumed(
        authority.authorize(&attempt, &clock(T_LATER)).unwrap_err()
    ));
    // A different attempt against the same single-use grant is also refused.
    let second = attempt_at(&grant_id, T_LATER);
    assert!(consumed(
        authority.authorize(&second, &clock(T_LATER)).unwrap_err()
    ));
    assert_eq!(action.count(&attempt), 1);
}

#[test]
fn dal_02_revocation_survives_restart() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    authority.revoke(&grant_id, &clock(T_ATTEMPT)).unwrap();
    drop(authority);

    let mut authority = reopen(&temp);
    assert_eq!(
        authority.registry().grant(&grant_id).unwrap().status,
        GrantStatus::Revoked
    );
    assert_eq!(
        authority.authorize(&attempt(&grant_id), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::InactiveGrant
        ))
    );
}

#[test]
fn dal_03_identity_suspension_survives_restart() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    authority
        .set_identity_status(
            &identity().identity_id,
            AgentIdentityStatus::Suspended,
            &clock(T_ATTEMPT),
        )
        .unwrap();
    drop(authority);

    let mut authority = reopen(&temp);
    assert_eq!(
        authority.authorize(&attempt(&grant_id), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::IdentityInactive
        ))
    );
    let other = request_for("repo/src/other.rs");
    assert_eq!(
        authority.admit(&other, &approval(&other), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::IdentityInactive
        ))
    );
}

#[test]
fn dal_04_trust_configuration_is_preserved_and_enforced_on_open() {
    let temp = TempDir::new().unwrap();
    let (authority, _) = with_grant(&temp);
    drop(authority);

    assert_eq!(reopen(&temp).trust(), &trust());

    let mut wrong_policy_key = trust();
    wrong_policy_key.policy_verifying_key =
        PolicyAuthority::from_seed("policy_engine_v1", [70u8; 32]).verifying_key();
    let mut wrong_execution_key = trust();
    wrong_execution_key.execution_verifying_key =
        ExecutionAuthority::from_seed([80u8; 32]).verifying_key();
    let mut wrong_policy_id = trust();
    wrong_policy_id.policy_id = "another_policy".into();

    for wrong in [wrong_policy_key, wrong_execution_key, wrong_policy_id] {
        let result =
            DurableAuthority::open(authority_log_config(log_dir(&temp)), &wrong, None).map(|_| ());
        assert_eq!(result, Err(AuthorityLogError::TrustMismatch));
    }
}

#[test]
fn dal_05_decision_times_and_grant_identifier_are_reconstructed_not_assumed() {
    let temp = TempDir::new().unwrap();
    let (authority, grant_id) = with_grant(&temp);
    let request = request();

    // The identifier is the runtime's own derivation over the recorded inputs.
    assert_eq!(
        grant_id,
        derive_grant_id(&request, &approval(&request), T_ADMIT)
    );
    let before = authority.registry().grant(&grant_id).unwrap().clone();
    assert_eq!(before.issued_at, T_ADMIT);
    drop(authority);

    let authority = reopen(&temp);
    let after = authority.registry().grant(&grant_id).expect("same id");
    assert_eq!(after, &before);
}

// --- Duplicate admission and registration -------------------------------------

#[test]
fn dal_06_duplicate_admission_is_rejected_before_and_after_restart() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let request = request();
    let head = authority.head();

    // Same request and evaluation at the same time, and at a later time.
    for time in [T_ADMIT, "2026-08-20T10:00:01Z"] {
        assert_eq!(
            authority.admit(&request, &approval(&request), &clock(time)),
            Err(AuthorityLogError::Authorization(
                ExecutionAuthorizationError::DuplicateAdmission
            ))
        );
    }
    assert_eq!(authority.head(), head);

    authority.revoke(&grant_id, &clock(T_ATTEMPT)).unwrap();
    drop(authority);
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.admit(&request, &approval(&request), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::DuplicateAdmission
        ))
    );
    assert_eq!(
        authority.registry().grant(&grant_id).unwrap().status,
        GrantStatus::Revoked
    );
}

#[test]
fn dal_07_refused_duplicates_write_nothing_and_do_not_block_the_store() {
    // Duplicate refusal now comes from the registry itself (see the
    // agent_beta_018_f tests); the wrapper only has to stay consistent.
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let request = request();
    let head = authority.head();

    assert!(authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .is_err());
    assert!(authority
        .register_identity(identity(), &clock(T_ADMIT))
        .is_err());
    assert_eq!(authority.head(), head);
    assert!(!authority.is_poisoned());
    assert!(authority
        .authorize(&attempt(&grant_id), &clock(T_ATTEMPT))
        .is_ok());
}

#[test]
fn dal_08_duplicate_identity_registration_cannot_reactivate_a_suspended_identity() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    authority
        .set_identity_status(
            &identity().identity_id,
            AgentIdentityStatus::Suspended,
            &clock(T_ATTEMPT),
        )
        .unwrap();

    assert_eq!(
        authority.register_identity(identity(), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::Identity(IdentityValidationError::AlreadyRegistered)
        ))
    );
    drop(authority);
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.register_identity(identity(), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::Identity(IdentityValidationError::AlreadyRegistered)
        ))
    );
    assert_eq!(
        authority.authorize(&attempt(&grant_id), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::IdentityInactive
        ))
    );
}

// --- Receipts -------------------------------------------------------------------

#[test]
fn dal_09_receipts_are_validated_against_their_authorization() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let attempt = attempt(&grant_id);

    // No authorization yet.
    let premature = receipt(&AuthorizedExecution {
        attempt_id: attempt.attempt_id.clone(),
        grant_id: grant_id.clone(),
        executor_identity: attempt.executor_identity.clone(),
    });
    assert_eq!(
        authority.record_receipt(&premature, &clock(T_ATTEMPT)),
        Err(AuthorityLogError::ReceiptWithoutAuthorization)
    );

    let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    let head = authority.head();

    let mut tampered = receipt(&token);
    tampered.output_digest = "tampered".into();
    assert_eq!(
        authority.record_receipt(&tampered, &clock(T_RECEIPT)),
        Err(AuthorityLogError::InvalidReceipt)
    );

    let foreign_signer = ExecutionAuthority::from_seed([80u8; 32]).issue_receipt(
        &token,
        ExecutionResult::Success,
        "output",
        T_RECEIPT,
    );
    assert_eq!(
        authority.record_receipt(&foreign_signer, &clock(T_RECEIPT)),
        Err(AuthorityLogError::InvalidReceipt)
    );

    // Correctly signed, but bound to a different grant than the one consumed.
    let other_grant = receipt(&AuthorizedExecution {
        attempt_id: token.attempt_id.clone(),
        grant_id: GrantId("f".repeat(64)),
        executor_identity: token.executor_identity.clone(),
    });
    assert_eq!(
        authority.record_receipt(&other_grant, &clock(T_RECEIPT)),
        Err(AuthorityLogError::ReceiptBindingMismatch)
    );

    // Correctly signed, but claims completion before the authorization time.
    let too_early = execution_authority().issue_receipt(
        &token,
        ExecutionResult::Success,
        "output",
        "2026-08-20T10:14:59Z",
    );
    assert_eq!(
        authority.record_receipt(&too_early, &clock(T_RECEIPT)),
        Err(AuthorityLogError::ReceiptPredatesAuthorization)
    );

    assert_eq!(authority.head(), head, "rejected receipts write nothing");
    assert_eq!(
        authority.attempt_outcome(&attempt.attempt_id),
        Some(&AttemptOutcome::InFlight)
    );

    authority
        .record_receipt(&receipt(&token), &clock(T_RECEIPT))
        .unwrap();
    assert_eq!(
        authority.record_receipt(&receipt(&token), &clock(T_RECEIPT)),
        Err(AuthorityLogError::DuplicateReceipt)
    );
}

// --- Failed and uncertain saves --------------------------------------------------

#[test]
fn dal_10_failed_save_blocks_all_further_authority_until_reopened() {
    let temp = TempDir::new().unwrap();
    let (authority, grant_id) = with_grant(&temp);
    let attempt = attempt(&grant_id);
    let mut action = SimulatedAction::default();
    let mut authority = faulty(authority, Fault::BeforeWrite);

    let result = authority.authorize(&attempt, &clock(T_ATTEMPT));
    assert_eq!(
        result.as_ref().err(),
        Some(&AuthorityLogError::DurabilityUncertain)
    );
    if let Ok(token) = &result {
        action.run(token);
    }
    assert!(authority.is_poisoned());

    // Even with the journal healthy again, nothing is authorized or changed.
    authority.journal_mut().fault = Fault::None;
    let other = request_for("repo/src/other.rs");
    assert_eq!(
        authority.authorize(&attempt, &clock(T_ATTEMPT)).err(),
        Some(AuthorityLogError::Poisoned)
    );
    assert_eq!(
        authority.admit(&other, &approval(&other), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Poisoned)
    );
    assert_eq!(
        authority.revoke(&grant_id, &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Poisoned)
    );
    assert_eq!(action.count(&attempt), 0);
    drop(authority);

    // Nothing was written, so after reconstruction the grant is usable exactly once.
    let mut authority = reopen(&temp);
    assert!(authority.unknown_outcomes().is_empty());
    let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    action.run(&token);
    assert!(consumed(
        authority
            .authorize(&attempt, &clock(T_ATTEMPT))
            .unwrap_err()
    ));
    assert_eq!(action.count(&attempt), 1);
}

#[test]
fn dal_11_ambiguous_save_blocks_authority_and_reopens_as_unknown_outcome() {
    let temp = TempDir::new().unwrap();
    let (authority, grant_id) = with_grant(&temp);
    let attempt = attempt(&grant_id);
    let mut action = SimulatedAction::default();
    let mut authority = faulty(authority, Fault::AfterWrite);

    let result = authority.authorize(&attempt, &clock(T_ATTEMPT));
    assert_eq!(
        result.as_ref().err(),
        Some(&AuthorityLogError::DurabilityUncertain)
    );
    if let Ok(token) = &result {
        action.run(token);
    }
    assert!(authority.is_poisoned());
    authority.journal_mut().fault = Fault::None;
    assert_eq!(
        authority.authorize(&attempt, &clock(T_ATTEMPT)).err(),
        Some(AuthorityLogError::Poisoned)
    );
    drop(authority);

    // The record did reach disk. The grant is consumed and the result unknown.
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.unknown_outcomes(),
        vec![attempt.attempt_id.clone()]
    );
    assert!(consumed(
        authority
            .authorize(&attempt, &clock(T_ATTEMPT))
            .unwrap_err()
    ));
    assert_eq!(action.count(&attempt), 0);
}

#[test]
fn dal_12_oversized_record_is_refused_without_changing_or_poisoning_state() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let head = authority.head();

    let huge = request_for(&"x".repeat(20 * 1024));
    assert_eq!(
        authority.admit(&huge, &approval(&huge), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::RecordTooLarge)
    );
    assert!(!authority.is_poisoned());
    assert_eq!(authority.head(), head);
    assert!(authority
        .authorize(&attempt(&grant_id), &clock(T_ATTEMPT))
        .is_ok());
}

#[test]
fn dal_13_refused_operations_and_clock_regression_write_nothing() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let head = authority.head();

    let mut out_of_scope = attempt(&grant_id);
    out_of_scope.target_resource = "repo/src/secret.rs".into();
    out_of_scope.attempt_id = derive_attempt_id(
        &out_of_scope.grant_reference,
        &out_of_scope.executor_identity,
        &out_of_scope.operation_type,
        &out_of_scope.target_resource,
        &out_of_scope.target_reference,
        &out_of_scope.created_at,
    );
    assert_eq!(
        authority.authorize(&out_of_scope, &clock(T_ATTEMPT)).err(),
        Some(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::ScopeMismatch
        ))
    );
    // A decision time earlier than the previous recorded decision is refused.
    assert_eq!(
        authority.revoke(&grant_id, &clock(T_REGISTER)),
        Err(AuthorityLogError::ClockRegression)
    );
    assert_eq!(authority.head(), head);
    assert!(!authority.is_poisoned());
    drop(authority);

    let mut authority = reopen(&temp);
    assert_eq!(authority.head(), head);
    assert!(authority
        .authorize(&attempt(&grant_id), &clock(T_ATTEMPT))
        .is_ok());
}

// --- Interrupted saves (ledger commit stages) -------------------------------------

fn crash_during_authorize(
    temp: &TempDir,
    crash_at: AppendCommitStage,
) -> (ExecutionAttempt, SimulatedAction) {
    let (mut authority, grant_id) = with_grant(temp);
    let attempt = attempt(&grant_id);
    let mut action = SimulatedAction::default();
    authority
        .journal_mut()
        .set_stage_observer(Box::new(move |stage| {
            if stage == crash_at {
                panic!("simulated crash at {stage:?}");
            }
        }));

    let crashed = catch_unwind(AssertUnwindSafe(|| {
        let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
        action.run(&token);
    }));
    assert!(
        crashed.is_err(),
        "the simulated crash must interrupt the save"
    );
    // The process is gone: its memory is discarded without any cleanup.
    std::mem::forget(authority);
    (attempt, action)
}

#[test]
fn dal_14_crash_before_publication_leaves_an_uncommitted_tail_that_is_ignored() {
    let temp = TempDir::new().unwrap();
    let (attempt, mut action) = crash_during_authorize(&temp, AppendCommitStage::PendingSynced);
    assert_eq!(action.count(&attempt), 0);
    assert_eq!(
        pending_files(&log_dir(&temp)),
        1,
        "the interrupted save left only a non-canonical pending file"
    );

    // The change was never acknowledged, so the grant is still usable once.
    let mut authority = reopen(&temp);
    assert!(authority.unknown_outcomes().is_empty());
    let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    action.run(&token);
    assert_eq!(action.count(&attempt), 1);
}

#[test]
fn dal_15_crash_after_publication_reopens_consumed_with_unknown_outcome() {
    let temp = TempDir::new().unwrap();
    let (attempt, action) = crash_during_authorize(&temp, AppendCommitStage::Published);
    assert_eq!(action.count(&attempt), 0);

    let mut authority = reopen(&temp);
    assert_eq!(
        authority.unknown_outcomes(),
        vec![attempt.attempt_id.clone()]
    );
    assert!(consumed(
        authority
            .authorize(&attempt, &clock(T_ATTEMPT))
            .unwrap_err()
    ));
    assert_eq!(action.count(&attempt), 0, "no automatic retry");
}

// --- Uncertain outcome ---------------------------------------------------------------

/// Authorizes durably, optionally runs the simulated action, then "crashes"
/// before any receipt is recorded.
fn crash_after_authorization(
    temp: &TempDir,
    run_action: bool,
) -> (ExecutionAttempt, u32, HeadAnchor) {
    let (mut authority, grant_id) = with_grant(temp);
    let attempt = attempt(&grant_id);
    let mut action = SimulatedAction::default();
    let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    if run_action {
        action.run(&token);
    }
    let head = authority.head();
    std::mem::forget(authority);
    let runs = action.count(&attempt);
    (attempt, runs, head)
}

#[test]
fn dal_16_unknown_outcome_is_never_retried_released_or_silently_resolved() {
    for run_action in [false, true] {
        let temp = TempDir::new().unwrap();
        let (attempt, runs_before, _) = crash_after_authorization(&temp, run_action);
        assert_eq!(runs_before, u32::from(run_action));

        let mut authority = reopen(&temp);
        assert_eq!(
            authority.attempt_outcome(&attempt.attempt_id),
            Some(&AttemptOutcome::Unknown)
        );
        assert!(consumed(
            authority.authorize(&attempt, &clock(T_LATER)).unwrap_err()
        ));
        // A receipt arriving after the restart is not accepted as proof here.
        let late = receipt(&AuthorizedExecution {
            attempt_id: attempt.attempt_id.clone(),
            grant_id: attempt.grant_reference.clone(),
            executor_identity: attempt.executor_identity.clone(),
        });
        assert_eq!(
            authority.record_receipt(&late, &clock(T_LATER)),
            Err(AuthorityLogError::OutcomeUnknown)
        );
        drop(authority);

        // Still unknown after a second restart.
        let authority = reopen(&temp);
        assert_eq!(
            authority.unknown_outcomes(),
            vec![attempt.attempt_id.clone()]
        );
    }
}

#[test]
fn dal_17_log_cannot_distinguish_action_ran_from_action_did_not_run() {
    let not_run = TempDir::new().unwrap();
    let ran = TempDir::new().unwrap();
    let (_, runs_a, head_a) = crash_after_authorization(&not_run, false);
    let (_, runs_b, head_b) = crash_after_authorization(&ran, true);

    assert_eq!((runs_a, runs_b), (0, 1));
    assert_eq!(
        head_a, head_b,
        "the two logs are byte-for-byte equivalent, so only reconciliation can decide"
    );
}

// --- Corruption versus an interrupted tail --------------------------------------------

/// Three records: genesis (0), identity (1), grant (2); then consumption (3).
fn consumed_log(temp: &TempDir) -> (ExecutionAttempt, HeadAnchor) {
    let (mut authority, grant_id) = with_grant(temp);
    let attempt = attempt(&grant_id);
    authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    let head = authority.head();
    assert_eq!(head.records, 4);
    (attempt, head)
}

#[test]
fn dal_18_damaged_newest_record_fails_closed_and_is_not_treated_as_a_torn_tail() {
    // Bit flip in the newest canonical record.
    let temp = TempDir::new().unwrap();
    consumed_log(&temp);
    let newest = record_path(&temp, 3);
    let mut bytes = std::fs::read(&newest).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0x01;
    std::fs::write(&newest, &bytes).unwrap();
    assert!(matches!(
        try_reopen(&temp).map(|_| ()),
        Err(AuthorityLogError::Ledger(_))
    ));

    // Newest canonical record cut short.
    let temp = TempDir::new().unwrap();
    consumed_log(&temp);
    let newest = record_path(&temp, 3);
    let bytes = std::fs::read(&newest).unwrap();
    std::fs::write(&newest, &bytes[..bytes.len() / 2]).unwrap();
    assert!(matches!(
        try_reopen(&temp).map(|_| ()),
        Err(AuthorityLogError::Ledger(_))
    ));
}

#[test]
fn dal_19_missing_middle_record_fails_closed() {
    let temp = TempDir::new().unwrap();
    consumed_log(&temp);
    std::fs::remove_file(record_path(&temp, 2)).unwrap();
    assert_eq!(
        try_reopen(&temp).map(|_| ()),
        Err(AuthorityLogError::Ledger(LedgerError::LsnSequenceGap))
    );
}

#[test]
fn dal_20_removed_newest_record_is_detected_only_with_an_anchor() {
    let temp = TempDir::new().unwrap();
    let (attempt, head) = consumed_log(&temp);
    std::fs::remove_file(record_path(&temp, 3)).unwrap();

    // With the pre-restart head, the missing acknowledged change is detected.
    let anchored =
        DurableAuthority::open(authority_log_config(log_dir(&temp)), &trust(), Some(&head))
            .map(|_| ());
    assert_eq!(anchored, Err(AuthorityLogError::AnchorMismatch));

    // KNOWN LIMITATION: without an anchor the log simply looks shorter, and the
    // consumed grant appears unconsumed. The log alone cannot establish this.
    let mut unanchored = reopen(&temp);
    assert!(unanchored.authorize(&attempt, &clock(T_ATTEMPT)).is_ok());
}

#[test]
fn dal_21_anchor_accepts_the_same_or_a_longer_log_and_rejects_a_different_history() {
    let temp = TempDir::new().unwrap();
    let (authority, grant_id) = with_grant(&temp);
    let early = authority.head();
    drop(authority);

    let config = || authority_log_config(log_dir(&temp));
    let mut authority = DurableAuthority::open(config(), &trust(), Some(&early)).unwrap();
    authority.revoke(&grant_id, &clock(T_ATTEMPT)).unwrap();
    drop(authority);
    assert!(DurableAuthority::open(config(), &trust(), Some(&early)).is_ok());

    let mut different = early.clone();
    different.digest[0] ^= 0x01;
    assert_eq!(
        DurableAuthority::open(config(), &trust(), Some(&different)).map(|_| ()),
        Err(AuthorityLogError::AnchorMismatch)
    );
}

#[test]
fn dal_22_record_substituted_from_another_history_breaks_the_chain() {
    // Two logs with the same shape but a different grant at record 2.
    let ours = TempDir::new().unwrap();
    consumed_log(&ours);

    let theirs = TempDir::new().unwrap();
    let mut other = create(&theirs);
    other
        .register_identity(identity(), &clock(T_REGISTER))
        .unwrap();
    let request = request_for("repo/src/other.rs");
    other
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .unwrap();
    other
        .set_identity_status(
            &identity().identity_id,
            AgentIdentityStatus::Suspended,
            &clock(T_ATTEMPT),
        )
        .unwrap();
    drop(other);

    // Their record 3 has a valid checksum and the right sequence number.
    std::fs::copy(record_path(&theirs, 3), record_path(&ours, 3)).unwrap();
    assert_eq!(
        try_reopen(&ours).map(|_| ()),
        Err(AuthorityLogError::ChainMismatch)
    );
}

#[test]
fn dal_23_foreign_or_undecodable_records_fail_closed() {
    // A record of another event type in the same directory.
    let temp = TempDir::new().unwrap();
    let (authority, _) = with_grant(&temp);
    drop(authority);
    assert_ne!(AUTHORITY_LOG_EVENT_TYPE, EventType::RegistryMutation);
    let mut engine = LedgerAppendEngine::bootstrap(authority_log_config(log_dir(&temp))).unwrap();
    engine
        .append(EventType::RegistryMutation, b"not an authority record")
        .unwrap();
    assert_eq!(
        try_reopen(&temp).map(|_| ()),
        Err(AuthorityLogError::WrongEventType)
    );

    // The right event type, but not an authority record.
    let temp = TempDir::new().unwrap();
    let (authority, _) = with_grant(&temp);
    drop(authority);
    let mut engine = LedgerAppendEngine::bootstrap(authority_log_config(log_dir(&temp))).unwrap();
    engine
        .append(AUTHORITY_LOG_EVENT_TYPE, b"not an authority record")
        .unwrap();
    assert_eq!(
        try_reopen(&temp).map(|_| ()),
        Err(AuthorityLogError::Malformed)
    );
}

#[test]
fn dal_24_open_and_create_refuse_the_wrong_starting_state() {
    let temp = TempDir::new().unwrap();
    // An empty directory is never silently treated as a fresh, clean history.
    assert_eq!(
        try_reopen(&temp).map(|_| ()),
        Err(AuthorityLogError::EmptyLog)
    );

    let (authority, _) = with_grant(&temp);
    drop(authority);
    assert_eq!(
        DurableAuthority::create(authority_log_config(log_dir(&temp)), trust()).map(|_| ()),
        Err(AuthorityLogError::LogNotEmpty)
    );
}

// --- More failed and uncertain saves ---------------------------------------------

#[test]
fn dal_25_revocation_save_failure_and_ambiguity_resolve_from_the_log() {
    // Failed before writing: the revocation was never acknowledged and is absent.
    let temp = TempDir::new().unwrap();
    let (authority, grant_id) = with_grant(&temp);
    let mut authority = faulty(authority, Fault::BeforeWrite);
    assert_eq!(
        authority.revoke(&grant_id, &clock(T_ATTEMPT)),
        Err(AuthorityLogError::DurabilityUncertain)
    );
    // Memory says revoked, the log does not: nothing more is decided here.
    authority.journal_mut().fault = Fault::None;
    assert_eq!(
        authority
            .authorize(&attempt(&grant_id), &clock(T_ATTEMPT))
            .err(),
        Some(AuthorityLogError::Poisoned)
    );
    drop(authority);
    assert_eq!(
        reopen(&temp).registry().grant(&grant_id).unwrap().status,
        GrantStatus::Active
    );

    // Written, then reported as uncertain: the revocation holds after reopening.
    let temp = TempDir::new().unwrap();
    let (authority, grant_id) = with_grant(&temp);
    let mut authority = faulty(authority, Fault::AfterWrite);
    assert_eq!(
        authority.revoke(&grant_id, &clock(T_ATTEMPT)),
        Err(AuthorityLogError::DurabilityUncertain)
    );
    drop(authority);
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.authorize(&attempt(&grant_id), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::InactiveGrant
        ))
    );
}

#[test]
fn dal_26_suspension_save_failure_and_ambiguity_resolve_from_the_log() {
    let id = identity().identity_id;
    for (fault, expect_active) in [(Fault::BeforeWrite, true), (Fault::AfterWrite, false)] {
        let temp = TempDir::new().unwrap();
        let (authority, grant_id) = with_grant(&temp);
        let mut authority = faulty(authority, fault);
        assert_eq!(
            authority.set_identity_status(&id, AgentIdentityStatus::Suspended, &clock(T_ATTEMPT)),
            Err(AuthorityLogError::DurabilityUncertain)
        );
        assert!(authority.is_poisoned());
        drop(authority);

        let mut authority = reopen(&temp);
        let result = authority.authorize(&attempt(&grant_id), &clock(T_ATTEMPT));
        if expect_active {
            assert!(result.is_ok());
        } else {
            assert_eq!(
                result,
                Err(AuthorityLogError::Authorization(
                    ExecutionAuthorizationError::IdentityInactive
                ))
            );
        }
    }
}

#[test]
fn dal_27_receipt_save_failure_leaves_a_consumed_permission_with_unknown_outcome() {
    for (fault, expected) in [
        (Fault::BeforeWrite, AttemptOutcome::Unknown),
        (
            Fault::AfterWrite,
            AttemptOutcome::Confirmed(ExecutionResult::Success),
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let (mut authority, grant_id) = with_grant(&temp);
        let attempt = attempt(&grant_id);
        let mut action = SimulatedAction::default();
        let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
        action.run(&token);

        let mut authority = faulty(authority, fault);
        assert_eq!(
            authority.record_receipt(&receipt(&token), &clock(T_RECEIPT)),
            Err(AuthorityLogError::DurabilityUncertain)
        );
        assert!(authority.is_poisoned());
        drop(authority);

        let mut authority = reopen(&temp);
        assert_eq!(
            authority.attempt_outcome(&attempt.attempt_id),
            Some(&expected)
        );
        // Either way the permission stays consumed and the action is not repeated.
        assert!(consumed(
            authority.authorize(&attempt, &clock(T_LATER)).unwrap_err()
        ));
        assert_eq!(action.count(&attempt), 1);
    }
}

// --- More receipt validation -------------------------------------------------------

#[test]
fn dal_28_receipt_field_tampering_and_foreign_executor_are_rejected() {
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let token = authority
        .authorize(&attempt(&grant_id), &clock(T_ATTEMPT))
        .unwrap();
    let head = authority.head();

    // Only the stored receipt digest is changed (the case fixed by PR 263).
    let mut digest_only = receipt(&token);
    digest_only.receipt_digest = "0".repeat(64);
    // An unsupported schema version.
    let mut wrong_schema = receipt(&token);
    wrong_schema.schema_version = "EXECUTION_RECEIPT-v2".into();
    // The result flipped after signing.
    let mut flipped = receipt(&token);
    flipped.result = ExecutionResult::Failure;
    for invalid in [digest_only, wrong_schema, flipped] {
        assert_eq!(
            authority.record_receipt(&invalid, &clock(T_RECEIPT)),
            Err(AuthorityLogError::InvalidReceipt)
        );
    }

    // Correctly signed for this attempt and grant, but naming another executor.
    let other_executor = receipt(&AuthorizedExecution {
        attempt_id: token.attempt_id.clone(),
        grant_id: token.grant_id.clone(),
        executor_identity: AgentIdentityId("another-executor".into()),
    });
    assert_eq!(
        authority.record_receipt(&other_executor, &clock(T_RECEIPT)),
        Err(AuthorityLogError::ReceiptBindingMismatch)
    );
    assert_eq!(authority.head(), head);
}

#[test]
fn dal_29_a_result_is_still_recorded_after_later_revocation_or_suspension() {
    // Intended boundary: a receipt records an action that was authorized before the
    // revocation or suspension. Refusing it would turn a known result into an
    // unknown one. It grants nothing new.
    let temp = TempDir::new().unwrap();
    let (mut authority, grant_id) = with_grant(&temp);
    let attempt = attempt(&grant_id);
    let token = authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    authority.revoke(&grant_id, &clock(T_ATTEMPT)).unwrap();
    authority
        .set_identity_status(
            &identity().identity_id,
            AgentIdentityStatus::Suspended,
            &clock(T_ATTEMPT),
        )
        .unwrap();

    authority
        .record_receipt(&receipt(&token), &clock(T_RECEIPT))
        .unwrap();
    drop(authority);
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.attempt_outcome(&attempt.attempt_id),
        Some(&AttemptOutcome::Confirmed(ExecutionResult::Success))
    );
    assert_eq!(
        authority.authorize(&attempt_at(&grant_id, T_LATER), &clock(T_LATER)),
        Err(AuthorityLogError::Authorization(
            ExecutionAuthorizationError::IdentityInactive
        ))
    );
}

// --- Separately retained checkpoint ---------------------------------------------------

fn checkpoint_path(temp: &TempDir) -> PathBuf {
    // Deliberately outside the log directory.
    let directory = temp.path().join("checkpoint");
    std::fs::create_dir_all(&directory).unwrap();
    directory.join("authority.checkpoint")
}

fn file_checkpoint(temp: &TempDir) -> Box<FileCheckpoint> {
    Box::new(FileCheckpoint::new(checkpoint_path(temp)))
}

fn reopen_checkpointed(temp: &TempDir) -> Result<Authority, AuthorityLogError> {
    DurableAuthority::open_with_checkpoint(
        authority_log_config(log_dir(temp)),
        &trust(),
        file_checkpoint(temp),
    )
}

/// Checkpointed store with one identity, one grant and that grant consumed.
/// Records: genesis (0), identity (1), grant (2), consumption (3).
fn checkpointed_consumed(temp: &TempDir) -> (ExecutionAttempt, HeadAnchor) {
    let mut authority = DurableAuthority::create_with_checkpoint(
        authority_log_config(log_dir(temp)),
        trust(),
        file_checkpoint(temp),
    )
    .unwrap();
    authority
        .register_identity(identity(), &clock(T_REGISTER))
        .unwrap();
    let request = request();
    let grant_id = authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .unwrap();
    let attempt = attempt(&grant_id);
    authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    let head = authority.head();
    (attempt, head)
}

#[test]
fn dal_31_checkpoint_tracks_every_acknowledged_record_and_detects_loss_of_the_newest() {
    let temp = TempDir::new().unwrap();
    let (attempt, head) = checkpointed_consumed(&temp);
    assert_eq!(file_checkpoint(&temp).load(), Ok(Some(head.clone())));

    // Intact: opens, and the consumed permission is still consumed.
    let mut authority = reopen_checkpointed(&temp).unwrap();
    assert!(consumed(
        authority.authorize(&attempt, &clock(T_LATER)).unwrap_err()
    ));
    drop(authority);

    // The newest acknowledged record (the consumption) is lost.
    std::fs::remove_file(record_path(&temp, 3)).unwrap();
    assert_eq!(
        reopen_checkpointed(&temp).map(|_| ()),
        Err(AuthorityLogError::AnchorMismatch)
    );
}

#[test]
fn dal_32_missing_or_damaged_checkpoint_fails_closed() {
    let temp = TempDir::new().unwrap();
    checkpointed_consumed(&temp);
    let path = checkpoint_path(&temp);
    let good = std::fs::read(&path).unwrap();

    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        reopen_checkpointed(&temp).map(|_| ()),
        Err(AuthorityLogError::CheckpointMissing)
    );

    let mut flipped = good.clone();
    flipped[20] ^= 0x01;
    for damaged in [flipped, good[..40].to_vec(), Vec::new()] {
        std::fs::write(&path, &damaged).unwrap();
        assert_eq!(
            reopen_checkpointed(&temp).map(|_| ()),
            Err(AuthorityLogError::CheckpointCorrupt)
        );
    }

    std::fs::write(&path, &good).unwrap();
    assert!(reopen_checkpointed(&temp).is_ok());
}

#[test]
fn dal_33_create_refuses_an_existing_checkpoint_and_a_foreign_one_is_rejected() {
    let temp = TempDir::new().unwrap();
    checkpointed_consumed(&temp);

    // A new log must not adopt a checkpoint that already exists.
    let fresh = TempDir::new().unwrap();
    let result = DurableAuthority::create_with_checkpoint(
        authority_log_config(log_dir(&fresh)),
        trust(),
        file_checkpoint(&temp),
    )
    .map(|_| ());
    assert_eq!(result, Err(AuthorityLogError::CheckpointAlreadyExists));

    // A checkpoint from a different history does not match this log.
    let other = TempDir::new().unwrap();
    let (authority, _) = with_grant(&other);
    drop(authority);
    let result = DurableAuthority::open_with_checkpoint(
        authority_log_config(log_dir(&other)),
        &trust(),
        file_checkpoint(&temp),
    )
    .map(|_| ());
    assert_eq!(result, Err(AuthorityLogError::AnchorMismatch));
}

/// Checkpoint store whose saves can be made to fail.
struct FaultyCheckpoint {
    inner: FileCheckpoint,
    fail: Rc<Cell<bool>>,
}

impl CheckpointStore for FaultyCheckpoint {
    fn load(&self) -> Result<Option<HeadAnchor>, AuthorityLogError> {
        self.inner.load()
    }
    fn save(&mut self, head: &HeadAnchor) -> Result<(), AuthorityLogError> {
        if self.fail.get() {
            return Err(AuthorityLogError::CheckpointUnavailable);
        }
        self.inner.save(head)
    }
}

#[test]
fn dal_34_checkpoint_save_failure_blocks_authority_and_reopens_as_unknown_outcome() {
    let temp = TempDir::new().unwrap();
    let fail = Rc::new(Cell::new(false));
    let mut authority = DurableAuthority::create_with_checkpoint(
        authority_log_config(log_dir(&temp)),
        trust(),
        Box::new(FaultyCheckpoint {
            inner: FileCheckpoint::new(checkpoint_path(&temp)),
            fail: Rc::clone(&fail),
        }),
    )
    .unwrap();
    authority
        .register_identity(identity(), &clock(T_REGISTER))
        .unwrap();
    let request = request();
    let grant_id = authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .unwrap();
    let attempt = attempt(&grant_id);
    let mut action = SimulatedAction::default();
    let before = authority.head();

    fail.set(true);
    let result = authority.authorize(&attempt, &clock(T_ATTEMPT));
    assert_eq!(
        result.as_ref().err(),
        Some(&AuthorityLogError::DurabilityUncertain)
    );
    if let Ok(token) = &result {
        action.run(token);
    }
    fail.set(false);
    assert_eq!(
        authority.revoke(&grant_id, &clock(T_ATTEMPT)),
        Err(AuthorityLogError::Poisoned)
    );
    drop(authority);

    // The checkpoint is one record behind the log. That record was durable but
    // never acknowledged, so it is kept, and the outcome is unknown.
    assert_eq!(file_checkpoint(&temp).load(), Ok(Some(before)));
    let mut authority = reopen_checkpointed(&temp).unwrap();
    assert_eq!(
        authority.unknown_outcomes(),
        vec![attempt.attempt_id.clone()]
    );
    assert!(consumed(
        authority.authorize(&attempt, &clock(T_LATER)).unwrap_err()
    ));
    assert_eq!(action.count(&attempt), 0);
    // Opening brought the checkpoint up to the log's head.
    assert_eq!(file_checkpoint(&temp).load(), Ok(Some(authority.head())));
}

#[test]
fn dal_35_limitation_an_outdated_or_rewritten_checkpoint_hides_later_loss() {
    // KNOWN LIMITATION, asserted so that nobody assumes otherwise.
    let temp = TempDir::new().unwrap();
    let mut authority = DurableAuthority::create_with_checkpoint(
        authority_log_config(log_dir(&temp)),
        trust(),
        file_checkpoint(&temp),
    )
    .unwrap();
    authority
        .register_identity(identity(), &clock(T_REGISTER))
        .unwrap();
    let request = request();
    let grant_id = authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .unwrap();
    let older_checkpoint = std::fs::read(checkpoint_path(&temp)).unwrap();
    let attempt = attempt(&grant_id);
    authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    drop(authority);

    // An older checkpoint is put back (or, since it is unkeyed, simply rewritten)
    // and the consumption record is removed.
    std::fs::write(checkpoint_path(&temp), &older_checkpoint).unwrap();
    std::fs::remove_file(record_path(&temp, 3)).unwrap();

    let mut authority = reopen_checkpointed(&temp).expect("the loss is not detected");
    assert!(
        authority.authorize(&attempt, &clock(T_ATTEMPT)).is_ok(),
        "the consumed permission is usable again"
    );
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
    }
}

#[test]
fn dal_36_limitation_rolling_back_log_and_checkpoint_together_is_not_detected() {
    // KNOWN LIMITATION: a consistent earlier copy of both is indistinguishable from
    // the truth. Only something outside both directories could tell.
    let temp = TempDir::new().unwrap();
    let mut authority = DurableAuthority::create_with_checkpoint(
        authority_log_config(log_dir(&temp)),
        trust(),
        file_checkpoint(&temp),
    )
    .unwrap();
    authority
        .register_identity(identity(), &clock(T_REGISTER))
        .unwrap();
    let request = request();
    let grant_id = authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .unwrap();

    let backup = TempDir::new().unwrap();
    copy_dir(&log_dir(&temp), &backup.path().join("log"));
    copy_dir(
        checkpoint_path(&temp).parent().unwrap(),
        &backup.path().join("checkpoint"),
    );

    let attempt = attempt(&grant_id);
    authority.authorize(&attempt, &clock(T_ATTEMPT)).unwrap();
    drop(authority);

    std::fs::remove_dir_all(log_dir(&temp)).unwrap();
    std::fs::remove_dir_all(checkpoint_path(&temp).parent().unwrap()).unwrap();
    copy_dir(&backup.path().join("log"), &log_dir(&temp));
    copy_dir(
        &backup.path().join("checkpoint"),
        checkpoint_path(&temp).parent().unwrap(),
    );

    let mut authority = reopen_checkpointed(&temp).expect("the rollback is not detected");
    assert!(authority.authorize(&attempt, &clock(T_ATTEMPT)).is_ok());
}

// --- Real process interruption ------------------------------------------------------
//
// These tests start a second copy of this test binary and let it exit abruptly,
// without unwinding or cleanup, at a chosen point. That is a real process death
// with the operating system closing its files. It is not power loss: data the OS
// had accepted is still written out. No power-loss durability is claimed.

const WORKER_DIR: &str = "DAL_WORKER_DIR";
const WORKER_CRASH: &str = "DAL_WORKER_CRASH";
const WORKER_EXIT: i32 = 86;

#[test]
fn dal_40_process_crash_worker() {
    let (Ok(directory), Ok(crash)) = (std::env::var(WORKER_DIR), std::env::var(WORKER_CRASH))
    else {
        return; // Not running as a worker: nothing to do.
    };
    let config = authority_log_config(PathBuf::from(directory).join("authority-log"));
    let mut authority = DurableAuthority::create(config, trust()).unwrap();
    authority
        .register_identity(identity(), &clock(T_REGISTER))
        .unwrap();
    let request = request();
    let grant_id = authority
        .admit(&request, &approval(&request), &clock(T_ADMIT))
        .unwrap();

    let stage = match crash.as_str() {
        "pending" => Some(AppendCommitStage::PendingSynced),
        "published" => Some(AppendCommitStage::Published),
        "durable" => Some(AppendCommitStage::Durable),
        _ => None,
    };
    if let Some(stage) = stage {
        authority
            .journal_mut()
            .set_stage_observer(Box::new(move |reached| {
                if reached == stage {
                    std::process::exit(WORKER_EXIT);
                }
            }));
    }
    let _token = authority
        .authorize(&attempt(&grant_id), &clock(T_ATTEMPT))
        .unwrap();
    // "after-authorize": the permission is durably consumed, no receipt follows.
    std::process::exit(WORKER_EXIT);
}

fn run_worker(temp: &TempDir, crash: &str) {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["dal_40_process_crash_worker", "--exact", "--nocapture"])
        .env(WORKER_DIR, temp.path())
        .env(WORKER_CRASH, crash)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("start worker");
    assert_eq!(
        status.code(),
        Some(WORKER_EXIT),
        "worker must die at the crash point"
    );
}

fn worker_attempt(authority: &Authority) -> ExecutionAttempt {
    let request = request();
    let grant_id = derive_grant_id(&request, &approval(&request), T_ADMIT);
    assert!(authority.registry().grant(&grant_id).is_some());
    attempt(&grant_id)
}

#[test]
fn dal_41_process_death_before_publication_leaves_the_permission_unconsumed() {
    let temp = TempDir::new().unwrap();
    run_worker(&temp, "pending");
    assert_eq!(pending_files(&log_dir(&temp)), 1);

    let mut authority = reopen(&temp);
    // The dead process's pending file is removed by the ledger on open.
    assert_eq!(pending_files(&log_dir(&temp)), 0);
    assert!(authority.unknown_outcomes().is_empty());
    let attempt = worker_attempt(&authority);
    assert!(authority.authorize(&attempt, &clock(T_ATTEMPT)).is_ok());
}

#[test]
fn dal_42_process_death_after_publication_leaves_consumed_with_unknown_outcome() {
    for crash in ["published", "durable", "after-authorize"] {
        let temp = TempDir::new().unwrap();
        run_worker(&temp, crash);

        let mut authority = reopen(&temp);
        let attempt = worker_attempt(&authority);
        assert_eq!(
            authority.unknown_outcomes(),
            vec![attempt.attempt_id.clone()],
            "crash point {crash}"
        );
        assert!(consumed(
            authority.authorize(&attempt, &clock(T_LATER)).unwrap_err()
        ));
    }
}
