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
and compatible LLVM/Clang tools. These components are commissioned on the
ThinkPad development workspace.

Run the repository helper from PowerShell. It locates the current Visual Studio
installation and adds the matching AddressSanitizer DLL directory to the child
process environment:

```text
.\fuzz\run-windows.cmd 1000
```

The bounded native Windows commissioning run passed on 2026-08-11.

## WSL commissioning

Use the native Linux worktree at `~/workspaces/sovereign-os-v01-hardening` for
Linux builds and fuzzing. Avoid compiling from `/mnt/c`; the Windows-to-Linux
filesystem bridge is substantially slower for Cargo workloads.

The WSL workspace is pinned to Rust 1.97.1, with nightly, rustfmt, Clippy, LLVM
coverage tools, Rust source, cargo-fuzz 0.13.2, Clang 18, and build essentials.
