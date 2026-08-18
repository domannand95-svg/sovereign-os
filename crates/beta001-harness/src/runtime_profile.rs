use std::fmt;

pub const T5_RUNTIME_PROFILE_VERSION_V1: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredPath(String);

impl DeclaredPath {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeProfileError> {
        let value = value.into();
        validate_declared_path(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalNetworkPolicy {
    Denied,
    Allowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocalInferenceTransport {
    HostLocalOnly,
    ExternalBridgeAllowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkPolicy {
    pub external_network: ExternalNetworkPolicy,
    pub local_inference_transport: LocalInferenceTransport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StateSurfaceKind {
    Repository,
    AgentMemory,
    ProceduralSkillState,
    RuntimeCache,
    RuntimeConfiguration,
    CredentialState,
    EvidenceStore,
    OtherExplicitlyDeclared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersistenceClass {
    Ephemeral,
    Persistent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTreatment {
    PrePostIntegrity,
    ContinuousContainment,
    HarnessControlledFinalization,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateSurface {
    id: String,
    kind: StateSurfaceKind,
    path: Option<DeclaredPath>,
    persistence: PersistenceClass,
    evidence_treatment: EvidenceTreatment,
}

impl StateSurface {
    pub fn new(
        id: impl Into<String>,
        kind: StateSurfaceKind,
        path: Option<DeclaredPath>,
        persistence: PersistenceClass,
        evidence_treatment: EvidenceTreatment,
    ) -> Result<Self, RuntimeProfileError> {
        let id = id.into();
        validate_identifier(&id)?;

        Ok(Self {
            id,
            kind,
            path,
            persistence,
            evidence_treatment,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn kind(&self) -> StateSurfaceKind {
        self.kind
    }

    pub fn path(&self) -> Option<&DeclaredPath> {
        self.path.as_ref()
    }

    pub const fn persistence(&self) -> PersistenceClass {
        self.persistence
    }

    pub const fn evidence_treatment(&self) -> EvidenceTreatment {
        self.evidence_treatment
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolPolicy {
    visible_read_tools: Vec<String>,
    unavailable_tools: Vec<String>,
    prohibited_tools: Vec<String>,
}

impl ToolPolicy {
    pub fn new(
        visible_read_tools: Vec<String>,
        unavailable_tools: Vec<String>,
        prohibited_tools: Vec<String>,
    ) -> Result<Self, RuntimeProfileError> {
        let policy = Self {
            visible_read_tools,
            unavailable_tools,
            prohibited_tools,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn visible_read_tools(&self) -> &[String] {
        &self.visible_read_tools
    }

    pub fn unavailable_tools(&self) -> &[String] {
        &self.unavailable_tools
    }

    pub fn prohibited_tools(&self) -> &[String] {
        &self.prohibited_tools
    }

    fn validate(&self) -> Result<(), RuntimeProfileError> {
        validate_tool_set(&self.visible_read_tools)?;
        validate_tool_set(&self.unavailable_tools)?;
        validate_tool_set(&self.prohibited_tools)?;

        for tool in &self.visible_read_tools {
            if self.unavailable_tools.contains(tool) || self.prohibited_tools.contains(tool) {
                return Err(RuntimeProfileError::ContradictoryToolClassification(
                    tool.clone(),
                ));
            }
        }

        for tool in &self.unavailable_tools {
            if self.prohibited_tools.contains(tool) {
                return Err(RuntimeProfileError::ContradictoryToolClassification(
                    tool.clone(),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct T5RuntimeProfile {
    profile_version: u16,
    readable_roots: Vec<DeclaredPath>,
    ephemeral_write_roots: Vec<DeclaredPath>,
    protected_state_surfaces: Vec<DeclaredPath>,
    network_policy: NetworkPolicy,
    tool_policy: ToolPolicy,
    state_surface_inventory: Vec<StateSurface>,
}

impl T5RuntimeProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        profile_version: u16,
        readable_roots: Vec<DeclaredPath>,
        ephemeral_write_roots: Vec<DeclaredPath>,
        protected_state_surfaces: Vec<DeclaredPath>,
        network_policy: NetworkPolicy,
        tool_policy: ToolPolicy,
        state_surface_inventory: Vec<StateSurface>,
    ) -> Result<Self, RuntimeProfileError> {
        let profile = Self {
            profile_version,
            readable_roots,
            ephemeral_write_roots,
            protected_state_surfaces,
            network_policy,
            tool_policy,
            state_surface_inventory,
        };

        profile.validate()?;
        Ok(profile)
    }

    pub const fn profile_version(&self) -> u16 {
        self.profile_version
    }

    pub fn readable_roots(&self) -> &[DeclaredPath] {
        &self.readable_roots
    }

    pub fn ephemeral_write_roots(&self) -> &[DeclaredPath] {
        &self.ephemeral_write_roots
    }

    pub fn protected_state_surfaces(&self) -> &[DeclaredPath] {
        &self.protected_state_surfaces
    }

    pub const fn network_policy(&self) -> NetworkPolicy {
        self.network_policy
    }

    pub const fn tool_policy(&self) -> &ToolPolicy {
        &self.tool_policy
    }

    pub fn state_surface_inventory(&self) -> &[StateSurface] {
        &self.state_surface_inventory
    }

    pub fn validate(&self) -> Result<(), RuntimeProfileError> {
        if self.profile_version != T5_RUNTIME_PROFILE_VERSION_V1 {
            return Err(RuntimeProfileError::UnsupportedProfileVersion(
                self.profile_version,
            ));
        }

        if self.network_policy.external_network != ExternalNetworkPolicy::Denied {
            return Err(RuntimeProfileError::ExternalNetworkMustBeDenied);
        }

        if self.network_policy.local_inference_transport != LocalInferenceTransport::HostLocalOnly {
            return Err(RuntimeProfileError::ExternalInferenceBridgeForbidden);
        }

        validate_unique_paths(&self.readable_roots)?;
        validate_unique_paths(&self.ephemeral_write_roots)?;
        validate_unique_paths(&self.protected_state_surfaces)?;
        self.tool_policy.validate()?;

        for writable in &self.ephemeral_write_roots {
            for protected in &self.protected_state_surfaces {
                if paths_overlap(writable, protected) {
                    return Err(RuntimeProfileError::WritableProtectedOverlap {
                        writable: writable.as_str().to_owned(),
                        protected: protected.as_str().to_owned(),
                    });
                }
            }
        }

        let mut surface_ids = Vec::with_capacity(self.state_surface_inventory.len());

        for surface in &self.state_surface_inventory {
            if surface_ids.iter().any(|id| id == surface.id()) {
                return Err(RuntimeProfileError::DuplicateStateSurfaceIdentity(
                    surface.id().to_owned(),
                ));
            }
            surface_ids.push(surface.id().to_owned());
        }

        for protected in &self.protected_state_surfaces {
            let represented = self.state_surface_inventory.iter().any(|surface| {
                surface
                    .path()
                    .is_some_and(|path| path.as_str() == protected.as_str())
            });

            if !represented {
                return Err(RuntimeProfileError::ProtectedSurfaceMissingFromInventory(
                    protected.as_str().to_owned(),
                ));
            }
        }

        Ok(())
    }

    pub fn policy_allows_read(&self, path: &DeclaredPath) -> bool {
        self.readable_roots
            .iter()
            .any(|root| path_is_within(root, path))
    }

    pub fn policy_allows_write(&self, path: &DeclaredPath) -> bool {
        self.ephemeral_write_roots
            .iter()
            .any(|root| path_is_within(root, path))
            && !self
                .protected_state_surfaces
                .iter()
                .any(|protected| paths_overlap(protected, path))
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RuntimeProfileError> {
        self.validate()?;

        match self.profile_version {
            T5_RUNTIME_PROFILE_VERSION_V1 => Ok(self.canonical_bytes_v1()),
            version => Err(RuntimeProfileError::UnsupportedProfileVersion(version)),
        }
    }

    pub fn digest(&self) -> Result<[u8; 32], RuntimeProfileError> {
        let canonical = self.canonical_bytes()?;
        Ok(*blake3::hash(&canonical).as_bytes())
    }

    fn canonical_bytes_v1(&self) -> Vec<u8> {
        let mut encoded = Vec::new();

        encoded.extend_from_slice(&self.profile_version.to_be_bytes());

        encode_paths(&mut encoded, &self.readable_roots);
        encode_paths(&mut encoded, &self.ephemeral_write_roots);
        encode_paths(&mut encoded, &self.protected_state_surfaces);

        encoded.push(match self.network_policy.external_network {
            ExternalNetworkPolicy::Denied => 0,
            ExternalNetworkPolicy::Allowed => 1,
        });

        encoded.push(match self.network_policy.local_inference_transport {
            LocalInferenceTransport::HostLocalOnly => 0,
            LocalInferenceTransport::ExternalBridgeAllowed => 1,
        });

        encode_sorted_strings(&mut encoded, self.tool_policy.visible_read_tools());
        encode_sorted_strings(&mut encoded, self.tool_policy.unavailable_tools());
        encode_sorted_strings(&mut encoded, self.tool_policy.prohibited_tools());

        let mut surfaces: Vec<&StateSurface> = self.state_surface_inventory.iter().collect();
        surfaces.sort_by(|left, right| left.id().cmp(right.id()));

        encode_len(&mut encoded, surfaces.len());

        for surface in surfaces {
            encode_text(&mut encoded, surface.id());

            encoded.push(state_surface_kind_tag(surface.kind()));
            encoded.push(persistence_tag(surface.persistence()));
            encoded.push(evidence_treatment_tag(surface.evidence_treatment()));

            match surface.path() {
                None => encoded.push(0),
                Some(path) => {
                    encoded.push(1);
                    encode_text(&mut encoded, path.as_str());
                }
            }
        }

        encoded
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeProfileError {
    EmptyIdentifier,
    NulIdentifier,
    UnsupportedProfileVersion(u16),
    NonAbsolutePath(String),
    NonNormalizedPath(String),
    TraversalComponent(String),
    DuplicatePath(String),
    DuplicateTool(String),
    ContradictoryToolClassification(String),
    ExternalNetworkMustBeDenied,
    ExternalInferenceBridgeForbidden,
    WritableProtectedOverlap { writable: String, protected: String },
    DuplicateStateSurfaceIdentity(String),
    ProtectedSurfaceMissingFromInventory(String),
}

impl fmt::Display for RuntimeProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RuntimeProfileError {}

fn validate_identifier(value: &str) -> Result<(), RuntimeProfileError> {
    if value.is_empty() {
        return Err(RuntimeProfileError::EmptyIdentifier);
    }

    if value.as_bytes().contains(&0) {
        return Err(RuntimeProfileError::NulIdentifier);
    }

    Ok(())
}

fn validate_declared_path(value: &str) -> Result<(), RuntimeProfileError> {
    if value.is_empty() || value.as_bytes().contains(&0) {
        return Err(RuntimeProfileError::NonNormalizedPath(value.to_owned()));
    }

    if value.contains('\\') || value.contains("//") {
        return Err(RuntimeProfileError::NonNormalizedPath(value.to_owned()));
    }

    let bytes = value.as_bytes();

    let unix_absolute = value.starts_with('/');
    let windows_absolute =
        bytes.len() >= 3 && bytes[0].is_ascii_uppercase() && bytes[1] == b':' && bytes[2] == b'/';

    if !unix_absolute && !windows_absolute {
        return Err(RuntimeProfileError::NonAbsolutePath(value.to_owned()));
    }

    let windows_root = windows_absolute && value.len() == 3;

    if value.len() > 1 && !windows_root && value.ends_with('/') {
        return Err(RuntimeProfileError::NonNormalizedPath(value.to_owned()));
    }

    let remainder = if windows_absolute {
        &value[3..]
    } else {
        &value[1..]
    };

    for component in remainder.split('/') {
        if component == "." || component == ".." {
            return Err(RuntimeProfileError::TraversalComponent(value.to_owned()));
        }

        if component.is_empty() && !remainder.is_empty() {
            return Err(RuntimeProfileError::NonNormalizedPath(value.to_owned()));
        }
    }

    Ok(())
}

fn validate_unique_paths(paths: &[DeclaredPath]) -> Result<(), RuntimeProfileError> {
    for (index, path) in paths.iter().enumerate() {
        if paths[..index].contains(path) {
            return Err(RuntimeProfileError::DuplicatePath(path.as_str().to_owned()));
        }
    }

    Ok(())
}

fn validate_tool_set(tools: &[String]) -> Result<(), RuntimeProfileError> {
    for (index, tool) in tools.iter().enumerate() {
        validate_identifier(tool)?;

        if tools[..index].contains(tool) {
            return Err(RuntimeProfileError::DuplicateTool(tool.clone()));
        }
    }

    Ok(())
}

fn paths_overlap(left: &DeclaredPath, right: &DeclaredPath) -> bool {
    path_is_within(left, right) || path_is_within(right, left)
}

fn path_is_within(root: &DeclaredPath, candidate: &DeclaredPath) -> bool {
    let root = root.as_str();
    let candidate = candidate.as_str();

    if root == candidate {
        return true;
    }

    if root == "/" {
        return candidate.starts_with('/');
    }

    if root.len() == 3 && root.as_bytes()[1] == b':' && root.ends_with('/') {
        return candidate.starts_with(root);
    }

    candidate
        .strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn encode_paths(encoded: &mut Vec<u8>, paths: &[DeclaredPath]) {
    let mut sorted: Vec<&DeclaredPath> = paths.iter().collect();
    sorted.sort();

    encode_len(encoded, sorted.len());

    for path in sorted {
        encode_text(encoded, path.as_str());
    }
}

fn encode_sorted_strings(encoded: &mut Vec<u8>, values: &[String]) {
    let mut sorted: Vec<&str> = values.iter().map(String::as_str).collect();
    sorted.sort_unstable();

    encode_len(encoded, sorted.len());

    for value in sorted {
        encode_text(encoded, value);
    }
}

fn encode_len(encoded: &mut Vec<u8>, value: usize) {
    let value = u32::try_from(value).expect("runtime profile collection length exceeds u32");
    encoded.extend_from_slice(&value.to_be_bytes());
}

fn encode_text(encoded: &mut Vec<u8>, value: &str) {
    encode_len(encoded, value.len());
    encoded.extend_from_slice(value.as_bytes());
}

const fn state_surface_kind_tag(kind: StateSurfaceKind) -> u8 {
    match kind {
        StateSurfaceKind::Repository => 0,
        StateSurfaceKind::AgentMemory => 1,
        StateSurfaceKind::ProceduralSkillState => 2,
        StateSurfaceKind::RuntimeCache => 3,
        StateSurfaceKind::RuntimeConfiguration => 4,
        StateSurfaceKind::CredentialState => 5,
        StateSurfaceKind::EvidenceStore => 6,
        StateSurfaceKind::OtherExplicitlyDeclared => 7,
    }
}

const fn persistence_tag(persistence: PersistenceClass) -> u8 {
    match persistence {
        PersistenceClass::Ephemeral => 0,
        PersistenceClass::Persistent => 1,
    }
}

const fn evidence_treatment_tag(treatment: EvidenceTreatment) -> u8 {
    match treatment {
        EvidenceTreatment::PrePostIntegrity => 0,
        EvidenceTreatment::ContinuousContainment => 1,
        EvidenceTreatment::HarnessControlledFinalization => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> DeclaredPath {
        DeclaredPath::new(value).expect("test path must be valid")
    }

    fn repository_surface() -> StateSurface {
        StateSurface::new(
            "repository",
            StateSurfaceKind::Repository,
            Some(path("C:/sol")),
            PersistenceClass::Persistent,
            EvidenceTreatment::PrePostIntegrity,
        )
        .expect("surface must be valid")
    }

    fn skill_surface() -> StateSurface {
        StateSurface::new(
            "skill-state",
            StateSurfaceKind::ProceduralSkillState,
            Some(path("C:/sol/SKILL.md")),
            PersistenceClass::Persistent,
            EvidenceTreatment::PrePostIntegrity,
        )
        .expect("surface must be valid")
    }

    fn valid_profile_with_order(reverse: bool) -> T5RuntimeProfile {
        let mut readable = vec![path("C:/sol/fixtures"), path("C:/sol/docs")];
        let mut tools = vec!["read_file".to_owned(), "list_dir".to_owned()];

        if reverse {
            readable.reverse();
            tools.reverse();
        }

        T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            readable,
            vec![path("C:/t5-temp")],
            vec![path("C:/sol/SKILL.md")],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransport::HostLocalOnly,
            },
            ToolPolicy::new(
                tools,
                vec!["missing_optional_reader".to_owned()],
                vec!["write_file".to_owned(), "network_client".to_owned()],
            )
            .expect("tool policy must be valid"),
            vec![repository_surface(), skill_surface()],
        )
        .expect("profile must be valid")
    }

    #[test]
    fn canonical_encoding_and_digest_ignore_declaration_order() {
        let first = valid_profile_with_order(false);
        let second = valid_profile_with_order(true);

        assert_eq!(
            first.canonical_bytes().expect("encoding must succeed"),
            second.canonical_bytes().expect("encoding must succeed")
        );

        assert_eq!(
            first.digest().expect("digest must succeed"),
            second.digest().expect("digest must succeed")
        );
    }

    #[test]
    fn declarative_paths_reject_relative_traversal_and_non_normalized_forms() {
        assert!(matches!(
            DeclaredPath::new("relative/path"),
            Err(RuntimeProfileError::NonAbsolutePath(_))
        ));

        assert!(matches!(
            DeclaredPath::new("C:/sol/../escape"),
            Err(RuntimeProfileError::TraversalComponent(_))
        ));

        assert!(matches!(
            DeclaredPath::new("C:\\sol\\fixtures"),
            Err(RuntimeProfileError::NonNormalizedPath(_))
        ));

        assert!(matches!(
            DeclaredPath::new("C:/sol//fixtures"),
            Err(RuntimeProfileError::NonNormalizedPath(_))
        ));
    }

    #[test]
    fn external_network_policy_fails_closed() {
        let result = T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            vec![path("C:/sol/fixtures")],
            vec![path("C:/t5-temp")],
            vec![path("C:/sol/SKILL.md")],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Allowed,
                local_inference_transport: LocalInferenceTransport::HostLocalOnly,
            },
            ToolPolicy::new(vec![], vec![], vec![]).expect("tool policy"),
            vec![skill_surface()],
        );

        assert_eq!(
            result,
            Err(RuntimeProfileError::ExternalNetworkMustBeDenied)
        );
    }

    #[test]
    fn external_inference_bridge_fails_closed() {
        let result = T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            vec![path("C:/sol/fixtures")],
            vec![path("C:/t5-temp")],
            vec![path("C:/sol/SKILL.md")],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransport::ExternalBridgeAllowed,
            },
            ToolPolicy::new(vec![], vec![], vec![]).expect("tool policy"),
            vec![skill_surface()],
        );

        assert_eq!(
            result,
            Err(RuntimeProfileError::ExternalInferenceBridgeForbidden)
        );
    }

    #[test]
    fn writable_root_cannot_overlap_protected_surface() {
        let result = T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            vec![path("C:/sol/fixtures")],
            vec![path("C:/sol")],
            vec![path("C:/sol/SKILL.md")],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransport::HostLocalOnly,
            },
            ToolPolicy::new(vec![], vec![], vec![]).expect("tool policy"),
            vec![skill_surface()],
        );

        assert!(matches!(
            result,
            Err(RuntimeProfileError::WritableProtectedOverlap { .. })
        ));
    }

    #[test]
    fn protected_surface_must_exist_in_inventory() {
        let result = T5RuntimeProfile::new(
            T5_RUNTIME_PROFILE_VERSION_V1,
            vec![path("C:/sol/fixtures")],
            vec![path("C:/t5-temp")],
            vec![path("C:/sol/SKILL.md")],
            NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransport::HostLocalOnly,
            },
            ToolPolicy::new(vec![], vec![], vec![]).expect("tool policy"),
            vec![repository_surface()],
        );

        assert_eq!(
            result,
            Err(RuntimeProfileError::ProtectedSurfaceMissingFromInventory(
                "C:/sol/SKILL.md".to_owned()
            ))
        );
    }

    #[test]
    fn tool_categories_are_pairwise_disjoint() {
        let result = ToolPolicy::new(
            vec!["read_file".to_owned()],
            vec![],
            vec!["read_file".to_owned()],
        );

        assert_eq!(
            result,
            Err(RuntimeProfileError::ContradictoryToolClassification(
                "read_file".to_owned()
            ))
        );
    }

    #[test]
    fn read_and_write_policy_are_fail_closed() {
        let profile = valid_profile_with_order(false);

        assert!(profile.policy_allows_read(&path("C:/sol/fixtures/task.json")));
        assert!(!profile.policy_allows_read(&path("C:/sol/private.txt")));

        assert!(profile.policy_allows_write(&path("C:/t5-temp/output.tmp")));
        assert!(!profile.policy_allows_write(&path("C:/sol/SKILL.md")));
        assert!(!profile.policy_allows_write(&path("C:/other/output.tmp")));
    }

    #[test]
    fn unsupported_profile_version_cannot_select_new_encoding() {
        let mut profile = valid_profile_with_order(false);
        profile.profile_version = 2;

        assert_eq!(
            profile.canonical_bytes(),
            Err(RuntimeProfileError::UnsupportedProfileVersion(2))
        );
    }
}
