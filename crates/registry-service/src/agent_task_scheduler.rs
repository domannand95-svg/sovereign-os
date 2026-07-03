use crate::{AgentTask, AgentTaskQueue, QueueError, QueuePriority};
use serde::{Deserialize, Serialize};

/// Deterministic scheduler for dispatching queued agent tasks.
///
/// This scheduler does not execute work, spawn threads, use async runtimes,
/// inspect clocks, or communicate over the network. It only selects the next
/// queued task and advances it into the Running lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTaskScheduler {
    queue: AgentTaskQueue,
}

impl AgentTaskScheduler {
    pub fn new(queue: AgentTaskQueue) -> Self {
        Self { queue }
    }

    pub fn schedule(
        &mut self,
        task: AgentTask,
        priority: QueuePriority,
    ) -> Result<(), QueueError> {
        self.queue.enqueue(task, priority)
    }

    pub fn next_task(&mut self) -> Option<AgentTask> {
        let mut task = self.queue.dequeue()?;
        task.mark_running().ok()?;
        Some(task)
    }

    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentTaskKind, AgentTaskQueue, AgentTaskStatus};

    fn validated_task(id: &str) -> AgentTask {
        let mut task = AgentTask::new(
            id,
            AgentTaskKind::Research,
            "agent-alpha",
            "payload",
        );
        task.validate().unwrap();
        task
    }

    #[test]
    fn scheduler_starts_with_existing_queue() {
        let queue = AgentTaskQueue::new(10);
        let scheduler = AgentTaskScheduler::new(queue);

        assert!(!scheduler.has_pending());
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn schedule_adds_validated_task_to_queue() {
        let queue = AgentTaskQueue::new(10);
        let mut scheduler = AgentTaskScheduler::new(queue);

        scheduler
            .schedule(validated_task("task-1"), QueuePriority::Medium)
            .unwrap();

        assert!(scheduler.has_pending());
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn next_task_marks_task_running() {
        let queue = AgentTaskQueue::new(10);
        let mut scheduler = AgentTaskScheduler::new(queue);

        scheduler
            .schedule(validated_task("task-1"), QueuePriority::High)
            .unwrap();

        let task = scheduler.next_task().unwrap();

        assert_eq!(task.id.0, "task-1");
        assert_eq!(task.status, AgentTaskStatus::Running);
        assert!(!scheduler.has_pending());
    }

    #[test]
    fn next_task_respects_queue_priority_order() {
        let queue = AgentTaskQueue::new(10);
        let mut scheduler = AgentTaskScheduler::new(queue);

        scheduler
            .schedule(validated_task("low"), QueuePriority::Low)
            .unwrap();
        scheduler
            .schedule(validated_task("critical"), QueuePriority::Critical)
            .unwrap();

        assert_eq!(scheduler.next_task().unwrap().id.0, "critical");
        assert_eq!(scheduler.next_task().unwrap().id.0, "low");
    }
}
