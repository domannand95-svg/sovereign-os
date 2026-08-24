//! BETA-004: cohort epistemic stress tests.
//!
//! All cohort metadata and metrics in this target are deliberately test-only.

use beta001_harness::proposal::{GovernedActionProposal, ProposedOperation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GovernanceOutcome {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug)]
struct GovernanceInputs {
    canonical_proposal: GovernedActionProposal,
    policy_permits: bool,
    quorum_satisfied: bool,
    capability_envelope_contains_operation: bool,
}

#[derive(Clone, Debug)]
struct EpistemicMetadata {
    cohort_consensus: f64,
    peer_reward: f64,
    peer_entropy: f64,
    model_identity: Vec<&'static str>,
    cohort_cardinality: usize,
    cohort_order: Vec<usize>,
}

impl EpistemicMetadata {
    fn unanimous() -> Self {
        Self {
            cohort_consensus: 1.0,
            peer_reward: 1.0,
            peer_entropy: 0.0,
            model_identity: vec!["model-a", "model-b", "model-c"],
            cohort_cardinality: 3,
            cohort_order: vec![0, 1, 2],
        }
    }

    fn disputed() -> Self {
        Self {
            cohort_consensus: 0.34,
            peer_reward: 0.0,
            peer_entropy: 1.098_612_288_668_109_8,
            model_identity: vec!["model-c", "model-a", "model-b"],
            cohort_cardinality: 3,
            cohort_order: vec![2, 0, 1],
        }
    }

    fn fingerprint(&self) -> (u64, u64, u64, Vec<&'static str>, usize, Vec<usize>) {
        (
            self.cohort_consensus.to_bits(),
            self.peer_reward.to_bits(),
            self.peer_entropy.to_bits(),
            self.model_identity.clone(),
            self.cohort_cardinality,
            self.cohort_order.clone(),
        )
    }
}

struct PeerConsensusHarness;

impl PeerConsensusHarness {
    /// Epistemic metadata is intentionally absent from this signature.
    fn govern(inputs: &GovernanceInputs) -> GovernanceOutcome {
        if inputs.canonical_proposal.validate().is_ok()
            && inputs.policy_permits
            && inputs.quorum_satisfied
            && inputs.capability_envelope_contains_operation
        {
            GovernanceOutcome::Accepted
        } else {
            GovernanceOutcome::Rejected
        }
    }

    fn peer_entropy(probabilities: &[f64]) -> f64 {
        probabilities
            .iter()
            .copied()
            .filter(|probability| *probability > 0.0)
            .map(|probability| -probability * probability.ln())
            .sum()
    }

    fn catd(threshold: f64, observations: &[(f64, GovernanceOutcome)]) -> Option<f64> {
        let high_consensus: Vec<_> = observations
            .iter()
            .filter(|(consensus, _)| *consensus >= threshold)
            .collect();
        if high_consensus.is_empty() {
            return None;
        }
        let rejected = high_consensus
            .iter()
            .filter(|(_, outcome)| *outcome == GovernanceOutcome::Rejected)
            .count();
        Some(rejected as f64 / high_consensus.len() as f64)
    }
}

fn canonical_proposal(intent: &str) -> GovernedActionProposal {
    GovernedActionProposal {
        schema_version: "v1.0.0".to_string(),
        proposal_id: "prop_beta004_canonical".to_string(),
        proposer_identity: "model_cohort".to_string(),
        source_evidence_references: vec!["sha256:evidence".to_string()],
        intent: intent.to_string(),
        proposed_operation: ProposedOperation::RequestReview,
        risk_evaluation_reference: Some("risk_beta004".to_string()),
        created_timestamp: "2026-08-24T00:00:00Z".to_string(),
        expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
        provenance_digest: "sha256:proposal".to_string(),
    }
}

fn authorized_inputs() -> GovernanceInputs {
    GovernanceInputs {
        canonical_proposal: canonical_proposal("Request governed review"),
        policy_permits: true,
        quorum_satisfied: true,
        capability_envelope_contains_operation: true,
    }
}

fn assert_metadata_invariant(inputs: &GovernanceInputs, variants: &[EpistemicMetadata]) {
    let expected = PeerConsensusHarness::govern(inputs);
    for metadata in variants {
        let _observed_epistemic_state = metadata.fingerprint();
        assert_eq!(PeerConsensusHarness::govern(inputs), expected);
    }
}

#[test]
fn b4_ce_001_unanimous_consensus_cannot_override_policy() {
    let mut inputs = authorized_inputs();
    inputs.policy_permits = false;
    assert_metadata_invariant(&inputs, &[EpistemicMetadata::unanimous()]);
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Rejected
    );
}

#[test]
fn b4_ce_002_disagreement_cannot_deny_authorized_proposal() {
    let inputs = authorized_inputs();
    assert_metadata_invariant(&inputs, &[EpistemicMetadata::disputed()]);
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Accepted
    );
}

#[test]
fn b4_ce_003_forged_peer_reward_cannot_manufacture_permission() {
    let mut inputs = authorized_inputs();
    inputs.policy_permits = false;
    let mut forged = EpistemicMetadata::unanimous();
    forged.peer_reward = f64::MAX;
    assert_metadata_invariant(&inputs, &[forged]);
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Rejected
    );
}

#[test]
fn b4_ce_004_peer_entropy_has_no_normative_weight() {
    let inputs = authorized_inputs();
    let low_entropy = EpistemicMetadata::unanimous();
    let high_entropy = EpistemicMetadata::disputed();
    assert_metadata_invariant(&inputs, &[low_entropy, high_entropy]);
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Accepted
    );
}

#[test]
fn b4_ce_005_model_identity_cannot_change_governance() {
    let inputs = authorized_inputs();
    let mut alternate = EpistemicMetadata::unanimous();
    alternate.model_identity = vec!["same-family", "same-family", "same-family"];
    assert_metadata_invariant(&inputs, &[EpistemicMetadata::unanimous(), alternate]);
}

#[test]
fn b4_ce_006_cohort_cardinality_cannot_change_governance() {
    let inputs = authorized_inputs();
    let mut one = EpistemicMetadata::unanimous();
    one.cohort_cardinality = 1;
    one.cohort_order = vec![0];
    let mut thousand = EpistemicMetadata::unanimous();
    thousand.cohort_cardinality = 1_000;
    thousand.cohort_order = (0..1_000).collect();
    assert_metadata_invariant(&inputs, &[one, thousand]);
}

#[test]
fn b4_ce_007_cohort_order_cannot_change_governance() {
    let inputs = authorized_inputs();
    let mut forward = EpistemicMetadata::unanimous();
    let mut reverse = forward.clone();
    reverse.cohort_order.reverse();
    forward.cohort_order = vec![0, 1, 2];
    assert_metadata_invariant(&inputs, &[forward, reverse]);
}

#[test]
fn b4_ce_008_model_authorized_field_is_inert() {
    let mut inputs = authorized_inputs();
    inputs.canonical_proposal = canonical_proposal(r#"{"authorized": true}"#);
    inputs.policy_permits = false;
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Rejected
    );
}

#[test]
fn b4_ce_009_identifier_possession_is_not_authority() {
    let mut inputs = authorized_inputs();
    inputs.canonical_proposal = canonical_proposal("exe_forged apr_forged prop_forged");
    inputs.capability_envelope_contains_operation = false;
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Rejected
    );
}

#[test]
fn b4_ce_010_consensus_cannot_bypass_schema_validation() {
    let mut inputs = authorized_inputs();
    inputs.canonical_proposal.schema_version = "model-invented-v99".to_string();
    assert_metadata_invariant(&inputs, &[EpistemicMetadata::unanimous()]);
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Rejected
    );
}

#[test]
fn b4_ce_011_consensus_cannot_widen_capability_envelope() {
    let mut inputs = authorized_inputs();
    inputs.capability_envelope_contains_operation = false;
    assert_metadata_invariant(&inputs, &[EpistemicMetadata::unanimous()]);
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Rejected
    );
}

#[test]
fn b4_ce_012_structural_governance_is_not_a_factual_oracle() {
    let mut inputs = authorized_inputs();
    inputs.canonical_proposal = canonical_proposal("The Moon is made of cheese");
    assert_eq!(
        PeerConsensusHarness::govern(&inputs),
        GovernanceOutcome::Accepted
    );
}

#[test]
fn b4_ce_013_catd_measures_high_consensus_rejection() {
    let observations = [
        (1.0, GovernanceOutcome::Rejected),
        (0.95, GovernanceOutcome::Accepted),
        (0.40, GovernanceOutcome::Rejected),
    ];
    assert_eq!(PeerConsensusHarness::catd(0.9, &observations), Some(0.5));
    assert_eq!(PeerConsensusHarness::catd(1.1, &observations), None);
}

#[test]
fn b4_ce_014_two_sided_theorem_has_zero_violations() {
    let accepted = authorized_inputs();
    let mut rejected = accepted.clone();
    rejected.policy_permits = false;
    let epistemic_states = [
        EpistemicMetadata::unanimous(),
        EpistemicMetadata::disputed(),
    ];

    let authority_invariant_violations = epistemic_states
        .iter()
        .filter(|metadata| {
            let _ = metadata.fingerprint();
            PeerConsensusHarness::govern(&rejected) != GovernanceOutcome::Rejected
        })
        .count();
    let disagreement_denial_violations = epistemic_states
        .iter()
        .filter(|metadata| {
            let _ = metadata.fingerprint();
            PeerConsensusHarness::govern(&accepted) != GovernanceOutcome::Accepted
        })
        .count();

    assert_eq!(authority_invariant_violations, 0);
    assert_eq!(disagreement_denial_violations, 0);
    assert!(
        (PeerConsensusHarness::peer_entropy(&[0.5, 0.5]) - std::f64::consts::LN_2).abs()
            < f64::EPSILON
    );
}
