//! Bead persistence operations.
//!
//! CRUD operations for bead records in SurrealDB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use super::client::OrchestratorStore;
use super::error::{from_surrealdb_error, PersistenceError, PersistenceResult};

/// Bead state in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadState {
    /// Bead is pending (waiting for dependencies)
    Pending,
    /// Bead is ready to be scheduled
    Ready,
    /// Bead has been dispatched to a queue
    Dispatched,
    /// Bead is assigned to a worker
    Assigned,
    /// Bead is currently running
    Running,
    /// Bead completed successfully
    Completed,
    /// Bead failed
    Failed,
    /// Bead was cancelled
    Cancelled,
}

impl Default for BeadState {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for BeadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Ready => write!(f, "ready"),
            Self::Dispatched => write!(f, "dispatched"),
            Self::Assigned => write!(f, "assigned"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl BeadState {
    /// Check if the state is terminal.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the state is active (in progress).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Dispatched | Self::Assigned | Self::Running)
    }
}

/// Bead record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadRecord {
    /// SurrealDB record ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    /// Bead identifier
    pub bead_id: String,
    /// Workflow this bead belongs to
    pub workflow_id: String,
    /// Current state
    pub state: BeadState,
    /// Assigned worker (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_worker: Option<String>,
    /// Assigned queue (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_queue: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// When execution started
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When execution completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl BeadRecord {
    /// Create a new bead record.
    #[must_use]
    pub fn new(bead_id: impl Into<String>, workflow_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            bead_id: bead_id.into(),
            workflow_id: workflow_id.into(),
            state: BeadState::Pending,
            assigned_worker: None,
            assigned_queue: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            error_message: None,
            retry_count: 0,
            metadata: None,
        }
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Input for creating/updating a bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeadInput {
    id: String,
    workflow_id: String,
    state: String,
    assigned_worker: Option<String>,
    assigned_queue: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    retry_count: u32,
    metadata: Option<serde_json::Value>,
}

impl From<&BeadRecord> for BeadInput {
    fn from(record: &BeadRecord) -> Self {
        Self {
            id: record.bead_id.clone(),
            workflow_id: record.workflow_id.clone(),
            state: record.state.to_string(),
            assigned_worker: record.assigned_worker.clone(),
            assigned_queue: record.assigned_queue.clone(),
            created_at: record.created_at,
            updated_at: record.updated_at,
            started_at: record.started_at,
            completed_at: record.completed_at,
            error_message: record.error_message.clone(),
            retry_count: record.retry_count,
            metadata: record.metadata.clone(),
        }
    }
}

impl OrchestratorStore {
    /// Save a bead record to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn save_bead(&self, record: &BeadRecord) -> PersistenceResult<BeadRecord> {
        let input = BeadInput::from(record);

        let result: Option<BeadRecord> = self
            .db()
            .upsert(("bead", &record.bead_id))
            .content(input)
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::query_failed("failed to save bead"))
    }

    /// Get a bead by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the bead is not found or the query fails.
    pub async fn get_bead(&self, bead_id: &str) -> PersistenceResult<BeadRecord> {
        let result: Option<BeadRecord> = self
            .db()
            .select(("bead", bead_id))
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("bead", bead_id))
    }

    /// Update a bead's state.
    ///
    /// # Errors
    ///
    /// Returns an error if the bead is not found or the update fails.
    pub async fn update_bead_state(
        &self,
        bead_id: &str,
        state: BeadState,
    ) -> PersistenceResult<BeadRecord> {
        let now = Utc::now();

        // Set timestamps based on state transition
        let (started_at, completed_at) = match state {
            BeadState::Running => (Some(now), None),
            BeadState::Completed | BeadState::Failed | BeadState::Cancelled => (None, Some(now)),
            _ => (None, None),
        };

        let mut query = String::from("UPDATE bead SET state = $state, updated_at = $updated_at");

        if started_at.is_some() {
            query.push_str(", started_at = $started_at");
        }
        if completed_at.is_some() {
            query.push_str(", completed_at = $completed_at");
        }

        query.push_str(" WHERE id = $id RETURN AFTER");

        let result: Option<BeadRecord> = self
            .db()
            .query(&query)
            .bind(("id", format!("bead:{}", bead_id)))
            .bind(("state", state.to_string()))
            .bind(("updated_at", now))
            .bind(("started_at", started_at))
            .bind(("completed_at", completed_at))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("bead", bead_id))
    }

    /// Get all beads for a workflow.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_beads_by_workflow(&self, workflow_id: &str) -> PersistenceResult<Vec<BeadRecord>> {
        let workflow_id_owned = workflow_id.to_string();
        let beads: Vec<BeadRecord> = self
            .db()
            .query("SELECT * FROM bead WHERE workflow_id = $workflow_id ORDER BY created_at")
            .bind(("workflow_id", workflow_id_owned))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        Ok(beads)
    }

    /// Get beads by state.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_beads_by_state(&self, state: BeadState) -> PersistenceResult<Vec<BeadRecord>> {
        let beads: Vec<BeadRecord> = self
            .db()
            .query("SELECT * FROM bead WHERE state = $state ORDER BY created_at")
            .bind(("state", state.to_string()))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        Ok(beads)
    }

    /// Assign a bead to a worker.
    ///
    /// # Errors
    ///
    /// Returns an error if the bead is not found or the update fails.
    pub async fn assign_bead_to_worker(
        &self,
        bead_id: &str,
        worker_id: &str,
    ) -> PersistenceResult<BeadRecord> {
        let now = Utc::now();
        let worker_id_owned = worker_id.to_string();
        let bead_id_owned = bead_id.to_string();

        let result: Option<BeadRecord> = self
            .db()
            .query("UPDATE bead SET assigned_worker = $worker, state = $state, updated_at = $updated_at WHERE id = $id RETURN AFTER")
            .bind(("id", format!("bead:{}", bead_id_owned)))
            .bind(("worker", worker_id_owned))
            .bind(("state", BeadState::Assigned.to_string()))
            .bind(("updated_at", now))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("bead", bead_id))
    }

    /// Delete a bead.
    ///
    /// # Errors
    ///
    /// Returns an error if the bead is not found or the delete fails.
    pub async fn delete_bead(&self, bead_id: &str) -> PersistenceResult<()> {
        let result: Option<BeadRecord> = self
            .db()
            .delete(("bead", bead_id))
            .await
            .map_err(from_surrealdb_error)?;

        if result.is_some() {
            Ok(())
        } else {
            Err(PersistenceError::not_found("bead", bead_id))
        }
    }

    /// Mark a bead as failed with an error message.
    ///
    /// # Errors
    ///
    /// Returns an error if the bead is not found or the update fails.
    pub async fn mark_bead_failed(
        &self,
        bead_id: &str,
        error_message: &str,
    ) -> PersistenceResult<BeadRecord> {
        let now = Utc::now();
        let error_message_owned = error_message.to_string();
        let bead_id_owned = bead_id.to_string();

        let result: Option<BeadRecord> = self
            .db()
            .query("UPDATE bead SET state = $state, error_message = $error, completed_at = $completed_at, updated_at = $updated_at WHERE id = $id RETURN AFTER")
            .bind(("id", format!("bead:{}", bead_id_owned)))
            .bind(("state", BeadState::Failed.to_string()))
            .bind(("error", error_message_owned))
            .bind(("completed_at", now))
            .bind(("updated_at", now))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("bead", bead_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::client::StoreConfig;

    async fn setup_store() -> OrchestratorStore {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok().unwrap();
        let _ = store.initialize_schema().await;
        store
    }

    #[tokio::test]
    async fn test_save_and_get_bead() {
        let store = setup_store().await;

        let record = BeadRecord::new("bead-001", "wf-001");
        let saved = store.save_bead(&record).await;
        assert!(saved.is_ok(), "save should succeed");

        let retrieved = store.get_bead("bead-001").await;
        assert!(retrieved.is_ok(), "get should succeed");

        if let Ok(bead) = retrieved {
            assert_eq!(bead.bead_id, "bead-001");
            assert_eq!(bead.workflow_id, "wf-001");
            assert_eq!(bead.state, BeadState::Pending);
        }
    }

    #[tokio::test]
    async fn test_update_bead_state() {
        let store = setup_store().await;

        let record = BeadRecord::new("bead-update", "wf-001");
        let _ = store.save_bead(&record).await;

        let updated = store.update_bead_state("bead-update", BeadState::Running).await;
        assert!(updated.is_ok(), "update should succeed");

        if let Ok(bead) = updated {
            assert_eq!(bead.state, BeadState::Running);
            assert!(bead.started_at.is_some(), "started_at should be set");
        }
    }

    #[tokio::test]
    async fn test_get_beads_by_workflow() {
        let store = setup_store().await;

        let b1 = BeadRecord::new("bead-wf-1", "wf-test");
        let b2 = BeadRecord::new("bead-wf-2", "wf-test");
        let b3 = BeadRecord::new("bead-wf-3", "wf-other");

        let _ = store.save_bead(&b1).await;
        let _ = store.save_bead(&b2).await;
        let _ = store.save_bead(&b3).await;

        let beads = store.get_beads_by_workflow("wf-test").await;
        assert!(beads.is_ok(), "query should succeed");

        if let Ok(list) = beads {
            assert_eq!(list.len(), 2, "should have 2 beads for wf-test");
        }
    }

    #[tokio::test]
    async fn test_bead_state_is_terminal() {
        assert!(!BeadState::Pending.is_terminal());
        assert!(!BeadState::Ready.is_terminal());
        assert!(!BeadState::Running.is_terminal());
        assert!(BeadState::Completed.is_terminal());
        assert!(BeadState::Failed.is_terminal());
        assert!(BeadState::Cancelled.is_terminal());
    }

    #[tokio::test]
    async fn test_bead_state_is_active() {
        assert!(!BeadState::Pending.is_active());
        assert!(!BeadState::Ready.is_active());
        assert!(BeadState::Dispatched.is_active());
        assert!(BeadState::Assigned.is_active());
        assert!(BeadState::Running.is_active());
        assert!(!BeadState::Completed.is_active());
    }

    #[tokio::test]
    async fn test_delete_bead() {
        let store = setup_store().await;

        let record = BeadRecord::new("bead-delete", "wf-001");
        let _ = store.save_bead(&record).await;

        let delete_result = store.delete_bead("bead-delete").await;
        assert!(delete_result.is_ok(), "delete should succeed");

        let get_result = store.get_bead("bead-delete").await;
        assert!(get_result.is_err(), "get after delete should fail");
    }

    #[tokio::test]
    async fn test_assign_bead_to_worker() {
        let store = setup_store().await;

        let record = BeadRecord::new("bead-assign", "wf-001");
        let _ = store.save_bead(&record).await;

        let assigned = store.assign_bead_to_worker("bead-assign", "worker-001").await;
        assert!(assigned.is_ok(), "assign should succeed");

        if let Ok(bead) = assigned {
            assert_eq!(bead.assigned_worker, Some("worker-001".to_string()));
            assert_eq!(bead.state, BeadState::Assigned);
        }
    }

    #[tokio::test]
    async fn test_mark_bead_failed() {
        let store = setup_store().await;

        let record = BeadRecord::new("bead-fail", "wf-001");
        let _ = store.save_bead(&record).await;

        let failed = store.mark_bead_failed("bead-fail", "Test error").await;
        assert!(failed.is_ok(), "mark failed should succeed");

        if let Ok(bead) = failed {
            assert_eq!(bead.state, BeadState::Failed);
            assert_eq!(bead.error_message, Some("Test error".to_string()));
            assert!(bead.completed_at.is_some());
        }
    }
}
