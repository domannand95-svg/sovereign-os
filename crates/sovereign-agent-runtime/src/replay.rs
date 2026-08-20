//! Deterministic, side-effect-free reconstruction of agent runtime authority.

use crate::capability::{parse_timestamp, CapabilityGrant, GrantId, GrantStatus};
use crate::execution::{
    authorize_execution, verify_receipt, AttemptId, ExecutionAttempt, ExecutionReceipt,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    GrantAdmitted {
        grant: CapabilityGrant,
        recorded_at: String,
    },
    GrantRevoked {
        grant_id: GrantId,
        recorded_at: String,
    },
    ExecutionAuthorized {
        attempt: ExecutionAttempt,
        recorded_at: String,
    },
    ReceiptRecorded {
        receipt: ExecutionReceipt,
        recorded_at: String,
    },
}

impl RuntimeEvent {
    fn recorded_at(&self) -> &str {
        match self {
            Self::GrantAdmitted { recorded_at, .. }
            | Self::GrantRevoked { recorded_at, .. }
            | Self::ExecutionAuthorized { recorded_at, .. }
            | Self::ReceiptRecorded { recorded_at, .. } => recorded_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayedRuntimeState {
    pub grants: HashMap<GrantId, GrantStatus>,
    pub consumed_grants: HashSet<GrantId>,
    pub authorized_attempts: HashMap<AttemptId, GrantId>,
    pub recorded_receipts: HashSet<AttemptId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    InvalidTimestamp,
    NonMonotonicTime,
    DuplicateGrant,
    UnknownGrant,
    InactiveGrant,
    DuplicateExecution,
    InvalidExecution,
    InvalidReceipt,
    ReceiptWithoutExecution,
    DuplicateReceipt,
    ReceiptBindingMismatch,
}

pub fn replay(
    events: &[RuntimeEvent],
    execution_verifying_key: &[u8; 32],
) -> Result<ReplayedRuntimeState, ReplayError> {
    let mut grants = HashMap::<GrantId, CapabilityGrant>::new();
    let mut consumed_grants = HashSet::new();
    let mut authorized_attempts = HashMap::<AttemptId, GrantId>::new();
    let mut recorded_receipts = HashSet::new();
    let mut previous_time = None;

    for event in events {
        let timestamp =
            parse_timestamp(event.recorded_at()).map_err(|_| ReplayError::InvalidTimestamp)?;
        if previous_time
            .as_ref()
            .is_some_and(|previous| timestamp < *previous)
        {
            return Err(ReplayError::NonMonotonicTime);
        }
        previous_time = Some(timestamp);

        match event {
            RuntimeEvent::GrantAdmitted { grant, .. } => {
                if grant.status != GrantStatus::Active {
                    return Err(ReplayError::InactiveGrant);
                }
                if grants
                    .insert(grant.grant_id.clone(), grant.clone())
                    .is_some()
                {
                    return Err(ReplayError::DuplicateGrant);
                }
            }
            RuntimeEvent::GrantRevoked { grant_id, .. } => {
                let grant = grants.get_mut(grant_id).ok_or(ReplayError::UnknownGrant)?;
                grant.status = GrantStatus::Revoked;
            }
            RuntimeEvent::ExecutionAuthorized {
                attempt,
                recorded_at,
            } => {
                let grant = grants
                    .get(&attempt.grant_reference)
                    .ok_or(ReplayError::UnknownGrant)?;
                if consumed_grants.contains(&grant.grant_id)
                    || authorized_attempts.contains_key(&attempt.attempt_id)
                {
                    return Err(ReplayError::DuplicateExecution);
                }
                authorize_execution(grant, attempt, recorded_at)
                    .map_err(|_| ReplayError::InvalidExecution)?;
                consumed_grants.insert(grant.grant_id.clone());
                authorized_attempts.insert(attempt.attempt_id.clone(), grant.grant_id.clone());
            }
            RuntimeEvent::ReceiptRecorded { receipt, .. } => {
                if !verify_receipt(receipt, execution_verifying_key) {
                    return Err(ReplayError::InvalidReceipt);
                }
                let expected_grant = authorized_attempts
                    .get(&receipt.attempt_reference)
                    .ok_or(ReplayError::ReceiptWithoutExecution)?;
                if expected_grant != &receipt.grant_reference {
                    return Err(ReplayError::ReceiptBindingMismatch);
                }
                if !recorded_receipts.insert(receipt.attempt_reference.clone()) {
                    return Err(ReplayError::DuplicateReceipt);
                }
            }
        }
    }

    Ok(ReplayedRuntimeState {
        grants: grants
            .into_iter()
            .map(|(grant_id, grant)| (grant_id, grant.status))
            .collect(),
        consumed_grants,
        authorized_attempts,
        recorded_receipts,
    })
}
