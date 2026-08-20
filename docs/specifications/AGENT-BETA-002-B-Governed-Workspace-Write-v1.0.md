# AGENT-BETA-002-B-Governed-Workspace-Write-v1.0.md

**Status:** APPROVED / SPECIFICATION RECORD
**Class:** EXPERIMENTAL / NON-PRODUCTION WORKLOAD EFFECT
**Scope:** Defines normative controls for bounded workspace.write.create operations, strict target/payload digest binding, and filesystem indirection/symlink/reparse-point confinement resistance.

---

## 1. Executive Summary
AGENT-BETA-002-B extends the workload capability track to include controlled write effects. It enforces create-only write semantics, absolute payload digest binding (AuthorizedDigest = ExecutedPayloadDigest), and rigorous protection against symbolic links, Windows junctions, and directory reparse-point escapes.

---

## 2. Normative Invariants
* **INVARIANT-100 (Read/Write Separation):** workspace.read ≠ workspace.write. Read capability never authorizes a write.
* **INVARIANT-101 & 102 (Resolved Confinement & Indirection Resistance):** Actual host filesystem targets must resolve and remain strictly within the granted write root, resisting symlinks, junctions, and ../ traversal.
* **INVARIANT-103 & 104 (Operation Non-Transitivity & Payload Binding):** Successful write #1 does not imply write #2, and executed bytes must strictly match the cryptographic digest bound to the enforcement decision.
* **INVARIANT-105 & 106 (Create-Only & Work Product Non-Authority):** Existing destinations fail closed under create-only semantics (Existing + Create = DENY), and written file data remains strictly inert.
