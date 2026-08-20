# AGENT-BETA-006-Governed-Commit-Construction-Charter-v1.0.md

**Status:** APPROVED / EXPERIMENTAL TRACK CHARTER
**Class:** EXPERIMENTAL / NON-PRODUCTION VERSION-CONTROL HISTORY CONSTRUCTION
**Scope:** Charters the commit construction track. Establishes that an exact Git commit object may be constructed from a verified index state via epository.commit.construct_exact, but neither commit verification nor construction confers branch movement (HEAD/ref update), merge, push, or signature authority.

---

## 1. Executive Summary
AGENT-BETA-006 bridges verified index states and immutable Git commit objects. It enforces host-derived tree and parent binding (INVARIANT-274 & 275), host-bound identity and timestamp enforcement (INVARIANT-277 & 278), and guarantees that reference movement remains completely unauthorized.

---

## 2. Core Principles & Invariants
* **INVARIANT-273 (Verified Index Is Precondition, Not Commit Authority):** A VERIFIED staging state does not automatically grant commit construction privileges. Commit construction requires independent governance.
* **INVARIANT-274 & 275 (Host-Derived Tree & Parent Binding):** Commit trees and parent commits derive strictly from host-observed state; model-supplied or candidate-embedded overrides are rejected.
* **INVARIANT-277 & 278 (Host-Bound Identity & Timestamps):** Author and committer identities and timestamps originate exclusively from governed host policy sources.
* **INVARIANT-283 (Commit Construction Does Not Move References):** Constructed commit objects exist independently in the object database without moving HEAD, branches, or tags.
