//! Workflow persistence operations.
//!
//! CRUD operations for workflow records in SurrealDB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use super::client::OrchestratorStore;
use super::error::{from_surrealdb_error, PersistenceError, PersistenceResult};

/// Workflow status in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is pending (not yet started)
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
}

impl Default for WorkflowStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Workflow record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecord {
    /// SurrealDB record ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    /// Workflow identifier
    pub workflow_id: String,
    /// Workflow name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Serialized DAG structure as JSON
    pub dag_json: String,
    /// Current status
    pub status: WorkflowStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl WorkflowRecord {
    /// Create a new workflow record.
    #[must_use]
    pub fn new(workflow_id: impl Into<String>, name: impl Into<String>, dag_json: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            workflow_id: workflow_id.into(),
            name: name.into(),
            description: None,
            dag_json: dag_json.into(),
            status: WorkflowStatus::Pending,
            created_at: now,
            updated_at: now,
            completed_at: None,
            metadata: None,
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Input for creating a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowInput {
    id: String,
    name: String,
    description: Option<String>,
    dag_json: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    metadata: Option<serde_json::Value>,
}

impl From<&WorkflowRecord> for WorkflowInput {
    fn from(record: &WorkflowRecord) -> Self {
        Self {
            id: record.workflow_id.clone(),
            name: record.name.clone(),
            description: record.description.clone(),
            dag_json: record.dag_json.clone(),
            status: record.status.to_string(),
            created_at: record.created_at,
            updated_at: record.updated_at,
            completed_at: record.completed_at,
            metadata: record.metadata.clone(),
        }
    }
}

impl OrchestratorStore {
    /// Save a workflow record to the database.
    ///
    /// Creates a new record if it doesn't exist, or updates if it does.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn save_workflow(&self, record: &WorkflowRecord) -> PersistenceResult<WorkflowRecord> {
        let input = WorkflowInput::from(record);

        let result: Option<WorkflowRecord> = self
            .db()
            .upsert(("workflow", &record.workflow_id))
            .content(input)
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::query_failed("failed to save workflow"))
    }

    /// Get a workflow by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow is not found or the query fails.
    pub async fn get_workflow(&self, workflow_id: &str) -> PersistenceResult<WorkflowRecord> {
        let result: Option<WorkflowRecord> = self
            .db()
            .select(("workflow", workflow_id))
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("workflow", workflow_id))
    }

    /// List all workflows, optionally filtered by status.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn list_workflows(&self, status: Option<WorkflowStatus>) -> PersistenceResult<Vec<WorkflowRecord>> {
        let workflows: Vec<WorkflowRecord> = match status {
            Some(s) => {
                self.db()
                    .query("SELECT * FROM workflow WHERE status = $status ORDER BY created_at DESC")
                    .bind(("status", s.to_string()))
                    .await
                    .map_err(from_surrealdb_error)?
                    .take(0)
                    .map_err(from_surrealdb_error)?
            }
            None => {
                self.db()
                    .query("SELECT * FROM workflow ORDER BY created_at DESC")
                    .await
                    .map_err(from_surrealdb_error)?
                    .take(0)
                    .map_err(from_surrealdb_error)?
            }
        };

        Ok(workflows)
    }

    /// Delete a workflow by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow is not found or the delete fails.
    pub async fn delete_workflow(&self, workflow_id: &str) -> PersistenceResult<()> {
        let result: Option<WorkflowRecord> = self
            .db()
            .delete(("workflow", workflow_id))
            .await
            .map_err(from_surrealdb_error)?;

        if result.is_some() {
            Ok(())
        } else {
            Err(PersistenceError::not_found("workflow", workflow_id))
        }
    }

    /// Update workflow status.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow is not found or the update fails.
    pub async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> PersistenceResult<WorkflowRecord> {
        let now = Utc::now();
        let completed_at = if matches!(status, WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled) {
            Some(now)
        } else {
            None
        };

        let result: Option<WorkflowRecord> = self
            .db()
            .query("UPDATE workflow SET status = $status, updated_at = $updated_at, completed_at = $completed_at WHERE id = $id RETURN AFTER")
            .bind(("id", format!("workflow:{}", workflow_id)))
            .bind(("status", status.to_string()))
            .bind(("updated_at", now))
            .bind(("completed_at", completed_at))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::not_found("workflow", workflow_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::client::StoreConfig;

    async fn setup_store() -> OrchestratorStore {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config)
            .await
            .ok()
            .filter(|_| true);

        let store = store.unwrap();
        let _ = store.initialize_schema().await;
        store
    }

    #[tokio::test]
    async fn test_save_and_get_workflow() {
        let store = setup_store().await;

        let record = WorkflowRecord::new("wf-001", "Test Workflow", r#"{"nodes":[]}"#);
        let saved = store.save_workflow(&record).await;
        assert!(saved.is_ok(), "save should succeed");

        let retrieved = store.get_workflow("wf-001").await;
        assert!(retrieved.is_ok(), "get should succeed");

        if let Ok(wf) = retrieved {
            assert_eq!(wf.workflow_id, "wf-001");
            assert_eq!(wf.name, "Test Workflow");
        }
    }

    #[tokio::test]
    async fn test_list_workflows() {
        let store = setup_store().await;

        let wf1 = WorkflowRecord::new("wf-001", "Workflow 1", "{}");
        let wf2 = WorkflowRecord::new("wf-002", "Workflow 2", "{}");

        let _ = store.save_workflow(&wf1).await;
        let _ = store.save_workflow(&wf2).await;

        let list = store.list_workflows(None).await;
        assert!(list.is_ok(), "list should succeed");

        if let Ok(workflows) = list {
            assert_eq!(workflows.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_delete_workflow() {
        let store = setup_store().await;

        let record = WorkflowRecord::new("wf-delete", "To Delete", "{}");
        let _ = store.save_workflow(&record).await;

        let delete_result = store.delete_workflow("wf-delete").await;
        assert!(delete_result.is_ok(), "delete should succeed");

        let get_result = store.get_workflow("wf-delete").await;
        assert!(get_result.is_err(), "get after delete should fail");
    }

    #[tokio::test]
    async fn test_get_nonexistent_workflow() {
        let store = setup_store().await;

        let result = store.get_workflow("nonexistent").await;
        assert!(result.is_err(), "should fail for nonexistent workflow");

        if let Err(e) = result {
            assert!(matches!(e, PersistenceError::NotFound { .. }));
        }
    }

    #[tokio::test]
    async fn test_workflow_status_display() {
        assert_eq!(WorkflowStatus::Pending.to_string(), "pending");
        assert_eq!(WorkflowStatus::Running.to_string(), "running");
        assert_eq!(WorkflowStatus::Completed.to_string(), "completed");
        assert_eq!(WorkflowStatus::Failed.to_string(), "failed");
        assert_eq!(WorkflowStatus::Cancelled.to_string(), "cancelled");
    }
}
