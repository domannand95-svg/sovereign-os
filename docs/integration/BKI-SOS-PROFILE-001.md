---
Document: BKI and Sovereign OS Language Compatibility Profile v1
ID: BKI-SOS-PROFILE-001
Version: 1.0
Status: Proposed Compatibility Profile
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-08-11
Evidence:
  - BKI PR 13
Depends On:
  - BKI-SOS-BETA-001
  - SPEC-EV-001
Supersedes: None
---

# BKI and Sovereign OS Language Compatibility Profile v1

## Purpose

Sovereign OS adopts `bki.sovereign.profile.v1` as the proposed metadata and
language boundary for designated BKI validation during beta testing. This
profile does not make BKI a Sovereign policy, disposition, promotion, registry,
or execution authority.

## Pinned Contract Candidate

| Item | Value |
| --- | --- |
| Profile version | `bki.sovereign.profile.v1` |
| BKI contract commit | `27d75a5c33a5e40bbbd0359704fe10ef52bdf1f2` |
| Schema | `bki-sovereign-profile-v1.schema.json` |
| Schema SHA-256 | `1a513455e09063f41d03023688ffc7b89bdababaf8ed9d1a78c101edb7b8845d` |
| Activation | Not activated |

The commit is the immutable merge result of BKI PR 13. Beta activation still
requires a tagged BKI pre-release and all gates defined below.

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

Until every gate passes, BKI integration remains not activated.
