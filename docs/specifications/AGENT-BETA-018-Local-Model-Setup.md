# AGENT-BETA-018 local model setup

This setup keeps probabilistic inference outside deterministic authority:

```text
local model -> inert AgentOutput -> host validation -> signed policy evaluation
            -> private capability registry -> one-time execution -> signed receipt
            -> deterministic replay
```

The model process must never receive policy signing seeds, execution signing
seeds, a mutable `CapabilityRegistry`, ambient tool credentials, or direct
filesystem/process handles.

## OpenAI-compatible server

Start an OpenAI-compatible local server. For Ollama, one example is:

```powershell
ollama pull qwen2.5-coder:7b
ollama serve
```

Configure the harness process:

```powershell
$env:SOVEREIGN_LOCAL_MODEL_ENDPOINT = "http://127.0.0.1:11434/v1/chat/completions"
$env:SOVEREIGN_LOCAL_MODEL_NAME = "qwen2.5-coder:7b"
cargo run -p beta001-harness --example local_model_candidate
```

LM Studio, vLLM, llama.cpp servers, and other implementations can be used when
they expose the same chat-completions response shape. Set the endpoint and model
name to the values advertised by that server.

If a server requires a bearer token, construct the adapter with
`with_api_key_env`. The environment variable name is host configuration; its
value is read only when the host sends the request and is never added to the
model prompt or `AgentOutput`.

## Authority wiring

Production hosts must keep three boundaries separate:

1. `LocalOpenAiCompatibleBackend` returns only `AgentOutput` candidates.
2. `PolicyAuthority` signs an evaluation only after deterministic host policy
   validates a content-derived `CapabilityRequest`.
3. `CapabilityRegistry` admits only evaluations signed by its pinned policy key,
   validates active registered identity, and atomically consumes a grant before
   an effect is dispatched.

The trusted execution adapter uses `ExecutionAuthority` to sign a receipt only
after it observes the actual effect result. Replay must be called with the
pinned execution verification key.

Never use example or test seeds in production. Generate policy and execution
signing seeds from an operating-system cryptographic random source, store them
outside model-visible files and environment, and persist only verification keys
with replay configuration.

## Verification

Before connecting mutation tools, run:

```powershell
cargo test -p beta001-harness --test agent_beta_018_a_agent_identity
cargo test -p beta001-harness --test agent_beta_018_b_observation_intake
cargo test -p beta001-harness --test agent_beta_018_c_proposal_generation
cargo test -p beta001-harness --test agent_beta_018_d_capability_negotiation
cargo test -p beta001-harness --test agent_beta_018_e_execution_replay
cargo test -p beta001-harness
```

Do not enable mutation tools if any boundary fails. A model response, proposal,
request, identity, hash, or receipt is evidence only; none is authority unless
the pinned policy and registry path validates it.
