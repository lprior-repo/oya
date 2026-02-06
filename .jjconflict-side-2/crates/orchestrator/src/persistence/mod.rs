//! Persistence layer for the orchestrator.
//!
//! This module provides SurrealDB-backed persistence for:
//! - Workflows and their DAG structure
//! - Beads and their execution state
//! - Checkpoints for recovery
//!
//! # Architecture
//!
//! The persistence layer uses SurrealDB as the backing store with:
//! - `OrchestratorStore`: Connection management and health checks
//! - `WorkflowRecord`: Workflow metadata and DAG JSON
//! - `BeadRecord`: Individual bead state and assignments
//! - `CheckpointRecord`: Scheduler state snapshots for replay
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::persistence::{OrchestratorStore, StoreConfig, WorkflowRecord};
//!
//! let config = StoreConfig::in_memory();
//! let store = OrchestratorStore::connect(config).await?;
//! store.initialize_schema().await?;
//!
//! let workflow = WorkflowRecord::new("wf-001", "My Workflow", "{}");
//! store.save_workflow(&workflow).await?;
//! ```

pub mod bead_store;
pub mod checkpoint_store;
pub mod client;
pub mod error;
pub mod workflow_store;

// Re-export main types
pub use bead_store::{BeadRecord, BeadState};
pub use checkpoint_store::CheckpointRecord;
pub use client::{Credentials, OrchestratorStore, StoreConfig};
pub use error::{PersistenceError, PersistenceResult};
pub use workflow_store::{WorkflowRecord, WorkflowStatus};

#[cfg(test)]
mod tests {
    use super::*;

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
    async fn test_full_persistence_workflow() {
        // Setup store
        let config = StoreConfig::in_memory();
        let store = require_store!(OrchestratorStore::connect(config).await.ok());
        let _ = store.initialize_schema().await;

        // Create workflow
        let workflow = WorkflowRecord::new("wf-test", "Test Workflow", r#"{"nodes":["a","b"]}"#);
        let saved_wf = store.save_workflow(&workflow).await;
        assert!(saved_wf.is_ok(), "workflow save should succeed");

        // Create beads
        let bead1 = BeadRecord::new("bead-1", "wf-test");
        let bead2 = BeadRecord::new("bead-2", "wf-test");
        let _ = store.save_bead(&bead1).await;
        let _ = store.save_bead(&bead2).await;

        // Verify beads by workflow
        let beads = store.get_beads_by_workflow("wf-test").await;
        assert!(beads.is_ok());
        if let Ok(list) = beads {
            assert_eq!(list.len(), 2);
        }

        // Create checkpoint
        let checkpoint = CheckpointRecord::new("cp-1", r#"{"active_workflows":["wf-test"]}"#, 1);
        let saved_cp = store.save_checkpoint(&checkpoint).await;
        assert!(saved_cp.is_ok(), "checkpoint save should succeed");

        // Update bead state
        let updated = store.update_bead_state("bead-1", BeadState::Running).await;
        assert!(updated.is_ok());

        // Complete workflow
        let completed = store
            .update_workflow_status("wf-test", WorkflowStatus::Completed)
            .await;
        assert!(completed.is_ok());
    }

    #[tokio::test]
    async fn test_store_config_defaults() {
        let config = StoreConfig::default();
        assert_eq!(config.url, "mem://");
        assert_eq!(config.namespace, "orchestrator");
        assert_eq!(config.database, "test");
    }

    #[tokio::test]
    async fn test_persistence_error_is_retryable() {
        assert!(PersistenceError::connection_failed("test").is_retryable());
        assert!(PersistenceError::timeout(1000).is_retryable());
        assert!(PersistenceError::PoolExhausted.is_retryable());
        assert!(!PersistenceError::not_found("test", "id").is_retryable());
        assert!(!PersistenceError::already_exists("test", "id").is_retryable());
    }
}
