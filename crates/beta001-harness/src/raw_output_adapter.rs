use serde::Deserialize;
use serde_json::Value;

const AGENT_OUTPUT_SCHEMA: &str =
    include_str!("../../../docs/experiments/local-agent-beta/AGENT_OUTPUT-v1.schema.json",);

const JSON_FENCE_PREFIX: &str = "```json\n";
const JSON_FENCE_SUFFIX: &str = "\n```\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationKind {
    ExactOuterJsonMarkdownFenceRemoval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawOutputRejection {
    InvalidUtf8,
    InvalidJson,
    SchemaViolation,
    DisallowedWrapper,
    TrailingContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawOutputAdmission {
    Admit {
        candidate: Value,
    },
    AdmitNormalized {
        candidate: Value,
        normalization: NormalizationKind,
    },
    Reject {
        reason: RawOutputRejection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawOutputAdmissionResult {
    pub raw_output: Vec<u8>,
    pub admission: RawOutputAdmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawOutputAdapter;

impl RawOutputAdapter {
    pub fn admit(raw_output: &[u8]) -> RawOutputAdmissionResult {
        let preserved_raw_output = raw_output.to_vec();

        let raw_text = match std::str::from_utf8(raw_output) {
            Ok(value) => value,
            Err(_) => {
                return Self::reject(preserved_raw_output, RawOutputRejection::InvalidUtf8);
            }
        };

        let (candidate_text, normalization) = match Self::normalize(raw_text) {
            Ok(value) => value,
            Err(reason) => {
                return Self::reject(preserved_raw_output, reason);
            }
        };

        let candidate = match Self::parse_complete_json(candidate_text) {
            Ok(value) => value,
            Err(reason) => {
                return Self::reject(preserved_raw_output, reason);
            }
        };

        if !Self::is_schema_valid(&candidate) {
            return Self::reject(preserved_raw_output, RawOutputRejection::SchemaViolation);
        }

        let admission = match normalization {
            Some(normalization) => RawOutputAdmission::AdmitNormalized {
                candidate,
                normalization,
            },
            None => RawOutputAdmission::Admit { candidate },
        };

        RawOutputAdmissionResult {
            raw_output: preserved_raw_output,
            admission,
        }
    }

    fn reject(raw_output: Vec<u8>, reason: RawOutputRejection) -> RawOutputAdmissionResult {
        RawOutputAdmissionResult {
            raw_output,
            admission: RawOutputAdmission::Reject { reason },
        }
    }

    fn normalize(raw_text: &str) -> Result<(&str, Option<NormalizationKind>), RawOutputRejection> {
        if raw_text.starts_with(JSON_FENCE_PREFIX) && raw_text.ends_with(JSON_FENCE_SUFFIX) {
            let start = JSON_FENCE_PREFIX.len();

            if raw_text.len() < JSON_FENCE_PREFIX.len() + JSON_FENCE_SUFFIX.len() {
                return Err(RawOutputRejection::DisallowedWrapper);
            }

            let end = raw_text.len() - JSON_FENCE_SUFFIX.len();
            let body = &raw_text[start..end];

            return Ok((
                body,
                Some(NormalizationKind::ExactOuterJsonMarkdownFenceRemoval),
            ));
        }

        if raw_text.starts_with("```") || raw_text.ends_with("```") || raw_text.ends_with("```\n") {
            return Err(RawOutputRejection::DisallowedWrapper);
        }

        Ok((raw_text, None))
    }

    fn parse_complete_json(raw_text: &str) -> Result<Value, RawOutputRejection> {
        let mut deserializer = serde_json::Deserializer::from_str(raw_text);

        let candidate =
            Value::deserialize(&mut deserializer).map_err(|_| RawOutputRejection::InvalidJson)?;

        deserializer
            .end()
            .map_err(|_| RawOutputRejection::TrailingContent)?;

        Ok(candidate)
    }

    fn is_schema_valid(candidate: &Value) -> bool {
        let schema: Value = match serde_json::from_str(AGENT_OUTPUT_SCHEMA) {
            Ok(value) => value,
            Err(_) => return false,
        };

        let validator = match jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
        {
            Ok(value) => value,
            Err(_) => return false,
        };

        validator.is_valid(candidate)
    }
}
