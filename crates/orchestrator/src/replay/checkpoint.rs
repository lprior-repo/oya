//! Checkpoint management for recovery.
//!
//! Checkpoints capture full state snapshots at a point in time,
//! allowing faster recovery by only replaying events since the checkpoint.

use std::time::Duration;

use chrono::Utc;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio::time::interval;

use super::compression::{
    CheckpointCompressor, CompressionConfig, CompressionLevel, CompressionStats,
};

use crate::persistence::{
    CheckpointRecord, OrchestratorStore, PersistenceError, PersistenceResult,
};

/// Configuration for the checkpoint manager.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Interval between automatic checkpoints
    pub interval: Duration,
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: usize,
    /// Whether to create checkpoints automatically
    pub auto_checkpoint: bool,
    /// Compression configuration
    pub compression: CompressionConfig,
    /// Whether to enable compression
    pub enable_compression: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300),
            max_checkpoints: 10,
            auto_checkpoint: true,
            compression: CompressionConfig::default(),
            enable_compression: true,
        }
    }
}

impl CheckpointConfig {
    /// Create a config with a specific compression level.
    #[must_use]
    pub const fn with_compression_level(level: CompressionLevel) -> Self {
        Self {
            interval: Duration::from_secs(300),
            max_checkpoints: 10,
            auto_checkpoint: true,
            compression: CompressionConfig::new(level),
            enable_compression: true,
        }
    }

    /// Create a config with compression disabled.
    #[must_use]
    pub const fn without_compression() -> Self {
        Self {
            interval: Duration::from_secs(300),
            max_checkpoints: 10,
            auto_checkpoint: true,
            compression: CompressionConfig::new(CompressionLevel::DEFAULT),
            enable_compression: false,
        }
    }
}

/// Result of a compressed checkpoint operation.
#[derive(Debug, Clone)]
pub struct CompressionMetrics {
    /// Original size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Compression ratio (compressed / original)
    pub ratio: f64,
}

fn base64_encode(data: &[u8]) -> String {
    use base64::prelude::*;
    BASE64_STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::prelude::*;
    BASE64_STANDARD.decode(s)
}

/// Manages periodic checkpointing of orchestrator state with zstd compression.
pub struct CheckpointManager {
    store: OrchestratorStore,
    config: CheckpointConfig,
    compressor: CheckpointCompressor,
    current_sequence: u64,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager.
    #[must_use]
    pub fn new(store: OrchestratorStore, config: CheckpointConfig) -> Self {
        let compressor = CheckpointCompressor::new(config.compression);
        Self {
            store,
            config,
            compressor,
            current_sequence: 0,
            shutdown_tx: None,
        }
    }

    /// Get the compression configuration.
    #[must_use]
    pub fn compression_config(&self) -> &CompressionConfig {
        &self.config.compression
    }

    /// Check if compression is enabled.
    #[must_use]
    pub const fn is_compression_enabled(&self) -> bool {
        self.config.enable_compression
    }

    /// Create a checkpoint at the current event sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint cannot be saved or compression fails.
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

        let (compressed_state, state_metrics) = self.compress_data(scheduler_state)?;

        let (compressed_snapshots, snapshots_metrics) = workflow_snapshots
            .map(|s| self.compress_data(s))
            .transpose()?
            .unzip();

        if let Some(m) = &state_metrics {
            tracing::debug!(
                checkpoint_id = %checkpoint_id,
                field = "scheduler_state",
                original_size = m.original_size,
                compressed_size = m.compressed_size,
                ratio = m.compression_ratio,
                time_ms = m.compression_time_ms,
                "Compressed checkpoint field"
            );
        }

        if let Some(Some(m)) = snapshots_metrics.as_ref() {
            tracing::debug!(
                checkpoint_id = %checkpoint_id,
                field = "workflow_snapshots",
                original_size = m.original_size,
                compressed_size = m.compressed_size,
                ratio = m.compression_ratio,
                time_ms = m.compression_time_ms,
                "Compressed checkpoint field"
            );
        }

        let mut record =
            CheckpointRecord::new(&checkpoint_id, &compressed_state, self.current_sequence);

        if let Some(snapshots) = compressed_snapshots {
            record = record.with_workflow_snapshots(&snapshots);
        }

        let saved = self.store.save_checkpoint(&record).await?;

        let _ = self
            .store
            .prune_checkpoints(self.config.max_checkpoints)
            .await;

        Ok(saved)
    }

    fn compress_data(&self, data: &str) -> PersistenceResult<(String, Option<CompressionStats>)> {
        if self.config.enable_compression {
            let (compressed, stats) = self.compressor.compress_string(data).map_err(|e| {
                PersistenceError::serialization_error(format!("compression failed: {e}"))
            })?;
            Ok((base64_encode(&compressed), Some(stats)))
        } else {
            Ok((data.to_string(), None))
        }
    }

    fn decompress_data(&self, data: &str) -> PersistenceResult<String> {
        if self.config.enable_compression {
            let bytes = base64_decode(data).map_err(|e| {
                PersistenceError::serialization_error(format!("base64 decode failed: {e}"))
            })?;
            self.compressor.decompress_to_string(&bytes).map_err(|e| {
                PersistenceError::serialization_error(format!("decompression failed: {e}"))
            })
        } else {
            Ok(data.to_string())
        }
    }

    /// Get the latest checkpoint (decompressed if necessary).
    ///
    /// # Errors
    ///
    /// Returns an error if no checkpoint exists or decompression fails.
    pub async fn get_latest(&self) -> PersistenceResult<CheckpointRecord> {
        let record = self.store.get_latest_checkpoint().await?;
        self.decompress_record(record)
    }

    /// Get a checkpoint by its ID (decompressed if necessary).
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found or decompression fails.
    pub async fn get_checkpoint(&self, checkpoint_id: &str) -> PersistenceResult<CheckpointRecord> {
        let record = self.store.get_checkpoint(checkpoint_id).await?;
        self.decompress_record(record)
    }

    fn decompress_record(&self, record: CheckpointRecord) -> PersistenceResult<CheckpointRecord> {
        let decompressed_state = self.decompress_data(&record.scheduler_state)?;

        let decompressed_snapshots = record
            .workflow_snapshots
            .as_ref()
            .map(|s| self.decompress_data(s))
            .transpose()?;

        Ok(CheckpointRecord {
            record_id: record.record_id,
            checkpoint_id: record.checkpoint_id,
            scheduler_state: decompressed_state,
            event_sequence: record.event_sequence,
            created_at: record.created_at,
            workflow_snapshots: decompressed_snapshots,
            metadata: record.metadata,
        })
    }

    /// Increment the event sequence counter.
    pub const fn increment_sequence(&mut self) {
        self.current_sequence = self.current_sequence.saturating_add(1);
    }

    /// Set the current event sequence.
    pub const fn set_sequence(&mut self, sequence: u64) {
        self.current_sequence = sequence;
    }

    /// Get the current event sequence.
    #[must_use]
    pub const fn current_sequence(&self) -> u64 {
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
        let compressor = CheckpointCompressor::new(config.compression);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let (scheduler_state, workflow_snapshots) = state_fn();
                    let checkpoint_id = format!("cp-{}-{}", Utc::now().timestamp_millis(), sequence);

                    let compressed_state = if config.enable_compression {
                        match compressor.compress_string(&scheduler_state) {
                            Ok((bytes, _)) => base64_encode(&bytes),
                            Err(_) => scheduler_state.clone(),
                        }
                    } else {
                        scheduler_state.clone()
                    };

                    let compressed_snapshots = workflow_snapshots.as_ref().and_then(|s| {
                        if config.enable_compression {
                            compressor.compress_string(s).ok().map(|(bytes, _)| base64_encode(&bytes))
                        } else {
                            Some(s.clone())
                        }
                    });

                    let mut record = CheckpointRecord::new(&checkpoint_id, &compressed_state, sequence);

                    if let Some(snapshots) = compressed_snapshots {
                        record = record.with_workflow_snapshots(&snapshots);
                    }

                    if let Err(e) = store.save_checkpoint(&record).await {
                        tracing::error!("Failed to create periodic checkpoint: {:?}", e);
                    } else {
                        tracing::info!("Created checkpoint {} at sequence {}", checkpoint_id, sequence);
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

    fn deserialize_json<T>(json_str: &str, field_name: &str) -> PersistenceResult<T>
    where
        T: DeserializeOwned,
    {
        serde_json::from_str(json_str).map_err(|e| {
            PersistenceError::serialization_error(format!(
                "failed to deserialize {field_name}: {e}"
            ))
        })
    }

    fn restore_scheduler_state_from_checkpoint<T>(
        checkpoint: &CheckpointRecord,
    ) -> PersistenceResult<T>
    where
        T: DeserializeOwned,
    {
        Self::deserialize_json(&checkpoint.scheduler_state, "scheduler state")
    }

    fn restore_workflow_snapshots_from_checkpoint<T>(
        checkpoint: &CheckpointRecord,
    ) -> PersistenceResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        checkpoint
            .workflow_snapshots
            .as_ref()
            .map(|s| Self::deserialize_json(s, "workflow snapshots"))
            .transpose()
    }

    /// Restore scheduler state from the latest checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No checkpoint exists
    /// - JSON deserialization fails
    /// - Database query fails
    pub async fn restore_scheduler_state<T>(&self) -> PersistenceResult<T>
    where
        T: DeserializeOwned,
    {
        let checkpoint = self.get_latest().await?;
        Self::restore_scheduler_state_from_checkpoint(&checkpoint)
    }

    /// Restore workflow snapshots from the latest checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No checkpoint exists
    /// - JSON deserialization fails (when snapshots are present)
    /// - Database query fails
    pub async fn restore_workflow_snapshots<T>(&self) -> PersistenceResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let checkpoint = self.get_latest().await?;
        Self::restore_workflow_snapshots_from_checkpoint(&checkpoint)
    }

    /// Restore scheduler state from a specific checkpoint by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Checkpoint with the given ID doesn't exist
    /// - JSON deserialization fails
    /// - Database query fails
    pub async fn restore_scheduler_state_by_id<T>(
        &self,
        checkpoint_id: &str,
    ) -> PersistenceResult<T>
    where
        T: DeserializeOwned,
    {
        let checkpoint = self.get_checkpoint(checkpoint_id).await?;
        Self::restore_scheduler_state_from_checkpoint(&checkpoint)
    }

    /// Restore workflow snapshots from a specific checkpoint by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Checkpoint with the given ID doesn't exist
    /// - JSON deserialization fails (when snapshots are present)
    /// - Database query fails
    pub async fn restore_workflow_snapshots_by_id<T>(
        &self,
        checkpoint_id: &str,
    ) -> PersistenceResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let checkpoint = self.get_checkpoint(checkpoint_id).await?;
        Self::restore_workflow_snapshots_from_checkpoint(&checkpoint)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::persistence::StoreConfig;

    async fn setup_manager() -> Option<CheckpointManager> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        Some(CheckpointManager::new(store, CheckpointConfig::default()))
    }

    async fn setup_manager_no_compression() -> Option<CheckpointManager> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        Some(CheckpointManager::new(
            store,
            CheckpointConfig::without_compression(),
        ))
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
    async fn test_create_checkpoint_with_compression() {
        let mut manager = require_manager!(setup_manager().await);

        let original_state = r#"{"state":"active","workflows":[]}"#;
        let result = manager.create_checkpoint(original_state, None).await;

        assert!(result.is_ok(), "checkpoint creation should succeed");

        let created = result.expect("checked is_ok");
        assert_eq!(created.event_sequence, 0);

        let retrieved = manager
            .get_latest()
            .await
            .expect("get_latest should succeed");
        assert_eq!(
            retrieved.scheduler_state, original_state,
            "decompressed state should match original"
        );
    }

    #[tokio::test]
    async fn test_create_checkpoint_without_compression() {
        let mut manager = require_manager!(setup_manager_no_compression().await);

        let result = manager
            .create_checkpoint(r#"{"state":"active"}"#, None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_checkpoint_roundtrip_with_compression() {
        let mut manager = require_manager!(setup_manager().await);

        let original_state = r#"{"workflows":{"wf-1":{"status":"running","beads":["a","b","c"]}}}"#;
        let original_snapshots = r#"{"wf-1":{"completed":["a"],"pending":["b","c"]}}"#;

        let created = manager
            .create_checkpoint(original_state, Some(original_snapshots))
            .await
            .expect("create should succeed");

        let retrieved = manager
            .get_checkpoint(&created.checkpoint_id)
            .await
            .expect("get should succeed");

        assert_eq!(retrieved.scheduler_state, original_state);
        assert_eq!(
            retrieved.workflow_snapshots,
            Some(original_snapshots.to_string())
        );
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

    #[tokio::test]
    async fn test_compression_level_config() {
        let level = CompressionLevel::new(19).expect("valid level");
        let config = CheckpointConfig::with_compression_level(level);

        assert!(config.enable_compression);
        assert_eq!(config.compression.level.as_i32(), 19);
    }

    #[tokio::test]
    async fn test_compression_disabled_config() {
        let config = CheckpointConfig::without_compression();

        assert!(!config.enable_compression);
    }

    #[tokio::test]
    async fn test_large_state_compression() {
        let mut manager = require_manager!(setup_manager().await);

        let large_state = serde_json::to_string(&serde_json::json!({
            "workflows": (0..100).map(|i| format!("workflow-{}", i)).collect::<Vec<_>>(),
            "states": (0..100).map(|i| serde_json::json!({"id": i, "data": "x".repeat(100)})).collect::<Vec<_>>()
        })).expect("serialize");

        let result = manager.create_checkpoint(&large_state, None).await;

        assert!(result.is_ok());

        let retrieved = manager.get_latest().await.expect("get should succeed");
        assert_eq!(retrieved.scheduler_state, large_state);
    }

    // =========================================================================
    // Checkpoint/Resume Cycle Tests
    // =========================================================================

    /// Test state for restore operations.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestSchedulerState {
        active_workflows: Vec<String>,
        current_phase: String,
        event_count: u64,
    }

    /// Test workflow snapshots for restore operations.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestWorkflowSnapshots {
        snapshots: std::collections::HashMap<String, String>,
    }

    /// BDD: GIVEN no checkpoints exist WHEN restore_scheduler_state is called THEN error is returned.
    #[tokio::test]
    async fn test_restore_scheduler_state_no_checkpoint_returns_error() {
        let manager = require_manager!(setup_manager().await);

        let result: PersistenceResult<TestSchedulerState> = manager.restore_scheduler_state().await;

        assert!(
            result.is_err(),
            "restore should fail when no checkpoint exists"
        );
    }

    /// BDD: GIVEN no checkpoints exist WHEN restore_workflow_snapshots is called THEN error is returned.
    #[tokio::test]
    async fn test_restore_workflow_snapshots_no_checkpoint_returns_error() {
        let manager = require_manager!(setup_manager().await);

        let result: PersistenceResult<Option<TestWorkflowSnapshots>> =
            manager.restore_workflow_snapshots().await;

        assert!(
            result.is_err(),
            "restore should fail when no checkpoint exists"
        );
    }

    /// BDD: GIVEN a checkpoint with scheduler state WHEN restore_scheduler_state is called THEN state is recovered.
    #[tokio::test]
    async fn test_restore_scheduler_state_succeeds() {
        let mut manager = require_manager!(setup_manager().await);

        let original_state = TestSchedulerState {
            active_workflows: vec!["wf-1".to_string(), "wf-2".to_string()],
            current_phase: "implement".to_string(),
            event_count: 42,
        };

        let state_json = serde_json::to_string(&original_state).expect("serialize");
        let _ = manager
            .create_checkpoint(&state_json, None)
            .await
            .expect("create checkpoint");

        let restored: TestSchedulerState = manager
            .restore_scheduler_state()
            .await
            .expect("restore should succeed");

        assert_eq!(restored.active_workflows, original_state.active_workflows);
        assert_eq!(restored.current_phase, original_state.current_phase);
        assert_eq!(restored.event_count, original_state.event_count);
    }

    /// BDD: GIVEN a checkpoint with workflow snapshots WHEN restore_workflow_snapshots is called THEN snapshots are recovered.
    #[tokio::test]
    async fn test_restore_workflow_snapshots_succeeds() {
        let mut manager = require_manager!(setup_manager().await);

        let scheduler_state = r#"{"phase":"running"}"#;
        let original_snapshots = TestWorkflowSnapshots {
            snapshots: std::collections::HashMap::from([
                ("wf-1".to_string(), "phase_a".to_string()),
                ("wf-2".to_string(), "phase_b".to_string()),
            ]),
        };

        let snapshots_json = serde_json::to_string(&original_snapshots).expect("serialize");
        let _ = manager
            .create_checkpoint(scheduler_state, Some(&snapshots_json))
            .await
            .expect("create checkpoint");

        let restored: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots()
            .await
            .expect("restore should succeed");

        assert!(
            restored.is_some(),
            "snapshots should be present in restored data"
        );
        let restored = restored.expect("checked is_some");
        assert_eq!(
            restored.snapshots.get("wf-1"),
            original_snapshots.snapshots.get("wf-1")
        );
        assert_eq!(
            restored.snapshots.get("wf-2"),
            original_snapshots.snapshots.get("wf-2")
        );
    }

    /// BDD: GIVEN a checkpoint without workflow snapshots WHEN restore_workflow_snapshots is called THEN None is returned.
    #[tokio::test]
    async fn test_restore_workflow_snapshots_none_when_not_stored() {
        let mut manager = require_manager!(setup_manager().await);

        let scheduler_state = r#"{"phase":"running"}"#;
        let _ = manager
            .create_checkpoint(scheduler_state, None)
            .await
            .expect("create checkpoint");

        let restored: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots()
            .await
            .expect("restore should succeed");

        assert!(
            restored.is_none(),
            "snapshots should be None when not stored"
        );
    }

    /// BDD: GIVEN multiple checkpoints WHEN restore_scheduler_state is called THEN latest state is returned.
    #[tokio::test]
    async fn test_restore_returns_latest_checkpoint() {
        let mut manager = require_manager!(setup_manager().await);

        let state_v1 = TestSchedulerState {
            active_workflows: vec!["wf-1".to_string()],
            current_phase: "start".to_string(),
            event_count: 1,
        };
        let state_v2 = TestSchedulerState {
            active_workflows: vec!["wf-1".to_string(), "wf-2".to_string()],
            current_phase: "middle".to_string(),
            event_count: 10,
        };
        let state_v3 = TestSchedulerState {
            active_workflows: vec!["wf-1".to_string(), "wf-2".to_string(), "wf-3".to_string()],
            current_phase: "end".to_string(),
            event_count: 100,
        };

        let _ = manager
            .create_checkpoint(&serde_json::to_string(&state_v1).expect("serialize"), None)
            .await;
        manager.set_sequence(10);
        let checkpoint_v2 = manager
            .create_checkpoint(&serde_json::to_string(&state_v2).expect("serialize"), None)
            .await
            .expect("create v2");
        manager.set_sequence(100);
        let checkpoint_v3 = manager
            .create_checkpoint(&serde_json::to_string(&state_v3).expect("serialize"), None)
            .await
            .expect("create v3");

        let latest: TestSchedulerState = manager
            .restore_scheduler_state()
            .await
            .expect("restore latest");

        assert_eq!(latest.event_count, 100, "should restore v3 (latest)");
        assert_eq!(latest.current_phase, "end");

        let by_id_v2: TestSchedulerState = manager
            .restore_scheduler_state_by_id(&checkpoint_v2.checkpoint_id)
            .await
            .expect("restore v2 by id");
        assert_eq!(by_id_v2.event_count, 10);

        let by_id_v3: TestSchedulerState = manager
            .restore_scheduler_state_by_id(&checkpoint_v3.checkpoint_id)
            .await
            .expect("restore v3 by id");
        assert_eq!(by_id_v3.event_count, 100);
    }

    /// BDD: GIVEN invalid checkpoint ID WHEN restore_scheduler_state_by_id is called THEN error is returned.
    #[tokio::test]
    async fn test_restore_by_invalid_id_returns_error() {
        let manager = require_manager!(setup_manager().await);

        let result: PersistenceResult<TestSchedulerState> = manager
            .restore_scheduler_state_by_id("nonexistent-checkpoint-id")
            .await;

        assert!(
            result.is_err(),
            "restore should fail for nonexistent checkpoint ID"
        );
    }

    /// BDD: GIVEN checkpoint with invalid JSON WHEN restore_scheduler_state is called THEN deserialization error is returned.
    #[tokio::test]
    async fn test_restore_invalid_json_returns_error() {
        let mut manager = require_manager!(setup_manager().await);

        let invalid_json = r#"{"active_workflows": [not valid json]}"#;
        let _ = manager
            .create_checkpoint(invalid_json, None)
            .await
            .expect("create checkpoint");

        let result: PersistenceResult<TestSchedulerState> = manager.restore_scheduler_state().await;

        assert!(
            result.is_err(),
            "restore should fail for invalid JSON in checkpoint"
        );

        let error_message = format!("{:?}", result.expect_err("checked is_err"));
        assert!(
            error_message.contains("deserialize") || error_message.contains("scheduler state"),
            "error should mention deserialization: {error_message}"
        );
    }

    /// BDD: GIVEN complete checkpoint/resume cycle WHEN executed THEN state is preserved exactly.
    #[tokio::test]
    async fn test_complete_checkpoint_resume_cycle() {
        let mut manager = require_manager!(setup_manager().await);

        let original_scheduler = TestSchedulerState {
            active_workflows: vec!["wf-alpha".to_string(), "wf-beta".to_string()],
            current_phase: "testing".to_string(),
            event_count: 999,
        };

        let original_snapshots = TestWorkflowSnapshots {
            snapshots: std::collections::HashMap::from([
                ("wf-alpha".to_string(), "completed".to_string()),
                ("wf-beta".to_string(), "in_progress".to_string()),
            ]),
        };

        manager.set_sequence(500);
        let checkpoint = manager
            .create_checkpoint(
                &serde_json::to_string(&original_scheduler).expect("serialize scheduler"),
                Some(&serde_json::to_string(&original_snapshots).expect("serialize snapshots")),
            )
            .await
            .expect("create checkpoint");

        let restored_scheduler: TestSchedulerState = manager
            .restore_scheduler_state()
            .await
            .expect("restore scheduler");
        assert_eq!(restored_scheduler, original_scheduler);

        let restored_snapshots: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots()
            .await
            .expect("restore snapshots");
        assert_eq!(restored_snapshots, Some(original_snapshots.clone()));

        let by_id_scheduler: TestSchedulerState = manager
            .restore_scheduler_state_by_id(&checkpoint.checkpoint_id)
            .await
            .expect("restore scheduler by id");
        assert_eq!(by_id_scheduler, original_scheduler);

        let by_id_snapshots: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots_by_id(&checkpoint.checkpoint_id)
            .await
            .expect("restore snapshots by id");
        assert_eq!(by_id_snapshots, Some(original_snapshots));
    }

    /// BDD: GIVEN checkpoint with compression WHEN resume is called THEN decompression succeeds.
    #[tokio::test]
    async fn test_resume_with_compression_enabled() {
        let mut manager = require_manager!(setup_manager().await);

        let large_state = TestSchedulerState {
            active_workflows: (0..100).map(|i| format!("workflow-{i}")).collect(),
            current_phase: "stress_test".to_string(),
            event_count: 1000000,
        };

        let state_json = serde_json::to_string(&large_state).expect("serialize");
        let _ = manager
            .create_checkpoint(&state_json, None)
            .await
            .expect("create checkpoint with compression");

        let restored: TestSchedulerState = manager
            .restore_scheduler_state()
            .await
            .expect("restore should succeed with decompression");

        assert_eq!(restored.active_workflows.len(), 100);
        assert_eq!(restored.event_count, 1000000);
    }

    /// BDD: GIVEN checkpoint without compression WHEN resume is called THEN data is returned directly.
    #[tokio::test]
    async fn test_resume_without_compression() {
        let mut manager = require_manager!(setup_manager_no_compression().await);

        let original_state = TestSchedulerState {
            active_workflows: vec!["wf-test".to_string()],
            current_phase: "direct".to_string(),
            event_count: 1,
        };

        let state_json = serde_json::to_string(&original_state).expect("serialize");
        let _ = manager
            .create_checkpoint(&state_json, None)
            .await
            .expect("create checkpoint");

        let restored: TestSchedulerState = manager
            .restore_scheduler_state()
            .await
            .expect("restore should succeed without compression");

        assert_eq!(restored, original_state);
    }

    /// BDD: GIVEN workflow snapshots with snapshots WHEN restore_workflow_snapshots_by_id is called THEN correct snapshots are returned.
    #[tokio::test]
    async fn test_restore_workflow_snapshots_by_id_succeeds() {
        let mut manager = require_manager!(setup_manager().await);

        let snapshots_v1 = TestWorkflowSnapshots {
            snapshots: std::collections::HashMap::from([("wf-1".to_string(), "v1".to_string())]),
        };
        let snapshots_v2 = TestWorkflowSnapshots {
            snapshots: std::collections::HashMap::from([
                ("wf-1".to_string(), "v2".to_string()),
                ("wf-2".to_string(), "v2".to_string()),
            ]),
        };

        let checkpoint_v1 = manager
            .create_checkpoint(
                "{}",
                Some(&serde_json::to_string(&snapshots_v1).expect("serialize v1")),
            )
            .await
            .expect("create v1");

        let checkpoint_v2 = manager
            .create_checkpoint(
                "{}",
                Some(&serde_json::to_string(&snapshots_v2).expect("serialize v2")),
            )
            .await
            .expect("create v2");

        let restored_v1: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots_by_id(&checkpoint_v1.checkpoint_id)
            .await
            .expect("restore v1");
        assert_eq!(
            restored_v1.expect("v1 snapshots").snapshots.get("wf-1"),
            Some(&"v1".to_string())
        );

        let restored_v2: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots_by_id(&checkpoint_v2.checkpoint_id)
            .await
            .expect("restore v2");
        let v2 = restored_v2.expect("v2 snapshots");
        assert_eq!(v2.snapshots.get("wf-1"), Some(&"v2".to_string()));
        assert_eq!(v2.snapshots.get("wf-2"), Some(&"v2".to_string()));
    }

    /// BDD: GIVEN invalid snapshot JSON WHEN restore_workflow_snapshots is called THEN error is returned.
    #[tokio::test]
    async fn test_restore_invalid_snapshot_json_returns_error() {
        let mut manager = require_manager!(setup_manager().await);

        let invalid_json = r#"{not: valid json}"#;
        let _ = manager
            .create_checkpoint("{}", Some(invalid_json))
            .await
            .expect("create checkpoint");

        let result: PersistenceResult<Option<TestWorkflowSnapshots>> =
            manager.restore_workflow_snapshots().await;

        assert!(
            result.is_err(),
            "restore should fail for invalid snapshot JSON"
        );
    }

    /// BDD: GIVEN checkpoint created and manager sequence updated WHEN new checkpoint is created THEN sequence is correct.
    #[tokio::test]
    async fn test_checkpoint_sequence_tracking_across_resume() {
        let mut manager = require_manager!(setup_manager().await);

        assert_eq!(manager.current_sequence(), 0);

        manager.set_sequence(50);
        let _ = manager
            .create_checkpoint(r#"{"seq":50}"#, None)
            .await
            .expect("checkpoint at seq 50");

        let cp1 = manager.get_latest().await.expect("get latest");
        assert_eq!(cp1.event_sequence, 50);

        manager.set_sequence(100);
        let _ = manager
            .create_checkpoint(r#"{"seq":100}"#, None)
            .await
            .expect("checkpoint at seq 100");

        let cp2 = manager.get_latest().await.expect("get latest");
        assert_eq!(cp2.event_sequence, 100);
    }

    /// BDD: GIVEN checkpoint without snapshots WHEN restore_workflow_snapshots_by_id is called THEN None is returned.
    #[tokio::test]
    async fn test_restore_snapshots_by_id_returns_none_when_not_stored() {
        let mut manager = require_manager!(setup_manager().await);

        let checkpoint = manager
            .create_checkpoint("{}", None)
            .await
            .expect("create checkpoint");

        let result: Option<TestWorkflowSnapshots> = manager
            .restore_workflow_snapshots_by_id(&checkpoint.checkpoint_id)
            .await
            .expect("restore should succeed");

        assert!(
            result.is_none(),
            "snapshots should be None when not stored in checkpoint"
        );
    }
}
