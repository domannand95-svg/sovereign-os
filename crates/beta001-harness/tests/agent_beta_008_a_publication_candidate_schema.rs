use serde_json::{json, Value};

pub struct CandidateSemanticValidator;

impl CandidateSemanticValidator {
    pub fn validate(candidate: &Value) -> Result<(), String> {
        // Enforce no additional properties conceptually (simulating schema constraint)
        let allowed_keys = [
            "schema_version", "candidate_id", "local_repository_identity",
            "candidate_commit_oid", "source_local_ref", "canonical_remote_identity",
            "destination_ref", "expected_remote_state", "proposed_transition"
        ];
        
        if let Some(obj) = candidate.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return Err(format!("Extraneous property found: {}", key));
                }
            }
        }

        let cand_oid = candidate["candidate_commit_oid"].as_str().ok_or("Missing candidate_commit_oid")?;
        let transition = &candidate["proposed_transition"];
        let to_oid = transition["to_oid"].as_str().ok_or("Missing to_oid")?;
        let from_oid = transition["from_oid"].as_str().ok_or("Missing from_oid")?;

        let expected_state = &candidate["expected_remote_state"];
        let state_type = expected_state["state_type"].as_str().ok_or("Missing state_type")?;

        // Cross-field equality: candidate_commit_oid == proposed_transition.to_oid
        if cand_oid != to_oid {
            return Err("candidate_commit_oid != proposed_transition.to_oid".into());
        }

        // Cross-field equality & state validation
        if state_type == "present" {
            if let Some(expected_oid) = expected_state.get("oid").and_then(|o| o.as_str()) {
                if expected_oid != from_oid {
                    return Err("expected_remote_state.oid != proposed_transition.from_oid".into());
                }
            } else {
                return Err("present state requires oid".into());
            }
        } else if state_type == "absent" {
            if expected_state.get("oid").is_some() {
                return Err("absent state forbids oid".into());
            }
            if from_oid != "0000000000000000000000000000000000000000" {
                return Err("absent state requires zeroed from_oid".into());
            }
        } else {
            return Err("Invalid state_type".into());
        }

        Ok(())
    }
}

fn valid_candidate_base() -> Value {
    json!({
        "schema_version": "REPOSITORY_PUBLICATION_CANDIDATE-v1",
        "candidate_id": "pub_cand_001",
        "local_repository_identity": "repo_1",
        "candidate_commit_oid": "cccccccccccccccccccccccccccccccccccccccc",
        "source_local_ref": "refs/heads/feature/x",
        "canonical_remote_identity": {
            "provider": "github.com",
            "provider_repository_id": "123456",
            "owner": "domannand95-svg",
            "name": "sovereign-os",
            "endpoint": "https://github.com/domannand95-svg/sovereign-os.git"
        },
        "destination_ref": "refs/heads/feature/x",
        "expected_remote_state": {
            "state_type": "present",
            "oid": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        },
        "proposed_transition": {
            "from_oid": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "to_oid": "cccccccccccccccccccccccccccccccccccccccc"
        }
    })
}

#[test]
fn test_tc_pub_cand_001_valid_candidate_accepted() {
    assert!(CandidateSemanticValidator::validate(&valid_candidate_base()).is_ok());
}

#[test]
fn test_tc_pub_cand_005_expected_remote_state_mismatch_rejected() {
    let mut cand = valid_candidate_base();
    cand["expected_remote_state"]["oid"] = json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let res = CandidateSemanticValidator::validate(&cand);
    assert_eq!(res.unwrap_err(), "expected_remote_state.oid != proposed_transition.from_oid");
}

#[test]
fn test_tc_pub_cand_006_candidate_oid_mismatch_rejected() {
    let mut cand = valid_candidate_base();
    cand["candidate_commit_oid"] = json!("dddddddddddddddddddddddddddddddddddddddd");
    let res = CandidateSemanticValidator::validate(&cand);
    assert_eq!(res.unwrap_err(), "candidate_commit_oid != proposed_transition.to_oid");
}

#[test]
fn test_tc_pub_cand_007_present_state_without_oid_rejected() {
    let mut cand = valid_candidate_base();
    cand["expected_remote_state"] = json!({ "state_type": "present" });
    let res = CandidateSemanticValidator::validate(&cand);
    assert_eq!(res.unwrap_err(), "present state requires oid");
}

#[test]
fn test_tc_pub_cand_008_absent_state_with_oid_rejected() {
    let mut cand = valid_candidate_base();
    cand["expected_remote_state"] = json!({ "state_type": "absent", "oid": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" });
    cand["proposed_transition"]["from_oid"] = json!("0000000000000000000000000000000000000000");
    let res = CandidateSemanticValidator::validate(&cand);
    assert_eq!(res.unwrap_err(), "absent state forbids oid");
}

#[test]
fn test_tc_pub_cand_009_credential_field_injection_rejected() {
    let mut cand = valid_candidate_base();
    cand["credential_id"] = json!("secret_token");
    let res = CandidateSemanticValidator::validate(&cand);
    assert!(res.unwrap_err().contains("Extraneous property"));
}

#[test]
fn test_tc_pub_cand_010_authorization_field_injection_rejected() {
    let mut cand = valid_candidate_base();
    cand["permission_scope"] = json!("admin");
    let res = CandidateSemanticValidator::validate(&cand);
    assert!(res.unwrap_err().contains("Extraneous property"));
}
