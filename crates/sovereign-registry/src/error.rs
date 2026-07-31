use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateEntity,
    UnresolvedReference,
    SchemaViolation,
    GraphCycleDetected,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEntity => {
                write!(f, "Registry Error: Duplicate entity identifier detected.")
            }
            Self::UnresolvedReference => write!(
                f,
                "Registry Error: Targeted capability reference is unresolved."
            ),
            Self::SchemaViolation => write!(
                f,
                "Registry Error: Node metadata schema format validation failure."
            ),
            Self::GraphCycleDetected => write!(
                f,
                "Registry Error: Acyclic constraint breached; graph cycle detected."
            ),
        }
    }
}

impl Error for RegistryError {}
