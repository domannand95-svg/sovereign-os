use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::{ActiveEvent, ActiveMemoryError};

pub fn append_event_to_jsonl(
    path: impl AsRef<Path>,
    event: &ActiveEvent,
) -> Result<(), ActiveMemoryError> {
    let serialized = serde_json::to_string(event)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    writeln!(file, "{serialized}")?;

    Ok(())
}
