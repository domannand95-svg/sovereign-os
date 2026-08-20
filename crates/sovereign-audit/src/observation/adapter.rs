use super::model::ObservationRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    SourceUnavailable,
    InvalidObservation,
}

pub trait ObservationAdapter {
    fn observe(&self) -> Result<ObservationRecord, ObservationError>;
}
