#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    InvalidEvidenceGraph,
    BrokenLineage,
    InvalidCryptographicAncestry,
    UnsupportedSchemaVersion,
    CyclicLineage,
}
