use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("event log error: {0}")]
    EventLog(#[from] event_log::EventLogError),

    #[error("registry error: {0}")]
    General(String),
}
