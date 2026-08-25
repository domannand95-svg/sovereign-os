# Sovereign LoRA artifact checkpoint

This directory records the completed Sovereign OS specialization run. The
directory name is historical: the actual base is the dense 3.24B-parameter
`unsloth/Llama-3.2-3B-Instruct`, not an 8B model.

## Disposition

- Training: COMPLETE.
- Held-out generalization validation: PARTIAL.
- Critical held-out authority-boundary probes: FAIL.
- Runtime authority: NONE.
- Adapter deployment: NOT APPROVED.
- FreeToken: DEFER / DO NOT INSTALL.

The adapter is probabilistic intelligence only. It cannot authorize execution,
widen capabilities, synthesize approval, or replace deterministic governance.

## Storage policy

Configuration, tokenizer metadata, the provenance ledger, and the canonical
artifact manifest are tracked in Git. Generated weights, full tokenizer data,
checkpoint directories, and the generated placeholder model card remain local
and are ignored by Git. Their identities are frozen in `artifact_manifest.json`.

Do not rename, move, merge, quantize, overwrite, or delete the local artifacts
without a separate evidence-backed change-set.
