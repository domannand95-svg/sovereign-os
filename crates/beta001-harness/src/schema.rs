use anyhow::{Context, Result};
use jsonschema::Validator;
use serde_json::{json, Value};
use std::path::Path;

pub struct TraceSchemaValidator {
    validator: Validator,
    candidate_events_validator: Validator,
}

impl TraceSchemaValidator {
    pub fn new<P: AsRef<Path>>(schema_path: P) -> Result<Self> {
        let schema_text =
            std::fs::read_to_string(schema_path).context("failed to read BETA-001 trace schema")?;

        let schema: Value =
            serde_json::from_str(&schema_text).context("failed to parse BETA-001 trace schema")?;

        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .context("failed to compile BETA-001 Draft 2020-12 trace schema")?;

        let definitions = schema
            .get("$defs")
            .cloned()
            .context("BETA-001 trace schema is missing $defs")?;

        let candidate_events_schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": definitions,
            "type": "array",
            "items": {
                "$ref": "#/$defs/event"
            }
        });

        let candidate_events_validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&candidate_events_schema)
            .context("failed to compile BETA-001 candidate-event schema view")?;

        Ok(Self {
            validator,
            candidate_events_validator,
        })
    }

    /// Performs structural validation of a complete authoritative trace only.
    ///
    /// A successful result does not establish BETA-001 acceptance.
    pub fn validate_structure(&self, trace: &Value) -> Result<()> {
        validate_with(
            &self.validator,
            trace,
            "BETA-001 structural validation failed",
        )
    }

    /// Performs structural validation of candidate events using the canonical
    /// event definition from BETA-001-TRACE-v1.
    ///
    /// This does not weaken or replace the canonical complete-trace schema.
    pub fn validate_candidate_events(&self, events: &Value) -> Result<()> {
        validate_with(
            &self.candidate_events_validator,
            events,
            "BETA-001 candidate-event structural validation failed",
        )
    }
}

fn validate_with(validator: &Validator, value: &Value, prefix: &str) -> Result<()> {
    if validator.is_valid(value) {
        return Ok(());
    }

    let errors = validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

    anyhow::bail!("{prefix}: {errors:?}")
}
