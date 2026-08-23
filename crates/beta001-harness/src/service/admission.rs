//! ADAM-011-C: Governed Service Gateway Admission Boundary
//!
//! Connects untrusted transport requests to the deterministic governance kernel.
//! Enforces relational binding, deterministic risk mapping, anti-replay, and pre-dispatch claim state.
//! Invariant: Admission Gate != Execution Capability (Δ Authority = 0)

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use sha2::digest::Digest;
use sha2::Sha256;

use crate::proposal::{
    GovernedActionProposal, ProposedOperation as CoreProposedOperation, ProposalValidationError,
};
use crate::risk::{
    RiskEvaluator, RiskEvaluationContext, RiskLevel as CoreRiskLevel, BlastRadius as CoreBlastRadius,
};
use crate::approval::{
    ApprovalReceipt, ApprovalLevel as CoreApprovalLevel, ApprovalValidationError,
};
use crate::service_contract::{
    ProposalRequest, ProposalResponse, ExecutionRequest, ExecutionResponse,
    RiskContext, ProposalId, ExecutionId, Sha256Digest,
    SchemaVersionV1, AuthorityDeltaZero, ProposedOperation,
    LifecycleState, RiskLevel, BlastRadius, RequiredApprovalLevel,
    ClaimState, DispatchState, OutcomeState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    ProposalValidationFailed(ProposalValidationError),
    ProposalNotFound,
    RelationalMismatch(String),
    InvalidApproval(ApprovalValidationError),
    RiskContextMismatch,
    InsufficientApprovalScope,
    ReplayDetected,
}

impl From<ProposalValidationError> for AdmissionError {
    fn from(err: ProposalValidationError) -> Self {
        AdmissionError::ProposalValidationFailed(err)
    }
}

impl From<ApprovalValidationError> for AdmissionError {
    fn from(err: ApprovalValidationError) -> Self {
        AdmissionError::InvalidApproval(err)
    }
}

/// Proposal Admission Gate: Ingests raw intent, computes risk, and admits proposal state.
pub struct ProposalAdmissionGate {
    admitted_proposals: Mutex<HashMap<String, (GovernedActionProposal, RiskEvaluationContext)>>,
    seen_proposal_digests: Mutex<HashSet<String>>,
}

impl Default for ProposalAdmissionGate {
    fn default() -> Self {
        Self::new()
    }
}

impl ProposalAdmissionGate {
    pub fn new() -> Self {
        Self {
            admitted_proposals: Mutex::new(HashMap::new()),
            seen_proposal_digests: Mutex::new(HashSet::new()),
        }
    }

    /// Admits an untrusted proposal request into the deterministic governance kernel.
    pub fn admit_proposal(&self, req: &ProposalRequest) -> Result<ProposalResponse, AdmissionError> {
        // 1. Compute deterministic hash for provenance & proposal ID
        let mut hasher = Sha256::new();
        hasher.update(req.user_id.as_str().as_bytes());
        hasher.update(req.session_id.as_str().as_bytes());
        hasher.update(req.intent.as_bytes());
        hasher.update(format!("{:?}", req.proposed_operation).as_bytes());
        for evidence in &req.source_evidence_references {
            hasher.update(evidence.as_str().as_bytes());
        }
        hasher.update(req.timestamp.to_rfc3339().as_bytes());
        let digest_hex = format!("{:x}", hasher.finalize());

        // 2. Check replay WITHOUT mutating state (anti-poisoning)
        {
            let seen = self.seen_proposal_digests.lock().unwrap();
            if seen.contains(&digest_hex) {
                return Err(AdmissionError::ReplayDetected);
            }
        }

        let proposal_id_str = format!("prop_{}", &digest_hex[..24]);
        let proposal_id = ProposalId::new(proposal_id_str.clone())
            .map_err(|e| AdmissionError::RelationalMismatch(e.to_string()))?;

        // 3. Hydrate core GovernedActionProposal
        let proposal = GovernedActionProposal {
            schema_version: "v1.0.0".to_string(),
            proposal_id: proposal_id_str.clone(),
            proposer_identity: req.user_id.as_str().to_string(),
            source_evidence_references: req.source_evidence_references.iter().map(|e| e.as_str().to_string()).collect(),
            intent: req.intent.clone(),
            proposed_operation: match req.proposed_operation {
                ProposedOperation::EmitNotification => CoreProposedOperation::EmitNotification,
                ProposedOperation::RequestReview => CoreProposedOperation::RequestReview,
                ProposedOperation::RequestApproval => CoreProposedOperation::RequestApproval,
                ProposedOperation::RequestStateMutation => CoreProposedOperation::RequestStateMutation,
            },
            risk_evaluation_reference: None,
            created_timestamp: req.timestamp.to_rfc3339(),
            expiration_timestamp: "2099-12-31T23:59:59Z".to_string(),
            provenance_digest: digest_hex.clone(),
        };

        // 4. Validate proposal semantics via Core Kernel
        proposal.validate()?;

        // 5. Deterministic Risk Classification
        let risk_context = RiskEvaluator::evaluate_proposal(&proposal);

        let required_approval_level = match risk_context.risk_level {
            CoreRiskLevel::Low => RequiredApprovalLevel::Peer,
            CoreRiskLevel::Medium => RequiredApprovalLevel::Operator,
            CoreRiskLevel::High | CoreRiskLevel::Critical => RequiredApprovalLevel::Governance,
        };

        let provenance_digest = Sha256Digest::new(digest_hex.clone())
            .map_err(|e| AdmissionError::RelationalMismatch(e.to_string()))?;

        let response = ProposalResponse {
            schema_version: SchemaVersionV1,
            proposal_id,
            session_id: req.session_id.clone(),
            lifecycle_state: LifecycleState::AwaitingApproval,
            risk_context: RiskContext {
                context_id: risk_context.context_id.clone(),
                risk_level: match risk_context.risk_level {
                    CoreRiskLevel::Low => RiskLevel::Low,
                    CoreRiskLevel::Medium => RiskLevel::Medium,
                    CoreRiskLevel::High => RiskLevel::High,
                    CoreRiskLevel::Critical => RiskLevel::Critical,
                },
                blast_radius: match risk_context.blast_radius {
                    CoreBlastRadius::Isolated => BlastRadius::Isolated,
                    CoreBlastRadius::Subsystem => BlastRadius::Subsystem,
                    CoreBlastRadius::System => BlastRadius::System,
                    CoreBlastRadius::External => BlastRadius::External,
                },
            },
            required_approval_level,
            authority_delta: AuthorityDeltaZero,
            provenance_digest,
            timestamp: req.timestamp,
        };

        // 6. Atomically record admitted state and replay digest
        {
            let mut seen = self.seen_proposal_digests.lock().unwrap();
            seen.insert(digest_hex);
        }
        {
            let mut store = self.admitted_proposals.lock().unwrap();
            store.insert(proposal_id_str, (proposal, risk_context));
        }

        Ok(response)
    }

    /// Retrieve admitted proposal and risk context for downstream execution binding
    pub fn get_admitted(&self, proposal_id: &str) -> Option<(GovernedActionProposal, RiskEvaluationContext)> {
        let store = self.admitted_proposals.lock().unwrap();
        store.get(proposal_id).cloned()
    }
}

/// Execution Admission Gate: Evaluates authorization binding and claims pre-dispatch execution tokens.
pub struct ExecutionAdmissionGate {
    consumed_authorizations: Mutex<HashSet<(String, String)>>,
}

impl Default for ExecutionAdmissionGate {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionAdmissionGate {
    pub fn new() -> Self {
        Self {
            consumed_authorizations: Mutex::new(HashSet::new()),
        }
    }

    /// Evaluates execution admission against admitted state, verifies cryptographic bindings,
    /// checks replay, and reserves an ExecutionId.
    pub fn admit_execution(
        &self,
        req: &ExecutionRequest,
        proposal_gate: &ProposalAdmissionGate,
        receipt: &ApprovalReceipt,
    ) -> Result<ExecutionResponse, AdmissionError> {
        // 1. Authoritative lookup from ProposalAdmissionGate
        let (proposal, risk_ctx) = proposal_gate
            .get_admitted(req.proposal_id.as_str())
            .ok_or(AdmissionError::ProposalNotFound)?;

        // 2. Relational Matching: Request vs Proposal vs Receipt
        if req.proposal_id.as_str() != proposal.proposal_id {
            return Err(AdmissionError::RelationalMismatch(
                "ExecutionRequest.proposal_id does not match GovernedActionProposal.proposal_id".to_string(),
            ));
        }

        if req.approval_receipt_id.as_str() != receipt.receipt_id {
            return Err(AdmissionError::RelationalMismatch(
                "ExecutionRequest.approval_receipt_id does not match ApprovalReceipt.receipt_id".to_string(),
            ));
        }

        if receipt.proposal_id != proposal.proposal_id {
            return Err(AdmissionError::RelationalMismatch(
                "ApprovalReceipt.proposal_id does not match GovernedActionProposal.proposal_id".to_string(),
            ));
        }

        // 3. Receipt Integrity Validation
        receipt.validate_integrity()?;

        // 4. Risk Context Binding Match
        if receipt.risk_context_id != risk_ctx.context_id {
            return Err(AdmissionError::RiskContextMismatch);
        }

        // 5. Approval Scope & Hierarchy Check
        let required_level = match risk_ctx.risk_level {
            CoreRiskLevel::Low => CoreApprovalLevel::Peer,
            CoreRiskLevel::Medium => CoreApprovalLevel::Operator,
            CoreRiskLevel::High | CoreRiskLevel::Critical => CoreApprovalLevel::Governance,
        };
        receipt.verify_scope(required_level)
            .map_err(|_| AdmissionError::InsufficientApprovalScope)?;

        // 6. Anti-Replay: Consume (proposal_id, approval_receipt_id) tuple
        let auth_key = (proposal.proposal_id.clone(), receipt.receipt_id.clone());
        {
            let mut consumed = self.consumed_authorizations.lock().unwrap();
            if consumed.contains(&auth_key) {
                return Err(AdmissionError::ReplayDetected);
            }
            consumed.insert(auth_key);
        }

        // 7. Atomically derive unique execution_id
        let mut hasher = Sha256::new();
        hasher.update(proposal.proposal_id.as_bytes());
        hasher.update(receipt.receipt_id.as_bytes());
        hasher.update(req.caller_identity.as_bytes());
        hasher.update(req.timestamp.to_rfc3339().as_bytes());
        let exec_digest = format!("{:x}", hasher.finalize());
        let execution_id_str = format!("exe_{}", &exec_digest[..24]);

        let execution_id = ExecutionId::new(execution_id_str)
            .map_err(|e| AdmissionError::RelationalMismatch(e.to_string()))?;

        let evidence_package_digest = Sha256Digest::new(proposal.provenance_digest.clone())
            .map_err(|e| AdmissionError::RelationalMismatch(e.to_string()))?;

        // 8. Emit Pre-Dispatch ExecutionResponse (NOT_DISPATCHED / PENDING)
        Ok(ExecutionResponse {
            schema_version: SchemaVersionV1,
            execution_id,
            proposal_id: req.proposal_id.clone(),
            approval_receipt_id: req.approval_receipt_id.clone(),
            claim_state: ClaimState::ExecutionClaimed,
            dispatch_state: DispatchState::NotDispatched,
            outcome_state: OutcomeState::Pending,
            authority_delta: AuthorityDeltaZero,
            evidence_package_digest,
            timestamp: req.timestamp,
        })
    }
}