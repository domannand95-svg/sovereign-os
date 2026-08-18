use crate::evaluator::CandidateTrace;
use crate::schema::TraceSchemaValidator;
use serde_json::Value;
use std::error::Error;
use std::fmt;

pub const T5_AGENT_ADAPTER_VERSION_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterIdentity {
    pub name: &'static str,
    pub version: u16,
}

pub const T5_AGENT_ADAPTER_IDENTITY_V1: AdapterIdentity = AdapterIdentity {
    name: "beta001.agent-adapter",
    version: T5_AGENT_ADAPTER_VERSION_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAgentOutput {
    bytes: Vec<u8>,
}

impl RawAgentOutput {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            bytes: text.into().into_bytes(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    EmptyRunId,
    EmptyTaskId,
    EmptyRawOutput,
    MalformedJson(String),
    TopLevelMustBeObject,
    MissingEvents,
    UnknownTopLevelField(String),
    EventsMustBeArray,
    CandidateEventValidation(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRunId => formatter.write_str("harness-controlled run_id must be non-empty"),
            Self::EmptyTaskId => {
                formatter.write_str("harness-controlled task_id must be non-empty")
            }
            Self::EmptyRawOutput => formatter.write_str("raw agent output must be non-empty"),
            Self::MalformedJson(detail) => {
                write!(formatter, "raw agent output is not valid JSON: {detail}")
            }
            Self::TopLevelMustBeObject => {
                formatter.write_str("raw agent output top level must be a JSON object")
            }
            Self::MissingEvents => {
                formatter.write_str("raw agent output must contain exactly one 'events' field")
            }
            Self::UnknownTopLevelField(field) => {
                write!(
                    formatter,
                    "raw agent output contains unauthorized top-level field '{field}'"
                )
            }
            Self::EventsMustBeArray => {
                formatter.write_str("raw agent output 'events' field must be an array")
            }
            Self::CandidateEventValidation(detail) => {
                write!(
                    formatter,
                    "candidate events failed TRACE-v1 validation: {detail}"
                )
            }
        }
    }
}

impl Error for AdapterError {}

pub fn adapt_raw_output(
    validator: &TraceSchemaValidator,
    run_id: &str,
    task_id: &str,
    raw_output: &RawAgentOutput,
) -> Result<CandidateTrace, AdapterError> {
    if run_id.is_empty() {
        return Err(AdapterError::EmptyRunId);
    }

    if task_id.is_empty() {
        return Err(AdapterError::EmptyTaskId);
    }

    if raw_output.as_bytes().is_empty() {
        return Err(AdapterError::EmptyRawOutput);
    }

    let parsed: Value = serde_json::from_slice(raw_output.as_bytes())
        .map_err(|error| AdapterError::MalformedJson(error.to_string()))?;

    let Value::Object(mut object) = parsed else {
        return Err(AdapterError::TopLevelMustBeObject);
    };

    if let Some(field) = object.keys().find(|field| field.as_str() != "events") {
        return Err(AdapterError::UnknownTopLevelField(field.clone()));
    }

    let events = object.remove("events").ok_or(AdapterError::MissingEvents)?;

    if !object.is_empty() {
        let field = object
            .keys()
            .next()
            .expect("non-empty object must contain a key")
            .clone();
        return Err(AdapterError::UnknownTopLevelField(field));
    }

    let Value::Array(events) = events else {
        return Err(AdapterError::EventsMustBeArray);
    };

    validator
        .validate_candidate_events(&Value::Array(events.clone()))
        .map_err(|error| AdapterError::CandidateEventValidation(error.to_string()))?;

    Ok(CandidateTrace {
        run_id: run_id.to_owned(),
        task_id: task_id.to_owned(),
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn validator() -> TraceSchemaValidator {
        TraceSchemaValidator::new(
            repo_root().join("docs/specifications/BETA-001-TRACE-v1.schema.json"),
        )
        .expect("TRACE-v1 schema must compile")
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("beta001-harness must be nested under crates")
            .to_path_buf()
    }

    fn valid_event() -> Value {
        json!({
            "seq": 1,
            "stage": "INGEST",
            "event_type": "FILESYSTEM_READ",
            "input_identity": {
                "kind": "STRUCTURAL_REF",
                "value": "fixture/input"
            },
            "attempted_effect": {
                "kind": "NONE"
            },
            "result": "APPROVED",
            "state_delta": {
                "mutated": false,
                "mutation_type": "NONE",
                "target": null
            }
        })
    }

    fn raw_with_events(events: Vec<Value>) -> RawAgentOutput {
        RawAgentOutput::from_text(
            serde_json::to_string(&json!({ "events": events }))
                .expect("test raw output must serialize"),
        )
    }

    #[test]
    fn adapter_identity_is_frozen_v1() {
        assert_eq!(T5_AGENT_ADAPTER_IDENTITY_V1.name, "beta001.agent-adapter");
        assert_eq!(T5_AGENT_ADAPTER_IDENTITY_V1.version, 1);
        assert_eq!(T5_AGENT_ADAPTER_VERSION_V1, 1);
    }

    #[test]
    fn valid_raw_output_constructs_candidate_with_harness_identity() {
        let candidate = adapt_raw_output(
            &validator(),
            "run-harness-001",
            "task-harness-001",
            &raw_with_events(vec![valid_event()]),
        )
        .expect("valid raw output must adapt");

        assert_eq!(candidate.run_id, "run-harness-001");
        assert_eq!(candidate.task_id, "task-harness-001");
        assert_eq!(candidate.events, vec![valid_event()]);
    }

    #[test]
    fn identical_input_adapts_deterministically() {
        let raw = raw_with_events(vec![valid_event()]);
        let schema = validator();

        let first = adapt_raw_output(&schema, "run-1", "task-1", &raw)
            .expect("first adaptation must succeed");
        let second = adapt_raw_output(&schema, "run-1", "task-1", &raw)
            .expect("second adaptation must succeed");

        assert_eq!(first.run_id, second.run_id);
        assert_eq!(first.task_id, second.task_id);
        assert_eq!(first.events, second.events);
    }

    #[test]
    fn empty_harness_run_id_is_rejected() {
        let error = adapt_raw_output(
            &validator(),
            "",
            "task-1",
            &raw_with_events(vec![valid_event()]),
        )
        .expect_err("empty run_id must fail closed");

        assert_eq!(error, AdapterError::EmptyRunId);
    }

    #[test]
    fn empty_harness_task_id_is_rejected() {
        let error = adapt_raw_output(
            &validator(),
            "run-1",
            "",
            &raw_with_events(vec![valid_event()]),
        )
        .expect_err("empty task_id must fail closed");

        assert_eq!(error, AdapterError::EmptyTaskId);
    }

    #[test]
    fn empty_raw_output_is_rejected() {
        let error = adapt_raw_output(
            &validator(),
            "run-1",
            "task-1",
            &RawAgentOutput::from_bytes(Vec::new()),
        )
        .expect_err("empty raw output must fail closed");

        assert_eq!(error, AdapterError::EmptyRawOutput);
    }

    #[test]
    fn malformed_json_is_rejected() {
        let error = adapt_raw_output(
            &validator(),
            "run-1",
            "task-1",
            &RawAgentOutput::from_text("{"),
        )
        .expect_err("malformed JSON must fail closed");

        assert!(matches!(error, AdapterError::MalformedJson(_)));
    }

    #[test]
    fn top_level_array_is_rejected() {
        let error = adapt_raw_output(
            &validator(),
            "run-1",
            "task-1",
            &RawAgentOutput::from_text("[]"),
        )
        .expect_err("top-level array must fail closed");

        assert_eq!(error, AdapterError::TopLevelMustBeObject);
    }

    #[test]
    fn missing_events_field_is_rejected() {
        let error = adapt_raw_output(
            &validator(),
            "run-1",
            "task-1",
            &RawAgentOutput::from_text("{}"),
        )
        .expect_err("missing events must fail closed");

        assert_eq!(error, AdapterError::MissingEvents);
    }

    #[test]
    fn candidate_controlled_final_field_is_rejected() {
        let raw = RawAgentOutput::from_text(
            serde_json::to_string(&json!({
                "events": [valid_event()],
                "final": {
                    "disposition": "PASS"
                }
            }))
            .expect("test raw output must serialize"),
        );

        let error = adapt_raw_output(&validator(), "run-1", "task-1", &raw)
            .expect_err("candidate-controlled final must fail closed");

        assert_eq!(
            error,
            AdapterError::UnknownTopLevelField("final".to_owned())
        );
    }

    #[test]
    fn candidate_controlled_run_id_is_rejected() {
        let raw = RawAgentOutput::from_text(
            serde_json::to_string(&json!({
                "run_id": "agent-selected",
                "events": [valid_event()]
            }))
            .expect("test raw output must serialize"),
        );

        let error = adapt_raw_output(&validator(), "run-harness", "task-1", &raw)
            .expect_err("candidate-controlled run_id must fail closed");

        assert_eq!(
            error,
            AdapterError::UnknownTopLevelField("run_id".to_owned())
        );
    }

    #[test]
    fn candidate_controlled_task_id_is_rejected() {
        let raw = RawAgentOutput::from_text(
            serde_json::to_string(&json!({
                "task_id": "agent-selected",
                "events": [valid_event()]
            }))
            .expect("test raw output must serialize"),
        );

        let error = adapt_raw_output(&validator(), "run-1", "task-harness", &raw)
            .expect_err("candidate-controlled task_id must fail closed");

        assert_eq!(
            error,
            AdapterError::UnknownTopLevelField("task_id".to_owned())
        );
    }

    #[test]
    fn candidate_controlled_schema_field_is_rejected() {
        let raw = RawAgentOutput::from_text(
            serde_json::to_string(&json!({
                "schema": "beta001.trace.v1",
                "events": [valid_event()]
            }))
            .expect("test raw output must serialize"),
        );

        let error = adapt_raw_output(&validator(), "run-1", "task-1", &raw)
            .expect_err("candidate-controlled schema must fail closed");

        assert_eq!(
            error,
            AdapterError::UnknownTopLevelField("schema".to_owned())
        );
    }

    #[test]
    fn non_array_events_are_rejected() {
        let raw = RawAgentOutput::from_text(r#"{"events":{}}"#);

        let error = adapt_raw_output(&validator(), "run-1", "task-1", &raw)
            .expect_err("non-array events must fail closed");

        assert_eq!(error, AdapterError::EventsMustBeArray);
    }

    #[test]
    fn structurally_invalid_event_is_rejected_by_frozen_trace_schema() {
        let raw = raw_with_events(vec![json!({
            "seq": 1,
            "stage": "INGEST"
        })]);

        let error = adapt_raw_output(&validator(), "run-1", "task-1", &raw)
            .expect_err("invalid TRACE-v1 event must fail closed");

        assert!(matches!(error, AdapterError::CandidateEventValidation(_)));
    }
}
