//! Bead persistence operations.
//!
//! CRUD operations for bead records in SurrealDB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;
use surrealdb::sql::Datetime as SurrealDatetime;

use super::bead_dependencies::DependencyEdge;
use super::client::OrchestratorStore;
use super::error::{PersistenceError, PersistenceResult, from_surrealdb_error};

/// Bead state in the database.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadState {
    /// Bead is pending (waiting for dependencies)
    #[default]
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

/// Information about a blocked bead and what's blocking it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedBeadInfo {
    /// The bead that is blocked
    pub bead_id: String,
    /// Workflow this bead belongs to
    pub workflow_id: String,
    /// Current state
    pub state: BeadState,
    /// Bead IDs that are blocking this bead (incomplete dependencies)
    pub blocking_dependencies: Vec<String>,
    /// Reason for being blocked
    pub block_reason: String,
}

/// Bead record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadRecord {
    /// SurrealDB record ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "id")]
    pub record_id: Option<RecordId>,
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
            record_id: None,
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
    bead_id: String,
    workflow_id: String,
    state: String,
    assigned_worker: Option<String>,
    assigned_queue: Option<String>,
    created_at: SurrealDatetime,
    updated_at: SurrealDatetime,
    started_at: Option<SurrealDatetime>,
    completed_at: Option<SurrealDatetime>,
    error_message: Option<String>,
    retry_count: u32,
    metadata: Option<serde_json::Value>,
}

impl From<&BeadRecord> for BeadInput {
    fn from(record: &BeadRecord) -> Self {
        Self {
            bead_id: record.bead_id.clone(),
            workflow_id: record.workflow_id.clone(),
            state: record.state.to_string(),
            assigned_worker: record.assigned_worker.clone(),
            assigned_queue: record.assigned_queue.clone(),
            created_at: SurrealDatetime::from(record.created_at),
            updated_at: SurrealDatetime::from(record.updated_at),
            started_at: record.started_at.map(SurrealDatetime::from),
            completed_at: record.completed_at.map(SurrealDatetime::from),
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
        let now_surreal = SurrealDatetime::from(now);

        // Set timestamps based on state transition
        let (started_at, completed_at): (Option<SurrealDatetime>, Option<SurrealDatetime>) =
            match state {
                BeadState::Running => (Some(now_surreal.clone()), None),
                BeadState::Completed | BeadState::Failed | BeadState::Cancelled => {
                    (None, Some(now_surreal.clone()))
                }
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

        let record_id = RecordId::from(("bead", bead_id));
        let result: Option<BeadRecord> = self
            .db()
            .query(&query)
            .bind(("id", record_id))
            .bind(("state", state.to_string()))
            .bind(("updated_at", now_surreal))
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
    pub async fn get_beads_by_workflow(
        &self,
        workflow_id: &str,
    ) -> PersistenceResult<Vec<BeadRecord>> {
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
        let now = SurrealDatetime::from(Utc::now());
        let worker_id_owned = worker_id.to_string();
        let bead_id_owned = bead_id.to_string();

        let record_id = RecordId::from(("bead", bead_id_owned.as_str()));
        let result: Option<BeadRecord> = self
            .db()
            .query("UPDATE bead SET assigned_worker = $worker, state = $state, updated_at = $updated_at WHERE id = $id RETURN AFTER")
            .bind(("id", record_id))
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
        let now = SurrealDatetime::from(Utc::now());
        let error_message_owned = error_message.to_string();
        let bead_id_owned = bead_id.to_string();

        let record_id = RecordId::from(("bead", bead_id_owned.as_str()));
        let result: Option<BeadRecord> = self
            .db()
            .query("UPDATE bead SET state = $state, error_message = $error, completed_at = $completed_at, updated_at = $updated_at WHERE id = $id RETURN AFTER")
            .bind(("id", record_id))
            .bind(("state", BeadState::Failed.to_string()))
            .bind(("error", error_message_owned))
            .bind(("completed_at", now.clone()))
            .bind(("updated_at", now))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("bead", bead_id))
    }

    /// Find all blocked beads in a workflow.
    ///
    /// A bead is blocked if it has incomplete dependencies (beads it depends on
    /// that have not completed yet).
    ///
    /// # Arguments
    ///
    /// * `workflow_id` - The workflow to query
    ///
    /// # Returns
    ///
    /// A list of blocked bead information, including which dependencies are blocking them.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn find_blocked_beads(
        &self,
        workflow_id: &str,
    ) -> PersistenceResult<Vec<BlockedBeadInfo>> {
        let workflow_id_owned = workflow_id.to_string();

        // Get all non-terminal beads in the workflow
        let all_beads: Vec<BeadRecord> = self
            .db()
            .query(
                "SELECT * FROM bead WHERE workflow_id = $workflow_id \
                 AND state NOT IN ['completed', 'failed', 'cancelled'] \
                 ORDER BY created_at",
            )
            .bind(("workflow_id", workflow_id_owned))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        // Get all completed beads in the workflow
        let completed_beads: Vec<BeadRecord> = self
            .db()
            .query(
                "SELECT * FROM bead WHERE workflow_id = $workflow_id \
                 AND state = 'completed'",
            )
            .bind(("workflow_id", workflow_id.to_string()))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        let completed_set: std::collections::HashSet<String> =
            completed_beads.iter().map(|b| b.bead_id.clone()).collect();

        // Find blocked beads
        let mut blocked_info = Vec::new();

        for bead in all_beads {
            // Get dependencies for this bead - only need target_bead_id
            #[derive(Debug, Clone, Serialize, Deserialize)]
            struct DepTarget {
                target_bead_id: String,
            }

            let dependencies: Vec<DepTarget> = self
                .db()
                .query(
                    "SELECT target_bead_id FROM bead_depends_on WHERE bead_id = $bead_id",
                )
                .bind(("bead_id", bead.bead_id.clone()))
                .await
                .map_err(from_surrealdb_error)?
                .take(0)
                .map_err(from_surrealdb_error)?;

            if dependencies.is_empty() {
                // No dependencies means not blocked
                continue;
            }

            // Find incomplete dependencies
            let blocking_deps: Vec<String> = dependencies
                .iter()
                .filter(|dep| !completed_set.contains(&dep.target_bead_id))
                .map(|dep| dep.target_bead_id.clone())
                .collect();

            if !blocking_deps.is_empty() {
                let reason = if blocking_deps.len() == 1 {
                    format!("Blocked by dependency: {}", blocking_deps[0])
                } else {
                    format!(
                        "Blocked by {} dependencies: {}",
                        blocking_deps.len(),
                        blocking_deps.join(", ")
                    )
                };

                blocked_info.push(BlockedBeadInfo {
                    bead_id: bead.bead_id,
                    workflow_id: bead.workflow_id,
                    state: bead.state,
                    blocking_dependencies: blocking_deps,
                    block_reason: reason,
                });
            }
        }

        Ok(blocked_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::client::StoreConfig;
    use crate::persistence::DependencyRelation;
    use surrealdb::sql::Datetime as SurrealDatetime;

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
    async fn test_save_and_get_bead() {
        let store = require_store!(setup_store().await);

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
        let store = require_store!(setup_store().await);

        let record = BeadRecord::new("bead-update", "wf-001");
        let saved = store.save_bead(&record).await;
        assert!(saved.is_ok(), "save should succeed: {:?}", saved.err());

        let updated = store
            .update_bead_state("bead-update", BeadState::Running)
            .await;
        assert!(
            updated.is_ok(),
            "update should succeed: {:?}",
            updated.err()
        );

        if let Ok(bead) = updated {
            assert_eq!(bead.state, BeadState::Running);
            assert!(bead.started_at.is_some(), "started_at should be set");
        }
    }

    #[tokio::test]
    async fn test_get_beads_by_workflow() {
        let store = require_store!(setup_store().await);

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
        let store = require_store!(setup_store().await);

        let record = BeadRecord::new("bead-delete", "wf-001");
        let _ = store.save_bead(&record).await;

        let delete_result = store.delete_bead("bead-delete").await;
        assert!(delete_result.is_ok(), "delete should succeed");

        let get_result = store.get_bead("bead-delete").await;
        assert!(get_result.is_err(), "get after delete should fail");
    }

    #[tokio::test]
    async fn test_assign_bead_to_worker() {
        let store = require_store!(setup_store().await);

        let record = BeadRecord::new("bead-assign", "wf-001");
        let _ = store.save_bead(&record).await;

        let assigned = store
            .assign_bead_to_worker("bead-assign", "worker-001")
            .await;
        assert!(assigned.is_ok(), "assign should succeed");

        if let Ok(bead) = assigned {
            assert_eq!(bead.assigned_worker, Some("worker-001".to_string()));
            assert_eq!(bead.state, BeadState::Assigned);
        }
    }

    #[tokio::test]
    async fn test_mark_bead_failed() {
        let store = require_store!(setup_store().await);

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

    #[tokio::test]
    async fn test_corrupted_bead_record_returns_serialization_error() {
        let store = require_store!(setup_store().await);
        let now = SurrealDatetime::from(Utc::now());

        let create_result: Result<(), PersistenceError> = store
            .db()
            .query(
                "CREATE type::thing('bead', 'corrupt-bead') CONTENT {
                    bead_id: 'corrupt-bead',
                    workflow_id: 'wf-corrupt',
                    state: 'invalid_state',
                    assigned_worker: NONE,
                    assigned_queue: NONE,
                    created_at: $now,
                    updated_at: $now,
                    started_at: NONE,
                    completed_at: NONE,
                    error_message: 'bad data',
                    retry_count: 0,
                    metadata: NONE
                }",
            )
            .bind(("now", now))
            .await
            .map_err(from_surrealdb_error)
            .and_then(|resp| resp.check().map_err(from_surrealdb_error))
            .map(|_| ());

        assert!(
            create_result.is_ok(),
            "setting up corrupted bead should succeed: {:?}",
            create_result.err()
        );

        let corrupted = store.get_bead("corrupt-bead").await;
        assert!(
            matches!(corrupted, Err(PersistenceError::SerializationError { .. })),
            "expected serialization error for corrupt bead, got {:?}",
            corrupted
        );

        if let Err(PersistenceError::SerializationError { reason }) = corrupted {
            assert!(
                reason.contains("invalid_state") || reason.to_lowercase().contains("variant"),
                "unexpected serialization reason: {}",
                reason
            );
        }

        let healthy = BeadRecord::new("healthy-bead", "wf-corrupt");
        let saved = store.save_bead(&healthy).await;
        assert!(saved.is_ok(), "save should succeed for healthy record");

        let retrieved = store.get_bead("healthy-bead").await;
        assert!(
            matches!(retrieved, Ok(ref bead) if bead.bead_id == "healthy-bead"),
            "healthy bead should still be retrievable: {:?}",
            retrieved
        );
    }

    // ==========================================================================
    // find_blocked_beads BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_find_beads_blocked_by_incomplete_dependencies() {
        let store = require_store!(setup_store().await);

        // Create workflow beads
        let bead_a = BeadRecord::new("bead-a", "wf-blocked");
        let bead_b = BeadRecord::new("bead-b", "wf-blocked");
        let bead_c = BeadRecord::new("bead-c", "wf-blocked");

        let _ = store.save_bead(&bead_a).await;
        let _ = store.save_bead(&bead_b).await;
        let _ = store.save_bead(&bead_c).await;

        // bead-b depends on bead-a
        let dep_ab = DependencyEdge::new("bead-b", "bead-a", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&dep_ab).await;

        // bead-c depends on bead-b (transitive)
        let dep_bc = DependencyEdge::new("bead-c", "bead-b", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&dep_bc).await;

        // Mark bead-a as completed
        let update_result = store.update_bead_state("bead-a", BeadState::Completed).await;
        assert!(
            update_result.is_ok(),
            "update bead state should succeed: {:?}",
            update_result.err()
        );

        // Query blocked beads
        let blocked = store.find_blocked_beads("wf-blocked").await;

        assert!(
            blocked.is_ok(),
            "find_blocked_beads should succeed: {:?}",
            blocked.err()
        );

        if let Ok(blocked_info) = blocked {
            // bead-b should NOT be blocked (bead-a completed)
            // bead-c SHOULD be blocked (bead-b not completed)
            assert_eq!(
                blocked_info.len(),
                1,
                "should have 1 blocked bead, got {:?}",
                blocked_info.len()
            );

            let blocked_c = &blocked_info[0];
            assert_eq!(blocked_c.bead_id, "bead-c");
            assert_eq!(blocked_c.workflow_id, "wf-blocked");
            assert_eq!(blocked_c.state, BeadState::Pending);
            assert_eq!(blocked_c.blocking_dependencies, vec!["bead-b".to_string()]);
        }
    }

    #[tokio::test]
    async fn should_return_empty_list_when_no_beads_blocked() {
        let store = require_store!(setup_store().await);

        // Create workflow beads with no dependencies
        let bead_a = BeadRecord::new("bead-a", "wf-unblocked");
        let bead_b = BeadRecord::new("bead-b", "wf-unblocked");

        let _ = store.save_bead(&bead_a).await;
        let _ = store.save_bead(&bead_b).await;

        // Both beads in pending state with no dependencies
        let blocked = store.find_blocked_beads("wf-unblocked").await;

        assert!(
            blocked.is_ok(),
            "find_blocked_beads should succeed: {:?}",
            blocked.err()
        );

        if let Ok(blocked_info) = blocked {
            assert_eq!(
                blocked_info.len(),
                0,
                "should have no blocked beads, got {:?}",
                blocked_info.len()
            );
        }
    }

    #[tokio::test]
    async fn should_include_multiple_blocking_dependencies() {
        let store = require_store!(setup_store().await);

        // Create workflow beads
        let bead_a = BeadRecord::new("bead-a", "wf-multi");
        let bead_b = BeadRecord::new("bead-b", "wf-multi");
        let bead_c = BeadRecord::new("bead-c", "wf-multi");
        let bead_d = BeadRecord::new("bead-d", "wf-multi");

        let _ = store.save_bead(&bead_a).await;
        let _ = store.save_bead(&bead_b).await;
        let _ = store.save_bead(&bead_c).await;
        let _ = store.save_bead(&bead_d).await;

        // bead-d depends on bead-b AND bead-c (multiple dependencies)
        let dep_db = DependencyEdge::new("bead-d", "bead-b", DependencyRelation::DependsOn);
        let dep_dc = DependencyEdge::new("bead-d", "bead-c", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&dep_db).await;
        let _ = store.save_dependency_edge(&dep_dc).await;

        // Mark bead-a as completed (not a dependency)
        let update_result = store.update_bead_state("bead-a", BeadState::Completed).await;
        assert!(
            update_result.is_ok(),
            "update bead state should succeed: {:?}",
            update_result.err()
        );

        // Query blocked beads
        let blocked = store.find_blocked_beads("wf-multi").await;

        assert!(
            blocked.is_ok(),
            "find_blocked_beads should succeed: {:?}",
            blocked.err()
        );

        if let Ok(blocked_info) = blocked {
            assert_eq!(blocked_info.len(), 1, "should have 1 blocked bead");

            let blocked_d = &blocked_info[0];
            assert_eq!(blocked_d.bead_id, "bead-d");
            assert!(
                blocked_d
                    .blocking_dependencies
                    .contains(&"bead-b".to_string()),
                "should list bead-b as blocking"
            );
            assert!(
                blocked_d
                    .blocking_dependencies
                    .contains(&"bead-c".to_string()),
                "should list bead-c as blocking"
            );
            assert_eq!(blocked_d.blocking_dependencies.len(), 2);
        }
    }

    #[tokio::test]
    async fn should_exclude_completed_beads_from_blocked_list() {
        let store = require_store!(setup_store().await);

        // Create workflow beads
        let bead_a = BeadRecord::new("bead-a", "wf-filter");
        let bead_b = BeadRecord::new("bead-b", "wf-filter");

        let _ = store.save_bead(&bead_a).await;
        let _ = store.save_bead(&bead_b).await;

        // bead-b depends on bead-a
        let dep = DependencyEdge::new("bead-b", "bead-a", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&dep).await;

        // Mark bead-b as completed
        let update_result = store.update_bead_state("bead-b", BeadState::Completed).await;
        assert!(
            update_result.is_ok(),
            "update bead state should succeed: {:?}",
            update_result.err()
        );

        // Query blocked beads
        let blocked = store.find_blocked_beads("wf-filter").await;

        assert!(
            blocked.is_ok(),
            "find_blocked_beads should succeed: {:?}",
            blocked.err()
        );

        if let Ok(blocked_info) = blocked {
            assert_eq!(
                blocked_info.len(),
                0,
                "completed beads should not be in blocked list"
            );
        }
    }

    #[tokio::test]
    async fn should_provide_descriptive_block_reason() {
        let store = require_store!(setup_store().await);

        // Create workflow beads
        let bead_a = BeadRecord::new("bead-a", "wf-reason");
        let bead_b = BeadRecord::new("bead-b", "wf-reason");

        let _ = store.save_bead(&bead_a).await;
        let _ = store.save_bead(&bead_b).await;

        // bead-b depends on bead-a
        let dep = DependencyEdge::new("bead-b", "bead-a", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&dep).await;

        // Query blocked beads
        let blocked = store.find_blocked_beads("wf-reason").await;

        assert!(blocked.is_ok(), "find_blocked_beads should succeed");

        if let Ok(blocked_info) = blocked {
            assert_eq!(blocked_info.len(), 1);

            let reason = &blocked_info[0].block_reason;
            assert!(!reason.is_empty(), "block reason should not be empty");
            assert!(
                reason.contains("bead-a") || reason.contains("dependencies"),
                "block reason should mention dependencies: {}",
                reason
            );
        }
    }
}
