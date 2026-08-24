//! Boundary Tests for ADAM-012-A
//!
//! Validates acceptance criteria A012-001 through A012-012.

use beta001_harness::state::{
    compute_delta_digest, normalize_mutations, JournalStageStatus, StateJournal, StateMutation,
    StateTree, STATE_ROOT_DOMAIN,
};

#[test]
fn test_a012_001_empty_state_canonical_root() {
    let tree = StateTree::new();
    assert_eq!(tree.len(), 0);
    assert!(tree.is_empty());
    assert_eq!(tree.revision(), 0);

    let mut expected_hasher = blake3::Hasher::new();
    expected_hasher.update(STATE_ROOT_DOMAIN.as_bytes());
    expected_hasher.update(&0u64.to_be_bytes());
    let expected_root = expected_hasher.finalize().to_hex().to_string();

    assert_eq!(tree.compute_state_root(), expected_root);
}

#[test]
fn test_a012_002_insertion_order_invariance() {
    let mut tree1 = StateTree::new();
    tree1.apply_raw_mutations(&[
        StateMutation::put(b"alpha", b"val_1"),
        StateMutation::put(b"gamma", b"val_3"),
        StateMutation::put(b"beta", b"val_2"),
    ]);

    let mut tree2 = StateTree::new();
    tree2.apply_raw_mutations(&[
        StateMutation::put(b"beta", b"val_2"),
        StateMutation::put(b"alpha", b"val_1"),
        StateMutation::put(b"gamma", b"val_3"),
    ]);

    assert_eq!(tree1.compute_state_root(), tree2.compute_state_root());
    assert_eq!(tree1.revision(), 1);
    assert_eq!(tree2.revision(), 1);
}

#[test]
fn test_a012_003_single_bit_mutation_changes_root() {
    let mut tree1 = StateTree::new();
    tree1.apply_raw_mutations(&[StateMutation::put(b"key_a", b"value_exact")]);

    let mut tree2 = StateTree::new();
    tree2.apply_raw_mutations(&[StateMutation::put(b"key_a", b"value_exacu")]); // 1 bit diff

    assert_ne!(tree1.compute_state_root(), tree2.compute_state_root());
}

#[test]
fn test_a012_004_put_updates_committed_state_deterministically() {
    let mut tree = StateTree::new();
    let changed = tree.apply_raw_mutations(&[
        StateMutation::put(b"config:timeout", b"30"),
        StateMutation::put(b"config:retries", b"3"),
    ]);

    assert!(changed);
    assert_eq!(tree.get(b"config:timeout"), Some(b"30".as_slice()));
    assert_eq!(tree.get(b"config:retries"), Some(b"3".as_slice()));
    assert_eq!(tree.len(), 2);
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_a012_005_delete_removes_existing_keys() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[
        StateMutation::put(b"temp_k1", b"v1"),
        StateMutation::put(b"temp_k2", b"v2"),
    ]);

    assert_eq!(tree.len(), 2);
    assert_eq!(tree.revision(), 1);

    let changed = tree.apply_raw_mutations(&[StateMutation::delete(b"temp_k1")]);
    assert!(changed);
    assert_eq!(tree.get(b"temp_k1"), None);
    assert_eq!(tree.get(b"temp_k2"), Some(b"v2".as_slice()));
    assert_eq!(tree.len(), 1);
    assert_eq!(tree.revision(), 2);
}

#[test]
fn test_a012_006_delete_absent_key_is_noop() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"persisted", b"v1")]);
    let root_before = tree.compute_state_root();

    let changed = tree.apply_raw_mutations(&[StateMutation::delete(b"non_existent")]);
    assert!(!changed);
    assert_eq!(tree.compute_state_root(), root_before);
    assert_eq!(tree.revision(), 1); // Revision unchanged
}

#[test]
fn test_a012_007_multiple_mutations_normalize_last_write_wins() {
    let raw = vec![
        StateMutation::put(b"key_x", b"first"),
        StateMutation::put(b"key_y", b"y_val"),
        StateMutation::delete(b"key_x"),
        StateMutation::put(b"key_x", b"final_winner"),
    ];

    let normalized = normalize_mutations(&raw);
    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0], StateMutation::put(b"key_x", b"final_winner"));
    assert_eq!(normalized[1], StateMutation::put(b"key_y", b"y_val"));

    let digest1 = compute_delta_digest(&normalized);

    // Reversing initial ordering between distinct keys does not alter normalized result
    let raw_reordered = vec![
        StateMutation::put(b"key_y", b"y_val"),
        StateMutation::put(b"key_x", b"first"),
        StateMutation::delete(b"key_x"),
        StateMutation::put(b"key_x", b"final_winner"),
    ];
    let normalized2 = normalize_mutations(&raw_reordered);
    assert_eq!(normalized, normalized2);
    assert_eq!(digest1, compute_delta_digest(&normalized2));
}

#[test]
fn test_a012_008_journal_staging_cannot_mutate_committed_state() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"base", b"root_val")]);
    let root_before = tree.compute_state_root();

    let mut journal = StateJournal::new();
    journal
        .stage(StateMutation::put(b"base", b"overwritten_staged"))
        .unwrap();
    journal
        .stage(StateMutation::put(b"staged_only", b"uncommitted"))
        .unwrap();

    assert_eq!(journal.status(), JournalStageStatus::Open);
    assert_eq!(journal.staged_len(), 2);

    // Committed tree remains completely isolated
    assert_eq!(tree.get(b"base"), Some(b"root_val".as_slice()));
    assert_eq!(tree.get(b"staged_only"), None);
    assert_eq!(tree.compute_state_root(), root_before);
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_a012_009_discard_leaves_root_and_revision_unchanged() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"stable", b"v")]);
    let root_before = tree.compute_state_root();

    let mut journal = StateJournal::new();
    journal
        .stage(StateMutation::put(b"stable", b"discard_me"))
        .unwrap();
    journal.discard();

    assert_eq!(journal.status(), JournalStageStatus::RolledBack);
    assert_eq!(journal.staged_len(), 0);

    assert_eq!(tree.compute_state_root(), root_before);
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_a012_010_apply_atomically_installs_normalized_state() {
    let mut tree = StateTree::new();
    let mut journal = StateJournal::new();

    journal.stage(StateMutation::put(b"k1", b"init")).unwrap();
    journal.stage(StateMutation::put(b"k2", b"v2")).unwrap();
    journal.stage(StateMutation::put(b"k1", b"final")).unwrap();

    let changed = journal.apply(&mut tree).unwrap();
    assert!(changed);
    assert_eq!(journal.status(), JournalStageStatus::Committed);

    assert_eq!(tree.get(b"k1"), Some(b"final".as_slice()));
    assert_eq!(tree.get(b"k2"), Some(b"v2".as_slice()));
    assert_eq!(tree.revision(), 1);
}

#[test]
fn test_a012_011_successful_non_empty_commit_increments_revision_once() {
    let mut tree = StateTree::new();
    assert_eq!(tree.revision(), 0);

    let mut j1 = StateJournal::new();
    j1.stage(StateMutation::put(b"k1", b"v1")).unwrap();
    j1.apply(&mut tree).unwrap();
    assert_eq!(tree.revision(), 1);

    let mut j2 = StateJournal::new();
    j2.stage(StateMutation::put(b"k2", b"v2")).unwrap();
    j2.apply(&mut tree).unwrap();
    assert_eq!(tree.revision(), 2);
}

#[test]
fn test_a012_012_zero_effective_mutation_preserves_root_and_revision() {
    let mut tree = StateTree::new();
    tree.apply_raw_mutations(&[StateMutation::put(b"key_a", b"val_a")]);
    let root_before = tree.compute_state_root();
    assert_eq!(tree.revision(), 1);

    // Apply identical value and delete absent key in a single transaction
    let mut journal = StateJournal::new();
    journal
        .stage(StateMutation::put(b"key_a", b"val_a"))
        .unwrap();
    journal.stage(StateMutation::delete(b"absent_key")).unwrap();

    let changed = journal.apply(&mut tree).unwrap();
    assert!(!changed);
    assert_eq!(tree.compute_state_root(), root_before);
    assert_eq!(tree.revision(), 1); // Preserves exact revision 1
}
