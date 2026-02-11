//! # Merge Queue
//!
//! Parallel task merging and conflict resolution for OYA.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::expect_used)]

mod conflict;

pub use oya_core::{Error, Result};

/// Queue management module
pub mod queue {
    use crate::{Error, Result};
    use std::collections::VecDeque;

    /// A task waiting to be merged.
    #[derive(Debug, Clone, PartialEq, Eq)]
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
        #[must_use]
        pub const fn new() -> Self {
            Self {
                tasks: VecDeque::new(),
                capacity: 100,
            }
        }

        /// Create a new queue with specified capacity.
        #[must_use]
        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                tasks: VecDeque::with_capacity(capacity),
                capacity,
            }
        }

        /// Get the current number of tasks in the queue.
        #[must_use]
        pub fn len(&self) -> usize {
            self.tasks.len()
        }

        /// Check if the queue is empty.
        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.tasks.is_empty()
        }

        /// Get the maximum capacity of the queue.
        #[must_use]
        pub const fn capacity(&self) -> usize {
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
        #[must_use]
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
pub use conflict::{attempt_rebase, detect, ConflictDetection, RebaseResult};
pub use queue::{MergeTask, Queue};

// Include test modules
#[cfg(test)]
mod tests;
