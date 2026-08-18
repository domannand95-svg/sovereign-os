//! BETA-001-T5.3 — Technical Containment & Isolation Boundary
//!
//! Enforces deterministic, fail-closed runtime containment over the
//! declarative `T5RuntimeProfile`.

use crate::runtime_profile::{
    ExternalNetworkPolicy, LocalInferenceTransportPolicy, T5RuntimeProfile,
};
use std::fmt;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};

/// Specific containment violation categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainmentBreach {
    PathEscapeAttempt { attempted: String, target: String },
    ProhibitedWriteAttempt { path: String },
    ProhibitedReadAttempt { path: String },
    ProtectedSurfaceMutationAttempt { path: String, surface: String },
    ExternalNetworkAttempt { endpoint: String },
    UnresolvablePath { path: String, reason: String },
    NonCanonicalPath { path: String, reason: String },
    InvalidTransport { transport: String, reason: String },
    AdmissionFailure { reason: String },
}

impl fmt::Display for ContainmentBreach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathEscapeAttempt { attempted, target } => {
                write!(f, "path escape attempt: '{attempted}' resolved to '{target}'")
            }
            Self::ProhibitedWriteAttempt { path } => {
                write!(f, "prohibited write attempt outside ephemeral roots: '{path}'")
            }
            Self::ProhibitedReadAttempt { path } => {
                write!(f, "prohibited read attempt outside readable roots: '{path}'")
            }
            Self::ProtectedSurfaceMutationAttempt { path, surface } => {
                write!(f, "write attempt targeting protected surface '{surface}': '{path}'")
            }
            Self::ExternalNetworkAttempt { endpoint } => {
                write!(f, "external network egress/ingress attempt: '{endpoint}'")
            }
            Self::UnresolvablePath { path, reason } => {
                write!(f, "unresolvable path '{path}': {reason}")
            }
            Self::NonCanonicalPath { path, reason } => {
                write!(f, "non-canonical path rejected '{path}': {reason}")
            }
            Self::InvalidTransport { transport, reason } => {
                write!(f, "invalid inference transport '{transport}': {reason}")
            }
            Self::AdmissionFailure { reason } => {
                write!(f, "containment pre-flight admission failed: {reason}")
            }
        }
    }
}

impl std::error::Error for ContainmentBreach {}

/// Abstract host filesystem environment interface for containment verification.
pub trait HostEnvironment {
    /// Canonicalize a path on the physical filesystem, resolving symlinks, junctions, and mount aliases.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, std::io::Error>;
    /// Check if a path physically exists on the host.
    fn exists(&self, path: &Path) -> bool;
}

/// Default standard filesystem environment using std::fs.
#[derive(Debug, Default, Clone, Copy)]
pub struct PhysicalHostEnvironment;

impl HostEnvironment for PhysicalHostEnvironment {
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        path.canonicalize()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Validates that a path string is strictly normalized and contains no traversal components.
pub fn validate_path_syntax(path: &Path) -> Result<(), ContainmentBreach> {
    if !path.is_absolute() {
        return Err(ContainmentBreach::NonCanonicalPath {
            path: path.to_string_lossy().to_string(),
            reason: "path must be absolute".to_string(),
        });
    }

    for comp in path.components() {
        match comp {
            Component::CurDir | Component::ParentDir => {
                return Err(ContainmentBreach::NonCanonicalPath {
                    path: path.to_string_lossy().to_string(),
                    reason: "relative traversal components (. or ..) are prohibited".to_string(),
                });
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

/// Checks if `target` is contained within or equal to `root`.
pub fn is_path_under_root(target: &Path, root: &Path) -> bool {
    target.starts_with(root)
}

/// Verifies whether an attempted read operation is permitted by the runtime profile.
pub fn verify_read_access(
    profile: &T5RuntimeProfile,
    env: &impl HostEnvironment,
    raw_path: &Path,
) -> Result<PathBuf, ContainmentBreach> {
    validate_path_syntax(raw_path)?;

    let canonical = env
        .canonicalize(raw_path)
        .map_err(|e| ContainmentBreach::UnresolvablePath {
            path: raw_path.to_string_lossy().to_string(),
            reason: e.to_string(),
        })?;

    let allowed = profile
        .readable_roots
        .iter()
        .any(|root| is_path_under_root(&canonical, Path::new(root)));

    if !allowed {
        return Err(ContainmentBreach::ProhibitedReadAttempt {
            path: canonical.to_string_lossy().to_string(),
        });
    }

    Ok(canonical)
}

/// Verifies whether an attempted write operation is strictly within ephemeral write roots
/// and does not target protected state surfaces.
///
/// Order of enforcement:
/// 1. Path syntax validation.
/// 2. Deep canonicalization (fail-closed if ancestor resolution fails).
/// 3. Protected surface collision check (higher-order invariant).
/// 4. Ephemeral write root membership check.
pub fn verify_write_access(
    profile: &T5RuntimeProfile,
    env: &impl HostEnvironment,
    raw_path: &Path,
) -> Result<PathBuf, ContainmentBreach> {
    validate_path_syntax(raw_path)?;

    // Resolve target path or deepest existing ancestor.
    let canonical = if env.exists(raw_path) {
        env.canonicalize(raw_path)
            .map_err(|e| ContainmentBreach::UnresolvablePath {
                path: raw_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?
    } else {
        let mut ancestor = raw_path.parent();
        let mut resolved = None;
        while let Some(p) = ancestor {
            if env.exists(p) {
                let c = env.canonicalize(p).map_err(|e| ContainmentBreach::UnresolvablePath {
                    path: p.to_string_lossy().to_string(),
                    reason: format!("failed to canonicalize existing ancestor: {e}"),
                })?;
                resolved = Some(c);
                break;
            }
            ancestor = p.parent();
        }

        match resolved {
            Some(c) => c,
            None => {
                return Err(ContainmentBreach::UnresolvablePath {
                    path: raw_path.to_string_lossy().to_string(),
                    reason: "no existing ancestor directory could be verified or resolved".to_string(),
                });
            }
        }
    };

    // 1. HIGHER-ORDER INVARIANT: Protected surface check must precede ephemeral root check.
    for surface in &profile.protected_state_surfaces {
        let surface_path = Path::new(&surface.path);
        if is_path_under_root(&canonical, surface_path)
            || surface_path.starts_with(&canonical)
            || is_path_under_root(raw_path, surface_path)
        {
            return Err(ContainmentBreach::ProtectedSurfaceMutationAttempt {
                path: raw_path.to_string_lossy().to_string(),
                surface: surface.path.clone(),
            });
        }
    }

    // 2. Must be contained within an explicitly declared ephemeral write root.
    let in_ephemeral_root = profile
        .ephemeral_write_roots
        .iter()
        .any(|root| is_path_under_root(&canonical, Path::new(root)));

    if !in_ephemeral_root {
        return Err(ContainmentBreach::ProhibitedWriteAttempt {
            path: raw_path.to_string_lossy().to_string(),
        });
    }

    Ok(canonical)
}

/// Evaluates network transport and destination to enforce external isolation.
/// Validates endpoint host/port or local IPC paths without substring/prefix ambiguity.
pub fn verify_network_transport(
    profile: &T5RuntimeProfile,
    destination: &str,
) -> Result<(), ContainmentBreach> {
    if profile.network_policy.external_network != ExternalNetworkPolicy::Denied {
        return Err(ContainmentBreach::AdmissionFailure {
            reason: "external network policy is not DENIED".to_string(),
        });
    }

    let dest_clean = destination.trim();

    // Check for named pipe (Windows)
    if dest_clean.to_ascii_lowercase().starts_with(r"\\.\pipe\") {
        return Ok(());
    }

    // Check for unix domain socket / local IPC paths
    if dest_clean.starts_with("unix:")
        || dest_clean.starts_with("/tmp/")
        || dest_clean.starts_with("/run/")
    {
        return Ok(());
    }

    // Parse host and optional port strictly
    let host_part = if let Some(stripped) = dest_clean.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = dest_clean.strip_prefix("https://") {
        stripped
    } else {
        dest_clean
    };

    let host = if host_part.starts_with('[') {
        // IPv6 bracketed format: [::1]:port or [::1]
        if let Some(close_idx) = host_part.find(']') {
            &host_part[1..close_idx]
        } else {
            return Err(ContainmentBreach::InvalidTransport {
                transport: destination.to_string(),
                reason: "malformed bracketed IPv6 host".to_string(),
            });
        }
    } else if let Some(colon_idx) = host_part.find(':') {
        &host_part[..colon_idx]
    } else {
        host_part
    };

    let is_host_local = if host.eq_ignore_ascii_case("localhost") {
        true
    } else if let Ok(ip) = host.parse::<IpAddr>() {
        ip.is_loopback()
    } else {
        false
    };

    if !is_host_local {
        return Err(ContainmentBreach::ExternalNetworkAttempt {
            endpoint: destination.to_string(),
        });
    }

    if profile.network_policy.local_inference_transport
        != LocalInferenceTransportPolicy::HostLocalOnly
    {
        return Err(ContainmentBreach::InvalidTransport {
            transport: destination.to_string(),
            reason: "local inference transport is not HOST_LOCAL_ONLY".to_string(),
        });
    }

    Ok(())
}

/// Pre-flight admission gate: checks that the environment satisfies all containment constraints
/// prior to starting an execution run.
pub fn check_preflight_admission(
    profile: &T5RuntimeProfile,
    env: &impl HostEnvironment,
) -> Result<(), ContainmentBreach> {
    if profile.network_policy.external_network != ExternalNetworkPolicy::Denied {
        return Err(ContainmentBreach::AdmissionFailure {
            reason: "profile network policy must be DENIED".to_string(),
        });
    }

    for root in &profile.readable_roots {
        let p = Path::new(root);
        validate_path_syntax(p)?;
        if !env.exists(p) {
            return Err(ContainmentBreach::AdmissionFailure {
                reason: format!("readable root does not exist: {root}"),
            });
        }
    }

    for root in &profile.ephemeral_write_roots {
        let p = Path::new(root);
        validate_path_syntax(p)?;
        if !env.exists(p) {
            return Err(ContainmentBreach::AdmissionFailure {
                reason: format!("ephemeral write root does not exist: {root}"),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_profile::{
        NetworkPolicy, ProtectedSurface, StateSurfaceKind, T5RuntimeProfile,
    };
    use std::collections::HashMap;

    #[derive(Default)]
    struct MockHostEnvironment {
        existing_paths: HashMap<PathBuf, PathBuf>,
    }

    impl MockHostEnvironment {
        fn register(&mut self, raw: &str, canonical: &str) {
            self.existing_paths
                .insert(PathBuf::from(raw), PathBuf::from(canonical));
        }
    }

    impl HostEnvironment for MockHostEnvironment {
        fn canonicalize(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
            self.existing_paths.get(path).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "path not found in mock")
            })
        }

        fn exists(&self, path: &Path) -> bool {
            self.existing_paths.contains_key(path)
        }
    }

    fn sample_profile() -> T5RuntimeProfile {
        T5RuntimeProfile {
            profile_version: "t5-profile-v1".to_string(),
            readable_roots: vec!["/workspace/repo/corpus".to_string()],
            ephemeral_write_roots: vec!["/workspace/repo/target/tmp".to_string()],
            protected_state_surfaces: vec![ProtectedSurface {
                path: "/workspace/repo/.git".to_string(),
                kind: StateSurfaceKind::Repository,
            }],
            network_policy: NetworkPolicy {
                external_network: ExternalNetworkPolicy::Denied,
                local_inference_transport: LocalInferenceTransportPolicy::HostLocalOnly,
            },
            tool_policy: Default::default(),
            state_surface_inventory: vec![],
        }
    }

    #[test]
    fn test_path_syntax_validation() {
        assert!(validate_path_syntax(Path::new("/workspace/corpus/file.json")).is_ok());
        assert!(validate_path_syntax(Path::new("relative/path.json")).is_err());
        assert!(validate_path_syntax(Path::new("/workspace/../secret.json")).is_err());
        assert!(validate_path_syntax(Path::new("/workspace/./corpus")).is_err());
    }

    #[test]
    fn test_read_access_containment() {
        let profile = sample_profile();
        let mut env = MockHostEnvironment::default();
        env.register(
            "/workspace/repo/corpus/task.json",
            "/workspace/repo/corpus/task.json",
        );
        env.register(
            "/workspace/repo/secret.key",
            "/workspace/repo/secret.key",
        );

        let res = verify_read_access(
            &profile,
            &env,
            Path::new("/workspace/repo/corpus/task.json"),
        );
        assert!(res.is_ok());

        let res = verify_read_access(&profile, &env, Path::new("/workspace/repo/secret.key"));
        assert!(matches!(res, Err(ContainmentBreach::ProhibitedReadAttempt { .. })));
    }

    #[test]
    fn test_symlink_escape_detection() {
        let profile = sample_profile();
        let mut env = MockHostEnvironment::default();
        env.register(
            "/workspace/repo/corpus/symlink_escape",
            "/etc/shadow",
        );

        let res = verify_read_access(
            &profile,
            &env,
            Path::new("/workspace/repo/corpus/symlink_escape"),
        );
        assert!(matches!(res, Err(ContainmentBreach::ProhibitedReadAttempt { .. })));
    }

    #[test]
    fn test_write_access_containment_and_protected_precedence() {
        let profile = sample_profile();
        let mut env = MockHostEnvironment::default();
        env.register(
            "/workspace/repo/target/tmp",
            "/workspace/repo/target/tmp",
        );
        env.register(
            "/workspace/repo/.git",
            "/workspace/repo/.git",
        );

        // Write within ephemeral root -> Pass
        let res = verify_write_access(
            &profile,
            &env,
            Path::new("/workspace/repo/target/tmp/scratch.json"),
        );
        assert!(res.is_ok());

        // Write to protected surface -> ProtectedSurfaceMutationAttempt (must be specific)
        let res = verify_write_access(
            &profile,
            &env,
            Path::new("/workspace/repo/.git/config"),
        );
        assert!(matches!(res, Err(ContainmentBreach::ProtectedSurfaceMutationAttempt { .. })));
    }

    #[test]
    fn test_zero_degraded_fallback_on_unresolvable_ancestor() {
        let profile = sample_profile();
        let env = MockHostEnvironment::default(); // completely empty mock environment

        let res = verify_write_access(
            &profile,
            &env,
            Path::new("/completely/unresolvable/path/file.txt"),
        );
        assert!(matches!(res, Err(ContainmentBreach::UnresolvablePath { .. })));
    }

    #[test]
    fn test_network_isolation_and_endpoint_parsing() {
        let profile = sample_profile();

        // Valid host-local endpoints
        assert!(verify_network_transport(&profile, "127.0.0.1:11434").is_ok());
        assert!(verify_network_transport(&profile, "localhost:8080").is_ok());
        assert!(verify_network_transport(&profile, "localhost").is_ok());
        assert!(verify_network_transport(&profile, "[::1]:11434").is_ok());
        assert!(verify_network_transport(&profile, r"\\.\pipe\ollama").is_ok());

        // Adversarial spoofing endpoints -> Must be rejected as ExternalNetworkAttempt
        let res = verify_network_transport(&profile, "127.0.0.1.attacker.com:80");
        assert!(matches!(res, Err(ContainmentBreach::ExternalNetworkAttempt { .. })));

        let res = verify_network_transport(&profile, "localhost.evil.com:80");
        assert!(matches!(res, Err(ContainmentBreach::ExternalNetworkAttempt { .. })));

        let res = verify_network_transport(&profile, "api.openai.com:443");
        assert!(matches!(res, Err(ContainmentBreach::ExternalNetworkAttempt { .. })));
    }

    #[test]
    fn test_preflight_admission() {
        let profile = sample_profile();
        let mut env = MockHostEnvironment::default();

        let res = check_preflight_admission(&profile, &env);
        assert!(matches!(res, Err(ContainmentBreach::AdmissionFailure { .. })));

        env.register("/workspace/repo/corpus", "/workspace/repo/corpus");
        env.register("/workspace/repo/target/tmp", "/workspace/repo/target/tmp");
        let res = check_preflight_admission(&profile, &env);
        assert!(res.is_ok());
    }
}
