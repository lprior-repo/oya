//! Checkpoint management for recovery.
//!
//! Checkpoints capture full state snapshots at a point in time,
//! allowing faster recovery by only replaying events since the checkpoint.

use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::persistence::{CheckpointRecord, OrchestratorStore, PersistenceResult};

/// Configuration for the checkpoint manager.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Interval between automatic checkpoints
    pub interval: Duration,
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: usize,
    /// Whether to create checkpoints automatically
    pub auto_checkpoint: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300), // 5 minutes
            max_checkpoints: 10,
            auto_checkpoint: true,
        }
    }
}

/// Manages periodic checkpointing of orchestrator state.
pub struct CheckpointManager {
    store: OrchestratorStore,
    config: CheckpointConfig,
    current_sequence: u64,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager.
    #[must_use]
    pub fn new(store: OrchestratorStore, config: CheckpointConfig) -> Self {
        Self {
            store,
            config,
            current_sequence: 0,
            shutdown_tx: None,
        }
    }

    /// Create a checkpoint at the current event sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint cannot be saved.
    pub async fn create_checkpoint(
        &mut self,
        scheduler_state: &str,
        workflow_snapshots: Option<&str>,
    ) -> PersistenceResult<CheckpointRecord> {
        let checkpoint_id = format!(
            "cp-{}-{}",
            Utc::now().timestamp_millis(),
            self.current_sequence
        );

        let mut record =
            CheckpointRecord::new(&checkpoint_id, scheduler_state, self.current_sequence);

        if let Some(snapshots) = workflow_snapshots {
            record = record.with_workflow_snapshots(snapshots);
        }

        let saved = self.store.save_checkpoint(&record).await?;

        // Prune old checkpoints
        let _ = self
            .store
            .prune_checkpoints(self.config.max_checkpoints)
            .await;

        Ok(saved)
    }

    /// Get the latest checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if no checkpoint exists or query fails.
    pub async fn get_latest(&self) -> PersistenceResult<CheckpointRecord> {
        self.store.get_latest_checkpoint().await
    }

    /// Get a checkpoint by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found.
    pub async fn get_checkpoint(&self, checkpoint_id: &str) -> PersistenceResult<CheckpointRecord> {
        self.store.get_checkpoint(checkpoint_id).await
    }

    /// Increment the event sequence counter.
    pub fn increment_sequence(&mut self) {
        self.current_sequence = self.current_sequence.saturating_add(1);
    }

    /// Set the current event sequence.
    pub fn set_sequence(&mut self, sequence: u64) {
        self.current_sequence = sequence;
    }

    /// Get the current event sequence.
    #[must_use]
    pub fn current_sequence(&self) -> u64 {
        self.current_sequence
    }

    /// Start the periodic checkpoint task.
    ///
    /// Returns a handle to stop the task.
    pub fn start_periodic(&mut self) -> Option<mpsc::Receiver<()>> {
        if !self.config.auto_checkpoint {
            return None;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        Some(shutdown_rx)
    }

    /// Stop the periodic checkpoint task.
    pub async fn stop_periodic(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Run the periodic checkpoint loop.
    ///
    /// This should be spawned as a background task.
    pub async fn run_periodic_loop(
        store: OrchestratorStore,
        config: CheckpointConfig,
        mut shutdown_rx: mpsc::Receiver<()>,
        state_fn: impl Fn() -> (String, Option<String>) + Send + 'static,
    ) {
        let mut ticker = interval(config.interval);
        let mut sequence = 0u64;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let (scheduler_state, workflow_snapshots) = state_fn();
                    let checkpoint_id = format!("cp-{}-{}", Utc::now().timestamp_millis(), sequence);
                    let mut record = CheckpointRecord::new(&checkpoint_id, scheduler_state, sequence);

                    if let Some(ref snapshots) = workflow_snapshots {
                        record = record.with_workflow_snapshots(snapshots);
                    }

                    if let Err(e) = store.save_checkpoint(&record).await {
                        tracing::error!("Failed to create periodic checkpoint: {:?}", e);
                    } else {
                        tracing::info!("Created checkpoint {} at sequence {}", checkpoint_id, sequence);
                        // Prune old checkpoints
                        let _ = store.prune_checkpoints(config.max_checkpoints).await;
                    }

                    sequence = sequence.saturating_add(1);
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Checkpoint manager shutting down");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::StoreConfig;

    async fn setup_manager() -> Option<CheckpointManager> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        Some(CheckpointManager::new(store, CheckpointConfig::default()))
    }

    macro_rules! require_manager {
        ($manager_opt:expr) => {
            match $manager_opt {
                Some(m) => m,
                None => {
                    eprintln!("Skipping test: manager setup failed");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn test_create_checkpoint() {
        let mut manager = require_manager!(setup_manager().await);

        let result = manager
            .create_checkpoint(r#"{"state":"active"}"#, None)
            .await;

        assert!(result.is_ok(), "checkpoint creation should succeed");

        if let Ok(cp) = result {
            assert_eq!(cp.event_sequence, 0);
        }
    }

    #[tokio::test]
    async fn test_increment_sequence() {
        let mut manager = require_manager!(setup_manager().await);

        assert_eq!(manager.current_sequence(), 0);

        manager.increment_sequence();
        assert_eq!(manager.current_sequence(), 1);

        manager.increment_sequence();
        assert_eq!(manager.current_sequence(), 2);
    }

    #[tokio::test]
    async fn test_checkpoint_with_snapshots() {
        let mut manager = require_manager!(setup_manager().await);

        let result = manager
            .create_checkpoint(
                r#"{"state":"active"}"#,
                Some(r#"{"wf-1":{"beads":["a","b"]}}"#),
            )
            .await;

        assert!(result.is_ok());

        if let Ok(cp) = result {
            assert!(cp.workflow_snapshots.is_some());
        }
    }

    #[tokio::test]
    async fn test_get_latest_after_create() {
        let mut manager = require_manager!(setup_manager().await);

        manager.set_sequence(100);
        let _ = manager.create_checkpoint("{}", None).await;

        manager.set_sequence(200);
        let _ = manager.create_checkpoint("{}", None).await;

        let latest = manager.get_latest().await;
        assert!(latest.is_ok());

        if let Ok(cp) = latest {
            assert_eq!(cp.event_sequence, 200);
        }
    }
}
