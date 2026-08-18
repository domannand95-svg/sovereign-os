//! BETA-001-T5.3 — Technical Containment & Isolation Boundary
//!
//! Enforces deterministic, fail-closed runtime containment over the
//! declarative `T5RuntimeProfile`.

use crate::runtime_profile::{ExternalNetworkPolicy, LocalInferenceTransport, T5RuntimeProfile};
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
                write!(
                    f,
                    "path escape attempt: '{attempted}' resolved to '{target}'"
                )
            }
            Self::ProhibitedWriteAttempt { path } => {
                write!(
                    f,
                    "prohibited write attempt outside ephemeral roots: '{path}'"
                )
            }
            Self::ProhibitedReadAttempt { path } => {
                write!(
                    f,
                    "prohibited read attempt outside readable roots: '{path}'"
                )
            }
            Self::ProtectedSurfaceMutationAttempt { path, surface } => {
                write!(
                    f,
                    "write attempt targeting protected surface '{surface}': '{path}'"
                )
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

    let canonical =
        env.canonicalize(raw_path)
            .map_err(|e| ContainmentBreach::UnresolvablePath {
                path: raw_path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

    let allowed = profile
        .readable_roots()
        .iter()
        .any(|root| is_path_under_root(&canonical, Path::new(root.as_str())));

    if !allowed {
        return Err(ContainmentBreach::ProhibitedReadAttempt {
            path: canonical.to_string_lossy().to_string(),
        });
    }

    Ok(canonical)
}

/// Verifies whether an attempted write operation is strictly within ephemeral write roots
/// and does not target protected state surfaces.
pub fn verify_write_access(
    profile: &T5RuntimeProfile,
    env: &impl HostEnvironment,
    raw_path: &Path,
) -> Result<PathBuf, ContainmentBreach> {
    validate_path_syntax(raw_path)?;

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
                let c = env
                    .canonicalize(p)
                    .map_err(|e| ContainmentBreach::UnresolvablePath {
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
                    reason: "no existing ancestor directory could be verified or resolved"
                        .to_string(),
                });
            }
        }
    };

    // 1. Protected surface check must precede ephemeral root check.
    for surface in profile.protected_state_surfaces() {
        let surface_path = Path::new(surface.as_str());

        if is_path_under_root(&canonical, surface_path)
            || surface_path.starts_with(&canonical)
            || is_path_under_root(raw_path, surface_path)
        {
            return Err(ContainmentBreach::ProtectedSurfaceMutationAttempt {
                path: raw_path.to_string_lossy().to_string(),
                surface: surface.as_str().to_string(),
            });
        }
    }

    // 2. Must be contained within an explicitly declared ephemeral write root.
    let in_ephemeral_root = profile
        .ephemeral_write_roots()
        .iter()
        .any(|root| is_path_under_root(&canonical, Path::new(root.as_str())));

    if !in_ephemeral_root {
        return Err(ContainmentBreach::ProhibitedWriteAttempt {
            path: raw_path.to_string_lossy().to_string(),
        });
    }

    Ok(canonical)
}

/// Evaluates network transport and destination to enforce external isolation.
pub fn verify_network_transport(
    profile: &T5RuntimeProfile,
    destination: &str,
) -> Result<(), ContainmentBreach> {
    if profile.network_policy().external_network != ExternalNetworkPolicy::Denied {
        return Err(ContainmentBreach::AdmissionFailure {
            reason: "external network policy is not DENIED".to_string(),
        });
    }

    let dest_clean = destination.trim();

    if dest_clean.to_ascii_lowercase().starts_with(r"\\.\pipe\") {
        return Ok(());
    }

    if dest_clean.starts_with("unix:")
        || dest_clean.starts_with("/tmp/")
        || dest_clean.starts_with("/run/")
    {
        return Ok(());
    }

    let host_part = if let Some(stripped) = dest_clean.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = dest_clean.strip_prefix("https://") {
        stripped
    } else {
        dest_clean
    };

    let host = if host_part.starts_with('[') {
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

    if profile.network_policy().local_inference_transport != LocalInferenceTransport::HostLocalOnly
    {
        return Err(ContainmentBreach::InvalidTransport {
            transport: destination.to_string(),
            reason: "local inference transport is not HOST_LOCAL_ONLY".to_string(),
        });
    }

    Ok(())
}

/// Pre-flight admission gate: checks that the environment satisfies all containment constraints.
pub fn check_preflight_admission(
    profile: &T5RuntimeProfile,
    env: &impl HostEnvironment,
) -> Result<(), ContainmentBreach> {
    if profile.network_policy().external_network != ExternalNetworkPolicy::Denied {
        return Err(ContainmentBreach::AdmissionFailure {
            reason: "profile network policy must be DENIED".to_string(),
        });
    }

    for root in profile.readable_roots() {
        let p = Path::new(root.as_str());
        validate_path_syntax(p)?;
        if !env.exists(p) {
            return Err(ContainmentBreach::AdmissionFailure {
                reason: format!("readable root does not exist: {}", root.as_str()),
            });
        }
    }

    for root in profile.ephemeral_write_roots() {
        let p = Path::new(root.as_str());
        validate_path_syntax(p)?;
        if !env.exists(p) {
            return Err(ContainmentBreach::AdmissionFailure {
                reason: format!("ephemeral write root does not exist: {}", root.as_str()),
            });
        }
    }

    Ok(())
}
