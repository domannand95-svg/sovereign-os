use registry_service::RegistryError;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("registry failure: {0}")]
    Registry(#[from] RegistryError),

    #[error("policy violation: {0}")]
    PolicyViolation(String),

    #[error("duplicate node registration rejected: {0}")]
    DuplicateNode(Uuid),

    #[error("node not found: {0}")]
    NodeNotFound(Uuid),

    #[error("illegal lifecycle transition: {0}")]
    IllegalTransition(String),

    #[error("invalid capacity metrics: {0}")]
    InvalidCapacity(String),
}
