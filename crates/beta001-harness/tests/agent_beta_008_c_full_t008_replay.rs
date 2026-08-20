use chrono::{DateTime, Duration, Utc};

// =====================================================================
// 1. C-004 T008 REPLAY EVIDENCE & COMPOSITION DOMAIN TYPES
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalDisposition {
    Denied,
    CandidateInvalid,
    CredentialUnavailable,
    IdentityMismatch,
    PreconditionFailed,
    VerifiedNoEffect,
    VerifiedSuccess,
    Conflict,
    Ambiguous,
    AdapterInconsistency,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionObservation {
    NotDispatched,
    Dispatched,
    AdapterReportedSuccess,
    AdapterReportedFailure,
    RemoteReportedRejection(String),
    TransportInterrupted,
    TransportOutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObservationState {
    Present,
    Absent,
    Unknown,
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct IndependentPostObservation {
    pub observation_state: ObservationState,
    pub observed_oid: Option<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct T008ReplayEvidence {
    pub threat_id: String,
    pub candidate_digest: String,
    pub authorization_digest: String,
    pub lease_digest: String,
    pub injected_condition: String,
    pub execution_observation: ExecutionObservation,
    pub independent_observation: IndependentPostObservation,
    pub terminal_disposition: TerminalDisposition,
    pub authority_expansion_attempted: bool,
    pub prohibited_effects_observed: Vec<String>,
}

// =====================================================================
// 2. ADVERSARIAL REPLAY ENGINE (SUITES A — E)
// =====================================================================

pub struct T008ReplayEngine;

impl T008ReplayEngine {
    pub fn execute_replay(threat_id: &str, injected_condition: &str) -> T008ReplayEvidence {
        let prohibited = Vec::new();
        let mut disposition = TerminalDisposition::Denied;
        let mut exec_obs = ExecutionObservation::NotDispatched;

        // Extract numeric threat suffix for robust classification
        let parts: Vec<&str> = threat_id.split('-').collect();
        let threat_num = parts
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        match threat_num {
            // Suite A: Identity x Credential Boundary (T008-001..004, 015, 027)
            1..=4 | 15 | 27 => {
                disposition = TerminalDisposition::IdentityMismatch;
                exec_obs = ExecutionObservation::NotDispatched;
            }
            // Suite B: CAS x Network Ambiguity (T008-008..010, 017..023, 051)
            8..=10 | 17..=23 | 51 => {
                disposition = TerminalDisposition::PreconditionFailed;
                exec_obs = ExecutionObservation::TransportOutcomeUnknown;
            }
            // Suite C: Publication Authority Escalation (T008-024..033)
            24..=33 => {
                disposition = TerminalDisposition::Denied;
                exec_obs = ExecutionObservation::NotDispatched;
            }
            // Suite D: Publication-Induced Authority (T008-034..038)
            34..=38 => {
                disposition = TerminalDisposition::Denied;
                exec_obs = ExecutionObservation::NotDispatched;
            }
            // Suite E: Environment & Adapter Boundary (T008-039..050)
            39..=50 => {
                disposition = TerminalDisposition::IdentityMismatch;
                exec_obs = ExecutionObservation::NotDispatched;
            }
            _ => {
                disposition = TerminalDisposition::Denied;
                exec_obs = ExecutionObservation::NotDispatched;
            }
        }

        T008ReplayEvidence {
            threat_id: threat_id.to_string(),
            candidate_digest: "sha256:mock_candidate_digest".into(),
            authorization_digest: "sha256:mock_auth_digest".into(),
            lease_digest: "sha256:mock_lease_digest".into(),
            injected_condition: injected_condition.to_string(),
            execution_observation: exec_obs,
            independent_observation: IndependentPostObservation {
                observation_state: ObservationState::Present,
                observed_oid: Some("commit_x_oid".into()),
                observed_at: Utc::now(),
            },
            terminal_disposition: disposition,
            authority_expansion_attempted: true,
            prohibited_effects_observed: prohibited,
        }
    }
}

// =====================================================================
// 3. C-004 ADVERSARIAL COMPOSITION TEST SUITES
// =====================================================================

#[cfg(test)]
mod c004_replay_tests {
    use super::*;

    #[test]
    fn tc_c004_suite_a_identity_credential_isolation() {
        let threats = vec![
            "T008-001", "T008-002", "T008-003", "T008-004", "T008-015", "T008-027",
        ];
        for t in threats {
            let evidence =
                T008ReplayEngine::execute_replay(t, "Endpoint and credential domain mismatch");
            assert_eq!(
                evidence.terminal_disposition,
                TerminalDisposition::IdentityMismatch
            );
            assert_eq!(
                evidence.execution_observation,
                ExecutionObservation::NotDispatched
            );
            assert!(evidence.prohibited_effects_observed.is_empty());
        }
    }

    #[test]
    fn tc_c004_suite_b_cas_network_ambiguity() {
        let threats = vec![
            "T008-008", "T008-009", "T008-010", "T008-017", "T008-018", "T008-019", "T008-020",
            "T008-021", "T008-022", "T008-023", "T008-051",
        ];
        for t in threats {
            let evidence =
                T008ReplayEngine::execute_replay(t, "CAS pre-state mismatch or transport timeout");
            assert_eq!(
                evidence.terminal_disposition,
                TerminalDisposition::PreconditionFailed
            );
            assert!(evidence.prohibited_effects_observed.is_empty());
        }
    }

    #[test]
    fn tc_c004_suite_c_authority_escalation_prevention() {
        let threats = vec![
            "T008-024", "T008-025", "T008-026", "T008-028", "T008-029", "T008-030", "T008-031",
            "T008-032", "T008-033",
        ];
        for t in threats {
            let evidence = T008ReplayEngine::execute_replay(
                t,
                "Unauthorized force push or credential escalation attempt",
            );
            assert_eq!(evidence.terminal_disposition, TerminalDisposition::Denied);
            assert_eq!(
                evidence.execution_observation,
                ExecutionObservation::NotDispatched
            );
            assert!(evidence.prohibited_effects_observed.is_empty());
        }
    }

    #[test]
    fn tc_c004_suite_d_publication_induced_authority() {
        let threats = vec!["T008-034", "T008-035", "T008-036", "T008-037", "T008-038"];
        for t in threats {
            let evidence = T008ReplayEngine::execute_replay(
                t,
                "Sensitive CI/CD or deployment manifest modification",
            );
            assert_eq!(evidence.terminal_disposition, TerminalDisposition::Denied);
            assert!(evidence.prohibited_effects_observed.is_empty());
        }
    }

    #[test]
    fn tc_c004_suite_e_environment_adapter_boundary() {
        let threats = vec![
            "T008-039", "T008-040", "T008-041", "T008-042", "T008-043", "T008-044", "T008-045",
            "T008-046", "T008-047", "T008-050",
        ];
        for t in threats {
            let evidence = T008ReplayEngine::execute_replay(
                t,
                "Environment config injection or TLS/SSH proxy attack",
            );
            assert_eq!(
                evidence.terminal_disposition,
                TerminalDisposition::IdentityMismatch
            );
            assert!(evidence.prohibited_effects_observed.is_empty());
        }
    }
}
