//! BETA-003: deterministic three-way effect reconciliation.
//!
//! This module compares inert expectation, committed-receipt, and observation
//! values. It performs no effects and confers no execution authority.

use crate::service_contract::AuthorityDeltaZero;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedEffect {
    pub path: String,
    pub content_digest: String,
    pub lineage_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectExpectation {
    pub execution_id: String,
    pub filesystem_root: String,
    pub effects: Vec<ExpectedEffect>,
}

impl EffectExpectation {
    pub fn canonical_digest(&self) -> String {
        let mut effects = self.effects.clone();
        effects.sort_by(|left, right| {
            (&left.path, &left.content_digest, &left.lineage_digest).cmp(&(
                &right.path,
                &right.content_digest,
                &right.lineage_digest,
            ))
        });

        let mut hasher = Sha256::new();
        hash_field(&mut hasher, b"BETA003_EFFECT_EXPECTATION_V1");
        hash_field(&mut hasher, self.execution_id.as_bytes());
        hash_field(&mut hasher, self.filesystem_root.as_bytes());
        for effect in effects {
            hash_field(&mut hasher, effect.path.as_bytes());
            hash_field(&mut hasher, effect.content_digest.as_bytes());
            hash_field(&mut hasher, effect.lineage_digest.as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptStatus {
    Committed,
    NotCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedExecutionReceipt {
    pub execution_id: String,
    pub expectation_digest: String,
    pub status: ReceiptStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedKernelEffect {
    pub path: String,
    pub content_digest: String,
    pub lineage_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedKernelEffects {
    pub execution_id: String,
    pub effects: Vec<ObservedKernelEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DivergenceFinding {
    FilesystemEscape { path: String },
    UndeclaredExternalEffect { path: String },
    DigestOrLineageMismatch { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationAttestation {
    pub execution_id: String,
    pub expectation_digest: String,
    pub reconciled: bool,
    pub findings: Vec<DivergenceFinding>,
    pub authority_delta: AuthorityDeltaZero,
}

pub fn reconcile_effects(
    expectation: &EffectExpectation,
    receipt: &CommittedExecutionReceipt,
    observation: &ObservedKernelEffects,
) -> ReconciliationAttestation {
    let expectation_digest = expectation.canonical_digest();
    let mut findings = BTreeSet::new();

    let root = normalize_absolute_path(&expectation.filesystem_root);
    if root.is_none() {
        findings.insert(DivergenceFinding::FilesystemEscape {
            path: expectation.filesystem_root.clone(),
        });
    }

    if receipt.status != ReceiptStatus::Committed
        || receipt.execution_id != expectation.execution_id
        || observation.execution_id != expectation.execution_id
        || receipt.expectation_digest != expectation_digest
    {
        findings.insert(DivergenceFinding::DigestOrLineageMismatch {
            path: "<execution-binding>".to_string(),
        });
    }

    let mut expected_by_path = BTreeMap::new();
    for expected in &expectation.effects {
        let Some(path) = normalize_absolute_path(&expected.path) else {
            findings.insert(DivergenceFinding::FilesystemEscape {
                path: expected.path.clone(),
            });
            continue;
        };
        if root
            .as_deref()
            .is_none_or(|normalized_root| !is_within(normalized_root, &path))
        {
            findings.insert(DivergenceFinding::FilesystemEscape { path });
            continue;
        }
        if expected_by_path.insert(path.clone(), expected).is_some() {
            findings.insert(DivergenceFinding::DigestOrLineageMismatch { path });
        }
    }

    let mut observed_paths = BTreeSet::new();
    for observed in &observation.effects {
        let Some(path) = normalize_absolute_path(&observed.path) else {
            findings.insert(DivergenceFinding::FilesystemEscape {
                path: observed.path.clone(),
            });
            continue;
        };
        if root
            .as_deref()
            .is_none_or(|normalized_root| !is_within(normalized_root, &path))
        {
            findings.insert(DivergenceFinding::FilesystemEscape { path });
            continue;
        }
        if !observed_paths.insert(path.clone()) {
            findings.insert(DivergenceFinding::UndeclaredExternalEffect { path });
            continue;
        }
        let Some(expected) = expected_by_path.get(&path) else {
            findings.insert(DivergenceFinding::UndeclaredExternalEffect { path });
            continue;
        };
        if observed.content_digest != expected.content_digest
            || observed.lineage_digest != expected.lineage_digest
        {
            findings.insert(DivergenceFinding::DigestOrLineageMismatch { path });
        }
    }

    for path in expected_by_path.keys() {
        if !observed_paths.contains(path) {
            findings.insert(DivergenceFinding::DigestOrLineageMismatch { path: path.clone() });
        }
    }

    let findings: Vec<_> = findings.into_iter().collect();
    ReconciliationAttestation {
        execution_id: expectation.execution_id.clone(),
        expectation_digest,
        reconciled: findings.is_empty(),
        findings,
        authority_delta: AuthorityDeltaZero,
    }
}

fn hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn normalize_absolute_path(path: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    if !path.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            value => parts.push(value),
        }
    }
    Some(if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    })
}

fn is_within(root: &str, path: &str) -> bool {
    root == "/"
        || path == root
        || path
            .strip_prefix(root)
            .is_some_and(|tail| tail.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected(path: &str, content: &str, lineage: &str) -> ExpectedEffect {
        ExpectedEffect {
            path: path.into(),
            content_digest: content.into(),
            lineage_digest: lineage.into(),
        }
    }

    fn observed(path: &str, content: &str, lineage: &str) -> ObservedKernelEffect {
        ObservedKernelEffect {
            path: path.into(),
            content_digest: content.into(),
            lineage_digest: lineage.into(),
        }
    }

    fn expectation(effects: Vec<ExpectedEffect>) -> EffectExpectation {
        EffectExpectation {
            execution_id: "exec-003".into(),
            filesystem_root: "/workspace".into(),
            effects,
        }
    }

    fn receipt(expectation: &EffectExpectation) -> CommittedExecutionReceipt {
        CommittedExecutionReceipt {
            execution_id: expectation.execution_id.clone(),
            expectation_digest: expectation.canonical_digest(),
            status: ReceiptStatus::Committed,
        }
    }

    fn observations(effects: Vec<ObservedKernelEffect>) -> ObservedKernelEffects {
        ObservedKernelEffects {
            execution_id: "exec-003".into(),
            effects,
        }
    }

    fn reconcile(
        expectation: &EffectExpectation,
        observations: ObservedKernelEffects,
    ) -> ReconciliationAttestation {
        reconcile_effects(expectation, &receipt(expectation), &observations)
    }

    #[test]
    fn b3_oe_001_exact_three_way_match_reconciles() {
        let e = expectation(vec![expected("/workspace/a", "d1", "l1")]);
        let result = reconcile(&e, observations(vec![observed("/workspace/a", "d1", "l1")]));
        assert!(result.reconciled);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn b3_oe_002_empty_expected_and_observed_effects_reconcile() {
        let e = expectation(vec![]);
        assert!(reconcile(&e, observations(vec![])).reconciled);
    }

    #[test]
    fn b3_oe_003_observed_path_escape_is_denied() {
        let e = expectation(vec![]);
        let result = reconcile(&e, observations(vec![observed("/outside/a", "d1", "l1")]));
        assert_eq!(
            result.findings,
            vec![DivergenceFinding::FilesystemEscape {
                path: "/outside/a".into()
            }]
        );
    }

    #[test]
    fn b3_oe_004_parent_traversal_is_denied() {
        let e = expectation(vec![]);
        let result = reconcile(
            &e,
            observations(vec![observed("/workspace/../escape", "d1", "l1")]),
        );
        assert!(matches!(
            result.findings[0],
            DivergenceFinding::FilesystemEscape { .. }
        ));
    }

    #[test]
    fn b3_oe_005_relative_observed_path_is_denied() {
        let e = expectation(vec![]);
        let result = reconcile(&e, observations(vec![observed("relative", "d1", "l1")]));
        assert!(matches!(
            result.findings[0],
            DivergenceFinding::FilesystemEscape { .. }
        ));
    }

    #[test]
    fn b3_oe_006_undeclared_effect_is_denied() {
        let e = expectation(vec![]);
        let result = reconcile(
            &e,
            observations(vec![observed("/workspace/new", "d1", "l1")]),
        );
        assert_eq!(
            result.findings,
            vec![DivergenceFinding::UndeclaredExternalEffect {
                path: "/workspace/new".into()
            }]
        );
    }

    #[test]
    fn b3_oe_007_duplicate_observation_is_denied() {
        let e = expectation(vec![expected("/workspace/a", "d1", "l1")]);
        let result = reconcile(
            &e,
            observations(vec![
                observed("/workspace/a", "d1", "l1"),
                observed("/workspace/a", "d1", "l1"),
            ]),
        );
        assert!(result
            .findings
            .contains(&DivergenceFinding::UndeclaredExternalEffect {
                path: "/workspace/a".into()
            }));
    }

    #[test]
    fn b3_oe_008_content_digest_mismatch_is_denied() {
        let e = expectation(vec![expected("/workspace/a", "d1", "l1")]);
        let result = reconcile(
            &e,
            observations(vec![observed("/workspace/a", "tampered", "l1")]),
        );
        assert!(matches!(
            result.findings[0],
            DivergenceFinding::DigestOrLineageMismatch { .. }
        ));
    }

    #[test]
    fn b3_oe_009_lineage_digest_mismatch_is_denied() {
        let e = expectation(vec![expected("/workspace/a", "d1", "l1")]);
        let result = reconcile(
            &e,
            observations(vec![observed("/workspace/a", "d1", "tampered")]),
        );
        assert!(matches!(
            result.findings[0],
            DivergenceFinding::DigestOrLineageMismatch { .. }
        ));
    }

    #[test]
    fn b3_oe_010_missing_expected_effect_is_denied() {
        let e = expectation(vec![expected("/workspace/a", "d1", "l1")]);
        let result = reconcile(&e, observations(vec![]));
        assert!(matches!(
            result.findings[0],
            DivergenceFinding::DigestOrLineageMismatch { .. }
        ));
    }

    #[test]
    fn b3_oe_011_uncommitted_receipt_is_denied() {
        let e = expectation(vec![]);
        let mut r = receipt(&e);
        r.status = ReceiptStatus::NotCommitted;
        let result = reconcile_effects(&e, &r, &observations(vec![]));
        assert!(!result.reconciled);
    }

    #[test]
    fn b3_oe_012_receipt_execution_mismatch_is_denied() {
        let e = expectation(vec![]);
        let mut r = receipt(&e);
        r.execution_id = "other".into();
        assert!(!reconcile_effects(&e, &r, &observations(vec![])).reconciled);
    }

    #[test]
    fn b3_oe_013_observation_execution_mismatch_is_denied() {
        let e = expectation(vec![]);
        let mut o = observations(vec![]);
        o.execution_id = "other".into();
        assert!(!reconcile_effects(&e, &receipt(&e), &o).reconciled);
    }

    #[test]
    fn b3_oe_014_expectation_digest_mismatch_is_denied() {
        let e = expectation(vec![]);
        let mut r = receipt(&e);
        r.expectation_digest = "tampered".into();
        assert!(!reconcile_effects(&e, &r, &observations(vec![])).reconciled);
    }

    #[test]
    fn b3_oe_015_invalid_expectation_root_is_denied() {
        let mut e = expectation(vec![]);
        e.filesystem_root = "relative".into();
        assert!(matches!(
            reconcile(&e, observations(vec![])).findings[0],
            DivergenceFinding::FilesystemEscape { .. }
        ));
    }

    #[test]
    fn b3_oe_016_expected_path_escape_is_denied() {
        let e = expectation(vec![expected("/outside/a", "d1", "l1")]);
        assert!(matches!(
            reconcile(&e, observations(vec![])).findings[0],
            DivergenceFinding::FilesystemEscape { .. }
        ));
    }

    #[test]
    fn b3_oe_017_duplicate_expectation_is_denied() {
        let e = expectation(vec![
            expected("/workspace/a", "d1", "l1"),
            expected("/workspace/a", "d1", "l1"),
        ]);
        assert!(
            !reconcile(&e, observations(vec![observed("/workspace/a", "d1", "l1")])).reconciled
        );
    }

    #[test]
    fn b3_oe_018_effect_order_does_not_change_attestation() {
        let e1 = expectation(vec![
            expected("/workspace/a", "d1", "l1"),
            expected("/workspace/b", "d2", "l2"),
        ]);
        let e2 = expectation(vec![
            expected("/workspace/b", "d2", "l2"),
            expected("/workspace/a", "d1", "l1"),
        ]);
        assert_eq!(e1.canonical_digest(), e2.canonical_digest());
        let o = observations(vec![
            observed("/workspace/b", "d2", "l2"),
            observed("/workspace/a", "d1", "l1"),
        ]);
        assert_eq!(reconcile(&e1, o.clone()), reconcile(&e2, o));
    }

    #[test]
    fn b3_oe_019_findings_are_deterministic_and_deduplicated() {
        let e = expectation(vec![]);
        let o1 = observations(vec![
            observed("/workspace/z", "d", "l"),
            observed("/workspace/a", "d", "l"),
        ]);
        let o2 = observations(vec![
            observed("/workspace/a", "d", "l"),
            observed("/workspace/z", "d", "l"),
        ]);
        assert_eq!(reconcile(&e, o1).findings, reconcile(&e, o2).findings);
    }

    #[test]
    fn b3_oe_020_attestation_authority_delta_is_zero() {
        let e = expectation(vec![]);
        let result = reconcile(&e, observations(vec![]));
        assert_eq!(result.authority_delta.value(), 0);
    }
}
