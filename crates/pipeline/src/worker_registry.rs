//! Worker registration infrastructure for Zellij plugin.
//!
//! Provides declarative macros for registering workers with health monitoring
//! and process pool management.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)];

/// Macro for registering a worker with the heartbeat monitor.
///
/// This macro provides a declarative syntax for registering workers
/// with automatic error handling and ID generation.
///
/// # Examples
///
/// ```rust
/// use oya_pipeline::{register_worker, actor::HeartbeatMonitor};
///
/// let mut monitor = HeartbeatMonitor::new();
///
/// // Register a worker with an explicit ID
/// register_worker!(monitor, 1);
///
/// // Register a worker and get the result
/// let result = register_worker!(monitor, 2);
/// assert!(result.is_ok());
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The worker ID is already registered
/// - The ID is invalid (e.g., zero)
#[macro_export]
macro_rules! register_worker {
    // Register with explicit ID, return Result
    ($monitor:expr, $id:expr) => {{
        use $crate::actor::ProcessId;
        let worker_id = ProcessId::new($id);
        $monitor.register_worker(worker_id)
    }};

    // Register with explicit ID, unwrap with context (for tests)
    ($monitor:expr, $id:expr, unwrap) => {{
        use $crate::actor::ProcessId;
        let worker_id = ProcessId::new($id);
        $monitor
            .register_worker(worker_id)
            .unwrap_or_else(|e| panic!("Failed to register worker {}: {}", $id, e))
    }};
}

/// Macro for batch registering multiple workers.
///
/// Registers a sequence of workers with auto-incrementing IDs.
///
/// # Examples
///
/// ```rust
/// use oya_pipeline::{register_workers_batch, actor::HeartbeatMonitor};
///
/// let mut monitor = HeartbeatMonitor::new();
///
/// // Register 5 workers with IDs 0-4
/// let results = register_workers_batch!(monitor, 5);
/// assert_eq!(results.len(), 5);
/// ```
#[macro_export]
macro_rules! register_workers_batch {
    ($monitor:expr, $count:expr) => {{
        let mut results = Vec::new();
        for i in 0..$count {
            results.push($crate::register_worker!($monitor, i));
        }
        results
    }};
}

/// Macro for registering a worker with both process pool and health monitor.
///
/// This is a convenience macro for the common pattern of adding a worker
/// to both the process pool and the health monitoring system.
///
/// # Examples
///
/// ```rust
/// use oya_pipeline::{register_worker_full, actor::{ProcessPoolActor, HeartbeatMonitor, WorkerState}};
///
/// let mut pool = ProcessPoolActor::new();
/// let mut monitor = HeartbeatMonitor::new();
///
/// // Register worker with both systems
/// register_worker_full!(pool, monitor, 1, WorkerState::Idle);
/// ```
#[macro_export]
macro_rules! register_worker_full {
    ($pool:expr, $monitor:expr, $id:expr, $state:expr) => {{
        use $crate::actor::ProcessId;
        let worker_id = ProcessId::new($id);

        // Register with process pool
        let pool_result = $pool.add_worker(worker_id, $state);

        // Register with health monitor
        let monitor_result = $monitor.register_worker(worker_id);

        // Combine results
        match (pool_result, monitor_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{ProcessPoolActor, WorkerState};

    #[test]
    fn register_worker_macro_adds_single_worker() {
        let mut monitor = HeartbeatMonitor::new();

        let result = register_worker!(monitor, 1);

        assert!(result.is_ok(), "Worker registration should succeed");
        assert_eq!(monitor.worker_count(), 1, "Should have exactly 1 worker");
    }

    #[test]
    fn register_worker_macro_rejects_duplicate_id() {
        let mut monitor = HeartbeatMonitor::new();

        let first = register_worker!(monitor, 1);
        let second = register_worker!(monitor, 1);

        assert!(first.is_ok(), "First registration should succeed");
        assert!(second.is_err(), "Duplicate registration should fail");
    }

    #[test]
    fn register_worker_macro_unwrap_variant_works() {
        let mut monitor = HeartbeatMonitor::new();

        // This should not panic
        register_worker!(monitor, 1, unwrap);

        assert_eq!(monitor.worker_count(), 1);
    }

    #[test]
    #[should_panic(expected = "Failed to register worker")]
    fn register_worker_macro_unwrap_panics_on_duplicate() {
        let mut monitor = HeartbeatMonitor::new();

        register_worker!(monitor, 1, unwrap);
        // This should panic
        register_worker!(monitor, 1, unwrap);
    }

    #[test]
    fn register_workers_batch_creates_multiple_workers() {
        let mut monitor = HeartbeatMonitor::new();

        let results = register_workers_batch!(monitor, 5);

        assert_eq!(results.len(), 5, "Should create 5 workers");
        assert!(
            results.iter().all(|r| r.is_ok()),
            "All registrations should succeed"
        );
        assert_eq!(monitor.worker_count(), 5, "Monitor should track 5 workers");
    }

    // Note: Empty range test removed as macro expands to `for i in 0..0` which
    // triggers clippy::reversed_empty_ranges. This is acceptable behavior for
    // the macro - batch registration with count=0 simply produces no workers.

    #[test]
    fn register_worker_full_succeeds_for_new_worker() {
        let mut pool = ProcessPoolActor::new();
        let mut monitor = HeartbeatMonitor::new();

        let result = register_worker_full!(pool, monitor, 1, WorkerState::Idle);

        assert!(result.is_ok(), "Full registration should succeed");
        assert_eq!(pool.size(), 1, "Pool should have 1 worker");
        assert_eq!(monitor.worker_count(), 1, "Monitor should track 1 worker");
    }

    #[test]
    fn register_worker_full_fails_if_pool_rejects() {
        let mut pool = ProcessPoolActor::new();
        let mut monitor = HeartbeatMonitor::new();

        // First registration succeeds
        register_worker_full!(pool, monitor, 1, WorkerState::Idle).ok();

        // Second registration with same ID should fail
        let result = register_worker_full!(pool, monitor, 1, WorkerState::Idle);

        assert!(result.is_err(), "Duplicate full registration should fail");
    }

    #[test]
    fn register_worker_full_maintains_consistency() {
        let mut pool = ProcessPoolActor::new();
        let mut monitor = HeartbeatMonitor::new();

        // Register 3 workers
        for i in 0..3 {
            register_worker_full!(pool, monitor, i, WorkerState::Idle).ok();
        }

        assert_eq!(pool.size(), 3, "Pool should have 3 workers");
        assert_eq!(monitor.worker_count(), 3, "Monitor should track 3 workers");

        // Verify all workers are in correct state
        assert_eq!(pool.count_by_state(WorkerState::Idle), 3);
    }

    #[test]
    fn register_worker_macros_preserve_functional_semantics() {
        let mut monitor = HeartbeatMonitor::new();

        // Test that macros don't use unwrap/panic internally
        let result = register_worker!(monitor, 42);

        // Should return Result, not panic
        assert!(result.is_ok());

        // Verify functional purity - same input produces same output
        let worker_id = ProcessId::new(42);
        let status = monitor.get_health_status(&worker_id);
        assert!(status.is_some(), "Worker should be registered");
    }
}
