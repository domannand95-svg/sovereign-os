//! ADAM TRIAL ASSIGNMENT 006
//!
//! End-to-End Governed Pipeline Closure & Evidence Package Sealing
//!
//! Purpose:
//! Combine synthetic model output capture, deterministic evaluation,
//! and provenance sealing into one verifiable evidence chain.
//!
//! # Invariant
//!
//! Evidence Package ≠ Execution Capability

use beta001_harness::{
    evaluator::{evaluate_candidate, CandidateTrace, EvaluationProfile, EvaluatedDisposition},
    provenance::ProvenanceManifest,
    evidence_package::EvidencePackage,
    schema::TraceSchemaValidator,
};

use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SOVEREIGN OS: ADAM TRIAL 006 ===");
    println!("End-to-End Governed Pipeline Closure\n");

    // Step 1: Initialize Schema Validator
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_path = Path::new(manifest_dir).join("../../docs/specifications/BETA-001-TRACE-v1.schema.json");
    let validator = TraceSchemaValidator::new(schema_path)
        .expect("canonical schema must compile");

    // Step 2: Synthetic Candidate Trace
    let run_id = "run-trial-006".to_string();
    let task_id = "task-trial-006".to_string();
    
    let events = vec![
        serde_json::json!({
            "seq": 1,
            "stage": "INGEST",
            "event_type": "TRIAL_006_EVENT",
            "input_identity": { "kind": "STRUCTURAL_REF", "value": "ingest:1" },
            "attempted_effect": { "kind": "NONE" },
            "result": "APPROVED",
            "state_delta": { "mutated": false, "mutation_type": "NONE", "target": null }
        }),
        serde_json::json!({
            "seq": 2,
            "stage": "READ",
            "event_type": "TRIAL_006_EVENT",
            "input_identity": { "kind": "STRUCTURAL_REF", "value": "read:2" },
            "attempted_effect": { "kind": "NONE" },
            "result": "APPROVED",
            "state_delta": { "mutated": false, "mutation_type": "NONE", "target": null }
        }),
        serde_json::json!({
            "seq": 3,
            "stage": "VALIDATE",
            "event_type": "TRIAL_006_EVENT",
            "input_identity": { "kind": "STRUCTURAL_REF", "value": "validate:3" },
            "attempted_effect": { "kind": "NONE" },
            "result": "APPROVED",
            "state_delta": { "mutated": false, "mutation_type": "NONE", "target": null }
        }),
        serde_json::json!({
            "seq": 4,
            "stage": "PROPOSE",
            "event_type": "TRIAL_006_EVENT",
            "input_identity": { "kind": "STRUCTURAL_REF", "value": "propose:4" },
            "attempted_effect": { "kind": "NONE" },
            "result": "APPROVED",
            "state_delta": { "mutated": false, "mutation_type": "NONE", "target": null }
        }),
        serde_json::json!({
            "seq": 5,
            "stage": "EVALUATE",
            "event_type": "TRIAL_006_EVENT",
            "input_identity": { "kind": "STRUCTURAL_REF", "value": "evaluate:5" },
            "attempted_effect": { "kind": "NONE" },
            "result": "APPROVED",
            "state_delta": { "mutated": false, "mutation_type": "NONE", "target": null }
        }),
    ];

    println!("Synthetic Candidate Trace");
    println!("        |");
    println!("        v");

    let evaluation = evaluate_candidate(
        &validator,
        CandidateTrace {
            run_id: run_id.clone(),
            task_id: task_id.clone(),
            events,
        },
        &EvaluationProfile::default(),
    ).map_err(|e| format!("evaluation failed: {}", e.detail))?;

    assert_eq!(evaluation.report.disposition, EvaluatedDisposition::Pass);
    println!("EvaluationReport PASS");
    println!("        |");
    println!("        v");

    let provenance = ProvenanceManifest::new(
        format!("{}:{}", run_id, task_id),
        &evaluation.report,
        "t5_9_evaluator".to_string(),
        1724410800,
    );
    provenance.validate().expect("provenance manifest must validate");
    println!("ProvenanceManifest VALID");
    println!("        |");
    println!("        v");

    let package = EvidencePackage::new(
        provenance,
        evaluation.report,
        evaluation.trace,
    );

    let sealed = package.seal().expect("evidence package must seal cleanly");
    println!("EvidencePackage SEALED");

    // Emit evidence artifact output directory
    let output_dir = Path::new("docs/evidence/ADAM-TRIAL-006");
    fs::create_dir_all(output_dir)?;

    let package_json = serde_json::to_string_pretty(&sealed)?;
    fs::write(output_dir.join("evidence_package.json"), package_json)?;

    println!("\nVERDICT:");
    println!("PASS — EVIDENCE_COMPLETE");

    Ok(())
}
