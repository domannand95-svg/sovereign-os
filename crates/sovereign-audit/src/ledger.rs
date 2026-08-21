use crate::identity::{AgentIdentityId, Digest};
use blake3::Hasher;

const AUDIT_LEDGER_DOMAIN: &[u8] = b"SOVEREIGN_AUDIT_LEDGER_V1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventType {
    AuthorizationRecorded,
    ExecutionAttempted,
    ExecutionCommitted,
    ExecutionFailed,
    ReceiptVerified,
}

impl AuditEventType {
    fn canonical_bytes(&self) -> &'static [u8] {
        match self {
            Self::AuthorizationRecorded => b"AuthorizationRecorded",
            Self::ExecutionAttempted => b"ExecutionAttempted",
            Self::ExecutionCommitted => b"ExecutionCommitted",
            Self::ExecutionFailed => b"ExecutionFailed",
            Self::ReceiptVerified => b"ReceiptVerified",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLedgerEntry {
    pub sequence: u64,
    pub previous_entry_digest: Digest,
    pub event_type: AuditEventType,
    pub subject_digest: Digest,
    pub payload_digest: Digest,
    pub recorded_at: String,
    pub recorded_by: AgentIdentityId,
    pub entry_digest: Digest,
}

impl AuditLedgerEntry {
    pub fn derive_digest(
        sequence: u64,
        previous_entry_digest: &Digest,
        event_type: &AuditEventType,
        subject_digest: &Digest,
        payload_digest: &Digest,
        recorded_at: &str,
        recorded_by: &AgentIdentityId,
    ) -> Digest {
        let mut hasher = Hasher::new();

        hasher.update(AUDIT_LEDGER_DOMAIN);
        hasher.update(&sequence.to_be_bytes());
        hasher.update(previous_entry_digest.0.as_bytes());
        hasher.update(event_type.canonical_bytes());
        hasher.update(subject_digest.0.as_bytes());
        hasher.update(payload_digest.0.as_bytes());
        hasher.update(recorded_at.as_bytes());
        hasher.update(recorded_by.0.as_bytes());

        Digest(hex::encode(hasher.finalize().as_bytes()))
    }

    pub fn verify_integrity(&self) -> bool {
        self.entry_digest
            == Self::derive_digest(
                self.sequence,
                &self.previous_entry_digest,
                &self.event_type,
                &self.subject_digest,
                &self.payload_digest,
                &self.recorded_at,
                &self.recorded_by,
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditLedgerError {
    InvalidDigest,
}
#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: &str) -> Digest {
        Digest(value.to_owned())
    }

    fn identity(value: &str) -> AgentIdentityId {
        AgentIdentityId(value.to_owned())
    }

    fn entry() -> AuditLedgerEntry {
        let previous = digest("genesis");
        let subject = digest("subject");
        let payload = digest("payload");
        let recorded_by = identity("agent-001");

        let entry_digest = AuditLedgerEntry::derive_digest(
            1,
            &previous,
            &AuditEventType::ExecutionCommitted,
            &subject,
            &payload,
            "2026-08-22T04:00:00Z",
            &recorded_by,
        );

        AuditLedgerEntry {
            sequence: 1,
            previous_entry_digest: previous,
            event_type: AuditEventType::ExecutionCommitted,
            subject_digest: subject,
            payload_digest: payload,
            recorded_at: "2026-08-22T04:00:00Z".to_owned(),
            recorded_by,
            entry_digest,
        }
    }

    #[test]
    fn ledger_entry_digest_is_deterministic() {
        let first = entry();
        let second = entry();

        assert_eq!(first.entry_digest, second.entry_digest);
    }

    #[test]
    fn ledger_entry_integrity_verifies() {
        let record = entry();

        assert!(record.verify_integrity());
    }

    #[test]
    fn payload_mutation_breaks_integrity() {
        let mut record = entry();

        record.payload_digest = digest("tampered");

        assert!(!record.verify_integrity());
    }

    #[test]
    fn sequence_mutation_breaks_integrity() {
        let mut record = entry();

        record.sequence = 2;

        assert!(!record.verify_integrity());
    }
}
