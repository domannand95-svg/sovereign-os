use crate::{AgentTask, AgentTaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QueuePriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueError {
    QueueFull,
    InvalidTaskState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedTaskItem {
    pub task: AgentTask,
    pub priority: QueuePriority,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTaskQueue {
    tasks: Vec<QueuedTaskItem>,
    capacity: usize,
    next_sequence: u64,
}

impl AgentTaskQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            tasks: Vec::new(),
            capacity,
            next_sequence: 0,
        }
    }

    pub fn enqueue(
        &mut self,
        mut task: AgentTask,
        priority: QueuePriority,
    ) -> Result<(), QueueError> {
        if self.tasks.len() >= self.capacity {
            return Err(QueueError::QueueFull);
        }

        if task.status != AgentTaskStatus::Validated {
            return Err(QueueError::InvalidTaskState);
        }

        task.queue().map_err(|_| QueueError::InvalidTaskState)?;

        let item = QueuedTaskItem {
            task,
            priority,
            sequence: self.next_sequence,
        };

        self.next_sequence += 1;
        self.tasks.push(item);

        self.tasks.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.sequence.cmp(&b.sequence))
        });

        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<AgentTask> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0).task)
        }
    }

    pub fn peek(&self) -> Option<&AgentTask> {
        self.tasks.first().map(|item| &item.task)
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentTaskKind, AgentTaskStatus};

    fn validated_task(id: &str) -> AgentTask {
        let mut task = AgentTask::new(
            id,
            AgentTaskKind::Research,
            "agent-alpha",
            "research-payload",
        );
        task.validate().unwrap();
        task
    }

    #[test]
    fn enqueue_requires_validated_task() {
        let mut queue = AgentTaskQueue::new(10);
        let task = AgentTask::new(
            "task-1",
            AgentTaskKind::Research,
            "agent-alpha",
            "payload",
        );

        let result = queue.enqueue(task, QueuePriority::Medium);

        assert_eq!(result, Err(QueueError::InvalidTaskState));
        assert!(queue.is_empty());
    }

    #[test]
    fn enqueue_moves_task_to_queued() {
        let mut queue = AgentTaskQueue::new(10);
        let task = validated_task("task-1");

        queue.enqueue(task, QueuePriority::Medium).unwrap();

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.peek().unwrap().status, AgentTaskStatus::Queued);
    }

    #[test]
    fn queue_enforces_capacity() {
        let mut queue = AgentTaskQueue::new(1);

        queue
            .enqueue(validated_task("task-1"), QueuePriority::Low)
            .unwrap();

        let result = queue.enqueue(validated_task("task-2"), QueuePriority::Low);

        assert_eq!(result, Err(QueueError::QueueFull));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn dequeue_returns_highest_priority_first() {
        let mut queue = AgentTaskQueue::new(10);

        queue
            .enqueue(validated_task("low"), QueuePriority::Low)
            .unwrap();
        queue
            .enqueue(validated_task("critical"), QueuePriority::Critical)
            .unwrap();
        queue
            .enqueue(validated_task("high"), QueuePriority::High)
            .unwrap();

        assert_eq!(queue.dequeue().unwrap().id.0, "critical");
        assert_eq!(queue.dequeue().unwrap().id.0, "high");
        assert_eq!(queue.dequeue().unwrap().id.0, "low");
    }

    #[test]
    fn equal_priority_preserves_fifo_order() {
        let mut queue = AgentTaskQueue::new(10);

        queue
            .enqueue(validated_task("first"), QueuePriority::High)
            .unwrap();
        queue
            .enqueue(validated_task("second"), QueuePriority::High)
            .unwrap();

        assert_eq!(queue.dequeue().unwrap().id.0, "first");
        assert_eq!(queue.dequeue().unwrap().id.0, "second");
    }
}
