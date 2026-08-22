use crate::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructionStatus {
    Valid,
    Partial,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconstructionAnomaly {
    SequenceGap {
        expected: u64,
        observed: u64,
    },

    DuplicateSequence {
        sequence: u64,
    },

    PreviousDigestMismatch {
        sequence: u64,
    },

    EntryIntegrityFailure {
        sequence: u64,
    },

    UnexpectedGenesis,

    ConflictingEntry {
        sequence: u64,
    },

    OutOfOrderInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReconstructionReport {
    pub total_entries_inspected: usize,
    pub genesis_digest: Option<Digest>,
    pub head_digest: Option<Digest>,
    pub status: ReconstructionStatus,
    pub anomalies: Vec<ReconstructionAnomaly>,
}

impl AuditReconstructionReport {
    pub fn empty() -> Self {
        Self {
            total_entries_inspected: 0,
            genesis_digest: None,
            head_digest: None,
            status: ReconstructionStatus::Valid,
            anomalies: Vec::new(),
        }
    }
}