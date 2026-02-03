//! Durable timers for scheduled task execution.
//!
//! This module provides Restate-style durable timers that survive
//! restarts and ensure scheduled work is executed.
//!
//! # Architecture
//!
//! The timer system uses:
//! 1. Persistent storage for durability
//! 2. A scheduler for managing pending timers
//! 3. An executor for firing timers
//! 4. Recovery to resume timers after restart
//!
//! # Key Types
//!
//! - `DurableTimer`: A timer that persists and fires at a scheduled time
//! - `TimerScheduler`: Schedules and manages timers
//! - `TimerExecutor`: Executes fired timers

// Allow dead_code until this module is fully integrated
#![allow(dead_code)]

mod executor;
mod persistence;
mod scheduler;

pub use executor::{TimerCallback, TimerExecutor, TimerExecutorConfig};
pub use persistence::{TimerPersistence, TimerRecord};
pub use scheduler::{DurableTimer, TimerId, TimerScheduler, TimerSchedulerConfig, TimerStatus};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_timer_id_generation() {
        let id1 = TimerId::new();
        let id2 = TimerId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_timer_status_variants() {
        assert!(TimerStatus::Pending.is_pending());
        assert!(TimerStatus::Fired.is_fired());
        assert!(TimerStatus::Cancelled.is_cancelled());
    }

    #[test]
    fn test_durable_timer_creation() {
        let timer = DurableTimer::new(
            Utc::now() + chrono::Duration::seconds(60),
            serde_json::json!({"task": "test"}),
        );
        assert!(timer.status().is_pending());
    }
}
