use serde_json::{json, Value};

pub struct ReceiptEpistemicValidator;

impl ReceiptEpistemicValidator {
    pub fn validate(receipt: &Value, expected_candidate_y: &str) -> Result<(), String> {
        let allowed_keys = [
            "schema_version", "receipt_id", "provenance_chain", "execution_observation",
            "independent_post_observation", "terminal_disposition", "disposition_reason_code",
            "authorization_consumption", "timestamp"
        ];
        
        if let Some(obj) = receipt.as_object() {
            for key in obj.keys() {
                if !allowed_keys.contains(&key.as_str()) {
                    return Err(format!("Extraneous property found: {}", key));
                }
            }
        }

        let prov = receipt.get("provenance_chain").ok_or("RECEIPT_INVALID: Missing provenance_chain")?;
        prov.get("candidate_digest").ok_or("RECEIPT_INVALID: Missing candidate_digest")?;
        
        let lease_ref = prov.get("lease_reference").ok_or("RECEIPT_INVALID: Missing lease_reference")?;
        if lease_ref.is_string() {
            return Err("RECEIPT_INVALID: lease_reference cannot be a string (e.g., NONE)".into());
        }

        receipt.get("authorization_consumption").ok_or("RECEIPT_INVALID: Missing authorization_consumption")?;

        let exec_obs = receipt.get("execution_observation").and_then(|v| v.as_str()).unwrap_or("");
        let term_disp = receipt.get("terminal_disposition").and_then(|v| v.as_str()).unwrap_or("");
        let post_obs = receipt.get("independent_post_observation").ok_or("RECEIPT_INVALID: Missing independent_post_observation")?;
        let obs_state = post_obs.get("observation_state").and_then(|v| v.as_str()).unwrap_or("");
        let obs_oid = post_obs.get("observed_oid").and_then(|v| v.as_str());

        // Epistemic Rule: VERIFIED_SUCCESS requires independent observation == expected_y
        if term_disp == "VERIFIED_SUCCESS" {
            if obs_state != "PRESENT" || obs_oid != Some(expected_candidate_y) {
                return Err("EPISTEMIC_INVALID: VERIFIED_SUCCESS requires independent observation of candidate OID".into());
            }
        }

        // Epistemic Rule: False Negative Prevention (Adapter failure + remote is Y -> MUST be VERIFIED_SUCCESS)
        if exec_obs == "ADAPTER_REPORTED_FAILURE" && obs_state == "PRESENT" && obs_oid == Some(expected_candidate_y) {
            if term_disp != "VERIFIED_SUCCESS" {
                return Err("EPISTEMIC_INVALID: Adapter failure with independent observation of target state MUST resolve to VERIFIED_SUCCESS".into());
            }
        }

        Ok(())
    }
}

fn valid_receipt_base() -> Value {
    json!({
        "schema_version": "REPOSITORY_PUBLICATION_RECEIPT-v1",
        "receipt_id": "pub_receipt_001",
        "provenance_chain": {
            "candidate_id": "pub_cand_001",
            "candidate_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000001",
            "authorization_id": "pub_auth_001",
            "authorization_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000002",
            "lease_reference": {
                "lease_id": "cred_lease_001",
                "lease_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000003"
            }
        },
        "execution_observation": "ADAPTER_REPORTED_SUCCESS",
        "independent_post_observation": {
            "observation_state": "PRESENT",
            "observed_oid": "cccccccccccccccccccccccccccccccccccccccc",
            "observed_ref": "refs/heads/feature/x",
            "observed_at": "2026-08-20T10:05:00Z"
        },
        "terminal_disposition": "VERIFIED_SUCCESS",
        "disposition_reason_code": "EXACT_TRANSITION_VERIFIED",
        "authorization_consumption": "CONSUMED",
        "timestamp": "2026-08-20T10:05:01Z"
    })
}

#[test]
fn test_tc_pub_receipt_001_valid_verified_success_accepted() {
    assert!(ReceiptEpistemicValidator::validate(&valid_receipt_base(), "cccccccccccccccccccccccccccccccccccccccc").is_ok());
}

#[test]
fn test_tc_pub_receipt_002_missing_provenance_rejected() {
    let mut receipt = valid_receipt_base();
    receipt.as_object_mut().unwrap().remove("provenance_chain");
    let res = ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(res.unwrap_err(), "RECEIPT_INVALID: Missing provenance_chain");
}

#[test]
fn test_tc_pub_receipt_003_success_lie_rejected() {
    let mut receipt = valid_receipt_base();
    // Adapter reports success, but independent observation is old state X
    receipt["independent_post_observation"]["observed_oid"] = json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    // System incorrectly tries to claim VERIFIED_SUCCESS
    receipt["terminal_disposition"] = json!("VERIFIED_SUCCESS");
    
    let res = ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(res.unwrap_err(), "EPISTEMIC_INVALID: VERIFIED_SUCCESS requires independent observation of candidate OID");
}

#[test]
fn test_tc_pub_receipt_004_verified_success_with_unknown_observation_rejected() {
    let mut receipt = valid_receipt_base();
    receipt["independent_post_observation"]["observation_state"] = json!("UNKNOWN");
    receipt["independent_post_observation"].as_object_mut().unwrap().remove("observed_oid");
    let res = ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(res.unwrap_err(), "EPISTEMIC_INVALID: VERIFIED_SUCCESS requires independent observation of candidate OID");
}

#[test]
fn test_tc_pub_receipt_005_adapter_failure_remote_y_resolves_verified_success() {
    let mut receipt = valid_receipt_base();
    receipt["execution_observation"] = json!("ADAPTER_REPORTED_FAILURE");
    // Remote state IS Y
    receipt["independent_post_observation"]["observed_oid"] = json!("cccccccccccccccccccccccccccccccccccccccc");
    // System incorrectly logs failure
    receipt["terminal_disposition"] = json!("ADAPTER_INCONSISTENCY");
    
    let res = ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(res.unwrap_err(), "EPISTEMIC_INVALID: Adapter failure with independent observation of target state MUST resolve to VERIFIED_SUCCESS");
}

#[test]
fn test_tc_pub_receipt_006_ambiguous_with_unknown_observation_accepted() {
    let mut receipt = valid_receipt_base();
    receipt["execution_observation"] = json!("TRANSPORT_OUTCOME_UNKNOWN");
    receipt["independent_post_observation"]["observation_state"] = json!("UNKNOWN");
    receipt["independent_post_observation"].as_object_mut().unwrap().remove("observed_oid");
    receipt["terminal_disposition"] = json!("AMBIGUOUS");
    
    assert!(ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc").is_ok());
}

#[test]
fn test_tc_pub_receipt_007_fake_lease_none_rejected() {
    let mut receipt = valid_receipt_base();
    receipt["provenance_chain"]["lease_reference"] = json!("NONE");
    let res = ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(res.unwrap_err(), "RECEIPT_INVALID: lease_reference cannot be a string (e.g., NONE)");
}

#[test]
fn test_tc_pub_receipt_008_missing_consumption_state_rejected() {
    let mut receipt = valid_receipt_base();
    receipt.as_object_mut().unwrap().remove("authorization_consumption");
    let res = ReceiptEpistemicValidator::validate(&receipt, "cccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(res.unwrap_err(), "RECEIPT_INVALID: Missing authorization_consumption");
}
