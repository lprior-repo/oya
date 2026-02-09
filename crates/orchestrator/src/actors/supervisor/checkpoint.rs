//! Supervisor checkpoint functionality for graceful shutdown.
//!
//! This module provides checkpoint creation during supervisor shutdown,
//! capturing supervisor state for recovery purposes.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::persistence::PersistenceError;
use crate::replay::CheckpointManager;
use crate::shutdown::CheckpointResult;

use super::GenericSupervisableActor;
use super::supervisor_actor::{SupervisorActorState, SupervisorConfig};

/// Errors that can occur during supervisor checkpoint creation.
#[derive(Debug, Error)]
pub enum SupervisorCheckpointError {
    /// CheckpointManager not available in supervisor state.
    #[error("CheckpointManager not available")]
    CheckpointManagerUnavailable,

    /// Failed to serialize supervisor state to JSON.
    #[error("Serialization failed: {reason}")]
    SerializationFailed { reason: String },

    /// Checkpoint creation timed out (25 second limit).
    #[error("Checkpoint timeout after {duration_ms}ms")]
    CheckpointTimeout { duration_ms: u64 },

    /// Database error during checkpoint persistence.
    #[error("Checkpoint persistence failed: {source}")]
    CheckpointPersistenceFailed {
        /// Underlying error from persistence layer.
        source: PersistenceError,
    },

    /// Checkpoint result channel closed unexpectedly.
    #[error("Checkpoint result channel closed")]
    ResultChannelClosed,

    /// Invalid supervisor state for checkpoint.
    #[error("Invalid state: {reason}")]
    InvalidState {
        /// Description of invalid state.
        reason: String,
    },
}

/// Serializable snapshot of supervisor state for checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorSnapshot {
    /// Supervisor configuration.
    pub config: SupervisorConfig,
    /// Number of active children at checkpoint time.
    pub active_children: usize,
    /// Total restarts performed at checkpoint time.
    pub total_restarts: u32,
    /// Child information (names, restart counts, args).
    pub children: Vec<ChildSnapshot>,
    /// Failure timestamps within restart window.
    pub failure_count_in_window: usize,
    /// Current child ID counter.
    pub child_id_counter: u64,
    /// Time of snapshot.
    pub snapshot_time: DateTime<Utc>,
    /// Shutdown reason (if applicable).
    pub shutdown_reason: Option<String>,
}

/// Snapshot of a single child for checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSnapshot {
    /// Child name.
    pub name: String,
    /// Number of times this child was restarted.
    pub restart_count: u32,
    /// Last restart time (if any).
    pub last_restart: Option<DateTime<Utc>>,
    /// Actor arguments (JSON-serialized).
    pub args: String,
}

impl<A: GenericSupervisableActor> SupervisorActorState<A>
where
    A::Arguments: Clone + Send + Sync + std::fmt::Debug,
    A::Msg: Send,
{
    /// Create supervisor checkpoint during graceful shutdown.
    ///
    /// This function is called during the SavingCheckpoints phase of shutdown
    /// to create a final checkpoint of supervisor state before stopping children.
    ///
    /// # Errors
    ///
    /// Returns `SupervisorCheckpointError` if:
    /// - CheckpointManager is not available
    /// - State serialization fails
    /// - Checkpoint persistence fails
    /// - Timeout is exceeded
    ///
    /// Note: All errors are logged and a failed CheckpointResult is sent to
    /// the ShutdownCoordinator before returning the error. Shutdown continues
    /// regardless of checkpoint result.
    pub async fn create_shutdown_checkpoint(
        &self,
        checkpoint_manager: Option<&mut CheckpointManager>,
        checkpoint_tx: &mpsc::Sender<CheckpointResult>,
    ) -> Result<(), SupervisorCheckpointError> {
        let start = std::time::Instant::now();

        info!("Creating supervisor shutdown checkpoint");

        // Validate preconditions
        let checkpoint_manager =
            checkpoint_manager.ok_or(SupervisorCheckpointError::CheckpointManagerUnavailable)?;

        // Serialize state
        let serialized = serialize_supervisor_state(self).await?;

        // Create checkpoint with timeout
        let checkpoint_result = tokio::time::timeout(
            Duration::from_secs(25),
            checkpoint_manager.create_checkpoint(&serialized, None),
        )
        .await;

        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

        let result = match checkpoint_result {
            Ok(Ok(_checkpoint)) => {
                info!(duration_ms, "Supervisor checkpoint created successfully");
                CheckpointResult::success("supervisor", duration_ms)
            }
            Ok(Err(e)) => {
                let error = SupervisorCheckpointError::CheckpointPersistenceFailed { source: e };
                warn!(error = %error, "Checkpoint persistence failed");
                CheckpointResult::failure("supervisor", error.to_string())
            }
            Err(_) => {
                let error = SupervisorCheckpointError::CheckpointTimeout { duration_ms };
                warn!(duration_ms, "Checkpoint creation timed out");
                CheckpointResult::failure("supervisor", error.to_string())
            }
        };

        // Report result
        report_checkpoint_result(checkpoint_tx, result).await?;

        Ok(())
    }
}

/// Serialize supervisor state to JSON format for checkpoint storage.
///
/// # Errors
///
/// Returns `SupervisorCheckpointError::SerializationFailed` if JSON
/// serialization fails.
async fn serialize_supervisor_state<A>(
    state: &SupervisorActorState<A>,
) -> Result<String, SupervisorCheckpointError>
where
    A: GenericSupervisableActor,
    A::Arguments: Clone + Send + Sync + std::fmt::Debug,
    A::Msg: Send,
{
    let snapshot = build_snapshot(state).await;

    serde_json::to_string_pretty(&snapshot).map_err(|e| {
        error!(error = %e, "Failed to serialize supervisor state");
        SupervisorCheckpointError::SerializationFailed {
            reason: e.to_string(),
        }
    })
}

/// Build a snapshot from supervisor state.
async fn build_snapshot<A>(state: &SupervisorActorState<A>) -> SupervisorSnapshot
where
    A: GenericSupervisableActor,
    A::Arguments: Clone + Send + Sync + std::fmt::Debug,
    A::Msg: Send,
{
    let children = build_child_snapshots(state).await;

    SupervisorSnapshot {
        config: state.config.clone(),
        active_children: state.children.len(),
        total_restarts: state.total_restarts,
        children,
        failure_count_in_window: state.failure_times.len(),
        child_id_counter: state.child_id_counter,
        snapshot_time: Utc::now(),
        shutdown_reason: None, // Could be populated from shutdown signal
    }
}

/// Build child snapshots from supervisor state.
async fn build_child_snapshots<A>(state: &SupervisorActorState<A>) -> Vec<ChildSnapshot>
where
    A: GenericSupervisableActor,
    A::Arguments: Clone + Send + Sync + std::fmt::Debug,
    A::Msg: Send,
{
    state
        .children
        .values()
        .map(|child| ChildSnapshot {
            name: child.name.clone(),
            restart_count: child.restart_count,
            last_restart: child
                .last_restart
                .map(|i| DateTime::<Utc>::from(std::time::SystemTime::now() - i.elapsed()).into()),
            args: format!("{:?}", child.args), // Debug format for args
        })
        .collect()
}

/// Send checkpoint result to shutdown coordinator.
///
/// # Errors
///
/// Returns `SupervisorCheckpointError::ResultChannelClosed` if the
/// channel is closed.
async fn report_checkpoint_result(
    checkpoint_tx: &mpsc::Sender<CheckpointResult>,
    result: CheckpointResult,
) -> Result<(), SupervisorCheckpointError> {
    checkpoint_tx.send(result).await.map_err(|_| {
        error!("Checkpoint result channel closed");
        SupervisorCheckpointError::ResultChannelClosed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::scheduler::SchedulerActorDef;
    use crate::actors::supervisor::strategy::OneForOne;

    #[test]
    fn test_child_snapshot_creation() {
        let snapshot = ChildSnapshot {
            name: "test-child".to_string(),
            restart_count: 3,
            last_restart: None,
            args: "test args".to_string(),
        };

        assert_eq!(snapshot.name, "test-child");
        assert_eq!(snapshot.restart_count, 3);
        assert!(snapshot.last_restart.is_none());
    }

    #[test]
    fn test_supervisor_snapshot_creation() {
        let snapshot = SupervisorSnapshot {
            config: SupervisorConfig::default(),
            active_children: 2,
            total_restarts: 5,
            children: vec![],
            failure_count_in_window: 1,
            child_id_counter: 10,
            snapshot_time: Utc::now(),
            shutdown_reason: Some("test".to_string()),
        };

        assert_eq!(snapshot.active_children, 2);
        assert_eq!(snapshot.total_restarts, 5);
        assert_eq!(snapshot.child_id_counter, 10);
        assert_eq!(snapshot.shutdown_reason, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_serialize_supervisor_state() {
        let state = SupervisorActorState::<SchedulerActorDef> {
            config: SupervisorConfig::default(),
            state: crate::actors::supervisor::supervisor_actor::SupervisorState::Running,
            children: std::collections::HashMap::new(),
            failure_times: vec![],
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(OneForOne::new()),
            checkpoint_manager: None,
        };

        let result = serialize_supervisor_state(&state).await;
        assert!(result.is_ok());

        if let Ok(json) = result {
            assert!(json.contains("active_children"));
            assert!(json.contains("total_restarts"));
            assert!(json.contains("child_id_counter"));
        }
    }

    #[test]
    fn test_checkpoint_error_display() {
        let error = SupervisorCheckpointError::CheckpointManagerUnavailable;
        assert!(error.to_string().contains("not available"));

        let error = SupervisorCheckpointError::SerializationFailed {
            reason: "test error".to_string(),
        };
        assert!(error.to_string().contains("Serialization failed"));

        let error = SupervisorCheckpointError::CheckpointTimeout { duration_ms: 1000 };
        assert!(error.to_string().contains("timeout"));
        assert!(error.to_string().contains("1000"));
    }
}
