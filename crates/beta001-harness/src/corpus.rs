use crate::evaluator::CandidateTrace;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
