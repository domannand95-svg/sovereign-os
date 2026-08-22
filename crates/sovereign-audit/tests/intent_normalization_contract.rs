#[derive(Debug, PartialEq, Eq, Clone)]
struct AgentProposal {
    proposal_id: String,
    requested_operation: String,
    requested_scope: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum NormalizedOperation {
    DeleteRecord,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ScopeReference {
    SingleCustomer(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct NormalizedIntent {
    proposal_reference: String,
    normalized_operation: NormalizedOperation,
    normalized_scope: ScopeReference,
    risk_classification: String,
}

impl NormalizedIntent {
    fn from_proposal(
        proposal: &AgentProposal,
    ) -> Result<Self, &'static str> {
        if proposal.requested_operation == "DELETE"
            && proposal.requested_scope == "single_customer"
        {
            Ok(Self {
                proposal_reference: proposal.proposal_id.clone(),
                normalized_operation: NormalizedOperation::DeleteRecord,
                normalized_scope: ScopeReference::SingleCustomer(
                    "mock_id".to_string(),
                ),
                risk_classification: "High".to_string(),
            })
        } else {
            Err("Cannot normalize: ambiguous or unsupported request")
        }
    }
}

#[test]
fn test_normalization_creates_typed_artifact_without_authority() {
    let proposal = AgentProposal {
        proposal_id: "prop_001".to_string(),
        requested_operation: "DELETE".to_string(),
        requested_scope: "single_customer".to_string(),
    };

    let intent =
        NormalizedIntent::from_proposal(&proposal)
            .expect("Should normalize cleanly");

    assert_eq!(intent.proposal_reference, "prop_001");
    assert_eq!(
        intent.normalized_operation,
        NormalizedOperation::DeleteRecord
    );
    assert_eq!(intent.risk_classification, "High");
}

#[test]
fn test_normalization_does_not_expand_requested_scope() {
    let proposal = AgentProposal {
        proposal_id: "prop_002".to_string(),
        requested_operation: "DELETE".to_string(),
        requested_scope: "single_customer".to_string(),
    };

    let intent =
        NormalizedIntent::from_proposal(&proposal)
            .expect("Should normalize cleanly");

    assert_eq!(
        intent.normalized_scope,
        ScopeReference::SingleCustomer("mock_id".to_string())
    );
}

#[test]
fn test_invalid_intent_is_rejected() {
    let proposal = AgentProposal {
        proposal_id: "prop_invalid".to_string(),
        requested_operation: "UNKNOWN".to_string(),
        requested_scope: "unknown_scope".to_string(),
    };

    assert!(
        NormalizedIntent::from_proposal(&proposal).is_err()
    );
}

#[test]
fn test_normalization_is_deterministic() {
    let proposal = AgentProposal {
        proposal_id: "prop_003".to_string(),
        requested_operation: "DELETE".to_string(),
        requested_scope: "single_customer".to_string(),
    };

    let first =
        NormalizedIntent::from_proposal(&proposal).unwrap();

    let second =
        NormalizedIntent::from_proposal(&proposal).unwrap();

    assert_eq!(first, second);
}

#[test]
fn test_normalization_preserves_provenance() {
    let proposal = AgentProposal {
        proposal_id: "prop_trace_001".to_string(),
        requested_operation: "DELETE".to_string(),
        requested_scope: "single_customer".to_string(),
    };

    let intent =
        NormalizedIntent::from_proposal(&proposal)
            .expect("Should normalize cleanly");

    assert_eq!(
        intent.proposal_reference,
        "prop_trace_001"
    );

    // Normalization transforms representation.
    // It does not sever provenance or create authority.
}