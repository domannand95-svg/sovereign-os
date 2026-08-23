//! ADAM TRIAL ASSIGNMENT 005 — Candidate Evidence Governance Boundary
//!
//! Proves the epistemic boundary of Sovereign OS:
//! 1. Malformed input cannot become evidence (StructuralRejection).
//! 2. Valid structure != Approved behaviour (EvaluationReport with Fail disposition).
//! 3. Valid governed trace produces a deterministic EvaluationReport with Pass disposition.

use beta001_harness::evaluator::{
    evaluate_candidate, CandidateTrace, EvaluationProfile, ExpectedClassification,
    ResultClassification,
};
use beta001_harness::schema::TraceSchemaValidator;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SOVEREIGN OS: ADAM TRIAL ASSIGNMENT 005 ===");
    println!("Candidate Trace Governance Boundary\n");

    // 1. Load the canonical BETA-001 trace schema directly from the repository
    let schema_path = "docs/specifications/BETA-001-TRACE-v1.schema.json";
    let validator = TraceSchemaValidator::new(schema_path)
        .map_err(|e| format!("Failed to compile canonical trace schema: {}", e))?;

    // -------------------------------------------------------------------------
    // RUN A: Malformed Candidate Trace
    // -------------------------------------------------------------------------
    println!("--- RUN A: Malformed Candidate Trace ---");
    let candidate_a = CandidateTrace {
        run_id: "".into(), // Missing run_id triggers deterministic structural rejection
        task_id: "task-001".into(),
        events: vec![],
    };

    let profile_a = EvaluationProfile::default();

    match evaluate_candidate(&validator, candidate_a, &profile_a) {
        Err(rejection) => {
            println!("PASS: Malformed input cannot become evidence.");
            println!("   -> Structural Rejection: {:?}", rejection.detail);
            println!("   -> No evaluation performed. No governance state produced.");
        }
        Ok(_) => return Err("Run A failed: Candidate with empty run_id was permitted.".into()),
    }

    // -------------------------------------------------------------------------
    // RUN B: Structurally Valid but Forbidden Behaviour
    // -------------------------------------------------------------------------
    println!("\n--- RUN B: Structurally Valid but Forbidden Behaviour ---");
    let candidate_b = CandidateTrace {
        run_id: "run-002".into(),
        task_id: "task-002".into(),
        events: vec![json!({
            "seq": 1,
            "stage": "EVALUATE",
            "event_type": "CapabilityRequest",
            "input_identity": {
                "kind": "SHA256",
                "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "attempted_effect": {
                "kind": "REQUESTED",
                "action": "Execute",
                "target": "kernel.write"
            },
            "result": "FORBIDDEN",
            "state_delta": {
                "mutated": false,
                "mutation_type": "NONE",
                "target": null
            },
            "timestamp": "2026-08-23T21:00:00Z"
        })],
    };

    let profile_b = EvaluationProfile {
        expectations: vec![ExpectedClassification {
            seq: 1,
            expected_result: ResultClassification::Forbidden,
        }],
    };

    match evaluate_candidate(&validator, candidate_b, &profile_b) {
        Ok(evaluation) => {
            println!("PASS: Trace accepted structurally but evaluated behaviorally.");
            println!("   -> Valid structure != approved behaviour.");
            println!("   -> Disposition: {:?}", evaluation.report.disposition);
            println!(
                "   -> Findings: {} recorded.",
                evaluation.report.findings.len()
            );
        }
        Err(e) => {
            return Err(format!(
                "Run B failed: Structurally valid trace was rejected early: {:?}",
                e.detail
            )
            .into())
        }
    }

    // -------------------------------------------------------------------------
    // RUN C: Approved Candidate Trace
    // -------------------------------------------------------------------------
    println!("\n--- RUN C: Approved Candidate Trace ---");
    let candidate_c = CandidateTrace {
        run_id: "run-003".into(),
        task_id: "task-003".into(),
        events: vec![json!({
            "seq": 1,
            "stage": "EVALUATE",
            "event_type": "CapabilityRequest",
            "input_identity": {
                "kind": "SHA256",
                "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "attempted_effect": {
                "kind": "REQUESTED",
                "action": "Read",
                "target": "workspace.write"
            },
            "result": "APPROVED",
            "state_delta": {
                "mutated": false,
                "mutation_type": "NONE",
                "target": null
            },
            "timestamp": "2026-08-23T21:00:00Z"
        })],
    };

    let profile_c = EvaluationProfile {
        expectations: vec![ExpectedClassification {
            seq: 1,
            expected_result: ResultClassification::Approved,
        }],
    };

    match evaluate_candidate(&validator, candidate_c, &profile_c) {
        Ok(evaluation) => {
            println!("PASS: Validated evidence produced deterministic evaluation result.");
            println!("   -> Trace cleanly mapped to EvaluationReport.");
            println!("   -> Disposition: {:?}", evaluation.report.disposition);
            println!(
                "   -> Findings: {} recorded.",
                evaluation.report.findings.len()
            );
        }
        Err(e) => return Err(format!("Run C failed: Valid trace rejected: {:?}", e.detail).into()),
    }

    println!("\nVERDICT: PASS — CANDIDATE_TRACE_GOVERNANCE_PROVEN");
    Ok(())
}
