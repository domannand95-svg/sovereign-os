**Document:** SOL Identity Canon

**Version:** 0.1

**Status:** Active Identity Baseline

**Scope:** Non-normative project identity, architectural language, and visual direction

**Authority:** Subordinate to normative specifications and merged architecture decisions

**Change Rule:** Identity language must not introduce or imply technical authority absent from the normative architecture

---

# SOL Identity Canon v0.1

## 1. Voice Principles: Dual-Register Architecture

The SOL identity operates through a dual-register voice. The normative voice ensures procedural safety and auditability; the identity voice provides conceptual weight and architectural clarity. They never blur: normative documentation remains plain, exact, and fail-closed, while architectural essays and structural overviews use sparse, solar-aligned language.

* **Normative Register:** Procedural, exact, versioned, and fail-closed. Devoid of metaphor. Governs specification text, error mappings, and code invariants.
* **Identity Register:** Monumental, sparse, architectural. Treats system state and cryptographic boundaries with the gravity of physical law.

---

## 2. Axioms

### Primary Axiom

> **Probabilistic intelligence; deterministic authority.**

Intelligence may explore, infer, and adapt; authority must remain calculated, absolute, and reproducible. Intelligence proposes; the core admits.

### Secondary Axiom

> **Intelligence may explore beyond the boundary. Effects may not.**

Probabilistic processes may reason, speculate, and request capabilities without those operations automatically acquiring permission to alter external state.

---

## 3. The Identity Canon Lexicon

### Category A: Canonical Technical Metaphors

*These terms map directly to established engineering invariants and may safely appear in architectural prose.*

| Term | Technical Mapping | Invariant / Rule |
| --- | --- | --- |
| **Core** | The deterministic Capability V1 admission boundary presently implemented through Gates 1–6. | The core admits or rejects according to explicitly allocated validation and authorization rules; ambient authority is not inferred. |
| **State** | Explicit authoritative context and state references used by admission and replay. | Authority is evaluated against the bound context; ambient current/latest/default state cannot silently substitute for it. |
| **Boundary (Eclipse Boundary)** | The strict validation perimeter separating admitted state from rejected inputs. | What cannot be verified does not cross the boundary. (*Unverifiable authority = no authority.*) |
| **Admission** | The successful traversal of the Gate 1–6 verification sequence. | Admission is an all-or-nothing phase transition, never a partial state. |
| **Replay** | Historical audit and deterministic execution re-evaluation. | Replay follows the exact same trajectory when supplied with the original admission context. |
| **Proof** | Cryptographic content addressing (`Caid`) and structural coherence. | Absence of proof is identical to absence of authority. |
| **Illumination** | Authoritative identity, issuer-state, and governing-policy resolution across Gates 3B, 5, and 6. | Authority must be resolved against its authoritative context before it can be exercised. |

### Category B: Identity-Language Metaphors

*These terms remain strictly within the identity and conceptual layer to prevent branding vocabulary from leaking into technical specifications or API terminology.*

| Term | Conceptual Mapping | Architectural Intent |
| --- | --- | --- |
| **Corona** | Outer runtimes, agents, and client integrations operating around the control plane. | **Architectural Principle:** The corona participates; as the broader ecosystem matures, it is bounded by the core. |
| **Orbit** | Governed participation paths and operational lifecycles. | State defines the orbit; valid actions adhere to historical boundaries. |
| **Gravity** | Fail-closed evaluation rules and cryptographic binding. | Gravity does not negotiate; invalid payloads or missing proofs immediately collapse the pipeline. |

---

## 4. Canon Phrases

These axioms anchor the project’s conceptual framework across documentation intros, architectural overviews, and system boundaries:

* *Probabilistic intelligence; deterministic authority.*
* *Intelligence may explore beyond the boundary. Effects may not.*
* *The core governs; the ecosystem participates.*
* *Authority must be resolved against its authoritative context before it can be exercised.*
* *What cannot be verified does not cross the boundary.*
* *Unverifiable authority is extinguished at the perimeter.*

---

## 5. Visual System (Derived from Language)

The visual system translates these fixed linguistic and architectural concepts directly into UI components and documentation design:

* **The Dark Field (The Vacuum):** Deep, light-absorbing backgrounds (`#0A0A0C`) representing the absolute fail-closed perimeter where unverified operations cannot exist.
* **Solar Accents (The Core):** Incandescent gold or restrained amber (`#FFB020`) used exclusively for successfully admitted proofs, active gates, and authoritative state pointers.
* **Thin Boundary Geometry (The Eclipse Boundary):** Razor-thin, high-contrast borders separating verified data from untrusted inputs, echoing the sharp silhouette of total occlusion.
* **Orbital Diagrams:** Clean, geometric visualizations of the gate sequence where data passes from structural decoding to policy authorization in concentric, irreversible rings.
* **Archival Typography:** Combining hyper-precise, monospaced data readouts with monumental, editorial serifs for structural headings—balancing machine readability with legal-grade permanence.
