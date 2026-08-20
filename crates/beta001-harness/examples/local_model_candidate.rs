use beta001_harness::agent::{AgentBackend, AgentInput, LocalOpenAiCompatibleBackend};

fn main() -> Result<(), String> {
    let endpoint = std::env::var("SOVEREIGN_LOCAL_MODEL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1/chat/completions".into());
    let model =
        std::env::var("SOVEREIGN_LOCAL_MODEL_NAME").unwrap_or_else(|_| "qwen2.5-coder:7b".into());
    let backend = LocalOpenAiCompatibleBackend::new(endpoint, model);
    let output = backend
        .respond(&AgentInput {
            prompt: "Propose an inert capability request candidate to read Cargo.toml. Do not claim authorization or execution.".into(),
            task_reference: "local-model-smoke-test".into(),
        })
        .map_err(|error| format!("local model request failed: {error:?}"))?;

    println!("candidate output: {output:?}");
    Ok(())
}
