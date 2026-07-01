#[derive(Debug, Default)]
pub struct GovernanceWrapper;

impl GovernanceWrapper {
    pub fn new() -> Self {
        Self
    }

    pub fn allow(&self) -> bool {
        true
    }
}
