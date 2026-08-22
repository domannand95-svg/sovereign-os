use crate::{AuditLedgerEntry, Digest};

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

    pub fn reconstruct_entries(entries: &[AuditLedgerEntry]) -> Self {
        let mut report = Self::empty();

        report.total_entries_inspected = entries.len();

        if let Some(first) = entries.first() {
            report.genesis_digest = Some(first.entry_digest.clone());
        }

        if let Some(last) = entries.last() {
            report.head_digest = Some(last.entry_digest.clone());
        }

        for entry in entries {
            if !entry.verify_integrity() {
                report.status = ReconstructionStatus::Invalid;

                report
                    .anomalies
                    .push(ReconstructionAnomaly::EntryIntegrityFailure {
                        sequence: entry.sequence,
                    });
            }
        }

        report
    }
}