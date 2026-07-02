use serde::{Deserialize, Serialize};

/// Stable identifier for an agent-submitted task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentTaskId(pub String);

/// High-level category of work an agent may request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentTaskKind {
    Calculation,
    DataProcessing,
    Research,
    Planning,
    Simulation,
}

/// Current lifecycle state of an agent task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Registry-owned model for an agent task.
///
/// This type is intentionally data-only. It does not schedule, execute,
/// validate hardware, or perform network I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: AgentTaskId,
    pub kind: AgentTaskKind,
    pub submitted_by: String,
    pub payload: String,
    pub status: AgentTaskStatus,
}

impl AgentTask {
    pub fn new(
        id: impl Into<String>,
        kind: AgentTaskKind,
        submitted_by: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            id: AgentTaskId(id.into()),
            kind,
            submitted_by: submitted_by.into(),
            payload: payload.into(),
            status: AgentTaskStatus::Pending,
        }
    }

    pub fn mark_running(&mut self) {
        self.status = AgentTaskStatus::Running;
    }

    pub fn mark_completed(&mut self) {
        self.status = AgentTaskStatus::Completed;
    }

    pub fn mark_failed(&mut self) {
        self.status = AgentTaskStatus::Failed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_starts_pending() {
        let task = AgentTask::new(
            "task-1",
            AgentTaskKind::Calculation,
            "agent-alpha",
            "sum:1,2,3",
        );

        assert_eq!(task.id, AgentTaskId("task-1".to_string()));
        assert_eq!(task.kind, AgentTaskKind::Calculation);
        assert_eq!(task.submitted_by, "agent-alpha");
        assert_eq!(task.payload, "sum:1,2,3");
        assert_eq!(task.status, AgentTaskStatus::Pending);
    }

    #[test]
    fn task_status_transitions() {
        let mut task = AgentTask::new(
            "task-2",
            AgentTaskKind::DataProcessing,
            "agent-beta",
            "normalize-dataset",
        );

        task.mark_running();
        assert_eq!(task.status, AgentTaskStatus::Running);

        task.mark_completed();
        assert_eq!(task.status, AgentTaskStatus::Completed);

        task.mark_failed();
        assert_eq!(task.status, AgentTaskStatus::Failed);
    }
}
