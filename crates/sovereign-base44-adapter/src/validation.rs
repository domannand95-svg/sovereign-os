//! Ingress validation module for the Base44 adapter.

use crate::error::Base44AdapterError;
use sha2::Digest;

pub struct IngressValidator;

impl IngressValidator {
    pub const MAX_PAYLOAD_SIZE: usize = 64 * 1024; // 64 KiB
    pub const TIMESTAMP_WINDOW_SECS: u64 = 300; // +/- 5 minutes

    pub fn validate_ingress(
        request_id: &str,
        receipt_reference: &str,
        operation: &str,
        target: &str,
        content_digest_hex: &str,
        content: &[u8],
        timestamp: u64,
    ) -> Result<(), Base44AdapterError> {
        if request_id.is_empty() || operation.is_empty() || target.is_empty() {
            return Err(Base44AdapterError::IngressValidation(
                "Fields cannot be empty".to_string(),
            ));
        }

        if receipt_reference.len() != 64
            || !receipt_reference.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(Base44AdapterError::IngressValidation(
                "Invalid receipt reference format".to_string(),
            ));
        }

        if content.len() > Self::MAX_PAYLOAD_SIZE {
            return Err(Base44AdapterError::PayloadTooLarge {
                size: content.len(),
                limit: Self::MAX_PAYLOAD_SIZE,
            });
        }

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Base44AdapterError::InvalidTimestamp(e.to_string()))?
            .as_secs();

        if timestamp < current_time - Self::TIMESTAMP_WINDOW_SECS
            || timestamp > current_time + Self::TIMESTAMP_WINDOW_SECS
        {
            return Err(Base44AdapterError::InvalidTimestamp(
                "Timestamp outside allowable window".to_string(),
            ));
        }

        Self::verify_content_digest(content, content_digest_hex)?;

        Ok(())
    }

    pub fn verify_content_digest(
        content: &[u8],
        expected_hex: &str,
    ) -> Result<(), Base44AdapterError> {
        let computed_digest = sha2::Sha256::digest(content);
        let computed_hex = hex::encode(computed_digest);

        if computed_hex != expected_hex {
            return Err(Base44AdapterError::DigestMismatch {
                expected: expected_hex.to_string(),
                got: computed_hex,
            });
        }

        Ok(())
    }
}
