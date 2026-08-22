//! # Sovereign Base44 Adapter
//!
//! # Architecture Overview
//!
//! The Base44 adapter operates as an isolated translation layer within the sovereign-os
//! workspace. It guarantees that any untrusted workload originating from Base44 undergoes
//! rigorous cryptographic inspection before reaching the execution kernel.
//!
//! ## Lifecycle Flow
//! 1. **Ingress**: A [Base44IngressRequest] is received.
//! 2. **Validation**: [alidation::IngressValidator] checks the timestamp, payload size (<= 64 KiB),
//!    receipt reference format, and verifies the SHA-256 content digest.
//! 3. **Dispatch**: [Base44Dispatcher] converts the request into a GovernedExecutionRequest
//!    and invokes the kernel executor.
//! 4. **Egress**: [Base44EgressTranslator] sanitizes and maps the result into a [Base44EgressResponse].

pub mod error;
pub mod validation;

pub use error::Base44AdapterError;

use serde::{Deserialize, Serialize};
use sovereign_execution_api::{
    ExecutionApiError, ExecutionStatus, GovernedExecutionRequest, GovernedExecutionResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Base44ExecutionStatus {
    Success,
    AuthenticationFailed,
    ExecutionFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Base44IngressRequest {
    pub request_id: String,
    pub receipt_reference: String,
    pub operation: String,
    pub target: String,
    pub content_digest: String,
    pub content: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Base44EgressResponse {
    pub request_id: String,
    pub execution_id: String,
    pub status: Base44ExecutionStatus,
    pub report_reference: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Base44EgressTranslator;

impl Base44EgressTranslator {
    pub fn new() -> Self {
        Self
    }

    pub fn translate(
        &self,
        governed: GovernedExecutionResponse,
        request_id: String,
    ) -> Result<Base44EgressResponse, Base44AdapterError> {
        let status = match governed.status {
            ExecutionStatus::AuthorizedAndExecuted => Base44ExecutionStatus::Success,
            ExecutionStatus::AuthenticationFailed => Base44ExecutionStatus::AuthenticationFailed,
            ExecutionStatus::ExecutionFailed(_) => Base44ExecutionStatus::ExecutionFailed,
        };

        Ok(Base44EgressResponse {
            request_id,
            execution_id: governed.execution_id,
            status,
            report_reference: governed.report_reference,
        })
    }

    pub fn to_json(&self, response: &Base44EgressResponse) -> Result<String, Base44AdapterError> {
        serde_json::to_string(response)
            .map_err(|e| Base44AdapterError::Serialization(e.to_string()))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Base44Dispatcher;

impl Base44Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch<F>(
        &self,
        request: Base44IngressRequest,
        executor: F,
    ) -> Result<Base44EgressResponse, Base44AdapterError>
    where
        F: FnOnce(GovernedExecutionRequest) -> Result<GovernedExecutionResponse, ExecutionApiError>,
    {
        validation::IngressValidator::validate_ingress(
            &request.request_id,
            &request.receipt_reference,
            &request.operation,
            &request.target,
            &request.content_digest,
            &request.content,
            request.timestamp,
        )?;

        let governed_req = GovernedExecutionRequest {
            execution_id: request.request_id.clone(),
            authorization_receipt_id: request.receipt_reference.clone(),
            operation_payload: request.content.clone(),
        };

        let governed_res =
            executor(governed_req).map_err(|e| Base44AdapterError::ExecutionApi(e.to_string()))?;

        let translator = Base44EgressTranslator::new();
        translator.translate(governed_res, request.request_id)
    }
}
