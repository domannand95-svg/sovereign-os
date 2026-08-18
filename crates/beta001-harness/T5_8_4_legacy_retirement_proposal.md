# Sovereign OS BETA-001: Legacy Ingestion Retirement Proposal (T5.8.4)

## 1. Objective
Formally authorize the deprecation and removal of the legacy direct-JSON integration ingestion paths, transitioning entirely to the CorpusCase orchestration envelope. 

## 2. Invariant Guarantee: Execution Semantics Immutable
The underlying CandidateTrace payloads, their target directories (ixtures/golden/, ixtures/runtime_violations/, etc.), and the evaluator.rs logic remain **strictly untouched**. The retirement applies exclusively to the test harness's observation/ingestion layer.

## 3. Retirement Targets (T5.9 Scope)
- **Target File:** crates/beta001-harness/tests/corpus_integration_test.rs
- **Target Functions:** Legacy helper functions (e.g., candidate_from_fixture() and raw serde_json::Value parsing).

## 4. Replacement Authority
- **New Ingestion Path:** CorpusCase::load_fixture() resolving candidate_fixture payloads.
- **Migration Governance:** crates/beta001-harness/fixtures/T5_8_migration_manifest.json (Parity proven for 12/12 fixtures).
- **Validation Matrix:** Assertions are now fully handled by 	5_7_migration_phase1 through phase4.

## 5. Rollback Conditions
Should T5.9 introduce any downstream regressions, the authorized rollback protocol is a standard cryptographic git revert targeting the T5.8.4 baseline commit.

**Status:** APPROVED FOR T5.9 EXECUTION PENDING FINAL AUTHORIZATION.