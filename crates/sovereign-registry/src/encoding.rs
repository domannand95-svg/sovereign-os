use crate::{Caid, IdentityId, ObjectClass, RegistryEdge, RegistryError, RelationType};

pub const REGISTRY_NODE_MAGIC_V2: u8 = 0xFE;
pub const REGISTRY_EDGE_MAGIC_V2: u8 = 0xFD;
pub const REGISTRY_ENCODING_VERSION_V2: u16 = 0x0002;

const REGISTRY_NODE_DOMAIN_V2: &[u8] = b"SOVEREIGN_REGISTRY_NODE_V2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryGenesisPayloadV1 {
    environment_id: Vec<u8>,
    root_policy_caid: Option<Caid>,
}

impl RegistryGenesisPayloadV1 {
    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        if bytes.len() < 5 {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let environment_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        let environment_start = 2_usize;
        let environment_end = environment_start
            .checked_add(environment_len)
            .ok_or(RegistryError::MalformedGenesisPayload)?;

        let fixed_end = environment_end
            .checked_add(3)
            .ok_or(RegistryError::MalformedGenesisPayload)?;

        if bytes.len() < fixed_end {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let environment_id = &bytes[environment_start..environment_end];

        if std::str::from_utf8(environment_id).is_err() {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let protocol_version =
            u16::from_be_bytes([bytes[environment_end], bytes[environment_end + 1]]);

        if protocol_version != REGISTRY_ENCODING_VERSION_V2 {
            return Err(RegistryError::MalformedGenesisPayload);
        }

        let root_policy_present = bytes[environment_end + 2];

        let root_policy_caid = match root_policy_present {
            0x00 => {
                if bytes.len() != fixed_end {
                    return Err(RegistryError::MalformedGenesisPayload);
                }

                None
            }
            0x01 => {
                let expected_end = fixed_end
                    .checked_add(32)
                    .ok_or(RegistryError::MalformedGenesisPayload)?;

                if bytes.len() != expected_end {
                    return Err(RegistryError::MalformedGenesisPayload);
                }

                let mut caid = [0_u8; 32];
                caid.copy_from_slice(&bytes[fixed_end..expected_end]);
                Some(Caid(caid))
            }
            _ => return Err(RegistryError::MalformedGenesisPayload),
        };

        Ok(Self {
            environment_id: environment_id.to_vec(),
            root_policy_caid,
        })
    }

    pub fn environment_id(&self) -> &[u8] {
        &self.environment_id
    }

    pub const fn protocol_version(&self) -> u16 {
        REGISTRY_ENCODING_VERSION_V2
    }

    pub const fn root_policy_caid(&self) -> Option<Caid> {
        self.root_policy_caid
    }

    pub fn encode(&self) -> Vec<u8> {
        let environment_len = u16::try_from(self.environment_id.len())
            .expect("decoded Genesis environment length must remain representable as u16");

        let root_policy_len = if self.root_policy_caid.is_some() {
            32
        } else {
            0
        };
        let mut buffer = Vec::with_capacity(5 + self.environment_id.len() + root_policy_len);

        buffer.extend_from_slice(&environment_len.to_be_bytes());
        buffer.extend_from_slice(&self.environment_id);
        buffer.extend_from_slice(&REGISTRY_ENCODING_VERSION_V2.to_be_bytes());

        match self.root_policy_caid {
            Some(caid) => {
                buffer.push(0x01);
                buffer.extend_from_slice(&caid.0);
            }
            None => buffer.push(0x00),
        }

        buffer
    }
}

pub const CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1: u16 = 0x0001;
pub const CAPABILITY_PAYLOAD_MIN_LEN_V1: usize = 111;
pub const CAPABILITY_PAYLOAD_MAX_LEN_V1: usize = 538;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum OperationCodeV1 {
    Read = 0x0001,
    Create = 0x0002,
    Execute = 0x0003,
}

impl OperationCodeV1 {
    fn from_u16(value: u16) -> Result<Self, RegistryError> {
        match value {
            0x0001 => Ok(Self::Read),
            0x0002 => Ok(Self::Create),
            0x0003 => Ok(Self::Execute),
            _ => Err(RegistryError::MalformedCapabilityPayload),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetScopeV1 {
    ExactObject(Caid),
    NamedScope(String),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NetworkScopeV1 {
    GovernedService(Caid),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FilesystemReadScopeV1 {
    ExactObject(Caid),
    GovernedNamespace(Caid),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FilesystemWriteScopeV1 {
    GovernedNamespace(Caid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConstraintsV1 {
    network: Option<NetworkScopeV1>,
    filesystem_read: Option<FilesystemReadScopeV1>,
    filesystem_write: Option<FilesystemWriteScopeV1>,
}

impl ResourceConstraintsV1 {
    pub fn network(&self) -> Option<&NetworkScopeV1> {
        self.network.as_ref()
    }

    pub fn filesystem_read(&self) -> Option<&FilesystemReadScopeV1> {
        self.filesystem_read.as_ref()
    }

    pub fn filesystem_write(&self) -> Option<&FilesystemWriteScopeV1> {
        self.filesystem_write.as_ref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ExecutionBudgetV1 {
    wall_time_ms: u64,
    memory_bytes: u64,
    network_egress_bytes: u64,
    filesystem_write_bytes: u64,
}

impl ExecutionBudgetV1 {
    pub const fn wall_time_ms(&self) -> u64 {
        self.wall_time_ms
    }

    pub const fn memory_bytes(&self) -> u64 {
        self.memory_bytes
    }

    pub const fn network_egress_bytes(&self) -> u64 {
        self.network_egress_bytes
    }

    pub const fn filesystem_write_bytes(&self) -> u64 {
        self.filesystem_write_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPayloadV1 {
    issuer_identity: IdentityId,
    subject_identity: IdentityId,
    operation: OperationCodeV1,
    target_scope: TargetScopeV1,
    authorized_executable: Option<Caid>,
    resource_constraints: ResourceConstraintsV1,
    execution_budget: Option<ExecutionBudgetV1>,
    expiry: Option<u64>,
    governing_policy: Caid,
}

impl CapabilityPayloadV1 {
    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        if !(CAPABILITY_PAYLOAD_MIN_LEN_V1..=CAPABILITY_PAYLOAD_MAX_LEN_V1).contains(&bytes.len()) {
            return Err(RegistryError::MalformedCapabilityPayload);
        }

        let mut cursor = CapabilityCursor::new(bytes);

        let schema_version = cursor.take_u16()?;
        if schema_version != CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1 {
            return Err(RegistryError::MalformedCapabilityPayload);
        }

        let issuer_identity = IdentityId::from_bytes(cursor.take_array::<32>()?);
        let subject_identity = IdentityId::from_bytes(cursor.take_array::<32>()?);
        let operation = OperationCodeV1::from_u16(cursor.take_u16()?)?;
        let target_scope = decode_target_scope(&mut cursor)?;
        let authorized_executable = decode_optional_caid(&mut cursor)?;
        let resource_constraints = decode_resource_constraints(&mut cursor)?;
        let execution_budget = decode_optional_execution_budget(&mut cursor)?;
        let expiry = decode_optional_expiry(&mut cursor)?;
        let governing_policy = Caid(cursor.take_array::<32>()?);

        if !cursor.is_eof() {
            return Err(RegistryError::MalformedCapabilityPayload);
        }

        Ok(Self {
            issuer_identity,
            subject_identity,
            operation,
            target_scope,
            authorized_executable,
            resource_constraints,
            execution_budget,
            expiry,
            governing_policy,
        })
    }

    pub const fn schema_version(&self) -> u16 {
        CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1
    }

    pub const fn issuer_identity(&self) -> IdentityId {
        self.issuer_identity
    }

    pub const fn subject_identity(&self) -> IdentityId {
        self.subject_identity
    }

    pub const fn operation(&self) -> OperationCodeV1 {
        self.operation
    }

    pub const fn target_scope(&self) -> &TargetScopeV1 {
        &self.target_scope
    }

    pub const fn authorized_executable(&self) -> Option<Caid> {
        self.authorized_executable
    }

    pub const fn resource_constraints(&self) -> &ResourceConstraintsV1 {
        &self.resource_constraints
    }

    pub const fn execution_budget(&self) -> Option<ExecutionBudgetV1> {
        self.execution_budget
    }

    pub const fn expiry(&self) -> Option<u64> {
        self.expiry
    }

    pub const fn governing_policy(&self) -> Caid {
        self.governing_policy
    }
}

struct CapabilityCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> CapabilityCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], RegistryError> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(RegistryError::MalformedCapabilityPayload)?;

        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or(RegistryError::MalformedCapabilityPayload)?;

        let mut result = [0_u8; N];
        result.copy_from_slice(slice);
        self.position = end;

        Ok(result)
    }

    fn take_u8(&mut self) -> Result<u8, RegistryError> {
        Ok(self.take_array::<1>()?[0])
    }

    fn take_u16(&mut self) -> Result<u16, RegistryError> {
        Ok(u16::from_be_bytes(self.take_array::<2>()?))
    }

    fn take_u64(&mut self) -> Result<u64, RegistryError> {
        Ok(u64::from_be_bytes(self.take_array::<8>()?))
    }

    const fn is_eof(&self) -> bool {
        self.position == self.bytes.len()
    }
}

fn decode_target_scope(cursor: &mut CapabilityCursor<'_>) -> Result<TargetScopeV1, RegistryError> {
    match cursor.take_u8()? {
        0x01 => Ok(TargetScopeV1::ExactObject(Caid(cursor.take_array::<32>()?))),
        0x02 => {
            let length = usize::from(cursor.take_u16()?);

            if !(1..=256).contains(&length) {
                return Err(RegistryError::MalformedCapabilityPayload);
            }

            let end = cursor
                .position
                .checked_add(length)
                .ok_or(RegistryError::MalformedCapabilityPayload)?;

            let scope_bytes = cursor
                .bytes
                .get(cursor.position..end)
                .ok_or(RegistryError::MalformedCapabilityPayload)?;

            let scope = std::str::from_utf8(scope_bytes)
                .map_err(|_| RegistryError::MalformedCapabilityPayload)?
                .to_owned();

            cursor.position = end;

            Ok(TargetScopeV1::NamedScope(scope))
        }
        _ => Err(RegistryError::MalformedCapabilityPayload),
    }
}

fn decode_optional_caid(cursor: &mut CapabilityCursor<'_>) -> Result<Option<Caid>, RegistryError> {
    match cursor.take_u8()? {
        0x00 => Ok(None),
        0x01 => Ok(Some(Caid(cursor.take_array::<32>()?))),
        _ => Err(RegistryError::MalformedCapabilityPayload),
    }
}

fn decode_resource_constraints(
    cursor: &mut CapabilityCursor<'_>,
) -> Result<ResourceConstraintsV1, RegistryError> {
    if cursor.take_u8()? != 0x01 {
        return Err(RegistryError::MalformedCapabilityPayload);
    }

    let network = match cursor.take_u8()? {
        0x00 => None,
        0x01 => {
            if cursor.take_u8()? != 0x01 {
                return Err(RegistryError::MalformedCapabilityPayload);
            }

            Some(NetworkScopeV1::GovernedService(Caid(
                cursor.take_array::<32>()?,
            )))
        }
        _ => return Err(RegistryError::MalformedCapabilityPayload),
    };

    let filesystem_read = match cursor.take_u8()? {
        0x00 => None,
        0x01 => {
            let scope = match cursor.take_u8()? {
                0x01 => FilesystemReadScopeV1::ExactObject(Caid(cursor.take_array::<32>()?)),
                0x02 => FilesystemReadScopeV1::GovernedNamespace(Caid(cursor.take_array::<32>()?)),
                _ => return Err(RegistryError::MalformedCapabilityPayload),
            };

            Some(scope)
        }
        _ => return Err(RegistryError::MalformedCapabilityPayload),
    };

    let filesystem_write = match cursor.take_u8()? {
        0x00 => None,
        0x01 => {
            if cursor.take_u8()? != 0x01 {
                return Err(RegistryError::MalformedCapabilityPayload);
            }

            Some(FilesystemWriteScopeV1::GovernedNamespace(Caid(
                cursor.take_array::<32>()?,
            )))
        }
        _ => return Err(RegistryError::MalformedCapabilityPayload),
    };

    Ok(ResourceConstraintsV1 {
        network,
        filesystem_read,
        filesystem_write,
    })
}

fn decode_optional_execution_budget(
    cursor: &mut CapabilityCursor<'_>,
) -> Result<Option<ExecutionBudgetV1>, RegistryError> {
    match cursor.take_u8()? {
        0x00 => Ok(None),
        0x01 => {
            if cursor.take_u8()? != 0x01 {
                return Err(RegistryError::MalformedCapabilityPayload);
            }

            let budget = ExecutionBudgetV1 {
                wall_time_ms: cursor.take_u64()?,
                memory_bytes: cursor.take_u64()?,
                network_egress_bytes: cursor.take_u64()?,
                filesystem_write_bytes: cursor.take_u64()?,
            };

            Ok(Some(budget))
        }
        _ => Err(RegistryError::MalformedCapabilityPayload),
    }
}

fn decode_optional_expiry(cursor: &mut CapabilityCursor<'_>) -> Result<Option<u64>, RegistryError> {
    match cursor.take_u8()? {
        0x00 => Ok(None),
        0x01 => Ok(Some(cursor.take_u64()?)),
        _ => Err(RegistryError::MalformedCapabilityPayload),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedRegistryNode {
    class: ObjectClass,
    parents: Vec<Caid>,
    payload: Vec<u8>,
    caid: Caid,
}

impl VersionedRegistryNode {
    pub fn new(
        class: ObjectClass,
        parents: Vec<Caid>,
        payload: Vec<u8>,
    ) -> Result<Self, RegistryError> {
        if payload.is_empty() {
            return Err(RegistryError::SchemaViolation);
        }

        let encoded = encode_node_parts(class, &parents, &payload)?;
        let caid = derive_v2_node_caid(&encoded);

        Ok(Self {
            class,
            parents,
            payload,
            caid,
        })
    }

    pub const fn class(&self) -> ObjectClass {
        self.class
    }

    pub fn parents(&self) -> &[Caid] {
        &self.parents
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub const fn caid(&self) -> Caid {
        self.caid
    }

    pub fn encode(&self) -> Vec<u8> {
        encode_node_parts(self.class, &self.parents, &self.payload)
            .expect("validated VersionedRegistryNode must remain encodable")
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, RegistryError> {
        if bytes.len() < 12 {
            return Err(RegistryError::SchemaViolation);
        }

        if bytes[0] != REGISTRY_NODE_MAGIC_V2 {
            return Err(RegistryError::SchemaViolation);
        }

        let version = u16::from_be_bytes([bytes[1], bytes[2]]);
        if version != REGISTRY_ENCODING_VERSION_V2 {
            return Err(RegistryError::SchemaViolation);
        }

        let class = ObjectClass::from_u8(bytes[3]).ok_or(RegistryError::SchemaViolation)?;

        let parent_count = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;

        let parents_len = parent_count
            .checked_mul(32)
            .ok_or(RegistryError::SchemaViolation)?;

        let payload_len_offset = 8_usize
            .checked_add(parents_len)
            .ok_or(RegistryError::SchemaViolation)?;

        let minimum_len = payload_len_offset
            .checked_add(4)
            .ok_or(RegistryError::SchemaViolation)?;

        if bytes.len() < minimum_len {
            return Err(RegistryError::SchemaViolation);
        }

        let mut parents = Vec::with_capacity(parent_count);
        let mut cursor: usize = 8;

        for _ in 0..parent_count {
            let end = cursor
                .checked_add(32)
                .ok_or(RegistryError::SchemaViolation)?;

            let mut caid = [0_u8; 32];
            caid.copy_from_slice(&bytes[cursor..end]);
            parents.push(Caid(caid));
            cursor = end;
        }

        let payload_len = u32::from_be_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        let expected_len = cursor
            .checked_add(payload_len)
            .ok_or(RegistryError::SchemaViolation)?;

        if bytes.len() != expected_len || payload_len == 0 {
            return Err(RegistryError::SchemaViolation);
        }

        Self::new(class, parents, bytes[cursor..expected_len].to_vec())
    }
}

pub fn serialize_edge_v2(edge: &RegistryEdge) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(68);

    buffer.push(REGISTRY_EDGE_MAGIC_V2);
    buffer.extend_from_slice(&REGISTRY_ENCODING_VERSION_V2.to_be_bytes());
    buffer.push(edge.relation().as_u8());
    buffer.extend_from_slice(&edge.parent().0);
    buffer.extend_from_slice(&edge.child().0);

    buffer
}

pub fn deserialize_edge_v2(bytes: &[u8]) -> Result<RegistryEdge, RegistryError> {
    if bytes.len() != 68 {
        return Err(RegistryError::SchemaViolation);
    }

    if bytes[0] != REGISTRY_EDGE_MAGIC_V2 {
        return Err(RegistryError::SchemaViolation);
    }

    let version = u16::from_be_bytes([bytes[1], bytes[2]]);
    if version != REGISTRY_ENCODING_VERSION_V2 {
        return Err(RegistryError::SchemaViolation);
    }

    let relation = RelationType::from_u8(bytes[3]).ok_or(RegistryError::SchemaViolation)?;

    let mut parent = [0_u8; 32];
    parent.copy_from_slice(&bytes[4..36]);

    let mut child = [0_u8; 32];
    child.copy_from_slice(&bytes[36..68]);

    Ok(RegistryEdge::new(Caid(parent), Caid(child), relation))
}

fn encode_node_parts(
    class: ObjectClass,
    parents: &[Caid],
    payload: &[u8],
) -> Result<Vec<u8>, RegistryError> {
    if payload.is_empty() {
        return Err(RegistryError::SchemaViolation);
    }

    let parent_count = u32::try_from(parents.len()).map_err(|_| RegistryError::SchemaViolation)?;
    let payload_len = u32::try_from(payload.len()).map_err(|_| RegistryError::SchemaViolation)?;

    let parents_bytes = parents
        .len()
        .checked_mul(32)
        .ok_or(RegistryError::SchemaViolation)?;

    let capacity = 12_usize
        .checked_add(parents_bytes)
        .and_then(|value| value.checked_add(payload.len()))
        .ok_or(RegistryError::SchemaViolation)?;

    let mut buffer = Vec::with_capacity(capacity);

    buffer.push(REGISTRY_NODE_MAGIC_V2);
    buffer.extend_from_slice(&REGISTRY_ENCODING_VERSION_V2.to_be_bytes());
    buffer.push(class.as_u8());
    buffer.extend_from_slice(&parent_count.to_be_bytes());

    for parent in parents {
        buffer.extend_from_slice(&parent.0);
    }

    buffer.extend_from_slice(&payload_len.to_be_bytes());
    buffer.extend_from_slice(payload);

    Ok(buffer)
}

fn derive_v2_node_caid(encoded: &[u8]) -> Caid {
    let mut hasher = blake3::Hasher::new();
    hasher.update(REGISTRY_NODE_DOMAIN_V2);
    hasher.update(encoded);
    Caid(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_capability_payload() -> Vec<u8> {
        let mut payload = Vec::with_capacity(CAPABILITY_PAYLOAD_MIN_LEN_V1);

        payload.extend_from_slice(&CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1.to_be_bytes());
        payload.extend_from_slice(&[0x11; 32]);
        payload.extend_from_slice(&[0x22; 32]);
        payload.extend_from_slice(&(OperationCodeV1::Read as u16).to_be_bytes());

        payload.push(0x02);
        payload.extend_from_slice(&1_u16.to_be_bytes());
        payload.push(b'x');

        payload.push(0x00);

        payload.push(0x01);
        payload.push(0x00);
        payload.push(0x00);
        payload.push(0x00);

        payload.push(0x00);
        payload.push(0x00);

        payload.extend_from_slice(&[0x33; 32]);

        assert_eq!(payload.len(), CAPABILITY_PAYLOAD_MIN_LEN_V1);
        payload
    }

    #[test]
    fn capability_payload_v1_decodes_canonical_minimum_payload() {
        let payload = minimal_capability_payload();
        let decoded = CapabilityPayloadV1::decode(&payload).unwrap();

        assert_eq!(
            decoded.schema_version(),
            CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1
        );
        assert_eq!(
            decoded.issuer_identity(),
            IdentityId::from_bytes([0x11; 32])
        );
        assert_eq!(
            decoded.subject_identity(),
            IdentityId::from_bytes([0x22; 32])
        );
        assert_eq!(decoded.operation(), OperationCodeV1::Read);
        assert_eq!(
            decoded.target_scope(),
            &TargetScopeV1::NamedScope("x".to_owned())
        );
        assert_eq!(decoded.authorized_executable(), None);
        assert!(decoded.resource_constraints().network().is_none());
        assert!(decoded.resource_constraints().filesystem_read().is_none());
        assert!(decoded.resource_constraints().filesystem_write().is_none());
        assert_eq!(decoded.execution_budget(), None);
        assert_eq!(decoded.expiry(), None);
        assert_eq!(decoded.governing_policy(), Caid([0x33; 32]));
    }

    #[test]
    fn capability_payload_v1_rejects_preflight_length_violations() {
        assert_eq!(
            CapabilityPayloadV1::decode(&[0_u8; CAPABILITY_PAYLOAD_MIN_LEN_V1 - 1]),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&vec![0_u8; CAPABILITY_PAYLOAD_MAX_LEN_V1 + 1]),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_rejects_unknown_schema_and_operation() {
        let mut bad_schema = minimal_capability_payload();
        bad_schema[0..2].copy_from_slice(&0x0002_u16.to_be_bytes());

        assert_eq!(
            CapabilityPayloadV1::decode(&bad_schema),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut bad_operation = minimal_capability_payload();
        bad_operation[66..68].copy_from_slice(&0x0000_u16.to_be_bytes());

        assert_eq!(
            CapabilityPayloadV1::decode(&bad_operation),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_rejects_invalid_target_and_presence_marker() {
        let mut bad_target = minimal_capability_payload();
        bad_target[68] = 0x03;

        assert_eq!(
            CapabilityPayloadV1::decode(&bad_target),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut bad_marker = minimal_capability_payload();
        bad_marker[72] = 0x02;

        assert_eq!(
            CapabilityPayloadV1::decode(&bad_marker),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_rejects_trailing_bytes() {
        let mut payload = minimal_capability_payload();
        payload.push(0x00);

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    fn capability_payload_with_components(
        target_scope: &[u8],
        authorized_executable: &[u8],
        resource_constraints: &[u8],
        execution_budget: &[u8],
        expiry: &[u8],
    ) -> Vec<u8> {
        let mut payload = Vec::new();

        payload.extend_from_slice(&CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1.to_be_bytes());
        payload.extend_from_slice(&[0x11; 32]);
        payload.extend_from_slice(&[0x22; 32]);
        payload.extend_from_slice(&(OperationCodeV1::Read as u16).to_be_bytes());

        payload.extend_from_slice(target_scope);
        payload.extend_from_slice(authorized_executable);
        payload.extend_from_slice(resource_constraints);
        payload.extend_from_slice(execution_budget);
        payload.extend_from_slice(expiry);
        payload.extend_from_slice(&[0x33; 32]);

        payload
    }

    #[test]
    fn capability_payload_v1_rejects_invalid_named_scope_encodings() {
        let minimal_resources = [0x01, 0x00, 0x00, 0x00];
        let absent = [0x00];

        let invalid_utf8_target = [0x02, 0x00, 0x01, 0xFF];
        let invalid_utf8 = capability_payload_with_components(
            &invalid_utf8_target,
            &absent,
            &minimal_resources,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&invalid_utf8),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let zero_length_target = [0x02, 0x00, 0x00];

        let mut present_executable = vec![0x01];
        present_executable.extend_from_slice(&[0x44; 32]);

        let zero_length = capability_payload_with_components(
            &zero_length_target,
            &present_executable,
            &minimal_resources,
            &absent,
            &absent,
        );

        assert!(zero_length.len() >= CAPABILITY_PAYLOAD_MIN_LEN_V1);

        assert_eq!(
            CapabilityPayloadV1::decode(&zero_length),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut oversized_target = vec![0x02];
        oversized_target.extend_from_slice(&257_u16.to_be_bytes());
        oversized_target.extend_from_slice(&vec![b'x'; 257]);

        let oversized = capability_payload_with_components(
            &oversized_target,
            &absent,
            &minimal_resources,
            &absent,
            &absent,
        );

        assert!(oversized.len() <= CAPABILITY_PAYLOAD_MAX_LEN_V1);

        assert_eq!(
            CapabilityPayloadV1::decode(&oversized),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_rejects_invalid_resource_constraint_encodings() {
        let target = [0x02, 0x00, 0x01, b'x'];
        let absent = [0x00];

        let invalid_version = [0x02, 0x00, 0x00, 0x00];
        let payload = capability_payload_with_components(
            &target,
            &absent,
            &invalid_version,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let invalid_network_mode = [0x01, 0x02, 0x00, 0x00];
        let payload = capability_payload_with_components(
            &target,
            &absent,
            &invalid_network_mode,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut invalid_network_scope = vec![0x01, 0x01, 0x02];
        invalid_network_scope.extend_from_slice(&[0x44; 32]);
        invalid_network_scope.extend_from_slice(&[0x00, 0x00]);

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &invalid_network_scope,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut invalid_read_scope = vec![0x01, 0x00, 0x01, 0x03];
        invalid_read_scope.extend_from_slice(&[0x55; 32]);
        invalid_read_scope.push(0x00);

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &invalid_read_scope,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut invalid_write_scope = vec![0x01, 0x00, 0x00, 0x01, 0x02];
        invalid_write_scope.extend_from_slice(&[0x66; 32]);

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &invalid_write_scope,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_rejects_invalid_budget_and_expiry_encodings() {
        let target = [0x02, 0x00, 0x01, b'x'];
        let absent = [0x00];
        let resources = [0x01, 0x00, 0x00, 0x00];

        let invalid_budget_marker = [0x02];

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &resources,
            &invalid_budget_marker,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut invalid_budget_version = vec![0x01, 0x02];
        invalid_budget_version.extend_from_slice(&[0x00; 32]);

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &resources,
            &invalid_budget_version,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let invalid_expiry_marker = [0x02];

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &resources,
            &absent,
            &invalid_expiry_marker,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_rejects_nested_truncation() {
        let target = [0x02, 0x00, 0x01, b'x'];
        let resources = [0x01, 0x00, 0x00, 0x00];
        let absent = [0x00];

        let mut truncated_executable = vec![0x01];
        truncated_executable.extend_from_slice(&[0x44; 31]);

        let payload = capability_payload_with_components(
            &target,
            &truncated_executable,
            &resources,
            &absent,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut truncated_budget = vec![0x01, 0x01];
        truncated_budget.extend_from_slice(&[0x00; 31]);

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &resources,
            &truncated_budget,
            &absent,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut truncated_expiry = vec![0x01];
        truncated_expiry.extend_from_slice(&[0x00; 7]);

        let payload = capability_payload_with_components(
            &target,
            &absent,
            &resources,
            &absent,
            &truncated_expiry,
        );

        assert_eq!(
            CapabilityPayloadV1::decode(&payload),
            Err(RegistryError::MalformedCapabilityPayload)
        );

        let mut maximal = Vec::new();

        maximal.extend_from_slice(&CAPABILITY_PAYLOAD_SCHEMA_VERSION_V1.to_be_bytes());
        maximal.extend_from_slice(&[0x11; 32]);
        maximal.extend_from_slice(&[0x22; 32]);
        maximal.extend_from_slice(&(OperationCodeV1::Read as u16).to_be_bytes());

        maximal.push(0x02);
        maximal.extend_from_slice(&256_u16.to_be_bytes());
        maximal.extend_from_slice(&[b'n'; 256]);

        maximal.push(0x01);
        maximal.extend_from_slice(&[0x44; 32]);

        maximal.push(0x01);

        maximal.push(0x01);
        maximal.push(0x01);
        maximal.extend_from_slice(&[0x55; 32]);

        maximal.push(0x01);
        maximal.push(0x02);
        maximal.extend_from_slice(&[0x66; 32]);

        maximal.push(0x01);
        maximal.push(0x01);
        maximal.extend_from_slice(&[0x77; 32]);

        maximal.push(0x01);
        maximal.push(0x01);
        maximal.extend_from_slice(&1_u64.to_be_bytes());
        maximal.extend_from_slice(&2_u64.to_be_bytes());
        maximal.extend_from_slice(&3_u64.to_be_bytes());
        maximal.extend_from_slice(&4_u64.to_be_bytes());

        maximal.push(0x01);
        maximal.extend_from_slice(&5_u64.to_be_bytes());

        maximal.extend_from_slice(&[0x33; 32]);

        assert_eq!(maximal.len(), CAPABILITY_PAYLOAD_MAX_LEN_V1);

        maximal.pop();

        assert_eq!(
            CapabilityPayloadV1::decode(&maximal),
            Err(RegistryError::MalformedCapabilityPayload)
        );
    }

    #[test]
    fn capability_payload_v1_decodes_canonical_maximum_payload() {
        let mut target = vec![0x02];
        target.extend_from_slice(&256_u16.to_be_bytes());
        target.extend_from_slice(&[b'n'; 256]);

        let mut executable = vec![0x01];
        executable.extend_from_slice(&[0x44; 32]);

        let mut resources = vec![0x01];

        resources.push(0x01);
        resources.push(0x01);
        resources.extend_from_slice(&[0x55; 32]);

        resources.push(0x01);
        resources.push(0x02);
        resources.extend_from_slice(&[0x66; 32]);

        resources.push(0x01);
        resources.push(0x01);
        resources.extend_from_slice(&[0x77; 32]);

        let mut budget = vec![0x01, 0x01];
        budget.extend_from_slice(&u64::MAX.to_be_bytes());
        budget.extend_from_slice(&2_u64.to_be_bytes());
        budget.extend_from_slice(&3_u64.to_be_bytes());
        budget.extend_from_slice(&4_u64.to_be_bytes());

        let mut expiry = vec![0x01];
        expiry.extend_from_slice(&5_u64.to_be_bytes());

        let payload =
            capability_payload_with_components(&target, &executable, &resources, &budget, &expiry);

        assert_eq!(payload.len(), CAPABILITY_PAYLOAD_MAX_LEN_V1);

        let decoded = CapabilityPayloadV1::decode(&payload).unwrap();

        assert_eq!(
            decoded.target_scope(),
            &TargetScopeV1::NamedScope("n".repeat(256))
        );
        assert_eq!(decoded.authorized_executable(), Some(Caid([0x44; 32])));
        assert_eq!(decoded.execution_budget().unwrap().wall_time_ms(), u64::MAX);
        assert_eq!(decoded.expiry(), Some(5));
        assert_eq!(decoded.governing_policy(), Caid([0x33; 32]));
    }

    #[test]
    fn v2_node_fixed_encoding_and_identity_vector() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let mut expected = Vec::new();
        expected.push(0xFE);
        expected.extend_from_slice(&0x0002_u16.to_be_bytes());
        expected.push(0x02);
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.extend_from_slice(&[0xAA; 32]);
        expected.extend_from_slice(&1_u32.to_be_bytes());
        expected.push(0xFF);

        assert_eq!(expected.len(), 45);
        assert_eq!(node.encode(), expected);

        let expected_caid = Caid([
            0x8D, 0x3D, 0x7A, 0x43, 0xB7, 0xD6, 0x4D, 0x83, 0x2F, 0x16, 0x9B, 0x4D, 0x0D, 0x45,
            0x12, 0xBF, 0x79, 0x89, 0x54, 0x7F, 0xDB, 0x0A, 0xD5, 0xDA, 0x3A, 0xBB, 0x53, 0x75,
            0x16, 0x49, 0x12, 0x5D,
        ]);

        assert_eq!(node.caid(), expected_caid);
    }

    #[test]
    fn v2_node_round_trip_is_byte_identical() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Workflow,
            vec![Caid([0x11; 32]), Caid([0x22; 32])],
            b"deterministic-evaluation-workflow".to_vec(),
        )
        .unwrap();

        let encoded = node.encode();
        let decoded = VersionedRegistryNode::decode(&encoded).unwrap();

        assert_eq!(decoded, node);
        assert_eq!(decoded.encode(), encoded);
        assert_eq!(decoded.caid(), node.caid());
    }

    #[test]
    fn v2_edge_fixed_vector_is_exactly_68_bytes() {
        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        let encoded = serialize_edge_v2(&edge);

        let mut expected = Vec::new();
        expected.push(0xFD);
        expected.extend_from_slice(&0x0002_u16.to_be_bytes());
        expected.push(0x01);
        expected.extend_from_slice(&[0xAA; 32]);
        expected.extend_from_slice(&[0xBB; 32]);

        assert_eq!(encoded.len(), 68);
        assert_eq!(encoded, expected);
        assert_eq!(deserialize_edge_v2(&encoded).unwrap(), edge);
    }

    #[test]
    fn node_and_edge_magic_are_strictly_isolated() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        assert_eq!(
            VersionedRegistryNode::decode(&serialize_edge_v2(&edge)),
            Err(RegistryError::SchemaViolation)
        );

        assert_eq!(
            deserialize_edge_v2(&node.encode()),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn unsupported_v2_version_fails_closed() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let mut encoded = node.encode();
        encoded[1] = 0x00;
        encoded[2] = 0x03;

        assert_eq!(
            VersionedRegistryNode::decode(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn unknown_object_class_fails_closed() {
        let node = VersionedRegistryNode::new(
            ObjectClass::Specification,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let mut encoded = node.encode();
        encoded[3] = 0x0A;

        assert_eq!(
            VersionedRegistryNode::decode(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn unsupported_edge_v2_version_fails_closed() {
        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        let mut encoded = serialize_edge_v2(&edge);
        encoded[1] = 0x00;
        encoded[2] = 0x03;

        assert_eq!(
            deserialize_edge_v2(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }
    #[test]
    fn unknown_relation_type_fails_closed() {
        let edge = RegistryEdge::new(
            Caid([0xAA; 32]),
            Caid([0xBB; 32]),
            RelationType::DerivedFrom,
        );

        let mut encoded = serialize_edge_v2(&edge);
        encoded[3] = 0x08;

        assert_eq!(
            deserialize_edge_v2(&encoded),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn v2_node_rejects_truncation_and_trailing_bytes() {
        let node = VersionedRegistryNode::new(
            ObjectClass::EvidencePackage,
            vec![Caid([0xAA; 32])],
            vec![0xFF],
        )
        .unwrap();

        let encoded = node.encode();

        assert_eq!(
            VersionedRegistryNode::decode(&encoded[..encoded.len() - 1]),
            Err(RegistryError::SchemaViolation)
        );

        let mut trailing = encoded;
        trailing.push(0x00);

        assert_eq!(
            VersionedRegistryNode::decode(&trailing),
            Err(RegistryError::SchemaViolation)
        );
    }

    #[test]
    fn adding_edge_does_not_change_endpoint_node_caids() {
        let parent = VersionedRegistryNode::new(
            ObjectClass::Dataset,
            vec![Caid([0x11; 32])],
            b"dataset".to_vec(),
        )
        .unwrap();

        let child = VersionedRegistryNode::new(
            ObjectClass::Workflow,
            vec![parent.caid()],
            b"workflow".to_vec(),
        )
        .unwrap();

        let parent_before = parent.caid();
        let child_before = child.caid();

        let _edge = RegistryEdge::new(parent.caid(), child.caid(), RelationType::Consumes);

        assert_eq!(parent.caid(), parent_before);
        assert_eq!(child.caid(), child_before);
    }
}
