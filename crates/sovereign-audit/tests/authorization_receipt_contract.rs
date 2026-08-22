#[derive(Debug, PartialEq, Eq, Clone)]
struct Digest(String);

#[derive(Debug, PartialEq, Eq, Clone)]
struct Signature(String);

#[derive(Debug, PartialEq, Eq, Clone)]
struct Nonce(String);

#[derive(Debug, PartialEq, Eq, Clone)]
struct IssuerContext {
    issuer_reference: String,
    signing_key_reference: Digest,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum AdmissionOutcome {
    Permit,
    Deny,
    Quarantine,
    Escalate,
}

#[derive(Debug, Clone)]
struct AdmissionDecision {
    decision_reference: String,
    outcome: AdmissionOutcome,
    authorized_scope: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct AuthorizationReceipt {
    receipt_reference: Digest,
    subject_reference: String,
    intent_reference: Digest,
    admission_reference: Digest,
    authorized_scope: String,
    issued_at: u64,
    expires_at: u64,
    issuer_reference: String,
    nonce: Nonce,
    signature: Signature,
}

impl AuthorizationReceipt {
    const MAX_LIFETIME: u64 = 3600;

    fn generate(
        decision: &AdmissionDecision,
        subject: &str,
        intent_ref: Digest,
        issued_at: u64,
        expires_at: u64,
        issuer_context: &IssuerContext,
    ) -> Result<Self, &'static str> {

        if decision.outcome != AdmissionOutcome::Permit {
            return Err("Non permit cannot issue receipt");
        }

        if subject == issuer_context.issuer_reference {
            return Err("Subject cannot self issue");
        }

        if expires_at <= issued_at {
            return Err("Invalid lifetime");
        }

        if expires_at - issued_at > Self::MAX_LIFETIME {
            return Err("Lifetime exceeded");
        }

        Ok(Self {
            receipt_reference: Digest("receipt_test".to_string()),
            subject_reference: subject.to_string(),
            intent_reference: intent_ref,
            admission_reference: Digest(
                decision.decision_reference.clone()
            ),
            authorized_scope: decision.authorized_scope.clone(),
            issued_at,
            expires_at,
            issuer_reference: issuer_context.issuer_reference.clone(),
            nonce: Nonce("nonce_test".to_string()),
            signature: Signature("signature_test".to_string()),
        })
    }
}


#[test]
fn test_requester_cannot_self_issue_receipt() {
    let decision = AdmissionDecision {
        decision_reference: "adm_001".to_string(),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: "tenant_a".to_string(),
    };

    let issuer = IssuerContext {
        issuer_reference: "agent_42".to_string(),
        signing_key_reference: Digest("key".to_string()),
    };

    assert!(
        AuthorizationReceipt::generate(
            &decision,
            "agent_42",
            Digest("intent".to_string()),
            100,
            200,
            &issuer
        )
        .is_err()
    );
}


#[test]
fn test_receipt_preserves_governance_lineage() {
    let decision = AdmissionDecision {
        decision_reference: "adm_lineage".to_string(),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: "tenant_a".to_string(),
    };

    let issuer = IssuerContext {
        issuer_reference: "gov_node".to_string(),
        signing_key_reference: Digest("key".to_string()),
    };

    let receipt = AuthorizationReceipt::generate(
        &decision,
        "agent_42",
        Digest("intent_lineage".to_string()),
        100,
        200,
        &issuer,
    )
    .unwrap();

    assert_eq!(
        receipt.intent_reference,
        Digest("intent_lineage".to_string())
    );

    assert_eq!(
        receipt.admission_reference,
        Digest("adm_lineage".to_string())
    );
}


#[test]
fn test_receipt_scope_cannot_expand_beyond_admitted_scope() {
    let decision = AdmissionDecision {
        decision_reference: "adm_scope".to_string(),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: "tenant_a_read_only".to_string(),
    };

    let issuer = IssuerContext {
        issuer_reference: "gov_node".to_string(),
        signing_key_reference: Digest("key".to_string()),
    };

    let receipt = AuthorizationReceipt::generate(
        &decision,
        "agent_42",
        Digest("intent_scope".to_string()),
        100,
        200,
        &issuer,
    )
    .unwrap();

    assert_eq!(
        receipt.authorized_scope,
        "tenant_a_read_only"
    );

    assert_ne!(
        receipt.authorized_scope,
        "tenant_a_admin"
    );
}