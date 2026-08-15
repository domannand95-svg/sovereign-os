use crate::edge::ObjectClass;
use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateEntity,
    UnresolvedReference,
    MissingProvenance,
    InvalidGenesisProvenance,
    UnauthorizedGenesis,
    MalformedGenesisPayload,
    MalformedCapabilityPayload,
    GenesisAlreadyEstablished,
    GenesisNotPermittedInExistingGraph,
    ObjectClassMismatch {
        expected: ObjectClass,
        actual: ObjectClass,
    },
    ObjectClassUnavailable,
    SchemaViolation,
    GraphCycleDetected,
    EmptyIdentityDescriptor,
    IdentityDescriptorTooLarge,
    UnknownIdentityKind(u8),
    UnsupportedIdentityVersion(u16),
    IdentityKindNotPermittedForVersion {
        version: u16,
        kind: u8,
    },
    UnsupportedEnvironmentSchema(u8),
    UnallocatedEnvironmentNamespace(u8),
    UnsupportedEnvironmentDigestAlgorithm(u8),
    InvalidEnvironmentDescriptorLength(usize),
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
            Self::MissingProvenance => write!(
                f,
                "Registry Error: Registry v2 non-genesis node requires at least one provenance parent."
            ),
            Self::InvalidGenesisProvenance => write!(
                f,
                "Registry Error: Registry v2 Genesis object must not declare provenance parents."
            ),
            Self::UnauthorizedGenesis => write!(
                f,
                "Registry Error: Genesis candidate does not match the governance-provisioned expected identity."
            ),
            Self::MalformedGenesisPayload => write!(
                f,
                "Registry Error: Registry v2 Genesis payload violates the canonical schema."
            ),
            Self::MalformedCapabilityPayload => write!(
                f,
                "Registry Error: Registry v2 Capability payload violates the canonical schema."
            ),
            Self::GenesisAlreadyEstablished => write!(
                f,
                "Registry Error: The authorized Registry v2 Genesis object is already established."
            ),
            Self::GenesisNotPermittedInExistingGraph => write!(
                f,
                "Registry Error: Genesis admission is not permitted in an already populated Registry graph."
            ),
            Self::ObjectClassMismatch { expected, actual } => write!(
                f,
                "Registry Error: Object class mismatch; expected {expected:?}, got {actual:?}."
            ),
            Self::ObjectClassUnavailable => write!(
                f,
                "Registry Error: Referenced object does not expose a v2 ObjectClass."
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
            Self::IdentityKindNotPermittedForVersion { version, kind } => write!(
                f,
                "Registry Error: Identity kind tag {kind} is not permitted for identity encoding version {version}."
            ),
            Self::UnsupportedEnvironmentSchema(version) => write!(
                f,
                "Registry Error: Unsupported Environment descriptor schema version {version}."
            ),
            Self::UnallocatedEnvironmentNamespace(namespace) => write!(
                f,
                "Registry Error: Environment definition namespace {namespace} is not allocated."
            ),
            Self::UnsupportedEnvironmentDigestAlgorithm(algorithm) => write!(
                f,
                "Registry Error: Unsupported Environment digest algorithm {algorithm}."
            ),
            Self::InvalidEnvironmentDescriptorLength(length) => write!(
                f,
                "Registry Error: Environment descriptor length {length} is invalid; expected exactly 35 bytes."
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
