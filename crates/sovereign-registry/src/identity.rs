use crate::RegistryError;
use std::collections::HashSet;
use std::fmt;

pub const IDENTITY_ENCODING_VERSION: u16 = 1;
pub const IDENTITY_ENCODING_VERSION_V2: u16 = 2;
pub const LINEAGE_ENCODING_VERSION: u16 = 1;
pub const MAX_IDENTITY_DESCRIPTOR_LEN: usize = 64 * 1024;
pub const MAX_LINEAGE_PARENTS: usize = 64;

pub const ENVIRONMENT_DESCRIPTOR_VERSION: u8 = 0x01;
pub const ENVIRONMENT_DESCRIPTOR_LEN: usize = 35;
pub const ENVIRONMENT_TEST_NAMESPACE: u8 = 0xAA;
pub const ENVIRONMENT_DIGEST_SHA256: u8 = 0x01;
pub const ENVIRONMENT_DIGEST_BLAKE3: u8 = 0x02;

const IDENTITY_DOMAIN_SEPARATOR: &[u8] = b"SOVEREIGN_IDENTITY_V1";
const LINEAGE_DOMAIN_SEPARATOR: &[u8] = b"SOVEREIGN_LINEAGE_V1";
const IDENTITY_HEADER_LEN: usize = 2 + 1 + 4;
const LINEAGE_HEADER_LEN: usize = 2 + 32 + 2;
const ID_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum IdentityKind {
    Agent = 0x01,
    ModelVersion = 0x02,
    Tool = 0x03,
    Dataset = 0x04,
    Policy = 0x05,
    Evaluator = 0x06,
    Institution = 0x07,
    Environment = 0x08,
}

impl IdentityKind {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self, RegistryError> {
        match value {
            0x01 => Ok(Self::Agent),
            0x02 => Ok(Self::ModelVersion),
            0x03 => Ok(Self::Tool),
            0x04 => Ok(Self::Dataset),
            0x05 => Ok(Self::Policy),
            0x06 => Ok(Self::Evaluator),
            0x07 => Ok(Self::Institution),
            0x08 => Ok(Self::Environment),
            _ => Err(RegistryError::UnknownIdentityKind(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentityId([u8; ID_LEN]);

impl IdentityId {
    pub const fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

impl fmt::Display for IdentityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineageRoot([u8; ID_LEN]);

impl LineageRoot {
    pub const fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for LineageRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityRecord {
    encoding_version: u16,
    kind: IdentityKind,
    descriptor: Vec<u8>,
    id: IdentityId,
}

impl IdentityRecord {
    pub fn new(kind: IdentityKind, descriptor: Vec<u8>) -> Result<Self, RegistryError> {
        Self::new_versioned(
            IDENTITY_ENCODING_VERSION,
            kind,
            descriptor,
            EnvironmentNamespaceMode::Production,
        )
    }

    pub fn new_environment(
        definition_namespace: u8,
        digest_algorithm: u8,
        definition_digest: [u8; 32],
    ) -> Result<Self, RegistryError> {
        let descriptor = encode_environment_descriptor(
            definition_namespace,
            digest_algorithm,
            definition_digest,
        );

        Self::new_versioned(
            IDENTITY_ENCODING_VERSION_V2,
            IdentityKind::Environment,
            descriptor,
            EnvironmentNamespaceMode::Production,
        )
    }

    fn new_versioned(
        encoding_version: u16,
        kind: IdentityKind,
        descriptor: Vec<u8>,
        namespace_mode: EnvironmentNamespaceMode,
    ) -> Result<Self, RegistryError> {
        validate_identity_kind_for_version(encoding_version, kind)?;
        validate_descriptor(&descriptor)?;

        if kind == IdentityKind::Environment {
            validate_environment_descriptor(&descriptor, namespace_mode)?;
        }

        let encoded = encode_identity_parts(encoding_version, kind, &descriptor);
        let id = derive_identity_id(&encoded);

        Ok(Self {
            encoding_version,
            kind,
            descriptor,
            id,
        })
    }

    pub const fn encoding_version(&self) -> u16 {
        self.encoding_version
    }

    pub const fn kind(&self) -> IdentityKind {
        self.kind
    }

    pub fn descriptor(&self) -> &[u8] {
        &self.descriptor
    }

    pub const fn id(&self) -> IdentityId {
        self.id
    }

    pub fn encode(&self) -> Vec<u8> {
        encode_identity_parts(self.encoding_version, self.kind, &self.descriptor)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        decode_identity(bytes, EnvironmentNamespaceMode::Production)
    }

    #[cfg(test)]
    fn new_environment_normative_fixture(
        definition_namespace: u8,
        digest_algorithm: u8,
        definition_digest: [u8; 32],
    ) -> Result<Self, RegistryError> {
        let descriptor = encode_environment_descriptor(
            definition_namespace,
            digest_algorithm,
            definition_digest,
        );

        Self::new_versioned(
            IDENTITY_ENCODING_VERSION_V2,
            IdentityKind::Environment,
            descriptor,
            EnvironmentNamespaceMode::NormativeTest,
        )
    }

    #[cfg(test)]
    fn decode_normative_fixture(bytes: &[u8]) -> Result<Self, RegistryError> {
        decode_identity(bytes, EnvironmentNamespaceMode::NormativeTest)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineageRecord {
    subject: IdentityId,
    parents: Vec<IdentityId>,
    root: LineageRoot,
}

impl LineageRecord {
    pub fn new(subject: IdentityId, parents: Vec<IdentityId>) -> Result<Self, RegistryError> {
        validate_lineage(subject, &parents)?;
        let encoded = encode_lineage_parts(subject, &parents);
        let root = derive_lineage_root(&encoded);

        Ok(Self {
            subject,
            parents,
            root,
        })
    }

    pub const fn subject(&self) -> IdentityId {
        self.subject
    }

    pub fn parents(&self) -> &[IdentityId] {
        &self.parents
    }

    pub const fn root(&self) -> LineageRoot {
        self.root
    }

    pub fn encode(&self) -> Vec<u8> {
        encode_lineage_parts(self.subject, &self.parents)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        if bytes.len() < LINEAGE_HEADER_LEN {
            return Err(RegistryError::TruncatedLineageEncoding);
        }

        let version = u16::from_be_bytes([bytes[0], bytes[1]]);
        if version != LINEAGE_ENCODING_VERSION {
            return Err(RegistryError::UnsupportedLineageVersion(version));
        }

        let mut subject_bytes = [0_u8; ID_LEN];
        subject_bytes.copy_from_slice(&bytes[2..2 + ID_LEN]);
        let subject = IdentityId::from_bytes(subject_bytes);

        let count_offset = 2 + ID_LEN;
        let parent_count =
            u16::from_be_bytes([bytes[count_offset], bytes[count_offset + 1]]) as usize;

        if parent_count > MAX_LINEAGE_PARENTS {
            return Err(RegistryError::TooManyLineageParents);
        }

        let parents_len = parent_count
            .checked_mul(ID_LEN)
            .ok_or(RegistryError::TooManyLineageParents)?;
        let expected_len = LINEAGE_HEADER_LEN
            .checked_add(parents_len)
            .ok_or(RegistryError::TooManyLineageParents)?;

        if bytes.len() < expected_len {
            return Err(RegistryError::TruncatedLineageEncoding);
        }

        if bytes.len() > expected_len {
            return Err(RegistryError::TrailingLineageBytes);
        }

        let mut parents = Vec::with_capacity(parent_count);
        let mut cursor = LINEAGE_HEADER_LEN;
        for _ in 0..parent_count {
            let mut parent_bytes = [0_u8; ID_LEN];
            parent_bytes.copy_from_slice(&bytes[cursor..cursor + ID_LEN]);
            parents.push(IdentityId::from_bytes(parent_bytes));
            cursor += ID_LEN;
        }

        Self::new(subject, parents)
    }
}

#[derive(Clone, Copy)]
enum EnvironmentNamespaceMode {
    Production,
    #[cfg(test)]
    NormativeTest,
}

fn decode_identity(
    bytes: &[u8],
    namespace_mode: EnvironmentNamespaceMode,
) -> Result<IdentityRecord, RegistryError> {
    if bytes.len() < IDENTITY_HEADER_LEN {
        return Err(RegistryError::TruncatedIdentityEncoding);
    }

    let version = u16::from_be_bytes([bytes[0], bytes[1]]);
    validate_identity_version(version)?;

    let kind = IdentityKind::from_u8(bytes[2])?;
    validate_identity_kind_for_version(version, kind)?;

    let descriptor_len = u32::from_be_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]) as usize;

    if descriptor_len > MAX_IDENTITY_DESCRIPTOR_LEN {
        return Err(RegistryError::IdentityDescriptorTooLarge);
    }

    let expected_len = IDENTITY_HEADER_LEN
        .checked_add(descriptor_len)
        .ok_or(RegistryError::IdentityDescriptorTooLarge)?;

    if bytes.len() < expected_len {
        return Err(RegistryError::TruncatedIdentityEncoding);
    }

    if bytes.len() > expected_len {
        return Err(RegistryError::TrailingIdentityBytes);
    }

    IdentityRecord::new_versioned(
        version,
        kind,
        bytes[IDENTITY_HEADER_LEN..expected_len].to_vec(),
        namespace_mode,
    )
}

fn validate_identity_version(version: u16) -> Result<(), RegistryError> {
    match version {
        IDENTITY_ENCODING_VERSION | IDENTITY_ENCODING_VERSION_V2 => Ok(()),
        _ => Err(RegistryError::UnsupportedIdentityVersion(version)),
    }
}

fn validate_identity_kind_for_version(
    version: u16,
    kind: IdentityKind,
) -> Result<(), RegistryError> {
    match version {
        IDENTITY_ENCODING_VERSION => {
            if kind == IdentityKind::Environment {
                return Err(RegistryError::IdentityKindNotPermittedForVersion {
                    version,
                    kind: kind.as_u8(),
                });
            }
            Ok(())
        }
        IDENTITY_ENCODING_VERSION_V2 => Ok(()),
        _ => Err(RegistryError::UnsupportedIdentityVersion(version)),
    }
}

fn validate_descriptor(descriptor: &[u8]) -> Result<(), RegistryError> {
    if descriptor.is_empty() {
        return Err(RegistryError::EmptyIdentityDescriptor);
    }

    if descriptor.len() > MAX_IDENTITY_DESCRIPTOR_LEN {
        return Err(RegistryError::IdentityDescriptorTooLarge);
    }

    Ok(())
}

fn validate_environment_descriptor(
    descriptor: &[u8],
    namespace_mode: EnvironmentNamespaceMode,
) -> Result<(), RegistryError> {
    if descriptor.len() != ENVIRONMENT_DESCRIPTOR_LEN {
        return Err(RegistryError::InvalidEnvironmentDescriptorLength(
            descriptor.len(),
        ));
    }

    let schema_version = descriptor[0];
    if schema_version != ENVIRONMENT_DESCRIPTOR_VERSION {
        return Err(RegistryError::UnsupportedEnvironmentSchema(schema_version));
    }

    let definition_namespace = descriptor[1];
    validate_environment_namespace(definition_namespace, namespace_mode)?;

    let digest_algorithm = descriptor[2];
    if !matches!(
        digest_algorithm,
        ENVIRONMENT_DIGEST_SHA256 | ENVIRONMENT_DIGEST_BLAKE3
    ) {
        return Err(RegistryError::UnsupportedEnvironmentDigestAlgorithm(
            digest_algorithm,
        ));
    }

    Ok(())
}

fn validate_environment_namespace(
    namespace: u8,
    namespace_mode: EnvironmentNamespaceMode,
) -> Result<(), RegistryError> {
    #[cfg(test)]
    if matches!(namespace_mode, EnvironmentNamespaceMode::NormativeTest)
        && namespace == ENVIRONMENT_TEST_NAMESPACE
    {
        return Ok(());
    }

    let _ = namespace_mode;

    Err(RegistryError::UnallocatedEnvironmentNamespace(namespace))
}

fn encode_environment_descriptor(
    definition_namespace: u8,
    digest_algorithm: u8,
    definition_digest: [u8; 32],
) -> Vec<u8> {
    let mut descriptor = Vec::with_capacity(ENVIRONMENT_DESCRIPTOR_LEN);
    descriptor.push(ENVIRONMENT_DESCRIPTOR_VERSION);
    descriptor.push(definition_namespace);
    descriptor.push(digest_algorithm);
    descriptor.extend_from_slice(&definition_digest);
    descriptor
}

fn validate_lineage(subject: IdentityId, parents: &[IdentityId]) -> Result<(), RegistryError> {
    if subject.is_zero() {
        return Err(RegistryError::ZeroIdentity);
    }

    if parents.len() > MAX_LINEAGE_PARENTS {
        return Err(RegistryError::TooManyLineageParents);
    }

    let mut seen = HashSet::with_capacity(parents.len());
    for parent in parents {
        if parent.is_zero() {
            return Err(RegistryError::ZeroIdentity);
        }

        if *parent == subject {
            return Err(RegistryError::SelfReferentialLineage);
        }

        if !seen.insert(*parent) {
            return Err(RegistryError::DuplicateLineageParent);
        }
    }

    Ok(())
}

fn encode_identity_parts(encoding_version: u16, kind: IdentityKind, descriptor: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(IDENTITY_HEADER_LEN + descriptor.len());
    encoded.extend_from_slice(&encoding_version.to_be_bytes());
    encoded.push(kind.as_u8());
    encoded.extend_from_slice(&(descriptor.len() as u32).to_be_bytes());
    encoded.extend_from_slice(descriptor);
    encoded
}

fn encode_lineage_parts(subject: IdentityId, parents: &[IdentityId]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(LINEAGE_HEADER_LEN + parents.len() * ID_LEN);
    encoded.extend_from_slice(&LINEAGE_ENCODING_VERSION.to_be_bytes());
    encoded.extend_from_slice(subject.as_bytes());
    encoded.extend_from_slice(&(parents.len() as u16).to_be_bytes());

    for parent in parents {
        encoded.extend_from_slice(parent.as_bytes());
    }

    encoded
}

fn derive_identity_id(encoded: &[u8]) -> IdentityId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(IDENTITY_DOMAIN_SEPARATOR);
    hasher.update(encoded);
    IdentityId::from_bytes(*hasher.finalize().as_bytes())
}

fn derive_lineage_root(encoded: &[u8]) -> LineageRoot {
    let mut hasher = blake3::Hasher::new();
    hasher.update(LINEAGE_DOMAIN_SEPARATOR);
    hasher.update(encoded);
    LineageRoot::from_bytes(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(kind: IdentityKind, descriptor: &[u8]) -> IdentityRecord {
        IdentityRecord::new(kind, descriptor.to_vec()).unwrap()
    }

    fn environment_fixture_bytes() -> Vec<u8> {
        IdentityRecord::new_environment_normative_fixture(
            ENVIRONMENT_TEST_NAMESPACE,
            ENVIRONMENT_DIGEST_BLAKE3,
            [0_u8; 32],
        )
        .unwrap()
        .encode()
    }

    #[test]
    fn identity_round_trip_is_canonical() {
        let original = identity(IdentityKind::Agent, b"agent:researcher:v1");
        let encoded = original.encode();
        let restored = IdentityRecord::decode(&encoded).unwrap();

        assert_eq!(restored, original);
        assert_eq!(restored.encode(), encoded);
    }

    #[test]
    fn identity_kind_changes_content_id() {
        let agent = identity(IdentityKind::Agent, b"same-descriptor");
        let evaluator = identity(IdentityKind::Evaluator, b"same-descriptor");

        assert_ne!(agent.id(), evaluator.id());
    }

    #[test]
    fn identity_descriptor_changes_content_id() {
        let first = identity(IdentityKind::Tool, b"tool:v1");
        let second = identity(IdentityKind::Tool, b"tool:v2");

        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn identity_fixed_vector_is_stable() {
        let record = identity(IdentityKind::ModelVersion, b"model:sovereign-small:1");

        assert_eq!(record.encoding_version(), IDENTITY_ENCODING_VERSION);
        assert_eq!(
            record.id().to_string(),
            "5c7db193dade08b572b255609c5f129f40e176acfc2741b4621c064813d0e111"
        );
    }

    #[test]
    fn existing_identity_constructor_remains_version_one() {
        for kind in [
            IdentityKind::Agent,
            IdentityKind::ModelVersion,
            IdentityKind::Tool,
            IdentityKind::Dataset,
            IdentityKind::Policy,
            IdentityKind::Evaluator,
            IdentityKind::Institution,
        ] {
            let record = IdentityRecord::new(kind, b"stable:v1".to_vec()).unwrap();
            assert_eq!(record.encoding_version(), IDENTITY_ENCODING_VERSION);
            assert_eq!(
                &record.encode()[..2],
                &IDENTITY_ENCODING_VERSION.to_be_bytes()
            );
        }
    }

    #[test]
    fn environment_cannot_use_version_one_constructor() {
        assert_eq!(
            IdentityRecord::new(
                IdentityKind::Environment,
                vec![1; ENVIRONMENT_DESCRIPTOR_LEN]
            )
            .unwrap_err(),
            RegistryError::IdentityKindNotPermittedForVersion {
                version: IDENTITY_ENCODING_VERSION,
                kind: IdentityKind::Environment.as_u8(),
            }
        );
    }

    #[test]
    fn production_environment_namespace_is_fail_closed_until_allocated() {
        assert_eq!(
            IdentityRecord::new_environment(
                ENVIRONMENT_TEST_NAMESPACE,
                ENVIRONMENT_DIGEST_BLAKE3,
                [0_u8; 32],
            )
            .unwrap_err(),
            RegistryError::UnallocatedEnvironmentNamespace(ENVIRONMENT_TEST_NAMESPACE)
        );
    }

    #[test]
    fn environment_fixed_vector_env_v1_001_is_stable() {
        let record = IdentityRecord::new_environment_normative_fixture(
            ENVIRONMENT_TEST_NAMESPACE,
            ENVIRONMENT_DIGEST_BLAKE3,
            [0_u8; 32],
        )
        .unwrap();

        let expected = vec![
            0x00, 0x02, 0x08, 0x00, 0x00, 0x00, 0x23, 0x01, 0xAA, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(expected.len(), 42);
        assert_eq!(record.encoding_version(), IDENTITY_ENCODING_VERSION_V2);
        assert_eq!(record.kind(), IdentityKind::Environment);
        assert_eq!(record.encode(), expected);
        assert_eq!(
            record.id().to_string(),
            "8824d2f6ca5015a4d311756938257a00af1b70fe4f639241d28db6e241eb3d67"
        );

        let decoded = IdentityRecord::decode_normative_fixture(&expected).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(decoded.encode(), expected);
    }

    #[test]
    fn production_decoder_rejects_normative_test_namespace() {
        assert_eq!(
            IdentityRecord::decode(&environment_fixture_bytes()).unwrap_err(),
            RegistryError::UnallocatedEnvironmentNamespace(ENVIRONMENT_TEST_NAMESPACE)
        );
    }

    #[test]
    fn version_two_existing_kind_round_trips_without_changing_default_constructor() {
        let mut encoded = identity(IdentityKind::Tool, b"tool:v2-envelope").encode();
        encoded[..2].copy_from_slice(&IDENTITY_ENCODING_VERSION_V2.to_be_bytes());

        let decoded = IdentityRecord::decode(&encoded).unwrap();

        assert_eq!(decoded.encoding_version(), IDENTITY_ENCODING_VERSION_V2);
        assert_eq!(decoded.kind(), IdentityKind::Tool);
        assert_eq!(decoded.encode(), encoded);

        let default =
            IdentityRecord::new(IdentityKind::Tool, b"tool:v2-envelope".to_vec()).unwrap();
        assert_eq!(default.encoding_version(), IDENTITY_ENCODING_VERSION);
        assert_ne!(default.id(), decoded.id());
    }

    #[test]
    fn empty_identity_descriptor_is_rejected() {
        assert_eq!(
            IdentityRecord::new(IdentityKind::Dataset, Vec::new()).unwrap_err(),
            RegistryError::EmptyIdentityDescriptor
        );
    }

    #[test]
    fn oversized_identity_descriptor_is_rejected() {
        assert_eq!(
            IdentityRecord::new(
                IdentityKind::Dataset,
                vec![0_u8; MAX_IDENTITY_DESCRIPTOR_LEN + 1],
            )
            .unwrap_err(),
            RegistryError::IdentityDescriptorTooLarge
        );
    }

    #[test]
    fn unknown_identity_kind_is_rejected() {
        let mut encoded = identity(IdentityKind::Policy, b"policy:v1").encode();
        encoded[2] = 0xff;

        assert_eq!(
            IdentityRecord::decode(&encoded).unwrap_err(),
            RegistryError::UnknownIdentityKind(0xff)
        );
    }

    #[test]
    fn unknown_identity_kind_is_rejected_under_version_two() {
        let mut encoded = identity(IdentityKind::Policy, b"policy:v1").encode();
        encoded[..2].copy_from_slice(&IDENTITY_ENCODING_VERSION_V2.to_be_bytes());
        encoded[2] = 0xff;

        assert_eq!(
            IdentityRecord::decode(&encoded).unwrap_err(),
            RegistryError::UnknownIdentityKind(0xff)
        );
    }

    #[test]
    fn unsupported_identity_version_is_rejected() {
        let mut encoded = identity(IdentityKind::Institution, b"institution:v1").encode();
        encoded[..2].copy_from_slice(&3_u16.to_be_bytes());

        assert_eq!(
            IdentityRecord::decode(&encoded).unwrap_err(),
            RegistryError::UnsupportedIdentityVersion(3)
        );
    }

    #[test]
    fn environment_kind_is_rejected_under_version_one() {
        let mut encoded = environment_fixture_bytes();
        encoded[..2].copy_from_slice(&IDENTITY_ENCODING_VERSION.to_be_bytes());

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::IdentityKindNotPermittedForVersion {
                version: IDENTITY_ENCODING_VERSION,
                kind: IdentityKind::Environment.as_u8(),
            }
        );
    }

    #[test]
    fn unsupported_environment_schema_is_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded[IDENTITY_HEADER_LEN] = 0x02;

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::UnsupportedEnvironmentSchema(0x02)
        );
    }

    #[test]
    fn unallocated_environment_namespace_is_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded[IDENTITY_HEADER_LEN + 1] = 0x01;

        assert_eq!(
            IdentityRecord::decode(&encoded).unwrap_err(),
            RegistryError::UnallocatedEnvironmentNamespace(0x01)
        );
    }

    #[test]
    fn unsupported_environment_digest_algorithm_is_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded[IDENTITY_HEADER_LEN + 2] = 0x03;

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::UnsupportedEnvironmentDigestAlgorithm(0x03)
        );
    }

    #[test]
    fn environment_descriptor_shorter_than_35_bytes_is_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded[3..7].copy_from_slice(&34_u32.to_be_bytes());
        encoded.pop();

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::InvalidEnvironmentDescriptorLength(34)
        );
    }

    #[test]
    fn environment_descriptor_longer_than_35_bytes_is_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded[3..7].copy_from_slice(&36_u32.to_be_bytes());
        encoded.push(0);

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::InvalidEnvironmentDescriptorLength(36)
        );
    }

    #[test]
    fn truncated_environment_descriptor_is_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded.pop();

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::TruncatedIdentityEncoding
        );
    }

    #[test]
    fn trailing_environment_bytes_are_rejected() {
        let mut encoded = environment_fixture_bytes();
        encoded.push(0);

        assert_eq!(
            IdentityRecord::decode_normative_fixture(&encoded).unwrap_err(),
            RegistryError::TrailingIdentityBytes
        );
    }

    #[test]
    fn truncated_identity_is_rejected() {
        let mut encoded = identity(IdentityKind::Agent, b"agent:v1").encode();
        encoded.pop();

        assert_eq!(
            IdentityRecord::decode(&encoded).unwrap_err(),
            RegistryError::TruncatedIdentityEncoding
        );
    }

    #[test]
    fn trailing_identity_bytes_are_rejected() {
        let mut encoded = identity(IdentityKind::Agent, b"agent:v1").encode();
        encoded.push(0);

        assert_eq!(
            IdentityRecord::decode(&encoded).unwrap_err(),
            RegistryError::TrailingIdentityBytes
        );
    }

    #[test]
    fn lineage_round_trip_preserves_parent_order() {
        let subject = identity(IdentityKind::ModelVersion, b"model:v3").id();
        let first_parent = identity(IdentityKind::ModelVersion, b"model:v1").id();
        let second_parent = identity(IdentityKind::Dataset, b"training-set:v2").id();
        let original = LineageRecord::new(subject, vec![first_parent, second_parent]).unwrap();
        let encoded = original.encode();
        let restored = LineageRecord::decode(&encoded).unwrap();

        assert_eq!(restored, original);
        assert_eq!(restored.parents(), &[first_parent, second_parent]);
        assert_eq!(restored.encode(), encoded);
    }

    #[test]
    fn lineage_parent_order_changes_root() {
        let subject = identity(IdentityKind::ModelVersion, b"model:v3").id();
        let first_parent = identity(IdentityKind::ModelVersion, b"model:v1").id();
        let second_parent = identity(IdentityKind::Dataset, b"training-set:v2").id();

        let first = LineageRecord::new(subject, vec![first_parent, second_parent]).unwrap();
        let second = LineageRecord::new(subject, vec![second_parent, first_parent]).unwrap();

        assert_ne!(first.root(), second.root());
    }

    #[test]
    fn lineage_fixed_vector_is_stable() {
        let subject = identity(IdentityKind::ModelVersion, b"model:v2").id();
        let parent = identity(IdentityKind::ModelVersion, b"model:v1").id();
        let lineage = LineageRecord::new(subject, vec![parent]).unwrap();

        assert_eq!(
            lineage.root().to_string(),
            "a83f042f90d9d74fe8dcf41f963a7c0abaf39d30e28300fcf65e4d9273102eee"
        );
    }

    #[test]
    fn zero_subject_is_rejected() {
        assert_eq!(
            LineageRecord::new(IdentityId::from_bytes([0; ID_LEN]), Vec::new()).unwrap_err(),
            RegistryError::ZeroIdentity
        );
    }

    #[test]
    fn zero_parent_is_rejected() {
        let subject = identity(IdentityKind::Agent, b"agent:v1").id();

        assert_eq!(
            LineageRecord::new(subject, vec![IdentityId::from_bytes([0; ID_LEN])]).unwrap_err(),
            RegistryError::ZeroIdentity
        );
    }

    #[test]
    fn duplicate_lineage_parent_is_rejected() {
        let subject = identity(IdentityKind::ModelVersion, b"model:v2").id();
        let parent = identity(IdentityKind::ModelVersion, b"model:v1").id();

        assert_eq!(
            LineageRecord::new(subject, vec![parent, parent]).unwrap_err(),
            RegistryError::DuplicateLineageParent
        );
    }

    #[test]
    fn self_referential_lineage_is_rejected() {
        let subject = identity(IdentityKind::Agent, b"agent:v1").id();

        assert_eq!(
            LineageRecord::new(subject, vec![subject]).unwrap_err(),
            RegistryError::SelfReferentialLineage
        );
    }

    #[test]
    fn too_many_lineage_parents_are_rejected() {
        let subject = identity(IdentityKind::ModelVersion, b"model:v2").id();
        let parents = (1..=MAX_LINEAGE_PARENTS + 1)
            .map(|value| IdentityId::from_bytes([value as u8; ID_LEN]))
            .collect();

        assert_eq!(
            LineageRecord::new(subject, parents).unwrap_err(),
            RegistryError::TooManyLineageParents
        );
    }

    #[test]
    fn unsupported_lineage_version_is_rejected() {
        let subject = identity(IdentityKind::Agent, b"agent:v1").id();
        let mut encoded = LineageRecord::new(subject, Vec::new()).unwrap().encode();
        encoded[..2].copy_from_slice(&2_u16.to_be_bytes());

        assert_eq!(
            LineageRecord::decode(&encoded).unwrap_err(),
            RegistryError::UnsupportedLineageVersion(2)
        );
    }

    #[test]
    fn truncated_lineage_is_rejected() {
        let subject = identity(IdentityKind::Agent, b"agent:v1").id();
        let parent = identity(IdentityKind::ModelVersion, b"model:v1").id();
        let mut encoded = LineageRecord::new(subject, vec![parent]).unwrap().encode();
        encoded.pop();

        assert_eq!(
            LineageRecord::decode(&encoded).unwrap_err(),
            RegistryError::TruncatedLineageEncoding
        );
    }

    #[test]
    fn trailing_lineage_bytes_are_rejected() {
        let subject = identity(IdentityKind::Agent, b"agent:v1").id();
        let mut encoded = LineageRecord::new(subject, Vec::new()).unwrap().encode();
        encoded.push(0);

        assert_eq!(
            LineageRecord::decode(&encoded).unwrap_err(),
            RegistryError::TrailingLineageBytes
        );
    }
}
