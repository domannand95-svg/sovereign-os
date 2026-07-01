use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationRequest {
    pub required_compute_cores: u32,
    pub required_memory_bytes: u64,
    pub required_capabilities: Vec<String>,
}
