use crate::evaluator::CandidateTrace;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum CorpusError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl From<std::io::Error> for CorpusError {
    fn from(e: std::io::Error) -> Self {
        CorpusError::Io(e)
    }
}

impl From<serde_json::Error> for CorpusError {
    fn from(e: serde_json::Error) -> Self {
        CorpusError::Parse(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusCase {
    pub trace_id: String,
    pub intent: String,
    pub target_state: String,
    pub parameters: Value,
    pub events: Vec<Value>,
}

impl CorpusCase {
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    pub fn load_fixture(path: &Path) -> Result<Self, CorpusError> {
        let raw = fs::read_to_string(path)?;
        let case: CorpusCase = serde_json::from_str(&raw)?;
        Ok(case)
    }

    pub fn into_candidate_trace(&self, run_id: &str, task_id: &str) -> CandidateTrace {
        CandidateTrace {
            run_id: run_id.to_owned(),
            task_id: task_id.to_owned(),
            events: self.events.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_mock_corpus_case() -> CorpusCase {
        CorpusCase {
            trace_id: "fixture-001".to_string(),
            intent: "Create test resource".to_string(),
            target_state: "ResourceCreated".to_string(),
            parameters: json!({"resource_type": "db"}),
            events: vec![json!({
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
            })],
        }
    }

    #[test]
    fn corpus_case_to_trace_preserves_events() {
        let case = create_mock_corpus_case();
        let trace = case.into_candidate_trace("run-alpha", "task-beta");

        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events, case.events);
    }

    #[test]
    fn corpus_trace_id_does_not_control_run_id() {
        let case = create_mock_corpus_case();
        let trace = case.into_candidate_trace("run-authoritative", "task-authoritative");

        assert_eq!(trace.run_id, "run-authoritative");
        assert_ne!(trace.run_id, case.trace_id);
    }

    #[test]
    fn corpus_trace_id_does_not_control_task_id() {
        let case = create_mock_corpus_case();
        let trace = case.into_candidate_trace("run-authoritative", "task-authoritative");

        assert_eq!(trace.task_id, "task-authoritative");
        assert_ne!(trace.task_id, case.trace_id);
    }

    #[test]
    fn identical_corpus_case_produces_identical_candidate_trace() {
        let case = create_mock_corpus_case();

        let trace_one = case.into_candidate_trace("run-consistent", "task-consistent");
        let trace_two = case.into_candidate_trace("run-consistent", "task-consistent");

        assert_eq!(trace_one.run_id, trace_two.run_id);
        assert_eq!(trace_one.task_id, trace_two.task_id);
        assert_eq!(trace_one.events, trace_two.events);
    }
}

#[cfg(test)]
mod loader_tests {
    use super::*;
    use serde_json::json;
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_temp_fixture(name: &str, content: &Value) -> std::path::PathBuf {
        let mut path = env::temp_dir();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        path.push(format!("{}_{}", unique, name));
        fs::write(&path, serde_json::to_string(content).unwrap()).unwrap();
        path
    }

    #[test]
    fn corpus_fixture_loads_successfully() {
        let content = json!({
            "trace_id": "load-test-001",
            "intent": "Verify loading",
            "target_state": "Loaded",
            "parameters": {},
            "events": []
        });
        let path = setup_temp_fixture("valid_fixture.json", &content);

        let result = CorpusCase::load_fixture(&path);
        assert!(result.is_ok());
        let case = result.unwrap();
        assert_eq!(case.trace_id, "load-test-001");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn corpus_fixture_missing_events_rejects() {
        let content = json!({
            "trace_id": "load-test-002",
            "intent": "Verify rejection",
            "target_state": "Rejected",
            "parameters": {}
        });
        let path = setup_temp_fixture("invalid_fixture.json", &content);

        let result = CorpusCase::load_fixture(&path);
        assert!(result.is_err());
        match result {
            Err(CorpusError::Parse(_)) => (),
            _ => panic!("Expected Parse error for missing events field"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn corpus_fixture_deterministic_loading() {
        let content = json!({
            "trace_id": "det-test-001",
            "intent": "Verify determinism",
            "target_state": "Stable",
            "parameters": {"key": "value"},
            "events": [{"seq": 1}]
        });
        let path = setup_temp_fixture("deterministic_fixture.json", &content);

        let case_one = CorpusCase::load_fixture(&path).unwrap();
        let case_two = CorpusCase::load_fixture(&path).unwrap();

        assert_eq!(case_one, case_two);

        let _ = fs::remove_file(path);
    }
}
