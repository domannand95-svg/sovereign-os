//! BETA-001-T5.4 â€” Observation Plane (Telemetry)
//!
//! Captures independent observation telemetry (containment events, filesystem
//! access, network attempts) outside model control.

use crate::containment::ContainmentBreach;
use serde::{Deserialize, Serialize};

/// Version constant for the telemetry observation plane schema.
pub const TELEMETRY_PLANE_VERSION: u16 = 1;

/// Type of filesystem access attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessKind {
    Read,
    Write,
}

/// Record of an observed filesystem access resolution attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemAccessEvent {
    pub path: String,
    pub canonical_path: Option<String>,
    pub kind: AccessKind,
    pub permitted: bool,
}

/// Record of an observed network connection or transport attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAccessEvent {
    pub endpoint: String,
    pub permitted: bool,
    pub blocked: bool,
}

/// Complete observation telemetry captured during a single execution run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentTelemetry {
    pub version: u16,
    pub breaches: Vec<ContainmentBreachRecord>,
    pub filesystem_access_log: Vec<FilesystemAccessEvent>,
    pub network_access_log: Vec<NetworkAccessEvent>,
    pub is_clean: bool,
}

/// Serializable representation of a containment breach event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentBreachRecord {
    pub breach_kind: String,
    pub details: String,
}

impl From<&ContainmentBreach> for ContainmentBreachRecord {
    fn from(b: &ContainmentBreach) -> Self {
        let kind = match b {
            ContainmentBreach::PathEscapeAttempt { .. } => "PathEscapeAttempt",
            ContainmentBreach::ProhibitedWriteAttempt { .. } => "ProhibitedWriteAttempt",
            ContainmentBreach::ProhibitedReadAttempt { .. } => "ProhibitedReadAttempt",
            ContainmentBreach::ProtectedSurfaceMutationAttempt { .. } => {
                "ProtectedSurfaceMutationAttempt"
            }
            ContainmentBreach::ExternalNetworkAttempt { .. } => "ExternalNetworkAttempt",
            ContainmentBreach::UnresolvablePath { .. } => "UnresolvablePath",
            ContainmentBreach::NonCanonicalPath { .. } => "NonCanonicalPath",
            ContainmentBreach::InvalidTransport { .. } => "InvalidTransport",
            ContainmentBreach::AdmissionFailure { .. } => "AdmissionFailure",
        };
        Self {
            breach_kind: kind.to_string(),
            details: b.to_string(),
        }
    }
}

impl Default for ContainmentTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainmentTelemetry {
    /// Creates a fresh, empty telemetry collector.
    pub fn new() -> Self {
        Self {
            version: TELEMETRY_PLANE_VERSION,
            breaches: Vec::new(),
            filesystem_access_log: Vec::new(),
            network_access_log: Vec::new(),
            is_clean: true,
        }
    }

    /// Records an observed containment breach. Automatically sets `is_clean = false`.
    pub fn record_breach(&mut self, breach: &ContainmentBreach) {
        self.breaches.push(ContainmentBreachRecord::from(breach));
        self.is_clean = false;
    }

    /// Records a filesystem access attempt.
    pub fn record_fs_access(
        &mut self,
        path: String,
        canonical_path: Option<String>,
        kind: AccessKind,
        permitted: bool,
    ) {
        if !permitted {
            self.is_clean = false;
        }
        self.filesystem_access_log.push(FilesystemAccessEvent {
            path,
            canonical_path,
            kind,
            permitted,
        });
    }

    /// Records a network transport attempt.
    pub fn record_network_access(&mut self, endpoint: String, permitted: bool, blocked: bool) {
        if !permitted || blocked {
            self.is_clean = false;
        }
        self.network_access_log.push(NetworkAccessEvent {
            endpoint,
            permitted,
            blocked,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_clean_by_default() {
        let telem = ContainmentTelemetry::new();
        assert!(telem.is_clean);
        assert_eq!(telem.version, TELEMETRY_PLANE_VERSION);
        assert!(telem.breaches.is_empty());
    }

    #[test]
    fn test_record_breach_marks_dirty() {
        let mut telem = ContainmentTelemetry::new();
        let breach = ContainmentBreach::ExternalNetworkAttempt {
            endpoint: "api.openai.com:443".to_string(),
        };
        telem.record_breach(&breach);
        assert!(!telem.is_clean);
        assert_eq!(telem.breaches.len(), 1);
        assert_eq!(telem.breaches[0].breach_kind, "ExternalNetworkAttempt");
    }

    #[test]
    fn test_record_unpermitted_fs_access_marks_dirty() {
        let mut telem = ContainmentTelemetry::new();
        telem.record_fs_access("/repo/secret".to_string(), None, AccessKind::Read, false);
        assert!(!telem.is_clean);
        assert_eq!(telem.filesystem_access_log.len(), 1);
    }
}
