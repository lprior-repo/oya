//! Auto-checkpoint timer for periodic workflow state snapshots.
//!
//! This module provides a background task that automatically creates
//! checkpoints at regular intervals (default: 60 seconds). The timer
//! runs independently of workflow execution and can be gracefully
//! shut down when no longer needed.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::error::Result;
use crate::storage::WorkflowStorage;
use crate::types::{Checkpoint, PhaseId, WorkflowId};

/// Default auto-checkpoint interval (60 seconds).
pub const DEFAULT_AUTO_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(60);

/// Configuration for the auto-checkpoint timer.
#[derive(Debug, Clone)]
pub struct AutoCheckpointConfig {
    /// Interval between automatic checkpoints.
    pub interval: Duration,
}

impl Default for AutoCheckpointConfig {
    fn default() -> Self {
        Self {
            interval: DEFAULT_AUTO_CHECKPOINT_INTERVAL,
        }
    }
}

impl AutoCheckpointConfig {
    /// Create a new auto-checkpoint configuration.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }

    /// Create with the default 60-second interval.
    #[must_use]
    pub fn default_interval() -> Self {
        Self::default()
    }
}

/// State provider function type.
///
/// This function is called periodically to capture the current workflow state
/// for checkpointing. It should return the phase ID and serialized state data.
pub type StateProvider = fn() -> Option<(PhaseId, Vec<u8>)>;

/// Start the auto-checkpoint timer.
///
/// This spawns a background task that creates checkpoints at the configured
/// interval. The task runs until a shutdown signal is received or the
/// state provider returns `None` (indicating the workflow is complete).
///
/// # Arguments
///
/// * `storage` - Storage backend for persisting checkpoints
/// * `workflow_id` - ID of the workflow to checkpoint
/// * `config` - Auto-checkpoint configuration
/// * `state_provider` - Function to capture current workflow state
///
/// # Returns
///
/// A `JoinHandle` for the background task. Use `.await` to wait for
/// graceful shutdown, or `abort()` to force termination.
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
/// use std::time::Duration;
/// use oya_workflow::checkpoint::auto::start_auto_checkpoint;
/// use oya_workflow::{InMemoryStorage, WorkflowId, PhaseId};
///
/// # #[tokio::main]
/// # async fn main() {
/// let storage = Arc::new(InMemoryStorage::new());
/// let workflow_id = WorkflowId::new();
///
/// let handle = start_auto_checkpoint(
///     storage,
///     workflow_id,
///     Duration::from_secs(60),
///     || Some((PhaseId::new(), vec![1, 2, 3])),
/// );
///
/// // Let it run...
/// // Later: handle.abort(); to stop
/// # }
/// ```
pub fn start_auto_checkpoint(
    storage: Arc<dyn WorkflowStorage>,
    workflow_id: WorkflowId,
    interval: Duration,
    state_provider: StateProvider,
) -> JoinHandle<Result<()>> {
    tokio::spawn(auto_checkpoint_loop(
        storage,
        workflow_id,
        interval,
        state_provider,
    ))
}

/// Internal auto-checkpoint loop.
///
/// Runs the periodic checkpointing logic with graceful shutdown support.
async fn auto_checkpoint_loop(
    storage: Arc<dyn WorkflowStorage>,
    workflow_id: WorkflowId,
    interval: Duration,
    state_provider: StateProvider,
) -> Result<()> {
    let mut ticker = interval(interval);
    ticker.tick().await; // First tick completes immediately

    info!(
        workflow_id = %workflow_id,
        interval_secs = interval.as_secs(),
        "Auto-checkpoint timer started"
    );

    loop {
        ticker.tick().await;

        // Capture current state
        let (phase_id, state_data) = match state_provider() {
            Some(data) => data,
            None => {
                info!(
                    workflow_id = %workflow_id,
                    "State provider returned None, stopping auto-checkpoint"
                );
                break;
            }
        };

        // Create checkpoint
        let checkpoint = Checkpoint::new(phase_id, state_data.clone(), vec![]);

        // Persist checkpoint
        match storage
            .save_checkpoint(workflow_id, &checkpoint)
            .await
        {
            Ok(()) => {
                info!(
                    workflow_id = %workflow_id,
                    phase_id = %phase_id,
                    "Auto-checkpoint created successfully"
                );
            }
            Err(e) => {
                // Log error but continue running - don't crash the timer
                error!(
                    workflow_id = %workflow_id,
                    phase_id = %phase_id,
                    error = ?e,
                    "Failed to create auto-checkpoint"
                );
            }
        }
    }

    info!(
        workflow_id = %workflow_id,
        "Auto-checkpoint timer stopped"
    );

    Ok(())
}

/// Managed auto-checkpoint timer with shutdown support.
///
/// This struct provides a more structured way to manage the auto-checkpoint
/// timer lifecycle, including graceful shutdown.
pub struct AutoCheckpointTimer {
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<Result<()>>>,
}

impl AutoCheckpointTimer {
    /// Start a new auto-checkpoint timer with shutdown support.
    ///
    /// # Arguments
    ///
    /// * `storage` - Storage backend for persisting checkpoints
    /// * `workflow_id` - ID of the workflow to checkpoint
    /// * `interval` - Interval between checkpoints
    /// * `state_provider` - Function to capture current workflow state
    ///
    /// # Returns
    ///
    /// A timer handle that can be used to shutdown the task gracefully.
    pub fn start(
        storage: Arc<dyn WorkflowStorage>,
        workflow_id: WorkflowId,
        interval: Duration,
        state_provider: StateProvider,
    ) -> Self {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        let handle = tokio::spawn(async move {
            let mut ticker = interval(interval);
            ticker.tick().await; // First tick completes immediately

            info!(
                workflow_id = %workflow_id,
                interval_secs = interval.as_secs(),
                "Auto-checkpoint timer started"
            );

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Capture current state
                        let (phase_id, state_data) = match state_provider() {
                            Some(data) => data,
                            None => {
                                info!(
                                    workflow_id = %workflow_id,
                                    "State provider returned None, stopping auto-checkpoint"
                                );
                                break;
                            }
                        };

                        // Create checkpoint
                        let checkpoint = Checkpoint::new(phase_id, state_data.clone(), vec![]);

                        // Persist checkpoint
                        match storage.save_checkpoint(workflow_id, &checkpoint).await {
                            Ok(()) => {
                                info!(
                                    workflow_id = %workflow_id,
                                    phase_id = %phase_id,
                                    "Auto-checkpoint created successfully"
                                );
                            }
                            Err(e) => {
                                // Log error but continue running
                                error!(
                                    workflow_id = %workflow_id,
                                    phase_id = %phase_id,
                                    error = ?e,
                                    "Failed to create auto-checkpoint"
                                );
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!(
                            workflow_id = %workflow_id,
                            "Shutdown signal received, stopping auto-checkpoint"
                        );
                        break;
                    }
                }
            }

            info!(
                workflow_id = %workflow_id,
                "Auto-checkpoint timer stopped"
            );

            Ok(())
        });

        Self {
            shutdown_tx,
            handle: Some(handle),
        }
    }

    /// Stop the auto-checkpoint timer gracefully.
    ///
    /// This sends a shutdown signal and waits for the task to complete.
    /// If the task doesn't respond within the timeout, it will be aborted.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for graceful shutdown
    ///
    /// # Returns
    ///
    /// `Ok(())` if shutdown completed successfully, `Err` if the task
    /// panicked or was aborted.
    pub async fn shutdown(mut self, timeout: Duration) -> Result<()> {
        // Send shutdown signal
        if let Err(e) = self.shutdown_tx.send(()).await {
            warn!(
                error = ?e,
                "Failed to send shutdown signal, channel closed"
            );
        }

        // Wait for task completion with timeout
        if let Some(handle) = self.handle.take() {
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => {
                    error!(error = ?e, "Auto-checkpoint task panicked");
                    Err(e)
                }
                Err(_) => {
                    warn!("Auto-checkpoint shutdown timeout, aborting task");
                    // Task was dropped by timeout, it will be aborted
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}

/// Drop implementation to ensure task is aborted if not properly shut down.
impl Drop for AutoCheckpointTimer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Helper to create a counter-based state provider.
    fn create_counter_state_provider(counter: Arc<AtomicU64>) -> StateProvider {
        || {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count < 3 {
                Some((PhaseId::new(), vec![count as u8]))
            } else {
                None // Stop after 3 checkpoints
            }
        }
    }

    #[tokio::test]
    async fn test_auto_checkpoint_creates_checkpoints() {
        let storage = Arc::new(InMemoryStorage::new());
        let workflow_id = WorkflowId::new();
        let counter = Arc::new(AtomicU64::new(0));

        let handle = start_auto_checkpoint(
            storage.clone(),
            workflow_id,
            Duration::from_millis(100), // Fast interval for testing
            create_counter_state_provider(counter),
        );

        // Wait for completion
        let result = handle.await;
        assert!(result.is_ok(), "Task should complete successfully");
        let result = result.ok().unwrap();
        assert!(result.is_ok(), "Auto-checkpoint should succeed");

        // Verify checkpoints were created
        let checkpoints = storage.load_checkpoints(workflow_id).await;
        assert!(checkpoints.is_ok());
        assert_eq!(checkpoints.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_auto_checkpoint_with_shutdown() {
        let storage = Arc::new(InMemoryStorage::new());
        let workflow_id = WorkflowId::new();
        let counter = Arc::new(AtomicU64::new(0));

        // Create state provider that doesn't stop
        let state_provider: StateProvider = || {
            let _ = counter.fetch_add(1, Ordering::SeqCst);
            Some((PhaseId::new(), vec![1, 2, 3]))
        };

        let timer = AutoCheckpointTimer::start(
            storage.clone(),
            workflow_id,
            Duration::from_millis(100),
            state_provider,
        );

        // Let it run for a bit
        tokio::time::sleep(Duration::from_millis(250)).await;

        // Shutdown gracefully
        let result = timer
            .shutdown(Duration::from_millis(500))
            .await;
        assert!(result.is_ok(), "Shutdown should succeed");

        // Verify some checkpoints were created
        let checkpoints = storage.load_checkpoints(workflow_id).await;
        assert!(checkpoints.is_ok());
        let checkpoint_count = checkpoints.unwrap().len();
        assert!(
            checkpoint_count >= 2,
            "Should have created at least 2 checkpoints, got {}",
            checkpoint_count
        );
    }

    #[tokio::test]
    async fn test_auto_checkpoint_handles_storage_errors() {
        // Use a storage that will fail
        #[derive(Default)]
        struct FailingStorage;

        #[async_trait::async_trait]
        impl WorkflowStorage for FailingStorage {
            async fn save_workflow(&self, _workflow: &crate::types::Workflow) -> Result<()> {
                Ok(())
            }

            async fn load_workflow(&self, _id: WorkflowId) -> Result<Option<crate::types::Workflow>> {
                Ok(None)
            }

            async fn delete_workflow(&self, _id: WorkflowId) -> Result<()> {
                Ok(())
            }

            async fn list_workflows(&self) -> Result<Vec<crate::types::Workflow>> {
                Ok(Vec::new())
            }

            async fn save_checkpoint(
                &self,
                _workflow_id: WorkflowId,
                _checkpoint: &Checkpoint,
            ) -> Result<()> {
                Err(crate::error::Error::storage("Storage error"))
            }

            async fn load_checkpoints(&self, _workflow_id: WorkflowId) -> Result<Vec<Checkpoint>> {
                Ok(Vec::new())
            }

            async fn load_checkpoint(
                &self,
                _workflow_id: WorkflowId,
                _phase_id: PhaseId,
            ) -> Result<Option<Checkpoint>> {
                Ok(None)
            }

            async fn append_journal(
                &self,
                _workflow_id: WorkflowId,
                _entry: crate::types::JournalEntry,
            ) -> Result<()> {
                Ok(())
            }

            async fn load_journal(&self, _workflow_id: WorkflowId) -> Result<crate::types::Journal> {
                Ok(crate::types::Journal::default())
            }

            async fn clear_checkpoints_after(
                &self,
                _workflow_id: WorkflowId,
                _phase_id: PhaseId,
            ) -> Result<()> {
                Ok(())
            }
        }

        let storage = Arc::new(FailingStorage);
        let workflow_id = WorkflowId::new();
        let counter = Arc::new(AtomicU64::new(0));

        let handle = start_auto_checkpoint(
            storage.clone(),
            workflow_id,
            Duration::from_millis(100),
            create_counter_state_provider(counter),
        );

        // Task should still complete successfully despite errors
        let result = handle.await;
        assert!(result.is_ok(), "Task should complete");
        let result = result.ok().unwrap();
        assert!(result.is_ok(), "Should handle errors gracefully");
    }
}
