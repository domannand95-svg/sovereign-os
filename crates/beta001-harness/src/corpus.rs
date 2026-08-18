//! Frozen Candidate Corpus Integration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusCase {
    pub trace_id: String,
    pub intent: String,
    pub target_state: String,
    pub parameters: serde_json::Value,
}

impl CorpusCase {
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }
}
