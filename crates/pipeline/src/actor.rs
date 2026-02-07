//! Actor system for managing process pools.
//!
//! Provides ProcessPoolActor for tracking worker states in a process pool.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::sync::OnceLock;

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

/// Cached statistics for worker pool queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerStats {
    /// Total number of workers in the pool.
    pub total: usize,
    /// Number of available (Idle) workers.
    pub available: usize,
    /// Number of busy (Claimed) workers.
    pub busy: usize,
    /// Number of workers needing attention (Unhealthy or Dead).
    pub needing_attention: usize,
}

impl WorkerStats {
    /// Create new worker statistics.
    #[must_use]
    pub fn new(total: usize, available: usize, busy: usize, needing_attention: usize) -> Self {
        Self {
            total,
            available,
            busy,
            needing_attention,
        }
    }

    /// Create empty statistics (zero values).
    #[must_use]
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

/// Actor managing a pool of worker processes.
///
/// Tracks the state of each worker and maintains pool invariants:
/// - Each worker is in exactly one state
/// - Pool size is maintained
/// - Provides lazy evaluation caching for frequent queries
#[derive(Debug, Clone)]
pub struct ProcessPoolActor {
    /// Map of process IDs to their current states.
    workers: HashMap<ProcessId, WorkerState>,
    /// Cached statistics for worker queries (lazily computed).
    query_cache: OnceLock<WorkerStats>,
}

impl ProcessPoolActor {
    /// Create a new empty process pool actor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            query_cache: OnceLock::new(),
        }
    }

    /// Create a process pool with a specified number of idle workers.
    #[must_use]
    pub fn with_capacity(size: usize) -> Self {
        let workers = (0..size)
            .filter_map(|i| u64::try_from(i).ok())
            .map(|id| (ProcessId::new(id), WorkerState::Idle))
            .collect();

        Self {
            workers,
            query_cache: OnceLock::new(),
        }
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
        self.invalidate_stats_cache();
        Ok(())
    }

    /// Remove a worker from the pool.
    ///
    /// Returns the worker's state if it existed, or None if not found.
    pub fn remove_worker(&mut self, id: &ProcessId) -> Option<WorkerState> {
        let result = self.workers.remove(id);
        if result.is_some() {
            self.invalidate_stats_cache();
        }
        result
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
            })?;
        self.invalidate_stats_cache();
        Ok(())
    }

    /// Get the total number of workers in the pool.
    #[must_use]
    pub fn size(&self) -> usize {
        self.get_stats().total
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

    /// Get cached worker statistics (lazily computed).
    #[must_use]
    pub fn get_stats(&self) -> &WorkerStats {
        self.query_cache.get_or_init(|| WorkerStats {
            total: self.workers.len(),
            available: self
                .workers
                .values()
                .filter(|&&state| state.is_available())
                .count(),
            busy: self
                .workers
                .values()
                .filter(|&&state| !state.is_available() && !state.needs_attention())
                .count(),
            needing_attention: self
                .workers
                .values()
                .filter(|&&state| state.needs_attention())
                .count(),
        })
    }

    /// Invalidate the statistics cache.
    /// Call this when worker states change.
    pub fn invalidate_stats_cache(&mut self) {
        let _ = self.query_cache.take();
    }

    /// Get the number of available workers (using cache).
    #[must_use]
    pub fn available_count(&self) -> usize {
        self.get_stats().available
    }

    /// Get the number of busy workers (using cache).
    #[must_use]
    pub fn busy_count(&self) -> usize {
        self.get_stats().busy
    }

    /// Get the number of workers needing attention (using cache).
    #[must_use]
    pub fn needing_attention_count(&self) -> usize {
        self.get_stats().needing_attention
    }

    /// Claim a worker for use, transitioning it from Idle to Claimed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The worker doesn't exist
    /// - The worker is not in Idle state (already claimed or unavailable)
    pub fn claim_worker(&mut self, id: ProcessId) -> Result<()> {
        self.workers
            .get(&id)
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: format!("worker with id {} not found", id.get()),
            })
            .and_then(|&state| {
                if state == WorkerState::Idle {
                    Ok(())
                } else {
                    Err(crate::error::Error::InvalidRecord {
                        reason: format!(
                            "worker {} is not idle (current state: {:?})",
                            id.get(),
                            state
                        ),
                    })
                }
            })
            .map(|_| {
                self.workers.insert(id, WorkerState::Claimed);
                self.invalidate_stats_cache();
            })
    }

    /// Release a worker back to the pool, transitioning it from Claimed to Idle.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The worker doesn't exist
    /// - The worker is not in Claimed state (already idle or in other state)
    pub fn release_worker(&mut self, id: ProcessId) -> Result<()> {
        self.workers
            .get(&id)
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: format!("worker with id {} not found", id.get()),
            })
            .and_then(|&state| {
                if state == WorkerState::Claimed {
                    Ok(())
                } else {
                    Err(crate::error::Error::InvalidRecord {
                        reason: format!(
                            "worker {} is not claimed (current state: {:?})",
                            id.get(),
                            state
                        ),
                    })
                }
            })
            .map(|_| {
                self.workers.insert(id, WorkerState::Idle);
                self.invalidate_stats_cache();
            })
    }

    /// Clear all workers from the pool.
    pub fn clear(&mut self) {
        self.workers.clear();
        self.invalidate_stats_cache();
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
        if let Ok(mon) = monitor {
            assert_eq!(mon.check_interval(), 60);
        }
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
        if let Some(c) = check {
            assert_eq!(c.status(), HealthStatus::Degraded);
            assert_eq!(c.consecutive_failures(), 1);
        }
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
        if let Some(c) = check {
            assert_eq!(c.status(), HealthStatus::Unhealthy);
            assert_eq!(c.consecutive_failures(), 3);
        }
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
        if let Some(c) = check {
            assert_eq!(c.status(), HealthStatus::Healthy);
            assert_eq!(c.consecutive_failures(), 0);
        }
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

    // === Behavioral Tests (Martin Fowler style) ===
    //
    // These tests document BEHAVIOR, not implementation.
    // They should survive refactoring as long as behavior stays the same.
    // Each test has a clear Given/When/Then structure.

    #[test]
    fn should_track_worker_count_after_spawning() {
        // Given: An empty process pool
        let mut pool = ProcessPoolActor::new();
        assert_eq!(pool.size(), 0);

        // When: We spawn 3 workers
        let worker1 = ProcessId::new(1);
        let worker2 = ProcessId::new(2);
        let worker3 = ProcessId::new(3);

        let result1 = pool.add_worker(worker1, WorkerState::Idle);
        let result2 = pool.add_worker(worker2, WorkerState::Idle);
        let result3 = pool.add_worker(worker3, WorkerState::Idle);

        // Then: All additions succeed and pool tracks correct count
        assert!(result1.is_ok(), "First worker addition failed");
        assert!(result2.is_ok(), "Second worker addition failed");
        assert!(result3.is_ok(), "Third worker addition failed");
        assert_eq!(pool.size(), 3, "Pool should contain exactly 3 workers");
        assert_eq!(
            pool.count_by_state(WorkerState::Idle),
            3,
            "All workers should be in Idle state"
        );
    }

    #[test]
    fn should_release_worker_back_to_pool() {
        // Given: A pool with a claimed worker
        let mut pool = ProcessPoolActor::new();
        let worker_id = ProcessId::new(1);
        let add_result = pool.add_worker(worker_id, WorkerState::Claimed);
        assert!(add_result.is_ok(), "Failed to add worker");

        assert_eq!(
            pool.get_state(&worker_id),
            Some(WorkerState::Claimed),
            "Worker should start as claimed"
        );

        // When: We release the worker back to idle
        let result = pool.update_state(&worker_id, WorkerState::Idle);

        // Then: Worker transitions to idle state
        assert!(result.is_ok(), "Failed to update worker state");
        assert_eq!(
            pool.get_state(&worker_id),
            Some(WorkerState::Idle),
            "Worker should be idle after release"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Claimed),
            0,
            "No workers should be claimed"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Idle),
            1,
            "One worker should be idle"
        );
    }

    #[test]
    fn should_reject_spawn_when_pool_full() {
        // Given: A pool with maximum capacity (simulate by adding a worker with specific ID)
        let mut pool = ProcessPoolActor::new();
        let worker_id = ProcessId::new(1);
        let add_result = pool.add_worker(worker_id, WorkerState::Idle);
        assert!(add_result.is_ok(), "Failed to add initial worker");

        // When: We attempt to spawn a worker with the same ID
        let result = pool.add_worker(worker_id, WorkerState::Idle);

        // Then: The spawn is rejected with an error
        assert!(result.is_err(), "Should reject duplicate worker ID");
        assert_eq!(
            pool.size(),
            1,
            "Pool size should remain unchanged after rejection"
        );
    }

    #[test]
    fn should_claim_available_worker() {
        // Given: A pool with multiple idle workers
        let mut pool = ProcessPoolActor::new();
        let worker1 = ProcessId::new(1);
        let worker2 = ProcessId::new(2);

        let add1_result = pool.add_worker(worker1, WorkerState::Idle);
        assert!(add1_result.is_ok(), "Failed to add worker 1");
        let add2_result = pool.add_worker(worker2, WorkerState::Idle);
        assert!(add2_result.is_ok(), "Failed to add worker 2");

        let idle_before = pool.idle_workers();
        assert_eq!(idle_before.len(), 2, "Should have 2 idle workers initially");

        // When: We claim one of the idle workers
        let result = pool.update_state(&worker1, WorkerState::Claimed);

        // Then: The worker is successfully claimed and removed from idle list
        assert!(result.is_ok(), "Failed to claim worker");
        assert_eq!(
            pool.get_state(&worker1),
            Some(WorkerState::Claimed),
            "Worker should be in claimed state"
        );

        let idle_after = pool.idle_workers();
        assert_eq!(
            idle_after.len(),
            1,
            "Should have 1 idle worker after claiming"
        );
        assert!(
            idle_after.contains(&worker2),
            "Unclaimed worker should still be idle"
        );
        assert!(
            !idle_after.contains(&worker1),
            "Claimed worker should not appear in idle list"
        );
    }

    #[test]
    fn should_return_error_when_no_workers_available() {
        // Given: An empty pool with no workers
        let mut pool = ProcessPoolActor::new();
        assert!(pool.is_empty(), "Pool should be empty");
        assert_eq!(pool.idle_workers().len(), 0, "No idle workers available");

        // When: We attempt to update a non-existent worker
        let nonexistent_worker = ProcessId::new(999);
        let result = pool.update_state(&nonexistent_worker, WorkerState::Claimed);

        // Then: The operation fails with an appropriate error
        assert!(
            result.is_err(),
            "Should return error when worker doesn't exist"
        );
        assert!(
            pool.is_empty(),
            "Pool should remain empty after failed update"
        );
    }

    #[test]
    fn should_identify_workers_needing_attention() {
        // Given: A pool with workers in various states
        let mut pool = ProcessPoolActor::new();
        let healthy_worker = ProcessId::new(1);
        let unhealthy_worker = ProcessId::new(2);
        let dead_worker = ProcessId::new(3);
        let claimed_worker = ProcessId::new(4);

        let add1 = pool.add_worker(healthy_worker, WorkerState::Idle);
        assert!(add1.is_ok(), "Failed to add healthy worker");
        let add2 = pool.add_worker(unhealthy_worker, WorkerState::Unhealthy);
        assert!(add2.is_ok(), "Failed to add unhealthy worker");
        let add3 = pool.add_worker(dead_worker, WorkerState::Dead);
        assert!(add3.is_ok(), "Failed to add dead worker");
        let add4 = pool.add_worker(claimed_worker, WorkerState::Claimed);
        assert!(add4.is_ok(), "Failed to add claimed worker");

        // When: We query for workers needing attention
        let attention_list = pool.workers_needing_attention();

        // Then: Only unhealthy and dead workers are identified
        assert_eq!(
            attention_list.len(),
            2,
            "Should identify 2 workers needing attention"
        );
        assert!(
            attention_list.contains(&unhealthy_worker),
            "Unhealthy worker should need attention"
        );
        assert!(
            attention_list.contains(&dead_worker),
            "Dead worker should need attention"
        );
        assert!(
            !attention_list.contains(&healthy_worker),
            "Healthy worker should not need attention"
        );
        assert!(
            !attention_list.contains(&claimed_worker),
            "Claimed worker should not need attention"
        );
    }

    #[test]
    fn should_maintain_correct_state_counts_during_lifecycle() {
        // Given: A pool with workers transitioning through states
        let mut pool = ProcessPoolActor::with_capacity(3);

        // Initially all idle
        assert_eq!(pool.count_by_state(WorkerState::Idle), 3);

        // When: Workers go through a typical lifecycle
        let worker1 = ProcessId::new(0);
        let worker2 = ProcessId::new(1);

        // Claim two workers
        let claim1 = pool.update_state(&worker1, WorkerState::Claimed);
        assert!(claim1.is_ok(), "Failed to claim worker1");
        let claim2 = pool.update_state(&worker2, WorkerState::Claimed);
        assert!(claim2.is_ok(), "Failed to claim worker2");

        // Then: State counts are accurate
        assert_eq!(
            pool.count_by_state(WorkerState::Idle),
            1,
            "Should have 1 idle worker"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Claimed),
            2,
            "Should have 2 claimed workers"
        );

        // When: One worker becomes unhealthy
        let unhealthy_result = pool.update_state(&worker1, WorkerState::Unhealthy);
        assert!(unhealthy_result.is_ok(), "Failed to mark worker1 unhealthy");

        // Then: State counts reflect the change
        assert_eq!(
            pool.count_by_state(WorkerState::Claimed),
            1,
            "Should have 1 claimed worker"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Unhealthy),
            1,
            "Should have 1 unhealthy worker"
        );

        // When: Unhealthy worker is removed
        let removed_state = pool.remove_worker(&worker1);

        // Then: Worker is removed with correct final state
        assert_eq!(
            removed_state,
            Some(WorkerState::Unhealthy),
            "Should return worker's final state"
        );
        assert_eq!(pool.size(), 2, "Pool should have 2 workers remaining");
        assert_eq!(
            pool.count_by_state(WorkerState::Unhealthy),
            0,
            "No unhealthy workers should remain"
        );
    }

    // === Claim/Release Worker Tests ===

    #[test]
    fn claim_idle_worker_should_transition_to_claimed() {
        // Given: A pool with an idle worker
        let mut pool = ProcessPoolActor::new();
        let worker_id = ProcessId::new(1);
        if pool.add_worker(worker_id, WorkerState::Idle).is_err() {
            return;
        }

        assert_eq!(
            pool.get_state(&worker_id),
            Some(WorkerState::Idle),
            "Worker should start as idle"
        );

        // When: We claim the worker
        let result = pool.claim_worker(worker_id);

        // Then: Worker transitions to claimed state
        assert!(result.is_ok(), "Claim should succeed");
        assert_eq!(
            pool.get_state(&worker_id),
            Some(WorkerState::Claimed),
            "Worker should be claimed"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Idle),
            0,
            "No idle workers should remain"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Claimed),
            1,
            "One worker should be claimed"
        );
    }

    #[test]
    fn claim_already_claimed_worker_should_fail() {
        // Given: A pool with a claimed worker
        let mut pool = ProcessPoolActor::new();
        let worker_id = ProcessId::new(1);
        if pool.add_worker(worker_id, WorkerState::Claimed).is_err() {
            return;
        }

        // When: We attempt to claim an already claimed worker
        let result = pool.claim_worker(worker_id);

        // Then: Claim fails with appropriate error
        assert!(result.is_err(), "Claim should fail");
        let error_matches = result
            .map(|_| false)
            .unwrap_or_else(|err| matches!(err, crate::error::Error::InvalidRecord { .. }));
        assert!(error_matches, "Should return InvalidRecord error");
    }

    #[test]
    fn claim_nonexistent_worker_should_fail() {
        // Given: An empty pool
        let mut pool = ProcessPoolActor::new();
        let nonexistent_worker = ProcessId::new(999);

        // When: We attempt to claim a nonexistent worker
        let result = pool.claim_worker(nonexistent_worker);

        // Then: Claim fails with appropriate error
        assert!(result.is_err(), "Claim should fail");
    }

    #[test]
    fn release_claimed_worker_should_transition_to_idle() {
        // Given: A pool with a claimed worker
        let mut pool = ProcessPoolActor::new();
        let worker_id = ProcessId::new(1);
        if pool.add_worker(worker_id, WorkerState::Claimed).is_err() {
            return;
        }

        assert_eq!(
            pool.get_state(&worker_id),
            Some(WorkerState::Claimed),
            "Worker should start as claimed"
        );

        // When: We release the worker
        let result = pool.release_worker(worker_id);

        // Then: Worker transitions to idle state
        assert!(result.is_ok(), "Release should succeed");
        assert_eq!(
            pool.get_state(&worker_id),
            Some(WorkerState::Idle),
            "Worker should be idle after release"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Claimed),
            0,
            "No claimed workers should remain"
        );
        assert_eq!(
            pool.count_by_state(WorkerState::Idle),
            1,
            "One worker should be idle"
        );
    }

    #[test]
    fn release_already_idle_worker_should_fail() {
        // Given: A pool with an idle worker
        let mut pool = ProcessPoolActor::new();
        let worker_id = ProcessId::new(1);
        if pool.add_worker(worker_id, WorkerState::Idle).is_err() {
            return;
        }

        // When: We attempt to release an already idle worker
        let result = pool.release_worker(worker_id);

        // Then: Release fails with appropriate error
        assert!(result.is_err(), "Release should fail");
        let error_matches = result
            .map(|_| false)
            .unwrap_or_else(|err| matches!(err, crate::error::Error::InvalidRecord { .. }));
        assert!(error_matches, "Should return InvalidRecord error");
    }

    #[test]
    fn release_nonexistent_worker_should_fail() {
        // Given: An empty pool
        let mut pool = ProcessPoolActor::new();
        let nonexistent_worker = ProcessId::new(999);

        // When: We attempt to release a nonexistent worker
        let result = pool.release_worker(nonexistent_worker);

        // Then: Release fails with appropriate error
        assert!(result.is_err(), "Release should fail");
    }

    #[test]
    fn sequential_claim_release_should_maintain_invariants() {
        // Given: A pool with multiple workers
        let mut pool = ProcessPoolActor::with_capacity(3);
        let worker1 = ProcessId::new(0);
        let worker2 = ProcessId::new(1);
        let worker3 = ProcessId::new(2);

        // Initial state: all idle
        assert_eq!(pool.count_by_state(WorkerState::Idle), 3);

        // When: We claim and release workers sequentially
        let claim1 = pool.claim_worker(worker1);
        assert!(claim1.is_ok(), "First claim should succeed");
        assert_eq!(pool.count_by_state(WorkerState::Idle), 2);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 1);

        let claim2 = pool.claim_worker(worker2);
        assert!(claim2.is_ok(), "Second claim should succeed");
        assert_eq!(pool.count_by_state(WorkerState::Idle), 1);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 2);

        let release1 = pool.release_worker(worker1);
        assert!(release1.is_ok(), "First release should succeed");
        assert_eq!(pool.count_by_state(WorkerState::Idle), 2);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 1);

        let claim3 = pool.claim_worker(worker3);
        assert!(claim3.is_ok(), "Third claim should succeed");
        assert_eq!(pool.count_by_state(WorkerState::Idle), 1);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 2);

        let release2 = pool.release_worker(worker2);
        assert!(release2.is_ok(), "Second release should succeed");
        assert_eq!(pool.count_by_state(WorkerState::Idle), 2);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 1);

        let release3 = pool.release_worker(worker3);
        assert!(release3.is_ok(), "Third release should succeed");
        assert_eq!(pool.count_by_state(WorkerState::Idle), 3);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 0);

        // Then: Pool invariants are maintained
        assert_eq!(pool.size(), 3, "Pool size should remain constant");
    }

    // === Cache Tests ===

    #[test]
    fn test_cache_initialization() {
        let pool = ProcessPoolActor::new();

        // Cache should be empty initially
        assert_eq!(pool.size(), 0);
        assert_eq!(pool.available_count(), 0);
        assert_eq!(pool.busy_count(), 0);
        assert_eq!(pool.needing_attention_count(), 0);
    }

    #[test]
    fn test_cache_populates_on_first_access() -> Result<()> {
        let mut pool = ProcessPoolActor::new();

        // Add some workers
        pool.add_worker(ProcessId::new(1), WorkerState::Idle)?;
        pool.add_worker(ProcessId::new(2), WorkerState::Claimed)?;
        pool.add_worker(ProcessId::new(3), WorkerState::Unhealthy)?;

        // First access should populate cache
        let stats = pool.get_stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.available, 1); // Only worker 1 is Idle
        assert_eq!(stats.busy, 1); // Only worker 2 is Claimed
        assert_eq!(stats.needing_attention, 1); // Worker 3 is Unhealthy

        // Subsequent accesses should use cache
        assert_eq!(pool.available_count(), 1);
        assert_eq!(pool.busy_count(), 1);
        assert_eq!(pool.needing_attention_count(), 1);
        Ok(())
    }

    #[test]
    fn test_cache_invalidation_on_worker_add() -> Result<()> {
        let mut pool = ProcessPoolActor::new();

        // Add initial worker
        pool.add_worker(ProcessId::new(1), WorkerState::Idle)?;

        // Access cache to populate it
        let _stats = pool.get_stats();
        assert_eq!(pool.available_count(), 1);

        // Add new worker - should invalidate cache
        pool.add_worker(ProcessId::new(2), WorkerState::Claimed)?;

        // Cache should be repopulated
        assert_eq!(pool.size(), 2);
        assert_eq!(pool.available_count(), 1); // Only worker 1 is Idle
        assert_eq!(pool.busy_count(), 1); // Only worker 2 is Claimed
        Ok(())
    }

    #[test]
    fn test_cache_invalidation_on_worker_remove() -> Result<()> {
        let mut pool = ProcessPoolActor::new();

        // Add workers
        pool.add_worker(ProcessId::new(1), WorkerState::Idle)?;
        pool.add_worker(ProcessId::new(2), WorkerState::Claimed)?;

        // Access cache to populate it
        let _stats = pool.get_stats();
        assert_eq!(pool.available_count(), 1);

        // Remove worker - should invalidate cache
        let removed = pool.remove_worker(&ProcessId::new(1));
        assert_eq!(removed, Some(WorkerState::Idle));

        // Cache should be repopulated
        assert_eq!(pool.size(), 1);
        assert_eq!(pool.available_count(), 0); // No Idle workers remaining
        assert_eq!(pool.busy_count(), 1); // Worker 2 is still Claimed
        Ok(())
    }

    #[test]
    fn test_cache_invalidation_on_state_update() -> Result<()> {
        let mut pool = ProcessPoolActor::new();

        // Add idle worker
        pool.add_worker(ProcessId::new(1), WorkerState::Idle)?;

        // Access cache to populate it
        let _stats = pool.get_stats();
        assert_eq!(pool.available_count(), 1);
        assert_eq!(pool.busy_count(), 0);

        // Update state to claimed - should invalidate cache
        pool.update_state(&ProcessId::new(1), WorkerState::Claimed)?;

        // Cache should be repopulated
        assert_eq!(pool.available_count(), 0); // No Idle workers
        assert_eq!(pool.busy_count(), 1); // Worker 1 is now busy
        Ok(())
    }

    #[test]
    fn test_cache_consistency_with_count_by_state() -> Result<()> {
        let mut pool = ProcessPoolActor::new();

        // Add workers in various states
        pool.add_worker(ProcessId::new(1), WorkerState::Idle)?;
        pool.add_worker(ProcessId::new(2), WorkerState::Idle)?;
        pool.add_worker(ProcessId::new(3), WorkerState::Claimed)?;
        pool.add_worker(ProcessId::new(4), WorkerState::Unhealthy)?;
        pool.add_worker(ProcessId::new(5), WorkerState::Dead)?;

        // Use cached methods
        assert_eq!(pool.available_count(), 2); // Two Idle workers
        assert_eq!(pool.busy_count(), 1); // One Claimed worker
        assert_eq!(pool.needing_attention_count(), 2); // One Unhealthy + one Dead

        // Verify with non-cached method
        assert_eq!(pool.count_by_state(WorkerState::Idle), 2);
        assert_eq!(pool.count_by_state(WorkerState::Claimed), 1);
        assert_eq!(pool.count_by_state(WorkerState::Unhealthy), 1);
        assert_eq!(pool.count_by_state(WorkerState::Dead), 1);
        Ok(())
    }
}
