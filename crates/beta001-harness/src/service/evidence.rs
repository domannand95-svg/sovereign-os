//! ADAM-011-F: End-to-End Service Evidence Package Closure
//!
//! Provides aggregation, canonical domain-separated sealing, and complete cryptographic
//! and relational chain verification across all lifecycle artifacts.
//!
//! Invariant: Verified Evidence Chain != Authorization Proof (Δ Authority = 0)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::digest::Digest;
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;

use crate::approval::{ApprovalReceipt, ApprovalValidationError};
use crate::risk::RiskEvaluationContext;
use crate::service::client::SanitizedProposalRequest;
use crate::service::inference::InferenceGatewayAdapter;
use crate::service_contract::{
    ExecutionResponse, InferenceGateway, ProposalResponse, Sha256Digest, TransportStatus,
};

pub const SERVICE_EVIDENCE_DOMAIN_TAG: &str = "SOVEREIGN_OS:SERVICE_EVIDENCE:v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceClosureError {
    RelationalMismatch(String),
    IntegrityViolation(String),
    DigestDerivationFailed(String),
    ApprovalValidationFailed(ApprovalValidationError),
}

impl From<ApprovalValidationError> for EvidenceClosureError {
    fn from(err: ApprovalValidationError) -> Self {
        EvidenceClosureError::ApprovalValidationFailed(err)
    }
}

/// Sealed, immutable end-to-end evidence package capturing all stages of a governed interaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEvidencePackage {
    pub sanitized_ingress: SanitizedProposalRequest,
    pub proposal_response: ProposalResponse,
    pub risk_context: RiskEvaluationContext,
    pub approval_receipt: ApprovalReceipt,
    pub execution_response: ExecutionResponse,
    pub inference_records: Vec<InferenceGateway>,
    pub package_digest: Sha256Digest,
    pub closed_timestamp: DateTime<Utc>,
}

impl ServiceEvidencePackage {
    /// Builds and seals a ServiceEvidencePackage after verifying complete relational and hash integrity.
    pub fn build(
        sanitized_ingress: SanitizedProposalRequest,
        proposal_response: ProposalResponse,
        risk_context: RiskEvaluationContext,
        approval_receipt: ApprovalReceipt,
        execution_response: ExecutionResponse,
        inference_records: Vec<InferenceGateway>,
        closed_timestamp: DateTime<Utc>,
    ) -> Result<Self, EvidenceClosureError> {
        let package_digest = Self::derive_package_digest(
            &sanitized_ingress,
            &proposal_response,
            &risk_context,
            &approval_receipt,
            &execution_response,
            &inference_records,
            &closed_timestamp,
        )?;

        let package = Self {
            sanitized_ingress,
            proposal_response,
            risk_context,
            approval_receipt,
            execution_response,
            inference_records,
            package_digest,
            closed_timestamp,
        };

        // Enforce end-to-end verification upon build
        package.verify_chain()?;

        Ok(package)
    }

    /// Derives the canonical domain-separated package digest over length-prefixed component digests.
    pub fn derive_package_digest(
        sanitized_ingress: &SanitizedProposalRequest,
        proposal_response: &ProposalResponse,
        risk_context: &RiskEvaluationContext,
        approval_receipt: &ApprovalReceipt,
        execution_response: &ExecutionResponse,
        inference_records: &[InferenceGateway],
        closed_timestamp: &DateTime<Utc>,
    ) -> Result<Sha256Digest, EvidenceClosureError> {
        let mut hasher = Sha256::new();

        // 1. Domain tag
        hasher.update(format!("{}:", SERVICE_EVIDENCE_DOMAIN_TAG.len()).as_bytes());
        hasher.update(SERVICE_EVIDENCE_DOMAIN_TAG.as_bytes());

        // 2. Ingress Raw Intent Digest
        hasher
            .update(format!("{}:", sanitized_ingress.raw_intent_digest.as_str().len()).as_bytes());
        hasher.update(sanitized_ingress.raw_intent_digest.as_str().as_bytes());

        // 3. Ingress Normalized Intent Digest
        hasher.update(
            format!(
                "{}:",
                sanitized_ingress.normalized_intent_digest.as_str().len()
            )
            .as_bytes(),
        );
        hasher.update(
            sanitized_ingress
                .normalized_intent_digest
                .as_str()
                .as_bytes(),
        );

        // 4. Proposal Provenance Digest
        hasher
            .update(format!("{}:", proposal_response.provenance_digest.as_str().len()).as_bytes());
        hasher.update(proposal_response.provenance_digest.as_str().as_bytes());

        // 5. Proposal ID
        hasher.update(format!("{}:", proposal_response.proposal_id.as_str().len()).as_bytes());
        hasher.update(proposal_response.proposal_id.as_str().as_bytes());

        // 6. Risk Context ID
        hasher.update(format!("{}:", risk_context.context_id.len()).as_bytes());
        hasher.update(risk_context.context_id.as_bytes());

        // 7. Approval Receipt ID
        hasher.update(format!("{}:", approval_receipt.receipt_id.len()).as_bytes());
        hasher.update(approval_receipt.receipt_id.as_bytes());

        // 8. Execution ID
        hasher.update(format!("{}:", execution_response.execution_id.as_str().len()).as_bytes());
        hasher.update(execution_response.execution_id.as_str().as_bytes());

        // 9. Execution Evidence Package Digest
        hasher.update(
            format!(
                "{}:",
                execution_response.evidence_package_digest.as_str().len()
            )
            .as_bytes(),
        );
        hasher.update(
            execution_response
                .evidence_package_digest
                .as_str()
                .as_bytes(),
        );

        // 10. Sequential Inference Count & Position-bound Envelopes (T011-F-004, F011-005)
        let count_str = inference_records.len().to_string();
        hasher.update(format!("{}:", count_str.len()).as_bytes());
        hasher.update(count_str.as_bytes());

        for (idx, inf) in inference_records.iter().enumerate() {
            let idx_str = idx.to_string();
            hasher.update(format!("{}:", idx_str.len()).as_bytes());
            hasher.update(idx_str.as_bytes());

            hasher.update(format!("{}:", inf.evidence_seal_digest.as_str().len()).as_bytes());
            hasher.update(inf.evidence_seal_digest.as_str().as_bytes());
        }

        // 11. Closed Timestamp
        let ts_str = closed_timestamp.to_rfc3339();
        hasher.update(format!("{}:", ts_str.len()).as_bytes());
        hasher.update(ts_str.as_bytes());

        let seal_hex = format!("{:x}", hasher.finalize());
        Sha256Digest::new(seal_hex)
            .map_err(|e| EvidenceClosureError::DigestDerivationFailed(e.to_string()))
    }

    /// Recomputes all derivable component digests, checks relational bindings,
    /// and verifies cryptographic signatures end-to-end.
    pub fn verify_chain(&self) -> Result<(), EvidenceClosureError> {
        // 1. Relational Identity Bindings (F011-003)
        let session_id = &self.sanitized_ingress.original_request.session_id;
        if self.proposal_response.session_id != *session_id {
            return Err(EvidenceClosureError::RelationalMismatch(
                "ProposalResponse session_id mismatch with Ingress session_id".to_string(),
            ));
        }

        for (idx, inf) in self.inference_records.iter().enumerate() {
            if inf.session_id != *session_id {
                return Err(EvidenceClosureError::RelationalMismatch(format!(
                    "InferenceRecord[{}] session_id mismatch with Ingress session_id",
                    idx
                )));
            }
        }

        let proposal_id_str = self.proposal_response.proposal_id.as_str();
        if self.risk_context.proposal_id != proposal_id_str {
            return Err(EvidenceClosureError::RelationalMismatch(
                "RiskEvaluationContext proposal_id mismatch with ProposalResponse proposal_id"
                    .to_string(),
            ));
        }

        if self.approval_receipt.proposal_id != proposal_id_str {
            return Err(EvidenceClosureError::RelationalMismatch(
                "ApprovalReceipt proposal_id mismatch with ProposalResponse proposal_id"
                    .to_string(),
            ));
        }

        if self.execution_response.proposal_id.as_str() != proposal_id_str {
            return Err(EvidenceClosureError::RelationalMismatch(
                "ExecutionResponse proposal_id mismatch with ProposalResponse proposal_id"
                    .to_string(),
            ));
        }

        if self.proposal_response.risk_context.context_id != self.risk_context.context_id {
            return Err(EvidenceClosureError::RelationalMismatch(
                "ProposalResponse risk_context.context_id mismatch with RiskEvaluationContext"
                    .to_string(),
            ));
        }

        if self.approval_receipt.risk_context_id != self.risk_context.context_id {
            return Err(EvidenceClosureError::RelationalMismatch(
                "ApprovalReceipt risk_context_id mismatch with RiskEvaluationContext context_id"
                    .to_string(),
            ));
        }

        if self.execution_response.approval_receipt_id.as_str() != self.approval_receipt.receipt_id
        {
            return Err(EvidenceClosureError::RelationalMismatch(
                "ExecutionResponse approval_receipt_id mismatch with ApprovalReceipt receipt_id"
                    .to_string(),
            ));
        }

        // Authority deltas must be structurally zero
        if self.proposal_response.authority_delta.value() != 0
            || self.execution_response.authority_delta.value() != 0
        {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Non-zero authority delta detected in evidence package".to_string(),
            ));
        }

        // 2. Recompute Protected Raw & Normalized Ingress Intent (F011-004)
        let mut raw_hasher = Sha256::new();
        raw_hasher.update(self.sanitized_ingress.original_request.intent.as_bytes());
        let expected_raw_digest = format!("{:x}", raw_hasher.finalize());
        if expected_raw_digest != self.sanitized_ingress.raw_intent_digest.as_str() {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Recomputed raw_intent_digest does not match stored binding".to_string(),
            ));
        }

        let filtered: String = self
            .sanitized_ingress
            .original_request
            .intent
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect();
        let expected_norm_text: String = filtered.nfkc().collect();
        if expected_norm_text != self.sanitized_ingress.normalized_intent {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Recomputed normalized_intent text does not match stored normalized string"
                    .to_string(),
            ));
        }

        let mut norm_hasher = Sha256::new();
        norm_hasher.update(self.sanitized_ingress.normalized_intent.as_bytes());
        let expected_norm_digest = format!("{:x}", norm_hasher.finalize());
        if expected_norm_digest != self.sanitized_ingress.normalized_intent_digest.as_str() {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Recomputed normalized_intent_digest does not match stored binding".to_string(),
            ));
        }

        // 3. Recompute Proposal Intake Provenance Hash (F011-004)
        let mut prop_hasher = Sha256::new();
        prop_hasher.update(
            self.sanitized_ingress
                .original_request
                .user_id
                .as_str()
                .as_bytes(),
        );
        prop_hasher.update(
            self.sanitized_ingress
                .original_request
                .session_id
                .as_str()
                .as_bytes(),
        );
        prop_hasher.update(self.sanitized_ingress.original_request.intent.as_bytes());
        prop_hasher.update(
            format!(
                "{:?}",
                self.sanitized_ingress.original_request.proposed_operation
            )
            .as_bytes(),
        );
        for evidence in &self
            .sanitized_ingress
            .original_request
            .source_evidence_references
        {
            prop_hasher.update(evidence.as_str().as_bytes());
        }
        prop_hasher.update(
            self.sanitized_ingress
                .original_request
                .timestamp
                .to_rfc3339()
                .as_bytes(),
        );
        let expected_prop_digest = format!("{:x}", prop_hasher.finalize());

        if expected_prop_digest != self.proposal_response.provenance_digest.as_str() {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Recomputed proposal provenance_digest does not match ProposalResponse".to_string(),
            ));
        }

        let expected_prop_id = format!("prop_{}", &expected_prop_digest[..24]);
        if expected_prop_id != self.proposal_response.proposal_id.as_str() {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Derived proposal_id does not match ProposalResponse proposal_id".to_string(),
            ));
        }

        if self.execution_response.evidence_package_digest.as_str()
            != self.proposal_response.provenance_digest.as_str()
        {
            return Err(EvidenceClosureError::IntegrityViolation(
                "ExecutionResponse evidence_package_digest does not match ProposalResponse provenance_digest".to_string(),
            ));
        }

        // 4. Validate Approval Receipt Cryptographic Integrity (F011-004)
        self.approval_receipt.validate_integrity()?;

        // 5. Recompute Inference Record Seals (F011-004, F011-005)
        for (idx, inf) in self.inference_records.iter().enumerate() {
            if inf.transport_status == TransportStatus::Success {
                let raw_out = inf.raw_model_output.as_ref().ok_or_else(|| {
                    EvidenceClosureError::IntegrityViolation(format!(
                        "InferenceRecord[{}] has TransportStatus::Success but missing raw_model_output",
                        idx
                    ))
                })?;
                let mut resp_hasher = Sha256::new();
                resp_hasher.update(raw_out.as_bytes());
                let expected_resp_digest = format!("{:x}", resp_hasher.finalize());
                let stored_resp_digest = inf.response_digest.as_ref().ok_or_else(|| {
                    EvidenceClosureError::IntegrityViolation(format!(
                        "InferenceRecord[{}] has TransportStatus::Success but missing response_digest",
                        idx
                    ))
                })?;
                if expected_resp_digest != stored_resp_digest.as_str() {
                    return Err(EvidenceClosureError::IntegrityViolation(format!(
                        "InferenceRecord[{}] response_digest mismatch",
                        idx
                    )));
                }
            }

            let expected_seal = InferenceGatewayAdapter::derive_seal_digest(
                &inf.session_id,
                &inf.provider_identity,
                &inf.adapter_identity,
                &inf.request_digest,
                inf.response_digest.as_ref(),
                inf.transport_status,
                &inf.timestamp,
            )
            .map_err(|e| EvidenceClosureError::DigestDerivationFailed(format!("{:?}", e)))?;

            if expected_seal != inf.evidence_seal_digest {
                return Err(EvidenceClosureError::IntegrityViolation(format!(
                    "InferenceRecord[{}] evidence_seal_digest recomputation failed",
                    idx
                )));
            }
        }

        // 6. Recompute Cumulative Package Seal (F011-002, F011-004)
        let expected_package_digest = Self::derive_package_digest(
            &self.sanitized_ingress,
            &self.proposal_response,
            &self.risk_context,
            &self.approval_receipt,
            &self.execution_response,
            &self.inference_records,
            &self.closed_timestamp,
        )?;

        if expected_package_digest != self.package_digest {
            return Err(EvidenceClosureError::IntegrityViolation(
                "Cumulative package_digest does not match recomputed package seal".to_string(),
            ));
        }

        Ok(())
    }
}
