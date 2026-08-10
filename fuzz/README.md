# Decoder Fuzzing

The `decoder_boundaries` target exercises the authoritative state snapshot,
ledger event record, identity record, and lineage record decoders. Malformed
input must fail closed without panicking.

Run a bounded commissioning pass from the repository root:

```text
cargo +nightly fuzz run decoder_boundaries -- -runs=10000
```

For sustained local fuzzing, replace the run count with a time budget. Corpus,
artifact, and target directories are local evidence and must not be committed.

## Windows commissioning

The Windows MSVC target requires the Visual Studio C++ AddressSanitizer runtime
and compatible LLVM/Clang tools. Until those components are installed, linking
fails on the missing `clang_rt.asan_dynamic_runtime_thunk-x86_64.lib` runtime;
this is an environment failure rather than a decoder or harness failure.

After installing the components through Visual Studio Installer, verify from a
Developer PowerShell session:

```text
cargo +nightly fuzz run decoder_boundaries -- -runs=1000
```

Keep the workstation checklist item open until that bounded Windows run passes.
