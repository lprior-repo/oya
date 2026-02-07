//! # Merge Queue
//!
//! Parallel task merging and conflict resolution for OYA.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

mod conflict;

pub use oya_core::{Error, Result};

/// Queue management module
pub mod queue {
    use crate::{Error, Result};
    use std::collections::VecDeque;

    /// A task waiting to be merged.
    #[derive(Debug, Clone, PartialEq)]
    pub struct MergeTask {
        /// Unique task identifier
        pub id: String,
        /// Source branch to merge
        pub branch: String,
        /// Target branch for merge
        pub target: String,
    }

    /// A queue of merge tasks with conflict detection.
    #[derive(Debug)]
    pub struct Queue {
        tasks: VecDeque<MergeTask>,
        capacity: usize,
    }

    impl Queue {
        /// Create a new empty queue with default capacity.
        pub fn new() -> Self {
            Self {
                tasks: VecDeque::new(),
                capacity: 100,
            }
        }

        /// Create a new queue with specified capacity.
        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                tasks: VecDeque::with_capacity(capacity),
                capacity,
            }
        }

        /// Get the current number of tasks in the queue.
        pub fn len(&self) -> usize {
            self.tasks.len()
        }

        /// Check if the queue is empty.
        pub fn is_empty(&self) -> bool {
            self.tasks.is_empty()
        }

        /// Get the maximum capacity of the queue.
        pub fn capacity(&self) -> usize {
            self.capacity
        }

        /// Add a task to the back of the queue.
        pub fn enqueue(&mut self, task: MergeTask) -> Result<()> {
            if self.tasks.len() >= self.capacity {
                return Err(Error::invalid_record("Queue capacity exceeded"));
            }
            self.tasks.push_back(task);
            Ok(())
        }

        /// Remove and return the next task from the front of the queue.
        pub fn dequeue(&mut self) -> Option<MergeTask> {
            self.tasks.pop_front()
        }

        /// Peek at the next task without removing it.
        pub fn peek(&self) -> Option<&MergeTask> {
            self.tasks.front()
        }
    }

    impl Default for Queue {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Conflict resolution module
pub use conflict::{ConflictDetection, RebaseResult, attempt_rebase, detect};

// Include test modules
#[cfg(test)]
mod tests;
