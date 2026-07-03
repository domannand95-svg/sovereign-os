use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CapabilityTier {
    Tier0Sandbox,
    Tier1Standard,
    Tier2Advanced,
    Tier3Institutional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRecord {
    pub agent_id: [u8; 16],
    pub tier: CapabilityTier,
    pub performance_points: i64,
    pub total_tasks_completed: u64,
    pub is_isolated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRegistryError {
    AgentAlreadyRegistered,
    AgentNotFound,
    AgentIsolated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRegistry {
    pub agents: HashMap<[u8; 16], AgentRecord>,
    pub promotion_threshold: i64,
    pub demotion_threshold: i64,
}

impl AgentRegistry {
    pub fn new(promotion_threshold: i64, demotion_threshold: i64) -> Self {
        Self {
            agents: HashMap::new(),
            promotion_threshold,
            demotion_threshold,
        }
    }

    pub fn register_agent(&mut self, id: [u8; 16]) -> Result<(), AgentRegistryError> {
        if self.agents.contains_key(&id) {
            return Err(AgentRegistryError::AgentAlreadyRegistered);
        }

        self.agents.insert(
            id,
            AgentRecord {
                agent_id: id,
                tier: CapabilityTier::Tier0Sandbox,
                performance_points: 0,
                total_tasks_completed: 0,
                is_isolated: false,
            },
        );

        Ok(())
    }

    pub fn record_success(
        &mut self,
        id: [u8; 16],
        delta: u32,
    ) -> Result<(), AgentRegistryError> {
        let threshold = self.promotion_threshold;
        let agent = self
            .agents
            .get_mut(&id)
            .ok_or(AgentRegistryError::AgentNotFound)?;

        if agent.is_isolated {
            return Err(AgentRegistryError::AgentIsolated);
        }

        agent.performance_points += delta as i64;
        agent.total_tasks_completed += 1;

        if agent.performance_points >= threshold {
            agent.tier = promote_one_tier(agent.tier);
        }

        Ok(())
    }

    pub fn record_slashing(
        &mut self,
        id: [u8; 16],
        penalty: u32,
    ) -> Result<(), AgentRegistryError> {
        let threshold = self.demotion_threshold;
        let agent = self
            .agents
            .get_mut(&id)
            .ok_or(AgentRegistryError::AgentNotFound)?;

        agent.performance_points -= penalty as i64;

        if agent.performance_points < threshold {
            if agent.tier == CapabilityTier::Tier0Sandbox {
                agent.is_isolated = true;
            } else {
                agent.tier = demote_one_tier(agent.tier);
            }
        }

        Ok(())
    }

    pub fn verify_trust_boundary(
        &self,
        id: [u8; 16],
        required_tier: CapabilityTier,
    ) -> bool {
        self.agents
            .get(&id)
            .map(|agent| !agent.is_isolated && agent.tier >= required_tier)
            .unwrap_or(false)
    }
}

fn promote_one_tier(tier: CapabilityTier) -> CapabilityTier {
    match tier {
        CapabilityTier::Tier0Sandbox => CapabilityTier::Tier1Standard,
        CapabilityTier::Tier1Standard => CapabilityTier::Tier2Advanced,
        CapabilityTier::Tier2Advanced => CapabilityTier::Tier3Institutional,
        CapabilityTier::Tier3Institutional => CapabilityTier::Tier3Institutional,
    }
}

fn demote_one_tier(tier: CapabilityTier) -> CapabilityTier {
    match tier {
        CapabilityTier::Tier0Sandbox => CapabilityTier::Tier0Sandbox,
        CapabilityTier::Tier1Standard => CapabilityTier::Tier0Sandbox,
        CapabilityTier::Tier2Advanced => CapabilityTier::Tier1Standard,
        CapabilityTier::Tier3Institutional => CapabilityTier::Tier2Advanced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_id(value: u8) -> [u8; 16] {
        [value; 16]
    }

    #[test]
    fn register_agent_starts_in_sandbox() {
        let mut registry = AgentRegistry::new(10, -10);
        let id = agent_id(1);

        registry.register_agent(id).unwrap();

        let agent = registry.agents.get(&id).unwrap();
        assert_eq!(agent.tier, CapabilityTier::Tier0Sandbox);
        assert_eq!(agent.performance_points, 0);
        assert_eq!(agent.total_tasks_completed, 0);
        assert!(!agent.is_isolated);
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = AgentRegistry::new(10, -10);
        let id = agent_id(1);

        registry.register_agent(id).unwrap();

        assert_eq!(
            registry.register_agent(id),
            Err(AgentRegistryError::AgentAlreadyRegistered)
        );
    }

    #[test]
    fn success_adds_points_and_promotes_one_tier() {
        let mut registry = AgentRegistry::new(10, -10);
        let id = agent_id(1);

        registry.register_agent(id).unwrap();
        registry.record_success(id, 10).unwrap();

        let agent = registry.agents.get(&id).unwrap();
        assert_eq!(agent.performance_points, 10);
        assert_eq!(agent.total_tasks_completed, 1);
        assert_eq!(agent.tier, CapabilityTier::Tier1Standard);
    }

    #[test]
    fn slashing_demotes_or_isolates_agent() {
        let mut registry = AgentRegistry::new(10, -10);
        let id = agent_id(1);

        registry.register_agent(id).unwrap();
        registry.record_slashing(id, 11).unwrap();

        let agent = registry.agents.get(&id).unwrap();
        assert!(agent.is_isolated);
    }

    #[test]
    fn isolated_agents_cannot_record_success() {
        let mut registry = AgentRegistry::new(10, -10);
        let id = agent_id(1);

        registry.register_agent(id).unwrap();
        registry.record_slashing(id, 11).unwrap();

        assert_eq!(
            registry.record_success(id, 20),
            Err(AgentRegistryError::AgentIsolated)
        );
    }

    #[test]
    fn trust_boundary_requires_existing_unisolated_agent_with_sufficient_tier() {
        let mut registry = AgentRegistry::new(10, -10);
        let id = agent_id(1);

        registry.register_agent(id).unwrap();

        assert!(registry.verify_trust_boundary(id, CapabilityTier::Tier0Sandbox));
        assert!(!registry.verify_trust_boundary(id, CapabilityTier::Tier1Standard));

        registry.record_success(id, 10).unwrap();

        assert!(registry.verify_trust_boundary(id, CapabilityTier::Tier1Standard));
    }
}
