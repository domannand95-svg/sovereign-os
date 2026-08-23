//! # Sovereign Base44 Adapter
//!
//! The Base44 adapter is an isolated translation layer.
//! It validates external ingress data, converts it into governed execution
//! requests, and translates governed responses back into Base44 responses.
//!
//! ## Lifecycle Flow
//!
//! Base44IngressRequest
//!     -> validation
//!     -> GovernedExecutionRequest
//!     -> ExecutionApiFacade
//!     -> KernelInvoker
//!     -> GovernedExecutionResponse
//!     -> Base44EgressResponse

pub mod error;
pub mod validation;

pub use error::Base44AdapterError;

use serde::{Deserialize, Serialize};
use sovereign_execution_api::{
    ExecutionApiFacade, ExecutionStatus, GovernedExecutionRequest, GovernedExecutionResponse,
    KernelInvoker,
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
            ExecutionStatus::ExecutionFailed => Base44ExecutionStatus::ExecutionFailed,
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
            .map_err(|error| Base44AdapterError::Serialization(error.to_string()))
    }
}

pub struct Base44Dispatcher<K: KernelInvoker> {
    api: ExecutionApiFacade<K>,
}

impl<K: KernelInvoker> Base44Dispatcher<K> {
    pub fn new(api: ExecutionApiFacade<K>) -> Self {
        Self { api }
    }

    pub fn dispatch(
        &self,
        request: Base44IngressRequest,
    ) -> Result<Base44EgressResponse, Base44AdapterError> {
        validation::IngressValidator::validate_ingress(
            &request.request_id,
            &request.receipt_reference,
            &request.operation,
            &request.target,
            &request.content_digest,
            &request.content,
            request.timestamp,
        )?;

        let governed_request = GovernedExecutionRequest {
            execution_id: request.request_id.clone(),
            authorization_receipt_id: request.receipt_reference.clone(),
            operation_payload: request.content.clone(),
        };

        let governed_response = self
            .api
            .execute(governed_request)
            .map_err(|error| Base44AdapterError::ExecutionApi(error.to_string()))?;

        Base44EgressTranslator::new().translate(governed_response, request.request_id)
    }
}
