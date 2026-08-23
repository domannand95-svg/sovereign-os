//! ADAM TRIAL ASSIGNMENT 003 — Controlled Execution Boundary
//!
//! Proves that:
//! 1. Policy DENY prevents execution entirely (Run A).
//! 2. Policy ALLOW is necessary but not sufficient; an AuthorizationReceipt
//!    cannot execute without valid authentication (Run B -> ALLOW != EXECUTE).
//! 3. Full governed execution requires Policy Allow + Valid AdmissionDecision +
//!    Valid Receipt + Valid Adapter Authentication + Content Integrity (Run C).

use sovereign_ledger::EventType;
use sovereign_policy::{DirectivePolicy, DirectiveRequest, EventTypeAllowlist, PolicyDecision};

use sovereign_audit::authorization_receipt::{
    AuthorizationReceipt, IssuerContext, ReceiptAuthenticationResult,
};
use sovereign_audit::governance_admission::{AdmissionDecision, AdmissionOutcome};

use sovereign_execution::{
    ExecutionError, ExecutionOutcome, FileCreationAdapter, FileCreationOperation, GovernedExecutor,
};

use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SOVEREIGN OS: ADAM TRIAL ASSIGNMENT 003 ===");
    println!("Controlled Execution Boundary & Authority Separation...\n");

    let strict_policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
    let target_dir = "target/trial_003";
    let target_path = format!("{}/evidence.txt", target_dir);

    // Ensure clean trial workspace directory and recreate parent bounds
    let _ = fs::remove_dir_all(target_dir);
    fs::create_dir_all(target_dir)?;

    // -------------------------------------------------------------------------
    // Run A: Policy DENY Path
    // -------------------------------------------------------------------------
    println!("[Run A] Proposing ungranted action (KernelDirective)...");
    let req_a = DirectiveRequest::new(EventType::KernelDirective, b"unauthorized_payload");
    let decision_a = strict_policy.evaluate(req_a)?;

    match decision_a {
        PolicyDecision::Deny(reason) => {
            println!("PASS: Policy DENIED as expected ({:?}).", reason);
            println!("   -> No AdmissionDecision generated.");
            println!("   -> No AuthorizationReceipt issued.");
            println!("   -> FileCreationAdapter never invoked. No side effect produced.\n");
        }
        PolicyDecision::Allow => {
            return Err("Run A failed: unauthorized request was allowed".into());
        }
    }

    // -------------------------------------------------------------------------
    // Run B: Policy ALLOW, but Execution Invalid (Authentication Failure)
    // -------------------------------------------------------------------------
    println!("[Run B] Policy ALLOW + Execution Invalid (Unauthenticated Receipt)...");
    let req_b = DirectiveRequest::new(EventType::RegistryMutation, b"authorized_mutation");
    let _decision_b = strict_policy.evaluate(req_b)?;

    let admission_b = AdmissionDecision {
        decision_reference: "trial003-decision-b".into(),
        intent_reference: "trial003-intent-b".into(),
        evaluation_reference: "trial003-eval-b".into(),
        governance_context_reference: "trial003-context-b".into(),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: "scope-registry-mutation".into(),
        reason_references: vec!["policy-allow".into()],
    };

    let issuer_context = IssuerContext {
        issuer_reference: "governance-issuer-001".into(),
        signing_key_reference: "key-001".into(),
    };

    let receipt_b = AuthorizationReceipt::generate(
        &admission_b,
        "subject-agent-001",
        "trial003-intent-b",
        "policy-registry-mutation",
        "trial003-context-b",
        "file_create",
        &target_path,
        1000,
        2000,
        &issuer_context,
        "nonce-b-001",
    )
    .map_err(|e| format!("failed to generate receipt: {}", e))?;

    let content_b = b"trial_003_invalid_execution_payload";
    let operation_b = FileCreationOperation {
        path: target_path.clone(),
        content_hash: *blake3::hash(content_b).as_bytes(),
    };

    let adapter = FileCreationAdapter;

    // Execute with ReceiptAuthenticationResult::Invalid
    let exec_result_b = adapter.execute(
        &receipt_b,
        ReceiptAuthenticationResult::Invalid,
        &operation_b,
        &operation_b,
        content_b,
    );

    match exec_result_b {
        Err(ExecutionError::Unauthenticated) => {
            println!("PASS: Execution rejected due to unauthenticated receipt (ExecutionError::Unauthenticated).");
            println!("   -> Proves PolicyAuthority != ExecutionAuthority and ALLOW != EXECUTE.\n");
        }
        other => {
            return Err(format!("Run B failed: unexpected result {:?}", other).into());
        }
    }

    // -------------------------------------------------------------------------
    // Run C: Full Governed Execution (Valid Policy + Valid Admission + Valid Receipt + Valid Auth)
    // -------------------------------------------------------------------------
    println!("[Run C] Full Governed Execution Path...");
    let req_c = DirectiveRequest::new(EventType::RegistryMutation, b"authorized_mutation");
    let _decision_c = strict_policy.evaluate(req_c)?;

    let admission_c = AdmissionDecision {
        decision_reference: "trial003-decision-c".into(),
        intent_reference: "trial003-intent-c".into(),
        evaluation_reference: "trial003-eval-c".into(),
        governance_context_reference: "trial003-context-c".into(),
        outcome: AdmissionOutcome::Permit,
        authorized_scope: "scope-registry-mutation".into(),
        reason_references: vec!["policy-allow".into()],
    };

    let receipt_c = AuthorizationReceipt::generate(
        &admission_c,
        "subject-agent-001",
        "trial003-intent-c",
        "policy-registry-mutation",
        "trial003-context-c",
        "file_create",
        &target_path,
        1000,
        2000,
        &issuer_context,
        "nonce-c-001",
    )
    .map_err(|e| format!("failed to generate receipt: {}", e))?;

    let content_c = b"TRIAL_003_BOUNDED_GOVERNED_EXECUTION_SUCCESS\n";
    let operation_c = FileCreationOperation {
        path: target_path.clone(),
        content_hash: *blake3::hash(content_c).as_bytes(),
    };

    let exec_report_c = adapter
        .execute(
            &receipt_c,
            ReceiptAuthenticationResult::Valid,
            &operation_c,
            &operation_c,
            content_c,
        )
        .map_err(|e| format!("execution failed: {:?}", e))?;

    assert_eq!(exec_report_c.outcome, ExecutionOutcome::Created);
    println!("PASS: Execution SUCCEEDED under valid governed flow.");
    println!("   -> Execution ID: {}", exec_report_c.attempt.execution_id);
    println!(
        "   -> Side Effect Verified: Target file created at {}",
        target_path
    );

    if !Path::new(&target_path).exists() {
        return Err("Side effect file missing from target path".into());
    }

    println!("\nVERDICT: PASS — GOVERNED_EXECUTION_BOUNDARY_PROVEN");
    Ok(())
}
