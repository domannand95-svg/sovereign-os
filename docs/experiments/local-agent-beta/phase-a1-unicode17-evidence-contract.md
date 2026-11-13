# Phase A1: inactive Unicode 17 evidence boundary

Status: implementation for review only; no activation or migration. The envelope
module is private and has no production caller, public import API, endpoint,
writer, provenance catalog, or parser fallback. Its narrowly scoped non-test
`expect(dead_code)` documents intentional inactivity; strict workspace linting
and every existing security gate remain enabled. PRs #254 and #250 remain held.

## Preserved intent recipe

`intent-nfkc-u17-v1` uses the existing locked unicode-normalization 0.1.25.
A compile-time guard rejects tables other than Unicode 17.0.0. The shared helper
discards U+0000..001F and U+007F..009F except TAB/LF/CR, then performs whole-string
NFKC. It does not trim, case-fold, rewrite newlines, or insert stream-safe marks.
The explicit filter is exhaustively compared with the original Rust predicate.

Intake order remains identity binding, raw UTF-8 byte limit 65,536, reference
limit 32, clock-skew limit 300 seconds, raw SHA-256, filtering/NFKC, normalized
SHA-256. There is no scalar or post-normalization size gate. Historical v1
verification receives no new intake gate. Empty/fully filtered input and
Unicode-17-unassigned scalars retain existing behavior. Adjacent trim/lowercase,
receipt and clock behavior is unchanged; this recipe does not freeze all other
compiler-sensitive validation. Baseline toolchain: Rust 1.97.1 / std Unicode 17.

## Strict envelope

Exactly four required string fields are accepted:

```json
{"format":"sovereign.service-evidence.v2","normalization_profile":"intent-nfkc-u17-v1","payload_json":"<exact v1 JSON text>","seal_sha256":"<64 lowercase hex>"}
```

Unknown, duplicate, missing or non-string outer fields, unsupported format/profile,
malformed seals and trailing non-whitespace content are rejected. Outer JSON field
order/escaping may vary. Decoded payload UTF-8 bytes, including internal whitespace,
are retained exactly. The original v1 parser and `verify_chain` validate the inner
package; its package digest, receipts and serialization are unchanged. Unknown v1
payload fields retain their original inert semantics and never select a profile.

Define `F(s) = ASCII(decimal UTF-8 byte length) || ':' || UTF-8(s)` with no leading
zeroes except `0`. Domain is `SOVEREIGN_OS:SERVICE_EVIDENCE:PROFILED:v2`.
The outer seal is lowercase hexadecimal SHA-256 of:

```text
F(domain) || F(format) || F(normalization_profile) || F(payload_json)
```

Framing-only vector with payload `{}`:
`0ddaed98f9efb4b8a89c37c2629b9456e7165dbb6cba1c4ca6d6f68120ec7ea0`.
That payload is not valid evidence. A valid outer hash is unkeyed structural
consistency, not producer provenance, authentication, authority or admission.
No caller-supplied provenance boolean, catalog loader or legacy selection API is
introduced. Raw v1 input is rejected by the envelope parser, without fallback.
Existing low-level v1 verification remains unchanged; this is not a claim that
all existing callers enforce a future provenance policy.

## Fixtures and remaining gates

The frozen synthetic `tests/fixtures/unicode17_v1_package.json` was generated
before changes from main-equivalent code with fixed 2026-08-24 timestamps.
Its exact SHA-256 is
`23aeb15569569d96c1aa57a037964aff218814b79dd5ce3fdde5fa8e362c3fdd`.
It includes deliberately simulated receipt strings and is not production evidence.
Tests preserve its complete parsed fields, package digest and exact payload bytes.
A genuine correctly sealed envelope is rejected by the v1 reader, in contrast to
adding ignored labels to an old object. This is current-baseline characterization,
not certification of unknown historical releases or deployed producers.

The ignored `owner_provided_unicode17_conformance_oracle` test accepts authorized
official data only through these path/digest pairs:

```text
SOVEREIGN_U17_NORMALIZATION_TEST_PATH
SOVEREIGN_U17_NORMALIZATION_TEST_SHA256
SOVEREIGN_U17_DERIVED_AGE_PATH
SOVEREIGN_U17_DERIVED_AGE_SHA256
```

It verifies supplied hashes and Unicode 17 headers, checks all four normalization
forms against all five test columns, and tests assigned scalars outside Part 1.
Run it explicitly after owner approval with:

```text
cargo test -p beta001-harness --lib --locked service::intent_normalization::tests::owner_provided_unicode17_conformance_oracle -- --ignored --exact
```

The parser/invariant machinery also has a hand-authored synthetic self-test;
that is not an official conformance result.
No corpus is bundled. Expected digests must come from independently reviewed
official acquisition/provenance, not from self-approving an arbitrary test file.
The previous direct Unicode.org request was denied with HTTP 403; no mirror,
route, credential change or retry is authorized here. This test is NOT RUN until
owner-provided corpus acquisition/licensing is approved; an ignored test does not
establish conformance or merge readiness.

Future activation requires the approved provenance authority, verified producer
and consumer inventory, bounded transport/review resource policy, official oracle,
actual supported old-reader tests and unchanged Linux/Windows CI/security/coverage
gates. Unknown provenance remains a future review-only outcome; no accepted-evidence
store, review queue or migration is implemented. Any format writer or Unicode 18
change requires separate approval. Rollback leaves original v1 data and verification
intact and must retain readers for any subsequently approved emitted envelopes.
