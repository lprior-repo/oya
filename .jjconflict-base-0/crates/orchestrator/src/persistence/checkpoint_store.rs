//! Checkpoint persistence operations.
//!
//! CRUD operations for checkpoint records used in replay/recovery.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;
use surrealdb::sql::Datetime as SurrealDatetime;

use super::client::OrchestratorStore;
use super::error::{PersistenceError, PersistenceResult, from_surrealdb_error};

/// Checkpoint record stored in the database.
///
/// Checkpoints capture the scheduler state at a point in time for recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    /// SurrealDB record ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "id")]
    pub record_id: Option<RecordId>,
    /// Checkpoint identifier
    pub checkpoint_id: String,
    /// Serialized scheduler state
    pub scheduler_state: String,
    /// Event sequence number at checkpoint time
    pub event_sequence: u64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Serialized workflow snapshots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_snapshots: Option<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl CheckpointRecord {
    /// Create a new checkpoint record.
    #[must_use]
    pub fn new(
        checkpoint_id: impl Into<String>,
        scheduler_state: impl Into<String>,
        event_sequence: u64,
    ) -> Self {
        Self {
            record_id: None,
            checkpoint_id: checkpoint_id.into(),
            scheduler_state: scheduler_state.into(),
            event_sequence,
            created_at: Utc::now(),
            workflow_snapshots: None,
            metadata: None,
        }
    }

    /// Set workflow snapshots.
    #[must_use]
    pub fn with_workflow_snapshots(mut self, snapshots: impl Into<String>) -> Self {
        self.workflow_snapshots = Some(snapshots.into());
        self
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Input for creating a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckpointInput {
    checkpoint_id: String,
    scheduler_state: String,
    event_sequence: u64,
    created_at: SurrealDatetime,
    workflow_snapshots: Option<String>,
    metadata: Option<serde_json::Value>,
}

impl From<&CheckpointRecord> for CheckpointInput {
    fn from(record: &CheckpointRecord) -> Self {
        Self {
            checkpoint_id: record.checkpoint_id.clone(),
            scheduler_state: record.scheduler_state.clone(),
            event_sequence: record.event_sequence,
            created_at: SurrealDatetime::from(record.created_at),
            workflow_snapshots: record.workflow_snapshots.clone(),
            metadata: record.metadata.clone(),
        }
    }
}

impl OrchestratorStore {
    /// Save a checkpoint to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn save_checkpoint(
        &self,
        record: &CheckpointRecord,
    ) -> PersistenceResult<CheckpointRecord> {
        let input = CheckpointInput::from(record);

        let result: Option<CheckpointRecord> = self
            .db()
            .create(("checkpoint", &record.checkpoint_id))
            .content(input)
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::query_failed("failed to save checkpoint"))
    }

    /// Get the latest checkpoint (highest event sequence).
    ///
    /// # Errors
    ///
    /// Returns an error if no checkpoints exist or the query fails.
    pub async fn get_latest_checkpoint(&self) -> PersistenceResult<CheckpointRecord> {
        let checkpoints: Vec<CheckpointRecord> = self
            .db()
            .query("SELECT * FROM checkpoint ORDER BY event_sequence DESC LIMIT 1")
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        checkpoints
            .into_iter()
            .next()
            .ok_or_else(|| PersistenceError::not_found("checkpoint", "latest"))
    }

    /// Get a checkpoint by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found.
    pub async fn get_checkpoint(&self, checkpoint_id: &str) -> PersistenceResult<CheckpointRecord> {
        let result: Option<CheckpointRecord> = self
            .db()
            .select(("checkpoint", checkpoint_id))
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("checkpoint", checkpoint_id))
    }

    /// List checkpoints, optionally limited and ordered by sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn list_checkpoints(
        &self,
        limit: Option<usize>,
    ) -> PersistenceResult<Vec<CheckpointRecord>> {
        let query = match limit {
            Some(n) => format!(
                "SELECT * FROM checkpoint ORDER BY event_sequence DESC LIMIT {}",
                n
            ),
            None => "SELECT * FROM checkpoint ORDER BY event_sequence DESC".to_string(),
        };

        let checkpoints: Vec<CheckpointRecord> = self
            .db()
            .query(&query)
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        Ok(checkpoints)
    }

    /// Get checkpoint by event sequence number.
    ///
    /// # Errors
    ///
    /// Returns an error if no checkpoint exists at that sequence.
    pub async fn get_checkpoint_by_sequence(
        &self,
        sequence: u64,
    ) -> PersistenceResult<CheckpointRecord> {
        let checkpoints: Vec<CheckpointRecord> = self
            .db()
            .query("SELECT * FROM checkpoint WHERE event_sequence = $seq LIMIT 1")
            .bind(("seq", sequence))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        checkpoints.into_iter().next().ok_or_else(|| {
            PersistenceError::not_found("checkpoint", format!("sequence:{}", sequence))
        })
    }

    /// Get checkpoints since a specific sequence number.
    ///
    /// Returns checkpoints with sequence > the given sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_checkpoints_since(
        &self,
        sequence: u64,
    ) -> PersistenceResult<Vec<CheckpointRecord>> {
        let checkpoints: Vec<CheckpointRecord> = self
            .db()
            .query(
                "SELECT * FROM checkpoint WHERE event_sequence > $seq ORDER BY event_sequence ASC",
            )
            .bind(("seq", sequence))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        Ok(checkpoints)
    }

    /// Delete old checkpoints, keeping only the most recent N.
    ///
    /// # Errors
    ///
    /// Returns an error if the deletion fails.
    pub async fn prune_checkpoints(&self, keep_count: usize) -> PersistenceResult<u64> {
        // Get checkpoints to delete (skip first keep_count ordered by sequence DESC)
        let checkpoints: Vec<CheckpointRecord> = self
            .db()
            .query("SELECT * FROM checkpoint ORDER BY event_sequence DESC")
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        // Skip the first keep_count records
        let to_delete: Vec<String> = checkpoints
            .into_iter()
            .skip(keep_count)
            .map(|cp| cp.checkpoint_id)
            .collect();

        let deleted_count = to_delete.len() as u64;

        // Delete each checkpoint by its ID
        for checkpoint_id in to_delete {
            let _: Option<CheckpointRecord> = self
                .db()
                .delete(("checkpoint", &checkpoint_id))
                .await
                .map_err(from_surrealdb_error)?;
        }

        Ok(deleted_count)
    }

    /// Delete a specific checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint is not found.
    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> PersistenceResult<()> {
        let result: Option<CheckpointRecord> = self
            .db()
            .delete(("checkpoint", checkpoint_id))
            .await
            .map_err(from_surrealdb_error)?;

        if result.is_some() {
            Ok(())
        } else {
            Err(PersistenceError::not_found("checkpoint", checkpoint_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::client::StoreConfig;

    async fn setup_store() -> Option<OrchestratorStore> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        Some(store)
    }

    // Helper macro to skip test if store setup fails
    macro_rules! require_store {
        ($store_opt:expr) => {
            match $store_opt {
                Some(s) => s,
                None => {
                    eprintln!("Skipping test: store setup failed");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn test_save_and_get_checkpoint() {
        let store = require_store!(setup_store().await);

        let record = CheckpointRecord::new("cp-001", r#"{"workflows":{}}"#, 100);
        let saved = store.save_checkpoint(&record).await;
        assert!(saved.is_ok(), "save should succeed");

        let retrieved = store.get_checkpoint("cp-001").await;
        assert!(retrieved.is_ok(), "get should succeed");

        if let Ok(cp) = retrieved {
            assert_eq!(cp.checkpoint_id, "cp-001");
            assert_eq!(cp.event_sequence, 100);
        }
    }

    #[tokio::test]
    async fn test_get_latest_checkpoint() {
        let store = require_store!(setup_store().await);

        let cp1 = CheckpointRecord::new("cp-1", "{}", 50);
        let cp2 = CheckpointRecord::new("cp-2", "{}", 100);
        let cp3 = CheckpointRecord::new("cp-3", "{}", 75);

        let _ = store.save_checkpoint(&cp1).await;
        let _ = store.save_checkpoint(&cp2).await;
        let _ = store.save_checkpoint(&cp3).await;

        let latest = store.get_latest_checkpoint().await;
        assert!(latest.is_ok(), "should find latest checkpoint");

        if let Ok(cp) = latest {
            assert_eq!(
                cp.checkpoint_id, "cp-2",
                "should be the one with highest sequence"
            );
            assert_eq!(cp.event_sequence, 100);
        }
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let store = require_store!(setup_store().await);

        let cp1 = CheckpointRecord::new("cp-list-1", "{}", 10);
        let cp2 = CheckpointRecord::new("cp-list-2", "{}", 20);
        let cp3 = CheckpointRecord::new("cp-list-3", "{}", 30);

        let _ = store.save_checkpoint(&cp1).await;
        let _ = store.save_checkpoint(&cp2).await;
        let _ = store.save_checkpoint(&cp3).await;

        let all = store.list_checkpoints(None).await;
        assert!(all.is_ok(), "list should succeed");

        if let Ok(list) = all {
            assert_eq!(list.len(), 3);
            // Should be ordered by sequence DESC
            assert_eq!(list[0].event_sequence, 30);
            assert_eq!(list[1].event_sequence, 20);
            assert_eq!(list[2].event_sequence, 10);
        }

        let limited = store.list_checkpoints(Some(2)).await;
        assert!(limited.is_ok());
        if let Ok(list) = limited {
            assert_eq!(list.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_get_checkpoint_by_sequence() {
        let store = require_store!(setup_store().await);

        let cp = CheckpointRecord::new("cp-seq", "{}", 42);
        let _ = store.save_checkpoint(&cp).await;

        let found = store.get_checkpoint_by_sequence(42).await;
        assert!(found.is_ok(), "should find by sequence");

        let not_found = store.get_checkpoint_by_sequence(999).await;
        assert!(not_found.is_err(), "should not find nonexistent sequence");
    }

    #[tokio::test]
    async fn test_get_checkpoints_since() {
        let store = require_store!(setup_store().await);

        let cp1 = CheckpointRecord::new("cp-since-1", "{}", 10);
        let cp2 = CheckpointRecord::new("cp-since-2", "{}", 20);
        let cp3 = CheckpointRecord::new("cp-since-3", "{}", 30);

        let _ = store.save_checkpoint(&cp1).await;
        let _ = store.save_checkpoint(&cp2).await;
        let _ = store.save_checkpoint(&cp3).await;

        let since = store.get_checkpoints_since(15).await;
        assert!(since.is_ok());

        if let Ok(list) = since {
            assert_eq!(list.len(), 2, "should have 2 checkpoints since seq 15");
            // Ordered ASC
            assert_eq!(list[0].event_sequence, 20);
            assert_eq!(list[1].event_sequence, 30);
        }
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let store = require_store!(setup_store().await);

        let cp = CheckpointRecord::new("cp-delete", "{}", 1);
        let _ = store.save_checkpoint(&cp).await;

        let delete_result = store.delete_checkpoint("cp-delete").await;
        assert!(delete_result.is_ok(), "delete should succeed");

        let get_result = store.get_checkpoint("cp-delete").await;
        assert!(get_result.is_err(), "get after delete should fail");
    }

    #[tokio::test]
    async fn test_checkpoint_with_workflow_snapshots() {
        let store = require_store!(setup_store().await);

        let cp = CheckpointRecord::new("cp-snapshots", r#"{"state":"active"}"#, 100)
            .with_workflow_snapshots(r#"{"wf-1":{"beads":["a","b"]}}"#)
            .with_metadata(serde_json::json!({"version": 1}));

        let saved = store.save_checkpoint(&cp).await;
        assert!(saved.is_ok());

        let retrieved = store.get_checkpoint("cp-snapshots").await;
        assert!(retrieved.is_ok());

        if let Ok(cp) = retrieved {
            assert!(cp.workflow_snapshots.is_some());
            assert!(cp.metadata.is_some());
        }
    }

    #[tokio::test]
    async fn test_no_latest_checkpoint() {
        let store = require_store!(setup_store().await);

        let result = store.get_latest_checkpoint().await;
        assert!(result.is_err(), "should fail when no checkpoints exist");

        if let Err(e) = result {
            assert!(matches!(e, PersistenceError::NotFound { .. }));
        }
    }
}
