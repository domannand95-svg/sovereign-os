use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateEntity,
    UnresolvedReference,
    SchemaViolation,
    GraphCycleDetected,
    EmptyIdentityDescriptor,
    IdentityDescriptorTooLarge,
    UnknownIdentityKind(u8),
    UnsupportedIdentityVersion(u16),
    TruncatedIdentityEncoding,
    TrailingIdentityBytes,
    ZeroIdentity,
    TooManyLineageParents,
    DuplicateLineageParent,
    SelfReferentialLineage,
    UnsupportedLineageVersion(u16),
    TruncatedLineageEncoding,
    TrailingLineageBytes,
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
            Self::EmptyIdentityDescriptor => {
                write!(f, "Registry Error: Identity descriptor must not be empty.")
            }
            Self::IdentityDescriptorTooLarge => {
                write!(f, "Registry Error: Identity descriptor exceeds its limit.")
            }
            Self::UnknownIdentityKind(kind) => {
                write!(f, "Registry Error: Unknown identity kind tag {kind}.")
            }
            Self::UnsupportedIdentityVersion(version) => write!(
                f,
                "Registry Error: Unsupported identity encoding version {version}."
            ),
            Self::TruncatedIdentityEncoding => {
                write!(f, "Registry Error: Identity encoding is truncated.")
            }
            Self::TrailingIdentityBytes => {
                write!(
                    f,
                    "Registry Error: Identity encoding contains trailing bytes."
                )
            }
            Self::ZeroIdentity => {
                write!(f, "Registry Error: Zero identity is not valid in lineage.")
            }
            Self::TooManyLineageParents => {
                write!(f, "Registry Error: Lineage parent count exceeds its limit.")
            }
            Self::DuplicateLineageParent => {
                write!(f, "Registry Error: Lineage contains a duplicate parent.")
            }
            Self::SelfReferentialLineage => {
                write!(
                    f,
                    "Registry Error: Identity cannot be its own lineage parent."
                )
            }
            Self::UnsupportedLineageVersion(version) => write!(
                f,
                "Registry Error: Unsupported lineage encoding version {version}."
            ),
            Self::TruncatedLineageEncoding => {
                write!(f, "Registry Error: Lineage encoding is truncated.")
            }
            Self::TrailingLineageBytes => {
                write!(
                    f,
                    "Registry Error: Lineage encoding contains trailing bytes."
                )
            }
        }
    }
}

impl Error for RegistryError {}
