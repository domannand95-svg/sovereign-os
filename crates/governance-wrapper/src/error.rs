use thiserror::Error;

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("governance denied: {0}")]
    Denied(String),
}
