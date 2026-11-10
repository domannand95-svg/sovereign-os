//! EXPERIMENTAL restart-safety tests for the durable authority log.
//!
//! All actions are simulated: the "effect" is a counter that can only be advanced
//! with an authorization token. Nothing here touches real files or tools outside a
//! per-test temporary directory, and nothing here is the C01 milestone.

use beta001_harness::durable_authority::{
    authority_log_config, AttemptOutcome, AuthorityJournal, AuthorityLogError, DurableAuthority,
    HeadAnchor, LedgerJournal, TrustConfig, AUTHORITY_LOG_EVENT_TYPE,
};
use sovereign_agent_runtime::capability::{
    derive_grant_id, derive_request_id, CapabilityRequest, CapabilityScope, CapabilityType,
    EvaluationResult, GrantId, GrantStatus, PolicyAuthority, RequestId, RequestStatus,
    SignedEvaluation,
};
use sovereign_agent_runtime::execution::{
    derive_attempt_id, AttemptId, AuthorizedExecution, CapabilityRegistry, ExecutionAttempt,
    ExecutionAuthority, ExecutionAuthorizationError, ExecutionReceipt, ExecutionResult, FixedClock,
};
use sovereign_agent_runtime::identity::{
    derive_identity_id, AgentClass, AgentIdentity, AgentIdentityStatus, Digest, PolicyId,
    ReplayTimestamp, SchemaVersion,
};
use sovereign_audit::RecordId;
use sovereign_ledger::{AppendCommitStage, EventType, LedgerAppendEngine, LedgerError};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
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
            Err(AuthorityLogError::DuplicateAdmission)
        );
    }
    assert_eq!(authority.head(), head);

    authority.revoke(&grant_id, &clock(T_ATTEMPT)).unwrap();
    drop(authority);
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.admit(&request, &approval(&request), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::DuplicateAdmission)
    );
    assert_eq!(
        authority.registry().grant(&grant_id).unwrap().status,
        GrantStatus::Revoked
    );
}

#[test]
fn dal_07_characterization_raw_registry_allows_what_the_wrapper_refuses() {
    // Documents the behaviour of the unmodified in-memory registry that makes the
    // wrapper's duplicate checks necessary. This test does not endorse it.
    let request = request();
    let evaluation = approval(&request);
    let mut registry =
        CapabilityRegistry::new("policy_engine_v1", policy_authority().verifying_key());
    registry.register_identity(identity()).unwrap();

    let first = registry.admit(&request, &evaluation, T_ADMIT).unwrap();
    let second = registry
        .admit(&request, &evaluation, "2026-08-20T10:00:01Z")
        .unwrap();
    assert_ne!(first, second, "one approval yields two single-use grants");

    registry.revoke(&first).unwrap();
    let again = registry.admit(&request, &evaluation, T_ADMIT).unwrap();
    assert_eq!(again, first, "identifier is stable for identical inputs");
    assert_eq!(
        registry.grant(&first).unwrap().status,
        GrantStatus::Active,
        "re-admission silently reactivates a revoked grant"
    );

    let mut suspended = identity();
    suspended.status = AgentIdentityStatus::Suspended;
    registry.register_identity(suspended).unwrap();
    registry.register_identity(identity()).unwrap();
    assert!(
        registry
            .authorize_and_consume(&attempt(&second), &clock(T_ATTEMPT))
            .is_ok(),
        "re-registration silently reactivates a suspended identity"
    );
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
        Err(AuthorityLogError::DuplicateIdentity)
    );
    drop(authority);
    let mut authority = reopen(&temp);
    assert_eq!(
        authority.register_identity(identity(), &clock(T_ATTEMPT)),
        Err(AuthorityLogError::DuplicateIdentity)
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
