use std::path::PathBuf;

use crate::{
    AgentTaskQueue, Registry,
    AgentRecord, AgentRegistry, AgentTaskScheduler, CapabilityTier, ConsensusState, EventLedger, GovernanceEngine,
    LedgerEntry, LedgerEvent, LedgerHeader, Proposal, VerificationEngine, VoteType,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorCommand {
    RegisterAgent { agent_id: [u8; 16] },
    SubmitProposal { proposal: Proposal },
    CastVote {
        proposal_id: [u8; 16],
        voter_id: [u8; 16],
        vote: VoteType,
    },
    DispatchNextTask,
    SubmitTaskProof { task_id: [u8; 16] },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorEvent {
    CommandExecuted {
        sequence: u64,
        command_type: String,
    },
    SystemAlert {
        details: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorError {
    LedgerAppendFailed,
}

pub struct RuntimeOrchestrator {
    pub registry: AgentRegistry,
    pub scheduler: AgentTaskScheduler,
    pub verification: VerificationEngine,
    pub governance: GovernanceEngine,
    pub consensus: ConsensusState,
    pub ledger: EventLedger,
    pub command_sequence: u64,
}

impl RuntimeOrchestrator {
    pub fn new(
        registry: AgentRegistry,
        scheduler: AgentTaskScheduler,
        verification: VerificationEngine,
        governance: GovernanceEngine,
        consensus: ConsensusState,
        ledger: EventLedger,
    ) -> Self {
        Self {
            registry,
            scheduler,
            verification,
            governance,
            consensus,
            ledger,
            command_sequence: 0,
        }
    }

    pub fn boot(path: impl Into<PathBuf>) -> Result<Self, OrchestratorError> {
        let registry = AgentRegistry::new(100, -50);
        let scheduler = AgentTaskScheduler::new(AgentTaskQueue::new(10));
        let verification = VerificationEngine::new(10, true);
        let governance =
            GovernanceEngine::new(3, 6_600, 100).map_err(|_| OrchestratorError::LedgerAppendFailed)?;
        let consensus = ConsensusState::new();
        let ledger = EventLedger::new();

        let mut orchestrator = Self::new(
            registry,
            scheduler,
            verification,
            governance,
            consensus,
            ledger,
        );

        let rebuilt = Registry::open(path).map_err(|_| OrchestratorError::LedgerAppendFailed)?;
        orchestrator.registry = AgentRegistry::new(100, -50);
orchestrator.registry.agents = rebuilt
            .list_agents()
            .into_iter()
            .map(|node| {
                let agent_id = *node.node_id.as_bytes();
                (
                    agent_id,
                    AgentRecord {
                        agent_id,
                        tier: CapabilityTier::Tier0Sandbox,
                        performance_points: 0,
                        total_tasks_completed: 0,
                        is_isolated: false,
                    },
                )
            })
            .collect();

        Ok(orchestrator)
    }

    pub fn execute(
        &mut self,
        command: OrchestratorCommand,
    ) -> Result<OrchestratorEvent, OrchestratorError> {
        self.command_sequence += 1;

        let command_type = command.command_type();
        let ledger_event = command.to_ledger_event();

        let entry = LedgerEntry {
            header: LedgerHeader {
                index: self.ledger.current_height() + 1,
                term: self.consensus.current_term,
                timestamp: self.command_sequence,
                previous_hash: self.ledger.current_hash,
                payload_hash: deterministic_payload_hash(self.command_sequence, &command_type),
            },
            event: ledger_event,
        };

        self.ledger
            .append_entry(entry)
            .map_err(|_| OrchestratorError::LedgerAppendFailed)?;

        Ok(OrchestratorEvent::CommandExecuted {
            sequence: self.command_sequence,
            command_type,
        })
    }
}

impl OrchestratorCommand {
    fn command_type(&self) -> String {
        match self {
            OrchestratorCommand::RegisterAgent { .. } => "RegisterAgent".to_string(),
            OrchestratorCommand::SubmitProposal { .. } => "SubmitProposal".to_string(),
            OrchestratorCommand::CastVote { .. } => "CastVote".to_string(),
            OrchestratorCommand::DispatchNextTask => "DispatchNextTask".to_string(),
            OrchestratorCommand::SubmitTaskProof { .. } => "SubmitTaskProof".to_string(),
        }
    }

    fn to_ledger_event(&self) -> LedgerEvent {
        match self {
            OrchestratorCommand::RegisterAgent { agent_id } => {
                LedgerEvent::AgentRegistered { agent_id: *agent_id }
            }
            OrchestratorCommand::SubmitProposal { proposal } => LedgerEvent::ProposalCommitted {
                proposal_id: proposal.proposal_id,
                status: "submitted".to_string(),
            },
            OrchestratorCommand::CastVote { proposal_id, .. } => LedgerEvent::ProposalCommitted {
                proposal_id: *proposal_id,
                status: "vote_cast".to_string(),
            },
            OrchestratorCommand::DispatchNextTask => LedgerEvent::TaskStateChanged {
                task_id: [0; 16],
                new_status: "dispatch_requested".to_string(),
            },
            OrchestratorCommand::SubmitTaskProof { task_id } => LedgerEvent::TaskStateChanged {
                task_id: *task_id,
                new_status: "proof_submitted".to_string(),
            },
        }
    }
}

fn deterministic_payload_hash(sequence: u64, command_type: &str) -> [u8; 32] {
    let mut hash = [0u8; 32];

    for (i, byte) in sequence.to_le_bytes().iter().enumerate() {
        hash[i] = hash[i].wrapping_add(*byte);
    }

    for (i, byte) in command_type.as_bytes().iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte).rotate_left(1);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentTaskQueue, CapabilityTier};

    fn orchestrator() -> RuntimeOrchestrator {
        RuntimeOrchestrator::new(
            AgentRegistry::new(100, -50),
            AgentTaskScheduler::new(AgentTaskQueue::new(10)),
            VerificationEngine::new(10, true),
            GovernanceEngine::new(3, 6_600, 100).unwrap(),
            ConsensusState::new(),
            EventLedger::new(),
        )
    }

    #[test]
    fn new_orchestrator_starts_at_sequence_zero() {
        let runtime = orchestrator();

        assert_eq!(runtime.command_sequence, 0);
        assert_eq!(runtime.ledger.current_height(), 0);
    }

    #[test]
    fn execute_register_agent_records_ledger_entry() {
        let mut runtime = orchestrator();

        let event = runtime
            .execute(OrchestratorCommand::RegisterAgent { agent_id: [1; 16] })
            .unwrap();

        assert_eq!(
            event,
            OrchestratorEvent::CommandExecuted {
                sequence: 1,
                command_type: "RegisterAgent".to_string(),
            }
        );

        assert_eq!(runtime.command_sequence, 1);
        assert_eq!(runtime.ledger.current_height(), 1);
    }

    #[test]
    fn execute_submit_proposal_records_ledger_entry() {
        let mut runtime = orchestrator();

        let proposal = runtime
            .governance
            .create_proposal(
                [2; 16],
                [3; 16],
                CapabilityTier::Tier1Standard,
                CapabilityTier::Tier1Standard,
                50,
            )
            .unwrap();

        runtime
            .execute(OrchestratorCommand::SubmitProposal { proposal })
            .unwrap();

        assert_eq!(runtime.command_sequence, 1);
        assert_eq!(runtime.ledger.current_height(), 1);
    }

    #[test]
    fn execute_multiple_commands_advances_sequence_monotonically() {
        let mut runtime = orchestrator();

        runtime
            .execute(OrchestratorCommand::RegisterAgent { agent_id: [1; 16] })
            .unwrap();

        runtime
            .execute(OrchestratorCommand::DispatchNextTask)
            .unwrap();

        assert_eq!(runtime.command_sequence, 2);
        assert_eq!(runtime.ledger.current_height(), 2);
        assert_eq!(runtime.ledger.validate_chain(), Ok(()));
    }
}
