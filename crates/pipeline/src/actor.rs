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

/// Time interval for health check scheduling (in seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckInterval(u64);

impl HealthCheckInterval {
    /// Create a new health check interval.
    ///
    /// # Errors
    ///
    /// Returns an error if the interval is zero or exceeds maximum allowed (3600 seconds).
    pub fn new(seconds: u64) -> Result<Self> {
        if seconds == 0 {
            return Err(crate::error::Error::InvalidRecord {
                reason: "health check interval must be greater than zero".to_string(),
            });
        }
        if seconds > 3600 {
            return Err(crate::error::Error::InvalidRecord {
                reason: "health check interval must not exceed 3600 seconds".to_string(),
            });
        }
        Ok(Self(seconds))
    }

    /// Get the interval in seconds.
    #[must_use]
    pub const fn seconds(&self) -> u64 {
        self.0
    }

    /// Default interval of 30 seconds.
    #[must_use]
    pub fn default_interval() -> Self {
        // Safety: 30 is within valid range [1, 3600]
        Self(30)
    }
}

/// Health status of a worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Worker is healthy and responsive.
    Healthy,
    /// Worker is degraded but functional.
    Degraded,
    /// Worker is unresponsive or failing health checks.
    Unhealthy,
}

impl HealthStatus {
    /// Check if the worker is in a healthy state.
    #[must_use]
    pub const fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Check if the worker requires intervention.
    #[must_use]
    pub const fn requires_intervention(&self) -> bool {
        matches!(self, Self::Unhealthy)
    }
}

/// Health check record for a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheck {
    /// The worker being checked.
    worker_id: ProcessId,
    /// Current health status.
    status: HealthStatus,
    /// Number of consecutive failures.
    consecutive_failures: u32,
}

impl HealthCheck {
    /// Create a new health check record.
    #[must_use]
    pub fn new(worker_id: ProcessId, status: HealthStatus) -> Self {
        Self {
            worker_id,
            status,
            consecutive_failures: 0,
        }
    }

    /// Get the worker ID.
    #[must_use]
    pub const fn worker_id(&self) -> ProcessId {
        self.worker_id
    }

    /// Get the current health status.
    #[must_use]
    pub const fn status(&self) -> HealthStatus {
        self.status
    }

    /// Get the number of consecutive failures.
    #[must_use]
    pub const fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Record a successful health check.
    #[must_use]
    pub fn record_success(self) -> Self {
        Self {
            worker_id: self.worker_id,
            status: HealthStatus::Healthy,
            consecutive_failures: 0,
        }
    }

    /// Record a failed health check.
    #[must_use]
    pub fn record_failure(self) -> Self {
        let new_failures = self.consecutive_failures.saturating_add(1);
        let new_status = if new_failures >= 3 {
            HealthStatus::Unhealthy
        } else if new_failures >= 1 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        Self {
            worker_id: self.worker_id,
            status: new_status,
            consecutive_failures: new_failures,
        }
    }
}

/// Actor managing health checks for worker processes.
///
/// Tracks health status and schedules periodic checks:
/// - Monitors worker health state
/// - Tracks consecutive failures
/// - Provides health status queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMonitor {
    /// Map of worker IDs to their health check records.
    health_checks: HashMap<ProcessId, HealthCheck>,
    /// Interval between health checks.
    check_interval: HealthCheckInterval,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor with default 30-second interval.
    #[must_use]
    pub fn new() -> Self {
        Self {
            health_checks: HashMap::new(),
            check_interval: HealthCheckInterval::default_interval(),
        }
    }

    /// Create a heartbeat monitor with a custom interval.
    ///
    /// # Errors
    ///
    /// Returns an error if the interval is invalid (zero or > 3600 seconds).
    pub fn with_interval(interval_seconds: u64) -> Result<Self> {
        HealthCheckInterval::new(interval_seconds).map(|interval| Self {
            health_checks: HashMap::new(),
            check_interval: interval,
        })
    }

    /// Get the configured check interval in seconds.
    #[must_use]
    pub const fn check_interval(&self) -> u64 {
        self.check_interval.seconds()
    }

    /// Register a worker for health monitoring.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker is already registered.
    pub fn register_worker(&mut self, worker_id: ProcessId) -> Result<()> {
        if self.health_checks.contains_key(&worker_id) {
            return Err(crate::error::Error::InvalidRecord {
                reason: format!(
                    "worker {} already registered for health checks",
                    worker_id.get()
                ),
            });
        }

        self.health_checks.insert(
            worker_id,
            HealthCheck::new(worker_id, HealthStatus::Healthy),
        );
        Ok(())
    }

    /// Unregister a worker from health monitoring.
    ///
    /// Returns the final health check record if the worker was registered.
    pub fn unregister_worker(&mut self, worker_id: &ProcessId) -> Option<HealthCheck> {
        self.health_checks.remove(worker_id)
    }

    /// Get the health status of a worker.
    #[must_use]
    pub fn get_health_status(&self, worker_id: &ProcessId) -> Option<HealthStatus> {
        self.health_checks.get(worker_id).map(|check| check.status)
    }

    /// Get the full health check record for a worker.
    #[must_use]
    pub fn get_health_check(&self, worker_id: &ProcessId) -> Option<&HealthCheck> {
        self.health_checks.get(worker_id)
    }

    /// Record a successful health check for a worker.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker is not registered.
    pub fn record_success(&mut self, worker_id: &ProcessId) -> Result<()> {
        self.health_checks
            .get(worker_id)
            .map(|check| check.clone().record_success())
            .map(|updated_check| {
                self.health_checks.insert(*worker_id, updated_check);
            })
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: format!(
                    "worker {} not registered for health checks",
                    worker_id.get()
                ),
            })
    }

    /// Record a failed health check for a worker.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker is not registered.
    pub fn record_failure(&mut self, worker_id: &ProcessId) -> Result<()> {
        self.health_checks
            .get(worker_id)
            .map(|check| check.clone().record_failure())
            .map(|updated_check| {
                self.health_checks.insert(*worker_id, updated_check);
            })
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: format!(
                    "worker {} not registered for health checks",
                    worker_id.get()
                ),
            })
    }

    /// Get all workers currently registered for health monitoring.
    #[must_use]
    pub fn registered_workers(&self) -> Vec<ProcessId> {
        self.health_checks.keys().copied().collect()
    }

    /// Get all healthy workers.
    #[must_use]
    pub fn healthy_workers(&self) -> Vec<ProcessId> {
        self.health_checks
            .iter()
            .filter_map(|(id, check)| {
                if check.status.is_healthy() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all unhealthy workers requiring intervention.
    #[must_use]
    pub fn unhealthy_workers(&self) -> Vec<ProcessId> {
        self.health_checks
            .iter()
            .filter_map(|(id, check)| {
                if check.status.requires_intervention() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Count workers by health status.
    #[must_use]
    pub fn count_by_status(&self, target_status: HealthStatus) -> usize {
        self.health_checks
            .values()
            .filter(|check| check.status == target_status)
            .count()
    }

    /// Get the total number of registered workers.
    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.health_checks.len()
    }

    /// Check if the monitor is empty (no registered workers).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.health_checks.is_empty()
    }

    /// Clear all health check records.
    pub fn clear(&mut self) {
        self.health_checks.clear();
    }
}

impl Default for HeartbeatMonitor {
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

    // === HeartbeatMonitor Tests ===

    #[test]
    fn test_create_heartbeat_monitor() {
        let monitor = HeartbeatMonitor::new();
        assert_eq!(monitor.worker_count(), 0);
        assert!(monitor.is_empty());
        assert_eq!(monitor.check_interval(), 30);
    }

    #[test]
    fn test_create_monitor_with_custom_interval() {
        let monitor = HeartbeatMonitor::with_interval(60);
        assert!(monitor.is_ok());
        assert_eq!(monitor.unwrap().check_interval(), 60);
    }

    #[test]
    fn test_create_monitor_with_zero_interval_fails() {
        let monitor = HeartbeatMonitor::with_interval(0);
        assert!(monitor.is_err());
    }

    #[test]
    fn test_create_monitor_with_excessive_interval_fails() {
        let monitor = HeartbeatMonitor::with_interval(3601);
        assert!(monitor.is_err());
    }

    #[test]
    fn test_register_worker() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        let result = monitor.register_worker(worker_id);
        assert!(result.is_ok());
        assert_eq!(monitor.worker_count(), 1);
        assert_eq!(
            monitor.get_health_status(&worker_id),
            Some(HealthStatus::Healthy)
        );
    }

    #[test]
    fn test_register_duplicate_worker_fails() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        assert!(monitor.register_worker(worker_id).is_ok());
        assert!(monitor.register_worker(worker_id).is_err());
    }

    #[test]
    fn test_unregister_worker() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        monitor.register_worker(worker_id).ok();
        let removed = monitor.unregister_worker(&worker_id);

        assert!(removed.is_some());
        assert_eq!(monitor.worker_count(), 0);
        assert!(monitor.get_health_status(&worker_id).is_none());
    }

    #[test]
    fn test_record_success() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        monitor.register_worker(worker_id).ok();
        let result = monitor.record_success(&worker_id);

        assert!(result.is_ok());
        assert_eq!(
            monitor.get_health_status(&worker_id),
            Some(HealthStatus::Healthy)
        );
    }

    #[test]
    fn test_record_failure_degrades_health() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        monitor.register_worker(worker_id).ok();
        monitor.record_failure(&worker_id).ok();

        let check = monitor.get_health_check(&worker_id);
        assert!(check.is_some());
        assert_eq!(check.unwrap().status(), HealthStatus::Degraded);
        assert_eq!(check.unwrap().consecutive_failures(), 1);
    }

    #[test]
    fn test_three_failures_makes_unhealthy() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        monitor.register_worker(worker_id).ok();
        monitor.record_failure(&worker_id).ok();
        monitor.record_failure(&worker_id).ok();
        monitor.record_failure(&worker_id).ok();

        let check = monitor.get_health_check(&worker_id);
        assert!(check.is_some());
        assert_eq!(check.unwrap().status(), HealthStatus::Unhealthy);
        assert_eq!(check.unwrap().consecutive_failures(), 3);
    }

    #[test]
    fn test_success_resets_failures() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(1);

        monitor.register_worker(worker_id).ok();
        monitor.record_failure(&worker_id).ok();
        monitor.record_failure(&worker_id).ok();
        monitor.record_success(&worker_id).ok();

        let check = monitor.get_health_check(&worker_id);
        assert!(check.is_some());
        assert_eq!(check.unwrap().status(), HealthStatus::Healthy);
        assert_eq!(check.unwrap().consecutive_failures(), 0);
    }

    #[test]
    fn test_record_success_for_unregistered_worker_fails() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(999);

        let result = monitor.record_success(&worker_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_failure_for_unregistered_worker_fails() {
        let mut monitor = HeartbeatMonitor::new();
        let worker_id = ProcessId::new(999);

        let result = monitor.record_failure(&worker_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_healthy_workers() {
        let mut monitor = HeartbeatMonitor::new();
        let id1 = ProcessId::new(1);
        let id2 = ProcessId::new(2);
        let id3 = ProcessId::new(3);

        monitor.register_worker(id1).ok();
        monitor.register_worker(id2).ok();
        monitor.register_worker(id3).ok();

        monitor.record_failure(&id2).ok();
        monitor.record_failure(&id2).ok();
        monitor.record_failure(&id2).ok();

        let healthy = monitor.healthy_workers();
        assert_eq!(healthy.len(), 2);
        assert!(healthy.contains(&id1));
        assert!(healthy.contains(&id3));
    }

    #[test]
    fn test_unhealthy_workers() {
        let mut monitor = HeartbeatMonitor::new();
        let id1 = ProcessId::new(1);
        let id2 = ProcessId::new(2);
        let id3 = ProcessId::new(3);

        monitor.register_worker(id1).ok();
        monitor.register_worker(id2).ok();
        monitor.register_worker(id3).ok();

        monitor.record_failure(&id2).ok();
        monitor.record_failure(&id2).ok();
        monitor.record_failure(&id2).ok();

        let unhealthy = monitor.unhealthy_workers();
        assert_eq!(unhealthy.len(), 1);
        assert!(unhealthy.contains(&id2));
    }

    #[test]
    fn test_count_by_status() {
        let mut monitor = HeartbeatMonitor::new();

        monitor.register_worker(ProcessId::new(1)).ok();
        monitor.register_worker(ProcessId::new(2)).ok();
        monitor.register_worker(ProcessId::new(3)).ok();

        monitor.record_failure(&ProcessId::new(2)).ok();
        monitor.record_failure(&ProcessId::new(3)).ok();
        monitor.record_failure(&ProcessId::new(3)).ok();
        monitor.record_failure(&ProcessId::new(3)).ok();

        assert_eq!(monitor.count_by_status(HealthStatus::Healthy), 1);
        assert_eq!(monitor.count_by_status(HealthStatus::Degraded), 1);
        assert_eq!(monitor.count_by_status(HealthStatus::Unhealthy), 1);
    }

    #[test]
    fn test_registered_workers() {
        let mut monitor = HeartbeatMonitor::new();
        let id1 = ProcessId::new(1);
        let id2 = ProcessId::new(2);

        monitor.register_worker(id1).ok();
        monitor.register_worker(id2).ok();

        let workers = monitor.registered_workers();
        assert_eq!(workers.len(), 2);
        assert!(workers.contains(&id1));
        assert!(workers.contains(&id2));
    }

    #[test]
    fn test_clear_monitor() {
        let mut monitor = HeartbeatMonitor::new();

        monitor.register_worker(ProcessId::new(1)).ok();
        monitor.register_worker(ProcessId::new(2)).ok();
        assert_eq!(monitor.worker_count(), 2);

        monitor.clear();
        assert_eq!(monitor.worker_count(), 0);
        assert!(monitor.is_empty());
    }

    #[test]
    fn test_health_check_interval_validation() {
        assert!(HealthCheckInterval::new(1).is_ok());
        assert!(HealthCheckInterval::new(30).is_ok());
        assert!(HealthCheckInterval::new(3600).is_ok());
        assert!(HealthCheckInterval::new(0).is_err());
        assert!(HealthCheckInterval::new(3601).is_err());
    }

    #[test]
    fn test_health_status_checks() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Degraded.is_healthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());

        assert!(!HealthStatus::Healthy.requires_intervention());
        assert!(!HealthStatus::Degraded.requires_intervention());
        assert!(HealthStatus::Unhealthy.requires_intervention());
    }

    #[test]
    fn test_health_check_record_creation() {
        let worker_id = ProcessId::new(1);
        let check = HealthCheck::new(worker_id, HealthStatus::Healthy);

        assert_eq!(check.worker_id(), worker_id);
        assert_eq!(check.status(), HealthStatus::Healthy);
        assert_eq!(check.consecutive_failures(), 0);
    }

    #[test]
    fn test_health_check_failure_progression() {
        let worker_id = ProcessId::new(1);
        let check = HealthCheck::new(worker_id, HealthStatus::Healthy);

        let check = check.record_failure();
        assert_eq!(check.status(), HealthStatus::Degraded);
        assert_eq!(check.consecutive_failures(), 1);

        let check = check.record_failure();
        assert_eq!(check.status(), HealthStatus::Degraded);
        assert_eq!(check.consecutive_failures(), 2);

        let check = check.record_failure();
        assert_eq!(check.status(), HealthStatus::Unhealthy);
        assert_eq!(check.consecutive_failures(), 3);
    }

    #[test]
    fn test_default_heartbeat_monitor() {
        let monitor = HeartbeatMonitor::default();
        assert!(monitor.is_empty());
        assert_eq!(monitor.check_interval(), 30);
    }
}
