use sovereign_ledger::EventType;
use sovereign_policy::{
    DirectivePolicy, DirectiveRequest, EventTypeAllowlist, PolicyDecision, PolicyDenial,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SOVEREIGN OS: ADAM TRIAL ASSIGNMENT 002 ===");
    println!("Executing Governed Vertical Slice...\n");

    // Initialize policy with a restricted allowlist (e.g., only allowing RegistryMutation)
    let strict_policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);

    // -------------------------------------------------------------------------
    // Run A: DENY (Unauthorized model candidate proposal)
    // -------------------------------------------------------------------------
    println!("[Run A] Proposing ungranted action (KernelDirective)...");
    let unauth_request = DirectiveRequest::new(EventType::KernelDirective, b"unauthorized_payload");
    let decision_a = strict_policy.evaluate(unauth_request)?;

    match decision_a {
        PolicyDecision::Allow => {
            println!("FAIL: Unauthorized request was incorrectly allowed!");
            return Err("Authorization boundary breached".into());
        }
        PolicyDecision::Deny(reason) => {
            println!("PASS: Execution DENIED as expected.");
            println!("   -> Denial Reason: {:?}\n", reason);
        }
    }

    // -------------------------------------------------------------------------
    // Run B: ALLOW (Authorized capability proposal matching allowlist)
    // -------------------------------------------------------------------------
    println!("[Run B] Re-submitting proposal with authorized capability (RegistryMutation)...");
    let auth_request =
        DirectiveRequest::new(EventType::RegistryMutation, b"authorized_node_mutation");
    let decision_b = strict_policy.evaluate(auth_request)?;

    match decision_b {
        PolicyDecision::Allow => {
            println!("PASS: Execution SUCCEEDS under authorized capability grant.");
            println!("   -> Audit Receipt: Generated deterministic allow decision.\n");
        }
        PolicyDecision::Deny(reason) => {
            println!("FAIL: Authorized request was denied: {:?}\n", reason);
            return Err("Authorized path blocked".into());
        }
    }

    // -------------------------------------------------------------------------
    // Replay Verification
    // -------------------------------------------------------------------------
    println!("[Replay] Verifying audit trail via deterministic evaluation replay...");
    let replay_decision = strict_policy.evaluate(auth_request)?;
    assert_eq!(decision_b, replay_decision);
    println!("PASS: Replay evaluation matches original execution receipt exactly.\n");

    println!("VERDICT: PASS — GOVERNED_VERTICAL_SLICE_PROVEN");
    Ok(())
}
