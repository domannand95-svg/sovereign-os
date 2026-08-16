---
Document: Governed Research Artifacts and Evidence
ID: SPEC-EV-001
Version: 0.1
Status: Approved Implementation Baseline
Owner: Repository Maintainer
Reviewers:
- Core Reviewers
Last Updated: 2026-08-11
Evidence:
- A04 delivery-backlog contract
Depends On:
- ADR-0002
- SPEC-ID-001
Supersedes: None
---

# SPEC-EV-001: Governed Research Artifacts and Evidence

## 1. Purpose

This specification defines the canonical records required to represent a
research objective, claim, source, method, uncertainty, failed attempt,
reviewer finding, dispute, and disposition without silently losing provenance
or collapsing evidence, review, policy, and promotion authority.

It is the A04 contract for later Rust implementation. It does not authorize
tool execution, capability grants, artifact promotion, model training,
repository mutation, or production integration.

## 2. Design Invariants

Every conforming implementation shall preserve these invariants:

1. Every record has an explicit schema version and record kind.
2. Every record has a content-derived identifier computed from its complete
   canonical encoding.
3. Every actor, tool, policy, dataset, and institution reference uses a
   canonical identity defined by `SPEC-ID-001`.
4. Evidence is append-only. Correction and supersession create new records and
   preserve links to the records they correct or replace.
5. Negative evidence, failed attempts, disputes, and minority findings cannot
   be omitted from a disposition that claims to evaluate their subject.
6. A reviewer finding is evidence, not promotion authority.
7. A disposition records a decision made by a separately identified authority;
   it cannot certify its own subject or silently create a capability grant.
8. Unknown versions, kinds, required fields, enum values, invalid references,
   duplicate set members, and non-canonical encodings fail closed.
9. Decoding followed by encoding yields byte-identical canonical content.
10. Human-readable projections are non-authoritative views of canonical bytes.

## 3. Common Envelope

Every record shall contain the following fields in this exact logical order:

| Field | Type | Requirement |
| --- | --- | --- |
| `schema_version` | `u16` | Must equal the supported version |
| `record_kind` | `u8` | Stable discriminator from section 4 |
| `issuer_id` | `IdentityId` | Actor asserting or creating the record |
| `subject_id` | `IdentityId` | Governed subject; never inferred from issuer |
| `policy_id` | `IdentityId` | Policy governing creation or admission |
| `parent_ids` | ordered unique list of record IDs | Provenance and supersession ancestry |
| `payload` | kind-specific canonical bytes | Required fields for the selected kind |

The record ID is not encoded inside the content from which it is derived:

```text
record_id = BLAKE3("SOVEREIGN_EVIDENCE_V1" || canonical_record_bytes)
```

An all-zero identity or record ID is invalid. Parent order is meaningful and
shall be preserved. Duplicate parent IDs are invalid.

## 4. Record Kinds

Stable v1 discriminators are:

| Value | Kind |
| ---: | --- |
| `0x01` | Objective |
| `0x02` | Claim |
| `0x03` | Source |
| `0x04` | Method |
| `0x05` | Uncertainty |
| `0x06` | Failed Attempt |
| `0x07` | Reviewer Finding |
| `0x08` | Dispute |
| `0x09` | Disposition |

Discriminators shall never be reused. A new meaning requires a new value or a
new schema version.

## 5. Canonical Field Encoding

The Rust implementation shall define one allocation-bounded binary encoding.
For v1, every payload field is encoded in the order listed by its schema:

- unsigned integers use big-endian encoding;
- enums use stable unsigned discriminators;
- text is length-prefixed UTF-8 and must be valid, non-empty where required,
  and free of NUL characters;
- byte strings are length-prefixed and bounded;
- identifiers use their fixed canonical byte representation;
- lists are count-prefixed, bounded, and preserve declared order;
- set-like lists must be sorted by canonical identifier bytes and contain no
  duplicates; and
- optional fields use a one-byte presence discriminator followed by content
  only when present.

Decoders shall reject trailing bytes, truncation, integer overflow, invalid
UTF-8, unsupported discriminators, non-canonical ordering, and declared lengths
that exceed the remaining input or governed maximum.

Exact maximum lengths and temporal rules are frozen by
[`SPEC-EV-001-ENC-001`](SPEC-EV-001-A04-Binary-Encoding.md). Fixed vectors are
frozen with each typed Rust payload implementation.

## 6. Objective Record

An Objective payload contains:

| Field | Requirement |
| --- | --- |
| `statement` | Required bounded text describing the question or outcome |
| `scope` | Required bounded text defining included and excluded work |
| `success_criteria` | Non-empty ordered list of bounded criteria |
| `constraints` | Ordered list of declared legal, ethical, resource, or policy constraints |
| `expires_at` | Optional asserted UTC instant; never generated during decoding |

An objective does not grant tools, data, budget, execution, or promotion rights.

## 7. Claim Record

A Claim payload contains:

| Field | Requirement |
| --- | --- |
| `objective_id` | Required Objective record ID |
| `statement` | Required bounded proposition |
| `claim_kind` | Observation, inference, prediction, recommendation, or normative claim |
| `substantiation` | Explicit `UNSUBSTANTIATED` (`0x00`) or `EVIDENCE_CITED` (`0x01`) state |
| `supporting_evidence_ids` | Ordered unique evidence record IDs |
| `counter_evidence_ids` | Ordered unique evidence record IDs |
| `uncertainty_ids` | Ordered unique Uncertainty record IDs |

A claim with no supporting evidence is permitted only as explicitly
`UNSUBSTANTIATED`. It cannot be silently treated as verified.

`UNSUBSTANTIATED` requires an empty supporting-evidence list.
`EVIDENCE_CITED` requires a non-empty supporting-evidence list and asserts only
that identifiers are present; it confers no validity, truth, review, or
disposition authority. Other substantiation values fail closed.

## 8. Source Record

A Source payload contains:

| Field | Requirement |
| --- | --- |
| `locator` | Required bounded locator or repository-relative reference |
| `content_digest` | Required algorithm tag and digest of retrieved bytes |
| `retrieved_at` | Required asserted UTC instant |
| `media_type` | Required bounded media type |
| `publication_date_text` | Optional exact descriptive publication date text |

Source v1 preserves the exact asserted locator without normalization. Its
digest is one algorithm byte (`0x01` SHA-256 or `0x02` BLAKE3) followed by
exactly 32 bytes. `retrieved_at` is required unsigned Unix seconds. Media type
is a lowercase ASCII `type/subtype` without parameters. Historical publication
text is optional, descriptive, and never parsed as an execution timestamp.

A Source asserts only that matching bytes were retrieved at the asserted time
from the asserted locator. It does not prove retrieval, continued resolvability,
authenticity, truth, claim support, verification, or promotion authority.
| `licence` | Required bounded licence expression or explicit `UNKNOWN` |
| `access_class` | Public, internal, confidential, restricted, or unknown |
| `custodian_id` | Required canonical identity |

The locator is not authoritative identity. A changed digest creates distinct
source evidence even when the locator is unchanged.

## 9. Method Record

A Method payload contains:

| Field | Requirement |
| --- | --- |
| `objective_id` | Required Objective record ID |
| `procedure` | Required bounded, reproducible procedure description |
| `input_ids` | Ordered unique source, dataset, or artifact references |
| `tool_ids` | Ordered unique canonical tool identities |
| `environment_id` | Required canonical environment identity |
| `parameters_digest` | Required digest of canonical parameters |
| `budget_reference` | Optional future A05 grant reference |

The optional budget reference is descriptive until A05 is accepted and cannot
authorize resource consumption.

## 10. Uncertainty Record

An Uncertainty payload contains:

| Field | Requirement |
| --- | --- |
| `about_id` | Required governed record ID |
| `uncertainty_kind` | Measurement, model, sampling, source, interpretation, or unknown |
| `statement` | Required bounded explanation |
| `lower_bound` / `upper_bound` | Optional canonical decimal strings; both required as a pair |
| `confidence_basis` | Required bounded explanation; never a universal trust score |

Bounds must use one frozen decimal grammar. NaN, infinity, locale-specific
formatting, and a lower bound greater than the upper bound are invalid.

For schema version 1, a canonical decimal is either exactly `0`, or has an
optional `-`, an integer component of `0` or a non-zero ASCII digit followed by
zero or more ASCII digits, and an optional fractional component introduced by
`.`. A present fractional component contains one or more ASCII digits and ends
in a non-zero digit. Therefore `0.01`, `-0.01`, `1`, and `10.25` are valid;
`+1`, `01`, `.5`, `1.`, `1.0`, `-0`, exponent notation, NaN, infinity,
whitespace, non-ASCII digits, and locale-specific separators are invalid.
Implementations compare canonical decimals exactly and must not convert them to
binary floating point.

The two bounds form one optional pair. Their presence does not admit evidence,
authorize action, confer confidence, or establish a universal trust score.

## 11. Failed Attempt Record

A Failed Attempt payload contains:

| Field | Requirement |
| --- | --- |
| `objective_id` | Required Objective record ID |
| `method_id` | Required Method record ID |
| `failure_kind` | Invalid input, method failure, tool failure, timeout, inconclusive, or policy denial |
| `observed_result` | Required bounded factual description |
| `evidence_ids` | Ordered unique evidence references, including negative evidence |
| `retry_of` | Optional earlier Failed Attempt ID |

Retries append evidence. They do not replace the failed attempt.

The order of `evidence_ids` is preserved and duplicates are invalid. An empty
list is permitted when the bounded observed result is the only evidence
available at record construction. The optional `retry_of` reference describes
lineage only; record-kind, temporal ordering, and cycle checks remain admission
policy concerns. A policy-denial failure records an observed denial and does not
itself grant, expand, revoke, or exercise authority.

## 12. Reviewer Finding Record

A Reviewer Finding payload contains:

| Field | Requirement |
| --- | --- |
| `reviewed_id` | Required governed record ID |
| `reviewer_id` | Required canonical identity, distinct from the reviewed subject where policy requires |
| `finding_kind` | Support, contradiction, methodological flaw, provenance gap, security, privacy/licensing, or inconclusive |
| `severity` | Informational, low, medium, high, or critical |
| `rationale` | Required bounded explanation |
| `evidence_ids` | Ordered unique supporting evidence references |
| `conflict_declaration` | Required bounded declaration, including explicit `NONE_DECLARED` |
| `independence_result` | Established, not established, conflicted, or unknown |

Unknown independence fails closed for any transition whose policy requires an
independent reviewer.

## 13. Dispute Record

A Dispute payload contains:

| Field | Requirement |
| --- | --- |
| `disputed_id` | Required governed record ID |
| `position_ids` | At least two ordered unique Claim or Reviewer Finding IDs |
| `opened_by` | Required canonical identity |
| `status` | Open, under review, resolved, or withdrawn |
| `resolution_id` | Required only when resolved; points to a later Disposition |

Resolution does not delete or rewrite any position.

## 14. Disposition Record

A Disposition payload contains:

| Field | Requirement |
| --- | --- |
| `decided_id` | Required governed record ID |
| `decision` | Accept for review, reject, defer, quarantine, supersede, or revoke |
| `decision_authority_id` | Required canonical identity distinct from the subject |
| `criteria_id` | Required versioned criteria or policy identity |
| `evidence_ids` | Ordered unique evidence considered |
| `unresolved_dispute_ids` | Complete ordered list of known unresolved disputes |
| `reason` | Required bounded explanation |
| `supersedes_id` | Required only for supersession or revocation where applicable |

This record is an evidence disposition only. `Accept for review` is not
production promotion, tool authority, a capability grant, or permission to
mutate a repository.

## 15. Cross-Record Validation

Schema decoding validates local canonical form. Admission policy separately
validates graph relationships and shall reject:

- references to absent or wrong-kind records;
- self-review, self-disposition, or an issuer presented as independent from
  itself or a disqualifying common controller;
- a resolved dispute without a valid resolution disposition;
- disposition evidence that omits a known failed attempt, negative finding, or
  unresolved dispute required by policy;
- cycles in supersession, retry, or resolution relationships;
- use of an A05 budget or grant reference before its governing schema is active;
- a policy, criteria, tool, dataset, or environment reference lacking canonical
  identity; and
- retroactive mutation of an admitted record.

## 16. Required Implementation Tests

The A04 Rust implementation is not complete until tests cover:

1. byte-stable round trips for every record kind;
2. fixed identity vectors for every record kind;
3. truncation at every field boundary;
4. trailing-byte rejection;
5. unknown version, kind, and enum rejection;
6. invalid UTF-8, NUL, oversize, count overflow, and length overflow;
7. duplicate and non-canonical set ordering rejection;
8. all-zero identity and record-reference rejection;
9. missing provenance fields for every record kind;
10. failed-attempt and negative-evidence preservation;
11. self-review and self-disposition rejection at admission;
12. unresolved-dispute preservation;
13. decode failure without partial live-state mutation; and
14. fuzz targets for every canonical decoder.

The full authoritative workspace, Windows/Linux CI, coverage, dependency,
security, and fuzz gates remain mandatory.

## 17. Implementation Boundary

The first implementation shall add only canonical types, encoding, decoding,
local validation, fixed vectors, and malformed-input tests. Ledger admission,
query projections, capability decisions, tool execution, promotion, and BKI
integration remain separate later items.

## 18. Acceptance Decision

The owner-approved encoding baseline authorizes the A04 schema implementation
slice. It does not activate any runtime authority described elsewhere in the
Sovereign OS architecture.
