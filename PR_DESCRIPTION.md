## feat(beta025): sovereign-execution-api + governed execution boundary (BETA-014?BETA-025)

### Overview

This PR introduces the \sovereign-execution-api\ crate, establishing a formal translation boundary between the internal \GovernedExecutor\ kernel and external consumers (e.g., the upcoming Base44 application layer). The execution kernel remains protected by strict epistemic isolation: external systems can request governed effects but cannot bypass authorization, authentication, or integrity verification.

### Architectural Invariants Preserved

- **AuthorizationReceipt ? Execution capability** — Receipts are proof input, not direct authority.
- **Digest ? Signature** — Content integrity and cryptographic identity remain separate concerns.
- **Authentication ? Authorization** — The executor validates both before effecting mutation.
- **Fail-closed errors** — All unauthenticated or mismatched operations terminate without side effects.

### Dependency Model

\\\	ext
sovereign-audit
  +-- provides AuthorizationReceipt, ReceiptAuthenticationResult

sovereign-execution
  +-- consumes authenticated authority
  +-- exposes GovernedExecutor

sovereign-execution-api (NEW)
  +-- consumes sovereign-execution + sovereign-audit
  +-- exposes GovernedExecutionRequest / Response + ExecutionApiError
  +-- purpose: sole translation boundary for external consumers
\\\

### Crate Structure
\\\	ext
crates/sovereign-execution-api/
+-- src/
¦   +-- lib.rs          # Public API surface
¦   +-- request.rs      # GovernedExecutionRequest DTO
¦   +-- response.rs     # GovernedExecutionResponse, ExecutionStatus
¦   +-- adapter.rs      # KernelInvoker and ExecutionApiFacade
¦   +-- error.rs        # ExecutionApiError boundary
+-- Cargo.toml
\\\

### Execution Path
\Receipt\ 
  ? \Authentication Result\ 
      ? \Operation Match\ 
          ? \Content Integrity\ 
              ? \Filesystem Effect\ 
                  ? \ExecutionReport\

### Commit History & Merge Strategy

This branch contains **78 commits** representing the complete forensic audit trail from BETA-014 through BETA-025. The history documents:
- BETA-014: Federated policy evaluation
- BETA-015: Lifecycle boundaries and approval gates
- BETA-016: Authorization receipt formalization and execution replay
- BETA-025: Execution kernel, adapter, and API isolation boundary

?? **MERGE DIRECTIVE:** Maintainers **MUST** merge this branch using \--no-ff\ (Create a merge commit) to preserve the historical forensic chain. **Do not squash.** Minor \ustfmt\ normalization drift on older files is expected and should be accepted as the new baseline.

### Documentation Artifacts

- \docs/reports/BETA-025-CLOSURE-RECORD.md\ — Formal phase closure record
- \docs/handover/SOVEREIGN_OS_BETA025_EXECUTION_BOUNDARY.md\ — Definitive boundary specification
- \docs/INDEX.md\ — Updated architecture index

### Validation

\\\ash
cargo check --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace
\\\

---

### Reviewer Checklist

**Notice to Reviewers:** Please ensure you are reviewing the structural boundaries, not just the code. 

- [ ] **Epistemic Isolation:** Verify that \sovereign-execution-api\ strictly encapsulates the \GovernedExecutor\ without leaking internal kernel state.
- [ ] **DTO Enforcement:** Confirm \equest.rs\ and \esponse.rs\ enforce the strict boundary between external serialization and internal execution structures.
- [ ] **Zero-Trust Adherence:** Ensure no policy evaluation, cryptographic signing, or capability issuance logic has bled into the API facade.
- [ ] **Closure Record:** Review \docs/reports/BETA-025-CLOSURE-RECORD.md\ to ensure all architectural invariants are accurately represented.
