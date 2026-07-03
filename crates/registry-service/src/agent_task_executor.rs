use crate::{AgentTask, AgentTaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationProof {
    pub step_trace: Vec<String>,
    pub state_delta_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    Success {
        proof: VerificationProof,
        output_payload: Vec<u8>,
    },
    Failure {
        error_code: u32,
        diagnostic_log: String,
        slashing_triggered: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutorError {
    InvalidTaskState,
    InvalidProof,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTaskExecutor {
    pub executor_id: [u8; 16],
    pub capability_tier: u8,
}

impl AgentTaskExecutor {
    pub fn new(executor_id: [u8; 16], capability_tier: u8) -> Self {
        Self {
            executor_id,
            capability_tier,
        }
    }

    pub fn verify_proof(&self, _task: &AgentTask, proof: &VerificationProof) -> bool {
        !proof.step_trace.is_empty()
    }

    pub fn process_outcome(
        &self,
        mut task: AgentTask,
        outcome: ExecutionOutcome,
    ) -> Result<AgentTask, ExecutorError> {
        if task.status != AgentTaskStatus::Running {
            return Err(ExecutorError::InvalidTaskState);
        }

        match outcome {
            ExecutionOutcome::Success { proof, .. } => {
                if !self.verify_proof(&task, &proof) {
                    return Err(ExecutorError::InvalidProof);
                }

                task.mark_completed()
                    .map_err(|_| ExecutorError::InvalidTaskState)?;

                Ok(task)
            }
            ExecutionOutcome::Failure { .. } => {
                task.mark_failed()
                    .map_err(|_| ExecutorError::InvalidTaskState)?;

                Ok(task)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentTaskKind, AgentTaskStatus};

    fn running_task(id: &str) -> AgentTask {
        let mut task = AgentTask::new(
            id,
            AgentTaskKind::Research,
            "agent-alpha",
            "payload",
        );

        task.validate().unwrap();
        task.queue().unwrap();
        task.mark_running().unwrap();

        task
    }

    fn valid_proof() -> VerificationProof {
        VerificationProof {
            step_trace: vec![
                "loaded input payload".to_string(),
                "performed deterministic evaluation".to_string(),
                "produced output payload".to_string(),
            ],
            state_delta_hash: [7; 32],
        }
    }

    #[test]
    fn rejects_non_running_task() {
        let executor = AgentTaskExecutor::new([1; 16], 1);

        let task = AgentTask::new(
            "task-1",
            AgentTaskKind::Research,
            "agent-alpha",
            "payload",
        );

        let outcome = ExecutionOutcome::Success {
            proof: valid_proof(),
            output_payload: vec![1, 2, 3],
        };

        assert_eq!(
            executor.process_outcome(task, outcome),
            Err(ExecutorError::InvalidTaskState)
        );
    }

    #[test]
    fn success_requires_valid_proof() {
        let executor = AgentTaskExecutor::new([1; 16], 1);
        let task = running_task("task-1");

        let outcome = ExecutionOutcome::Success {
            proof: VerificationProof {
                step_trace: vec![],
                state_delta_hash: [0; 32],
            },
            output_payload: vec![1, 2, 3],
        };

        assert_eq!(
            executor.process_outcome(task, outcome),
            Err(ExecutorError::InvalidProof)
        );
    }

    #[test]
    fn success_marks_task_completed() {
        let executor = AgentTaskExecutor::new([1; 16], 1);
        let task = running_task("task-1");

        let outcome = ExecutionOutcome::Success {
            proof: valid_proof(),
            output_payload: vec![1, 2, 3],
        };

        let task = executor.process_outcome(task, outcome).unwrap();

        assert_eq!(task.status, AgentTaskStatus::Completed);
    }

    #[test]
    fn failure_marks_task_failed() {
        let executor = AgentTaskExecutor::new([1; 16], 1);
        let task = running_task("task-1");

        let outcome = ExecutionOutcome::Failure {
            error_code: 500,
            diagnostic_log: "deterministic execution failed".to_string(),
            slashing_triggered: false,
        };

        let task = executor.process_outcome(task, outcome).unwrap();

        assert_eq!(task.status, AgentTaskStatus::Failed);
    }
}
