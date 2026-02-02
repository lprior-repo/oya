//! Actor system for managing process pools.
//!
//! Provides ProcessPoolActor for tracking worker states in a process pool.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Unique identifier for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Create a new ProcessId.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the inner ID value.
    #[must_use]
    pub const fn get(&self) -> u64 {
        self.0
    }
}

/// State of a worker process in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerState {
    /// Worker is idle and available for work.
    Idle,
    /// Worker has been claimed for a task.
    Claimed,
    /// Worker is unhealthy but still running.
    Unhealthy,
    /// Worker process has terminated.
    Dead,
}

impl WorkerState {
    /// Check if the worker can accept new work.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Check if the worker needs attention (unhealthy or dead).
    #[must_use]
    pub const fn needs_attention(&self) -> bool {
        matches!(self, Self::Unhealthy | Self::Dead)
    }
}

/// Actor managing a pool of worker processes.
///
/// Tracks the state of each worker and maintains pool invariants:
/// - Each worker is in exactly one state
/// - Pool size is maintained
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPoolActor {
    /// Map of process IDs to their current states.
    workers: HashMap<ProcessId, WorkerState>,
}

impl ProcessPoolActor {
    /// Create a new empty process pool actor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    /// Create a process pool with a specified number of idle workers.
    #[must_use]
    pub fn with_capacity(size: usize) -> Self {
        let workers = (0..size)
            .map(|i| {
                let id = u64::try_from(i).unwrap_or(0);
                (ProcessId::new(id), WorkerState::Idle)
            })
            .collect();

        Self { workers }
    }

    /// Add a new worker to the pool.
    ///
    /// Returns Ok(()) if the worker was added, Err if the ID already exists.
    pub fn add_worker(&mut self, id: ProcessId, state: WorkerState) -> Result<()> {
        if self.workers.contains_key(&id) {
            return Err(crate::error::Error::InvalidRecord {
                reason: format!("worker with id {} already exists", id.get()),
            });
        }
        self.workers.insert(id, state);
        Ok(())
    }

    /// Remove a worker from the pool.
    ///
    /// Returns the worker's state if it existed, or None if not found.
    pub fn remove_worker(&mut self, id: &ProcessId) -> Option<WorkerState> {
        self.workers.remove(id)
    }

    /// Get the state of a worker.
    #[must_use]
    pub fn get_state(&self, id: &ProcessId) -> Option<WorkerState> {
        self.workers.get(id).copied()
    }

    /// Update the state of an existing worker.
    ///
    /// Returns Ok(()) if updated, Err if the worker doesn't exist.
    pub fn update_state(&mut self, id: &ProcessId, new_state: WorkerState) -> Result<()> {
        self.workers
            .get_mut(id)
            .map(|state| {
                *state = new_state;
            })
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: format!("worker with id {} not found", id.get()),
            })
    }

    /// Get the total number of workers in the pool.
    #[must_use]
    pub fn size(&self) -> usize {
        self.workers.len()
    }

    /// Count workers in a specific state.
    #[must_use]
    pub fn count_by_state(&self, target_state: WorkerState) -> usize {
        self.workers
            .values()
            .filter(|&&state| state == target_state)
            .count()
    }

    /// Get all idle workers.
    #[must_use]
    pub fn idle_workers(&self) -> Vec<ProcessId> {
        self.workers
            .iter()
            .filter_map(|(id, state)| {
                if state.is_available() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all workers needing attention (unhealthy or dead).
    #[must_use]
    pub fn workers_needing_attention(&self) -> Vec<ProcessId> {
        self.workers
            .iter()
            .filter_map(|(id, state)| {
                if state.needs_attention() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }

    /// Clear all workers from the pool.
    pub fn clear(&mut self) {
        self.workers.clear();
    }
}

impl Default for ProcessPoolActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_pool() {
        let pool = ProcessPoolActor::new();
        assert_eq!(pool.size(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_create_pool_with_capacity() {
        let pool = ProcessPoolActor::with_capacity(5);
        assert_eq!(pool.size(), 5);
        assert_eq!(pool.count_by_state(WorkerState::Idle), 5);
    }

    #[test]
    fn test_initialize_empty_worker_map() {
        let pool = ProcessPoolActor::new();
        assert!(pool.workers.is_empty());
    }

    #[test]
    fn test_add_worker() {
        let mut pool = ProcessPoolActor::new();
        let id = ProcessId::new(1);

        let result = pool.add_worker(id, WorkerState::Idle);
        assert!(result.is_ok());
        assert_eq!(pool.size(), 1);
        assert_eq!(pool.get_state(&id), Some(WorkerState::Idle));
    }

    #[test]
    fn test_add_duplicate_worker_fails() {
        let mut pool = ProcessPoolActor::new();
        let id = ProcessId::new(1);

        let first = pool.add_worker(id, WorkerState::Idle);
        assert!(first.is_ok());

        let second = pool.add_worker(id, WorkerState::Claimed);
        assert!(second.is_err());
    }

    #[test]
    fn test_remove_worker() {
        let mut pool = ProcessPoolActor::new();
        let id = ProcessId::new(1);

        pool.add_worker(id, WorkerState::Idle).ok();
        let removed = pool.remove_worker(&id);

        assert_eq!(removed, Some(WorkerState::Idle));
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_update_state() {
        let mut pool = ProcessPoolActor::new();
        let id = ProcessId::new(1);

        pool.add_worker(id, WorkerState::Idle).ok();
        let result = pool.update_state(&id, WorkerState::Claimed);

        assert!(result.is_ok());
        assert_eq!(pool.get_state(&id), Some(WorkerState::Claimed));
    }

    #[test]
    fn test_update_nonexistent_worker_fails() {
        let mut pool = ProcessPoolActor::new();
        let id = ProcessId::new(999);

        let result = pool.update_state(&id, WorkerState::Claimed);
        assert!(result.is_err());
    }

    #[test]
    fn test_count_by_state() {
        let mut pool = ProcessPoolActor::new();

        pool.add_worker(ProcessId::new(1), WorkerState::Idle).ok();
        pool.add_worker(ProcessId::new(2), WorkerState::Idle).ok();
        pool.add_worker(ProcessId::new(3), WorkerState::Claimed)
            .ok();
        pool.add_worker(ProcessId::new(4), WorkerState::Unhealthy)
            .ok();

        assert_eq!(pool.count_by_state(WorkerState::Idle), 2);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 1);
        assert_eq!(pool.count_by_state(WorkerState::Unhealthy), 1);
        assert_eq!(pool.count_by_state(WorkerState::Dead), 0);
    }

    #[test]
    fn test_idle_workers() {
        let mut pool = ProcessPoolActor::new();

        let id1 = ProcessId::new(1);
        let id2 = ProcessId::new(2);
        let id3 = ProcessId::new(3);

        pool.add_worker(id1, WorkerState::Idle).ok();
        pool.add_worker(id2, WorkerState::Claimed).ok();
        pool.add_worker(id3, WorkerState::Idle).ok();

        let idle = pool.idle_workers();
        assert_eq!(idle.len(), 2);
        assert!(idle.contains(&id1));
        assert!(idle.contains(&id3));
    }

    #[test]
    fn test_workers_needing_attention() {
        let mut pool = ProcessPoolActor::new();

        let id1 = ProcessId::new(1);
        let id2 = ProcessId::new(2);
        let id3 = ProcessId::new(3);

        pool.add_worker(id1, WorkerState::Idle).ok();
        pool.add_worker(id2, WorkerState::Unhealthy).ok();
        pool.add_worker(id3, WorkerState::Dead).ok();

        let attention = pool.workers_needing_attention();
        assert_eq!(attention.len(), 2);
        assert!(attention.contains(&id2));
        assert!(attention.contains(&id3));
    }

    #[test]
    fn test_clear() {
        let mut pool = ProcessPoolActor::with_capacity(5);
        assert_eq!(pool.size(), 5);

        pool.clear();
        assert_eq!(pool.size(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_worker_state_available() {
        assert!(WorkerState::Idle.is_available());
        assert!(!WorkerState::Claimed.is_available());
        assert!(!WorkerState::Unhealthy.is_available());
        assert!(!WorkerState::Dead.is_available());
    }

    #[test]
    fn test_worker_state_needs_attention() {
        assert!(!WorkerState::Idle.needs_attention());
        assert!(!WorkerState::Claimed.needs_attention());
        assert!(WorkerState::Unhealthy.needs_attention());
        assert!(WorkerState::Dead.needs_attention());
    }

    #[test]
    fn test_process_id_new() {
        let id = ProcessId::new(42);
        assert_eq!(id.get(), 42);
    }
}
