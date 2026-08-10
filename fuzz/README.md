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
