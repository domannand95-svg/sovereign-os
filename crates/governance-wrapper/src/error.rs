use registry_service::RegistryError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("registry failure: {0}")]
    Registry(#[from] RegistryError),

    #[error("policy violation: {0}")]
    PolicyViolation(String),
}
