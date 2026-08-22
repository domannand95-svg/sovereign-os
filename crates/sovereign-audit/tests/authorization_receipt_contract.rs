use sovereign_audit::authorization_receipt::{
    AuthorizationReceipt, AuthorizationReceiptIdentity,
    CanonicalAuthorizationReceiptIdentityPayloadV1, IssuerContext,
};
use sovereign_audit::governance_admission::{AdmissionDecision, AdmissionOutcome};

fn mock_permit() -> AdmissionDecision {
    AdmissionDecision {
        decision_reference: "adm_01".to_string(),
        intent_reference: "int_01".to_string(),
        evaluation_reference: "eval_01".to_string(),
        governance_context_reference: "gov_ctx_01".to_string(),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: "tenant_a".to_string(),
        reason_references: vec![],
    }
}

fn mock_issuer() -> IssuerContext {
    IssuerContext {
        issuer_reference: "gov_node_alpha".to_string(),
        signing_key_reference: "key_01".to_string(),
    }
}

#[test]
fn test_ar_001_permit_generates_receipt() {
    let receipt = AuthorizationReceipt::generate(
        &mock_permit(),
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        1000,
        2000,
        &mock_issuer(),
        "nonce_123",
    );

    assert!(receipt.is_ok());
}

#[test]
fn test_ar_002_deny_rejects_receipt() {
    let mut decision = mock_permit();
    decision.outcome = AdmissionOutcome::Deny;

    assert!(AuthorizationReceipt::generate(
        &decision,
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        1000,
        2000,
        &mock_issuer(),
        "nonce_123",
    )
    .is_err());
}

#[test]
fn test_ar_003_quarantine_rejects_receipt() {
    let mut decision = mock_permit();
    decision.outcome = AdmissionOutcome::Quarantine;

    assert!(AuthorizationReceipt::generate(
        &decision,
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        1000,
        2000,
        &mock_issuer(),
        "nonce_123",
    )
    .is_err());
}

#[test]
fn test_ar_004_invalid_expiry_rejected() {
    assert!(AuthorizationReceipt::generate(
        &mock_permit(),
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        2000,
        1000,
        &mock_issuer(),
        "nonce_123",
    )
    .is_err());
}

#[test]
fn test_ar_005_lifetime_overflow_rejected() {
    assert!(AuthorizationReceipt::generate(
        &mock_permit(),
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        1000,
        1000 + AuthorizationReceipt::MAX_LIFETIME + 1,
        &mock_issuer(),
        "nonce_123",
    )
    .is_err());
}

#[test]
fn test_ar_006_empty_issuer_rejected() {
    let issuer = IssuerContext {
        issuer_reference: " ".to_string(),
        signing_key_reference: "key".to_string(),
    };

    assert!(AuthorizationReceipt::generate(
        &mock_permit(),
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        1000,
        2000,
        &issuer,
        "nonce_123",
    )
    .is_err());
}

#[test]
fn test_ar_007_empty_nonce_rejected() {
    assert!(AuthorizationReceipt::generate(
        &mock_permit(),
        "agent_42",
        "int_01",
        "pol_01",
        "ctx_01",
        "READ",
        "data",
        1000,
        2000,
        &mock_issuer(),
        " ",
    )
    .is_err());
}

#[test]
fn test_ar_008_identity_changes_with_canonical_input() {
    let a = CanonicalAuthorizationReceiptIdentityPayloadV1 {
        admission_reference: "adm_01".to_string(),
        subject_reference: "agent_42".to_string(),
        issued_at: 1000,
        nonce: "nonce_a".to_string(),
    };

    let mut b = a.clone();
    b.nonce = "nonce_b".to_string();

    let first = AuthorizationReceiptIdentity::derive(&a);
    let second = AuthorizationReceiptIdentity::derive(&b);

    assert_ne!(first.receipt_id, second.receipt_id);
}

// --- BETA-024-E: RECEIPT CONTRACT EXPANSION ---

#[test]
fn test_ar_009_identity_mutates_on_admission_change() {
    let base_payload = CanonicalAuthorizationReceiptIdentityPayloadV1 {
        admission_reference: "adm_01".to_string(),
        subject_reference: "agent_42".to_string(),
        issued_at: 1000,
        nonce: "nonce_123".to_string(),
    };

    let mut mutated_payload = base_payload.clone();
    mutated_payload.admission_reference = "adm_02".to_string();

    let id_base = AuthorizationReceiptIdentity::derive(&base_payload);
    let id_mutated = AuthorizationReceiptIdentity::derive(&mutated_payload);

    assert_ne!(id_base.receipt_id, id_mutated.receipt_id);
}

#[test]
fn test_ar_010_identity_mutates_on_subject_change() {
    let base_payload = CanonicalAuthorizationReceiptIdentityPayloadV1 {
        admission_reference: "adm_01".to_string(),
        subject_reference: "agent_42".to_string(),
        issued_at: 1000,
        nonce: "nonce_123".to_string(),
    };

    let mut mutated_payload = base_payload.clone();
    mutated_payload.subject_reference = "agent_99".to_string();

    let id_base = AuthorizationReceiptIdentity::derive(&base_payload);
    let id_mutated = AuthorizationReceiptIdentity::derive(&mutated_payload);

    assert_ne!(id_base.receipt_id, id_mutated.receipt_id);
}

#[test]
fn test_ar_011_identity_mutates_on_issuance_time_change() {
    let base_payload = CanonicalAuthorizationReceiptIdentityPayloadV1 {
        admission_reference: "adm_01".to_string(),
        subject_reference: "agent_42".to_string(),
        issued_at: 1000,
        nonce: "nonce_123".to_string(),
    };

    let mut mutated_payload = base_payload.clone();
    mutated_payload.issued_at = 1001;

    let id_base = AuthorizationReceiptIdentity::derive(&base_payload);
    let id_mutated = AuthorizationReceiptIdentity::derive(&mutated_payload);

    assert_ne!(id_base.receipt_id, id_mutated.receipt_id);
}

#[test]
fn test_ar_012_receipt_is_passive_data_without_execution_surface() {
    fn assert_passive_data<T: Send + Sync + 'static + Clone + Eq>() {}

    assert_passive_data::<AuthorizationReceipt>();

    assert!(std::mem::size_of::<AuthorizationReceipt>() > 0);
}

#[test]
fn test_ar_013_identical_payloads_yield_identical_identity() {
    let first = CanonicalAuthorizationReceiptIdentityPayloadV1 {
        admission_reference: "adm_01".to_string(),
        subject_reference: "agent_42".to_string(),
        issued_at: 1000,
        nonce: "nonce_123".to_string(),
    };

    let second = first.clone();

    assert_eq!(
        AuthorizationReceiptIdentity::derive(&first).receipt_id,
        AuthorizationReceiptIdentity::derive(&second).receipt_id
    );
}