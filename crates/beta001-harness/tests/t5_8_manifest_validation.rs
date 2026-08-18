use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn get_manifest_path(relative: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative);
    path
}

#[test]
fn migration_manifest_is_structurally_valid() {
    let manifest_path = get_manifest_path("fixtures/T5_8_migration_manifest.json");
    let raw = fs::read_to_string(&manifest_path).expect("Manifest must exist");

    let manifest: serde_json::Value =
        serde_json::from_str(&raw).expect("Manifest must parse as JSON");

    assert_eq!(
        manifest["validation_status"].as_str().unwrap(),
        "PARITY_PROVEN",
        "Manifest must indicate parity has been proven"
    );

    let orphans = manifest["orphans"]
        .as_array()
        .expect("orphans must be an array");
    assert!(
        orphans.is_empty(),
        "Manifest must not contain orphaned legacy fixtures"
    );

    let inventory = manifest["inventory"]
        .as_array()
        .expect("inventory must be an array");
    assert_eq!(
        inventory.len(),
        12,
        "Manifest must map exactly 12 legacy fixtures"
    );

    let mut seen_envelopes = HashSet::new();
    let mut seen_targets = HashSet::new();

    for entry in inventory {
        let envelope = entry["corpus_envelope"]
            .as_str()
            .expect("Missing corpus_envelope");
        let trace_id = entry["trace_id"].as_str().expect("Missing trace_id");
        let target_state = entry["target_state"]
            .as_str()
            .expect("Missing target_state");
        let resolves_to = entry["resolves_to"].as_str().expect("Missing resolves_to");
        let eligibility = entry["retirement_eligibility"]
            .as_str()
            .expect("Missing retirement_eligibility");

        assert_eq!(
            eligibility, "ELIGIBLE",
            "All entries must be marked ELIGIBLE for retirement"
        );

        // Validate Uniqueness
        assert!(
            seen_envelopes.insert(envelope.to_string()),
            "Duplicate corpus envelope detected: {}",
            envelope
        );
        assert!(
            seen_targets.insert(resolves_to.to_string()),
            "Duplicate legacy target detected: {}",
            resolves_to
        );

        // Validate Filesystem Existence
        let envelope_path = get_manifest_path(&format!("fixtures/corpus/{}", envelope));
        assert!(
            envelope_path.exists(),
            "Corpus envelope does not exist on filesystem: {}",
            envelope
        );

        let target_path = get_manifest_path(&format!("fixtures/{}", resolves_to));
        assert!(
            target_path.exists(),
            "Legacy target does not exist on filesystem: {}",
            resolves_to
        );
    }
}
