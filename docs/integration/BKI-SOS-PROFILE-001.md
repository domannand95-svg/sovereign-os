---
Document: BKI and Sovereign OS Language Compatibility Profile v1
ID: BKI-SOS-PROFILE-001
Version: 1.0
Status: Active Read-Only Beta Compatibility Profile
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-08-21
Evidence:
  - BKI PR 13
Depends On:
  - BKI-SOS-BETA-001
  - SPEC-EV-001
Supersedes: None
---

# BKI and Sovereign OS Language Compatibility Profile v1

## Purpose

Sovereign OS adopts `bki.sovereign.profile.v1` as the active read-only-beta metadata and
language boundary for designated BKI validation during beta testing. This
profile does not make BKI a Sovereign policy, disposition, promotion, registry,
or execution authority.

## Pinned Contract Candidate

| Item | Value |
| --- | --- |
| Profile version | `bki.sovereign.profile.v1` |
| BKI release | `bki-sovereign-v1.0.0-beta.1` |
| BKI release candidate commit | `f1c2cde2865d2fb45cc4a748930dd90d55a55d83` |
| Schema | `bki-sovereign-profile-v1.schema.json` |
| Schema SHA-256 | `1a513455e09063f41d03023688ffc7b89bdababaf8ed9d1a78c101edb7b8845d` |
| Activation | Active for read-only beta evidence intake |

The release commit contains the qualified runtime contract and the activation
record. Consumers pin both the release tag and exact commit.

The schema digest is calculated over canonical repository bytes with LF line
endings. Compatibility checks normalize checkout line endings before hashing so
Windows and Linux verify the same governed content.

## Metadata Mapping

| Sovereign OS key | Shared key | Rule |
| --- | --- | --- |
| `ID` | `document_id` | Required identity; never inferred from `Document` or a path |
| `Version` | `version` | Required and represented as a string |
| `Status` | `status.value` | Literal value retained under namespace `sovereign` |
| `Last Updated` | `last_revised` | Required RFC 3339 full date |

`Document`, `Owner`, `Reviewers`, `Evidence`, `Depends On`, and `Supersedes`
remain Sovereign-governed extension fields. The shared core neither discards
their source values nor interprets them as BKI authority.

Canonical and aliased keys may not coexist in one source record. Missing,
duplicate, conflicting, null, type-incompatible, or unknown values fail closed.
Translation occurs only under the explicitly selected profile version.

## Shared Terms and Preserved Distinctions

- An **artifact** is a bounded identified object under evaluation or governance.
- **Evidence** is attributable information considered by a validator or
  decision authority.
- A BKI **validation outcome** is compliant, normalized, or quarantine.
- A Sovereign **disposition** is accept for review, reject, defer, quarantine,
  supersede, or revoke.
- **Quarantine** is fail-closed isolation, never deletion or approval.
- **Promotion** is a separate Sovereign governance transition.
- A **capability** is explicitly granted authority for a bounded operation.
- A compatibility **profile** translates a contract; it grants no authority.

A BKI quarantine is validation evidence, not a Sovereign disposition. A BKI
pass cannot authorize execution, registry mutation, evidence admission,
capability creation, or promotion.

## Activation Gates

Activation requires:

1. merge and tag of the BKI profile and read-only adapter;
2. verification of the pinned merged BKI commit against its release tag;
3. matching schema bytes and SHA-256 digest in both repositories;
4. positive and negative compatibility fixtures on Windows and Linux;
5. fail-closed handling of unknown profiles, schema drift, malformed output,
   metadata collisions, timeouts, and process failure; and
6. explicit owner approval after compatibility evidence is reviewed.

The `BKI Compatibility` workflow exercises this boundary on Windows and Linux
using a repository-scoped read-only deploy key. It checks out the runtime commit
above, never a moving branch, and verifies both worktrees remain unchanged.

Any gate failure suspends BKI consumption and fails closed.

## Qualification Evidence — 2026-08-20

The live cross-repository suite was executed on Windows against the exact pinned
BKI runtime commit `0ace31f9071969825b75187d31c5f418212e9ae9`. All eight checks passed,
including commit identity, canonical schema bytes and digest, profile
translation, schema-valid CLI output, exact content hashes, quarantine,
unknown-contract rejection, metadata-confusion rejection, and malformed-output
rejection. The BKI repository regression gate independently passed with 83
tests and one platform-specific skip.

The repository owner authorized activation. The qualified runtime is published
through `bki-sovereign-v1.0.0-beta.1`, and the Sovereign consumer pins its exact
commit. The profile is **active only for read-only beta evidence intake**.
Windows and Linux CI remain mandatory release evidence; a failure, identity
mismatch, or schema drift suspends consumption and fails closed.
