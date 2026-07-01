use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry error: {0}")]
        General(String),
        }