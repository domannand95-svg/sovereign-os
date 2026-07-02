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
    Validated,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Error returned when an invalid lifecycle transition is attempted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTaskTransitionError {
    pub from: AgentTaskStatus,
    pub to: AgentTaskStatus,
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

    pub fn validate(&mut self) -> Result<(), AgentTaskTransitionError> {
        self.transition_to(AgentTaskStatus::Validated)
    }

    pub fn queue(&mut self) -> Result<(), AgentTaskTransitionError> {
        self.transition_to(AgentTaskStatus::Queued)
    }

    pub fn mark_running(&mut self) -> Result<(), AgentTaskTransitionError> {
        self.transition_to(AgentTaskStatus::Running)
    }

    pub fn mark_completed(&mut self) -> Result<(), AgentTaskTransitionError> {
        self.transition_to(AgentTaskStatus::Completed)
    }

    pub fn mark_failed(&mut self) -> Result<(), AgentTaskTransitionError> {
        self.transition_to(AgentTaskStatus::Failed)
    }

    pub fn cancel(&mut self) -> Result<(), AgentTaskTransitionError> {
        self.transition_to(AgentTaskStatus::Cancelled)
    }

    fn transition_to(&mut self, next: AgentTaskStatus) -> Result<(), AgentTaskTransitionError> {
        if Self::can_transition(&self.status, &next) {
            self.status = next;
            Ok(())
        } else {
            Err(AgentTaskTransitionError {
                from: self.status.clone(),
                to: next,
            })
        }
    }

    fn can_transition(current: &AgentTaskStatus, next: &AgentTaskStatus) -> bool {
        use AgentTaskStatus::*;

        matches!(
            (current, next),
            (Pending, Validated)
                | (Validated, Queued)
                | (Queued, Running)
                | (Running, Completed)
                | (Running, Failed)
                | (Pending, Cancelled)
                | (Validated, Cancelled)
                | (Queued, Cancelled)
                | (Running, Cancelled)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> AgentTask {
        AgentTask::new(
            "task-1",
            AgentTaskKind::Calculation,
            "agent-alpha",
            "sum:1,2,3",
        )
    }

    #[test]
    fn new_task_starts_pending() {
        let task = task();

        assert_eq!(task.id, AgentTaskId("task-1".to_string()));
        assert_eq!(task.kind, AgentTaskKind::Calculation);
        assert_eq!(task.submitted_by, "agent-alpha");
        assert_eq!(task.payload, "sum:1,2,3");
        assert_eq!(task.status, AgentTaskStatus::Pending);
    }

    #[test]
    fn task_follows_valid_lifecycle() {
        let mut task = task();

        task.validate().unwrap();
        assert_eq!(task.status, AgentTaskStatus::Validated);

        task.queue().unwrap();
        assert_eq!(task.status, AgentTaskStatus::Queued);

        task.mark_running().unwrap();
        assert_eq!(task.status, AgentTaskStatus::Running);

        task.mark_completed().unwrap();
        assert_eq!(task.status, AgentTaskStatus::Completed);
    }

    #[test]
    fn task_cannot_skip_validation() {
        let mut task = task();

        let error = task.mark_running().unwrap_err();

        assert_eq!(error.from, AgentTaskStatus::Pending);
        assert_eq!(error.to, AgentTaskStatus::Running);
        assert_eq!(task.status, AgentTaskStatus::Pending);
    }

    #[test]
    fn terminal_states_do_not_transition() {
        let mut task = task();

        task.validate().unwrap();
        task.queue().unwrap();
        task.mark_running().unwrap();
        task.mark_completed().unwrap();

        let error = task.cancel().unwrap_err();

        assert_eq!(error.from, AgentTaskStatus::Completed);
        assert_eq!(error.to, AgentTaskStatus::Cancelled);
        assert_eq!(task.status, AgentTaskStatus::Completed);
    }

    #[test]
    fn task_can_cancel_before_terminal_state() {
        let mut task = task();

        task.validate().unwrap();
        task.cancel().unwrap();

        assert_eq!(task.status, AgentTaskStatus::Cancelled);
    }
}
