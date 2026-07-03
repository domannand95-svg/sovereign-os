pub mod error;
pub mod registry;

pub use error::RegistryError;
pub use agent_task::{AgentTask, AgentTaskId, AgentTaskKind, AgentTaskStatus};

pub use registry::{
    CapacityMetrics, NodeRecord, OperationalStatus, Registry, RegistryEvent, Workload,
    WorkloadState,
};

pub mod agent_task;

pub mod allocation;
pub use allocation::AllocationRequest;

pub mod snapshot;

pub use snapshot::{LogicalSequenceNumber, RegistrySnapshot, SnapshotMetadata};

pub mod agent_task_queue;
pub mod agent_task_scheduler;
pub use agent_task_queue::{
    AgentTaskQueue, QueueError, QueuePriority, QueuedTaskItem,
};
pub use agent_task_scheduler::AgentTaskScheduler;

pub mod agent_task_executor;
pub use agent_task_executor::{
    AgentTaskExecutor, ExecutionOutcome, ExecutorError, VerificationProof,
};

pub mod agent_registry;
pub use agent_registry::{
    AgentRecord, AgentRegistry, AgentRegistryError, CapabilityTier,
};

pub mod verification_engine;
pub use verification_engine::{
    ValidationError, VerificationEngine, VerificationMetrics,
};

pub mod governance_engine;
pub use governance_engine::{
    GovernanceEngine, GovernanceError, Proposal, ProposalStatus, VoteType,
};
