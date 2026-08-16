---
Document: A04 Governed Research Record Binary Encoding
ID: SPEC-EV-001-ENC-001
Version: 1.0
Status: Approved Baseline
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-08-16
Evidence:
  - Owner approval recorded 2026-08-11
  - Reviewer Finding v1 fixed vectors implemented 2026-08-16
  - Dispute v1 fixed vectors implemented 2026-08-16
Depends On:
  - SPEC-EV-001
  - SPEC-ID-001
Supersedes: None
---

# A04 Governed Research Record Binary Encoding

## 1. Architectural Intent

The A04 encoding rules establish a permanent, deterministic compatibility
contract for governed research records, including objectives, claims, sources,
methods, reviews, disputes, and decisions. Enforcing strict canonical encoding
ensures predictable memory usage, protection against hostile
resource-exhaustion vectors, consistent cross-machine behavior, and immutable
cryptographic identities.

## 2. Hard Limits and Resource Boundaries

Oversized or malformed values are strictly rejected prior to admission. The
system will never perform silent truncation.

- **Maximum Text Field:** 64 KiB (65,536 bytes) of valid UTF-8.
- **Maximum List Length:** 64 items per list, including success criteria,
  constraints, and evidence references.
- **Maximum Immediate Parents:** 64 direct ancestry or provenance records.
- **Maximum Overall Payload:** 1 MiB (1,048,576 bytes) total combined payload
  per record.

Exceptionally large research packages, datasets, or full papers must be stored
as separately identified external artifacts and referenced by digest. This
keeps individual governance records focused, attributable, safely reviewable,
and within the decoding budget.

## 3. Temporal Determinism

Operational timestamps, including retrieval, expiry, review, and decision
times, use the following immutable v1 representation:

- **Format:** Unsigned Unix seconds in UTC.
- **Precision:** Second-level.
- **Historical Dates:** Publication dates before 1 January 1970 UTC are not
  execution timestamps and remain separately governed descriptive metadata.
- **Floating Point:** Prohibited for timestamps.

This prevents locale ambiguity, floating-point rounding, and platform-specific
time encodings from changing record identity.

## 4. Canonical Encoding Rules

- Unsigned integers use big-endian byte order.
- Ordered lists retain their explicit governed order.
- Set-like lists are sorted exclusively by identifier bytes and reject
  duplicates.
- Text must be valid UTF-8, no longer than 65,536 bytes, and free of NUL
  characters. Required text must not be empty.
- Optional values use one explicit marker byte: `0x00` absent or `0x01`
  present. Any other marker is invalid.
- Unknown enum discriminators are rejected.
- Padding and trailing bytes are rejected.
- Declared counts and lengths are checked before allocation.
- Oversized values are rejected and never truncated.

## 5. Schema Evolution and Versioning

Schema Version 1 is immutable. Expanded field sizes, increased list capacities,
sub-second timestamps, or structural field changes require Schema Version 2.

Version 1 records remain indefinitely readable under Version 1 rules. A future
implementation must not reinterpret version 1 bytes or silently change their
identity.

## 6. Authority Boundary

Canonical encoding proves deterministic structure and identity only. It does
not admit a record to the ledger, verify a claim, approve a disposition, grant
a capability, authorize execution, or promote an artifact.

## 7. Initial Objective Fixed Vectors

The first typed Objective slice freezes these v1 vectors:

- Minimal field fixture `statement="S"`, `scope="P"`, one success criterion
  `"C"`, one constraint `"K"`, and `expires_at=42`:
  `00000001530000000150000100000001430001000000014b01000000000000002a`
- Governed Objective record fixture in the Rust test suite:
  `07f6521b9f6105551b1c356ebd92629d1f532298c180d2615b006bbff2264ade`

Changing either vector requires a new schema version rather than reinterpretation
of version 1.

## 8. Claim Fixed Vector

The typed Claim slice freezes the minimal fixture with an Objective identifier
of 32 `0x01` bytes, `statement="C"`, `claim_kind=OBSERVATION`,
`substantiation=UNSUBSTANTIATED`, and three empty evidence lists:

`010101010101010101010101010101010101010101010101010101010101010100000001430000000000000000`

The explicit substantiation byte is structural only. `EVIDENCE_CITED` never
asserts that cited evidence is true, sufficient, reviewed, or accepted.

## 9. Source Fixed Vector

Source v1 encodes fields in this order: exact locator text, one-byte digest
algorithm, exactly 32 digest bytes, required `u64` retrieval time, lowercase
bare media type, and optional descriptive publication-date text.

The minimal fixture `locator="L"`, SHA-256, a digest of 32 `0x02` bytes,
`retrieved_at=42`, `media_type="text/plain"`, and no publication text is:

`000000014c010202020202020202020202020202020202020202020202020202020202020202000000000000002a0000000a746578742f706c61696e00`

The typed governed Source record fixture in the Rust test suite has record ID:

`da049df7f223861cf14810b46c557c60ce6b7bf4e1991874bdde0319734e33da`

Changing this vector requires a new schema version rather than reinterpretation
of version 1.

## 10. Method Fixed Vector

Method v1 encodes fields in this order: Objective record identifier, procedure
text, ordered unique input record identifiers, ordered unique tool identity
identifiers, environment identity identifier, one-byte parameters-digest
algorithm, exactly 32 parameters-digest bytes, and an optional budget record
reference.

The minimal fixture uses an Objective identifier of 32 `0x01` bytes,
`procedure="P"`, no inputs, no tools, an environment identity of 32 `0x02`
bytes, SHA-256, a parameters digest of 32 `0x03` bytes, and no budget
reference:

`0101010101010101010101010101010101010101010101010101010101010101000000015000000000020202020202020202020202020202020202020202020202020202020202020201030303030303030303030303030303030303030303030303030303030303030300`

The typed governed Method record fixture in the Rust test suite has record ID:

`8aaa8fc818dd0dbbae00b092cfe5f3707549e56ef664a163ab001b8ac68ad31e`

Tool and environment identity references describe the declared method only.
They do not grant capabilities, authorize execution, admit the record, or
expand policy.

Changing this vector requires a new schema version rather than reinterpretation
of version 1.

## 11. Uncertainty Fixed Vector

Uncertainty v1 encodes fields in this order: governed record identifier being
described, one-byte uncertainty-kind discriminator, statement text, one-byte
bounds marker, the lower and upper canonical-decimal texts when that marker is
`0x01`, and confidence-basis text. Marker `0x00` means both bounds are absent.
No encoding permits only one bound to be present.

Uncertainty-kind discriminators are Measurement `0x00`, Model `0x01`, Sampling
`0x02`, Source `0x03`, Interpretation `0x04`, and Unknown `0x05`.

The minimal fixture uses an `about_id` of 32 `0x01` bytes, Measurement,
`statement="U"`, no bounds, and `confidence_basis="B"`:

`0101010101010101010101010101010101010101010101010101010101010101000000000155000000000142`

The typed governed Uncertainty record fixture in the Rust test suite has record
ID:

`d692732f05cf162c2984d79c30d6fef02332271024934d37f4a97be9e4542956`

Bounds and confidence basis are descriptive evidence only. They do not grant
capabilities, authorize execution, admit the record, expand policy, or dispose
of any proposal or dispute.

Changing this vector requires a new schema version rather than reinterpretation
of version 1.

## 12. Failed Attempt Fixed Vector

Failed Attempt v1 encodes fields in this order: Objective record identifier,
Method record identifier, one-byte failure-kind discriminator, observed-result
text, ordered unique evidence record identifiers, and an optional retry-of
record identifier.

Failure-kind discriminators are Invalid Input `0x00`, Method Failure `0x01`,
Tool Failure `0x02`, Timeout `0x03`, Inconclusive `0x04`, and Policy Denial
`0x05`.

The minimal fixture uses an Objective identifier of 32 `0x01` bytes, a Method
identifier of 32 `0x02` bytes, Invalid Input, `observed_result="F"`, no evidence
identifiers, and no retry reference:

`01010101010101010101010101010101010101010101010101010101010101010202020202020202020202020202020202020202020202020202020202020202000000000146000000`

The typed governed Failed Attempt record fixture in the Rust test suite has
record ID:

`aa792bd0335d98430c41b86ac3ea9289c3e09813ef82a4ed15dd4d286d0c4c48`

Failure kind, observed result, evidence, and retry lineage are descriptive
evidence only. They do not admit the record, expand policy, grant capabilities,
authorize retries, or exercise execution authority.

Changing this vector requires a new schema version rather than reinterpretation
of version 1.

## 13. Reviewer Finding Fixed Vector

Reviewer Finding v1 encodes fields in this order: reviewed governed-record
identifier, reviewer canonical identity, one-byte finding-kind discriminator,
one-byte severity discriminator, rationale text, ordered unique evidence record
identifiers, conflict-declaration text, and a one-byte independence-result
discriminator.

Finding-kind discriminators are Support `0x00`, Contradiction `0x01`,
Methodological Flaw `0x02`, Provenance Gap `0x03`, Security `0x04`,
Privacy/Licensing `0x05`, and Inconclusive `0x06`.

Severity discriminators are Informational `0x00`, Low `0x01`, Medium `0x02`,
High `0x03`, and Critical `0x04`. Independence-result discriminators are
Established `0x00`, Not Established `0x01`, Conflicted `0x02`, and Unknown
`0x03`.

The minimal fixture uses a reviewed-record identifier of 32 `0x01` bytes, a
reviewer identity of 32 `0x02` bytes, Support, Informational,
`rationale="R"`, no evidence identifiers, `conflict_declaration="NONE_DECLARED"`,
and Established independence:

`010101010101010101010101010101010101010101010101010101010101010102020202020202020202020202020202020202020202020202020202020202020000000000015200000000000d4e4f4e455f4445434c4152454400`

The typed governed Reviewer Finding record fixture in the Rust test suite has
record ID:

`5399568236fad9e71dcc88f498e3ebdaa7a2929d1a642c1d587a2b0d0593f609`

Finding kind, severity, rationale, evidence references, conflict declaration,
and independence result are descriptive epistemic evidence only. They do not
establish reviewer independence at admission, admit or dispose of the reviewed
record, grant a capability, authorize execution, expand policy, or exercise
promotion authority.

Changing this vector requires a new schema version rather than reinterpretation
of version 1.

## 14. Dispute Fixed Vector

Dispute v1 encodes fields in this order: disputed governed-record identifier,
a big-endian `u16` count followed by at least two and at most 64 ordered unique
position record identifiers, the canonical identity that opened the dispute,
a one-byte dispute-status discriminator, and an optional resolution record
identifier using the canonical `0x00` absent / `0x01` present marker.

Dispute-status discriminators are Open `0x00`, Under Review `0x01`,
Resolved `0x02`, and Withdrawn `0x03`.

`Resolved` requires a present, non-zero `resolution_id`. Open, Under Review,
and Withdrawn require the resolution identifier to be absent. These are local
structural invariants only. Whether position identifiers refer to Claims or
Reviewer Findings, whether referenced records exist, and whether a resolution
identifier refers to a later Disposition remain admission-policy concerns.

The minimal fixture uses a disputed-record identifier of 32 `0x01` bytes,
two ordered position identifiers of 32 `0x02` bytes and 32 `0x03` bytes,
an `opened_by` identity of 32 `0x04` bytes, Open status, and no resolution
identifier:

`010101010101010101010101010101010101010101010101010101010101010100020202020202020202020202020202020202020202020202020202020202020202030303030303030303030303030303030303030303030303030303030303030304040404040404040404040404040404040404040404040404040404040404040000`

The fixture is exactly 132 bytes.

The typed governed Dispute record fixture in the Rust test suite has record ID:

`29cbf3618d488d0c3e0cc4f025ad76060bc928d3d786b6fad6c7051b85d9c54f`

A Dispute is an epistemic challenge record only. Its presence, status,
positions, opener identity, or resolution reference does not reverse or nullify
a finding, admit or dispose of another record, grant or revoke capabilities,
authorize execution, expand policy, mutate governed state, or exercise
promotion authority.

Changing the status discriminators, field order, optional encoding, list
semantics, or either frozen vector requires a new schema version rather than
reinterpretation of version 1.

## 15. Disposition Fixed Vector

Disposition v1 encodes fields in this order: the governed-record identifier
being decided, a one-byte decision discriminator, the canonical decision
authority identity, the canonical versioned criteria or policy identity, a
big-endian `u16` count followed by at most 64 ordered unique evidence record
identifiers, a big-endian `u16` count followed by at most 64 ordered unique
unresolved-dispute record identifiers, a big-endian `u32` byte length followed
by the required UTF-8 reason text, and an optional superseded-record identifier
using the canonical `0x00` absent / `0x01` present marker.

Decision discriminators are Accept for Review `0x00`, Reject `0x01`, Defer
`0x02`, Quarantine `0x03`, Supersede `0x04`, and Revoke `0x05`.

Supersede and Revoke require a present, non-zero `supersedes_id`. Accept for
Review, Reject, Defer, and Quarantine require `supersedes_id` to be absent.
Both identifier lists preserve declared order and reject duplicate or all-zero
record identifiers. The reason is required, non-empty UTF-8, contains no NUL
byte, and is bounded by the A04 maximum text-field length.

These are local structural invariants only. Whether referenced records exist or
have the required kinds, whether `decision_authority_id` is independent from
the governed subject, whether `criteria_id` resolves to valid governing
criteria, and whether the unresolved-dispute list is complete remain
admission-policy concerns.

The minimal fixture uses a decided-record identifier of 32 `0x01` bytes,
Accept for Review decision, a decision-authority identity of 32 `0x02` bytes,
a criteria identity of 32 `0x03` bytes, empty evidence and unresolved-dispute
lists, `reason="R"`, and no superseded-record identifier:

`0101010101010101010101010101010101010101010101010101010101010101000202020202020202020202020202020202020202020202020202020202020202030303030303030303030303030303030303030303030303030303030303030300000000000000015200`

The fixture is exactly 107 bytes.

The typed governed Disposition record fixture in the Rust test suite has record ID:

`f051f39bc5c3301223227f60e32e000fbec038f98452a9f4a209cb9ec1daec67`

A Disposition records an evidence decision made by an explicitly identified
authority under explicitly identified criteria. Its presence, decision,
authority identity, criteria identity, evidence references, dispute references,
reason, or supersession reference does not itself grant or revoke capabilities,
authorize execution, promote an artifact, mutate a repository, alter policy, or
otherwise execute the consequence represented by the decision.

Changing the decision discriminators, field order, optional encoding, list
semantics, reason encoding, supersession coherence rules, or either frozen
vector requires a new schema version rather than reinterpretation of version 1.
