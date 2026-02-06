//! Tests for queue management.
//!
//! Validates queue operations, task scheduling, and capacity limits.

use crate::queue::{Queue, MergeTask};

#[test]
fn test_queue_operations() {
    let queue = Queue::new();
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
}

#[test]
fn test_capacity_limits() {
    let queue = Queue::with_capacity(10);
    assert_eq!(queue.capacity(), 10);
}

#[test]
fn test_enqueue_task() {
    let mut queue = Queue::new();

    let task = MergeTask {
        id: "task-1".to_string(),
        branch: "feature-1".to_string(),
        target: "main".to_string(),
    };

    let result = queue.enqueue(task);
    assert!(result.is_ok());
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_dequeue_task() {
    let mut queue = Queue::new();

    let task = MergeTask {
        id: "task-1".to_string(),
        branch: "feature-1".to_string(),
        target: "main".to_string(),
    };

    queue.enqueue(task).expect("Enqueue should succeed");
    let dequeued = queue.dequeue();

    assert!(dequeued.is_some());
    assert_eq!(dequeued.unwrap().id, "task-1");
}
