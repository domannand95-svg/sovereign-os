use anyhow::{Context, Result};
use jsonschema::Validator;
use serde_json::Value;
use std::path::Path;

pub struct TraceSchemaValidator {
    validator: Validator,
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

        Ok(Self { validator })
    }

    /// Performs structural validation only.
    ///
    /// A successful result does not establish BETA-001 acceptance.
    pub fn validate_structure(&self, trace: &Value) -> Result<()> {
        if self.validator.is_valid(trace) {
            return Ok(());
        }

        let errors = self
            .validator
            .iter_errors(trace)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();

        anyhow::bail!("BETA-001 structural validation failed: {errors:?}")
    }
}
