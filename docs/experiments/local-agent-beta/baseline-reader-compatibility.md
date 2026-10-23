# Source-pinned baseline reader characterization

This test tool characterizes the v1 reader at commit
`f8a10a3c639e4b8911f6b7069e1565d5eb99c48f` (tree
`b1b367e90b1565d699b4ea92c226930efa1495b7`). It does not designate a
supported release, certify historical deployed readers, establish provenance,
prove Unicode conformance, or authorize activation. PRs #250/#254 stay held;
the Phase A1 envelope stays inactive.

## Prerequisites and execution

Use Python 3.12 or later, Git with the pinned commit already available locally,
Rust/Cargo 1.97.1, and an already populated Cargo cache for that commit. Use a
trusted, reviewed toolchain and dependency cache. No installer, Git fetch,
registry update, Unicode corpus download or online fallback is performed.
Missing objects, cache entries, tools or disk space cause failure. Each build
uses a fresh target directory; allow approximately 1 GiB of disposable disk
space and enough time for two full baseline library builds. Builds run serially
with two Cargo jobs, debug information and incremental compilation disabled.
Commands have a 20-minute timeout and each reader invocation a 30-second timeout;
timeouts fail the run.

With `cargo` and `rustc` on PATH and your existing CARGO_HOME/RUSTUP_HOME set:

```sh
python3 -B -m unittest discover -s scripts/tests -p test_baseline_reader.py -v
python3 -B scripts/verify-baseline-reader.py --output /tmp/baseline-reader-report.json
```

The report path must not already exist. Preserve previous reports; choose a new
path for another run. Repository files and fixture bytes are never rewritten.
The tool exports the pinned Git tree into temporary storage and adds only the
checked-in test adapter as a Cargo example there. It uses the baseline manifest
and lockfile unchanged with `cargo build --offline --locked`; no adapter manifest
or new dependencies are introduced. Temporary sources and build output are
removed on normal completion or a handled build failure, without removing any
pre-existing build cache. An external forced termination may leave its temporary
directory for manual review.

## Acceptance contract

Each of two independent clean builds must:

1. Match the pinned source tree and lockfile SHA-256
   `e0fc3ed947a8d348e2d472db9e5ec22d834cb839070ddc71087701afea20e567`.
2. Match frozen fixture SHA-256
   `23aeb15569569d96c1aa57a037964aff218814b79dd5ce3fdde5fa8e362c3fdd`.
3. Accept the exact raw v1 fixture through the original parser and `verify_chain`.
4. Reject a genuine four-field envelope containing the exact valid v1 JSON and
   its correct length-framed SHA-256 seal, specifically because the old reader
   lacks the required top-level `sanitized_ingress` field. Crashes, arbitrary
   errors and unexpected acceptance do not count as rejection evidence.
5. Leave the baseline lockfile unchanged and produce matching semantic outcomes
   across builds. Binary hash equality is recorded but is not required: temporary
   build paths can affect binary bytes.

The standard-library-only tests exercise the reviewed independent framing golden
vector, UTF-8 length framing/exact payload preservation, altered/missing fixtures,
missing executable, failed commands, unexpected reader results, existing-report
preservation and refusal to certify a single build. These synthetic cases are
not historical release fixtures or official Unicode data.

The JSON report records baseline commit/tree, source archive SHA-256, lockfile,
fixture, adapter, harness, toolchain executable and reader binary hashes, full
compiler identity, Cargo identity, build arguments and per-build semantic
outcomes. It is emitted only after both builds pass; failures exit nonzero and
print their reason without creating a success report. Archive hashes describe
this export (Git versions may affect archive representation); the Git tree is
the immutable source identity. Hashes identify inputs, not provenance authority.

## Remaining gates

An owner-designated supported producer/consumer inventory, authentic historical
artifact/build provenance and per-supported-reader tests are still required for
released-reader certification. Available tags alone do not establish deployment
or support. The independent official Unicode 17 oracle remains separately
blocked pending approved owner-provided data and licensing; this harness does
not access the previously denied host. No writer, intake endpoint, provenance
loader, migration, policy change or production integration is added.
