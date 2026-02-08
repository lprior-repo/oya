//! Intra-bead workflow engine with checkpoints, rewind, and journal replay.
//!
//! This crate provides a workflow execution engine for managing phase-based
//! workflows. Key features include:
//!
//! - **Phase-based execution**: Workflows consist of ordered phases, each
//!   executed by a registered handler.
//! - **Automatic checkpointing**: State is checkpointed after each phase
//!   for recovery.
//! - **Rewind capability**: Rewind to any previous checkpoint to re-execute
//!   from that point.
//! - **Journal replay**: Full event sourcing within a workflow for debugging
//!   and recovery.
//! - **Retry with backoff**: Automatic retries with exponential backoff on
//!   transient failures.
//! - **Rollback on failure**: Optionally roll back completed phases when
//!   a later phase fails.
//!
//! # Example
//!
//! ```ignore
//! use oya_workflow::{
//!     WorkflowEngine, EngineConfig, Workflow, Phase,
//!     HandlerRegistry, NoOpHandler, InMemoryStorage,
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create storage and handlers
//!     let storage = Arc::new(InMemoryStorage::new());
//!     let mut registry = HandlerRegistry::new();
//!     registry.register("build", Arc::new(NoOpHandler::new("build")));
//!     registry.register("test", Arc::new(NoOpHandler::new("test")));
//!
//!     // Create engine
//!     let engine = WorkflowEngine::new(
//!         storage,
//!         Arc::new(registry),
//!         EngineConfig::default(),
//!     );
//!
//!     // Define workflow
//!     let workflow = Workflow::new("my-workflow")
//!         .add_phase(Phase::new("build"))
//!         .add_phase(Phase::new("test"));
//!
//!     // Execute
//!     let result = engine.run(workflow).await;
//!     println!("Workflow completed: {:?}", result);
//! }
//! ```

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub mod checkpoint;
pub mod cleanup;
pub mod engine;
pub mod error;
pub mod handler;
pub mod idempotent;
pub mod schema;
pub mod storage;
pub mod types;

// Re-export main types
pub use checkpoint::{
    compress, compression_ratio, decompress, restore_checkpoint, serialize_state, space_savings,
    start_auto_checkpoint, AutoCheckpointConfig, AutoCheckpointTimer, CheckpointDecision,
    CheckpointId, CheckpointManager, CheckpointMetadata, CheckpointStorage, CheckpointStrategy,
    RestoreError, RestoreResult, StateProvider, StorageStats, DEFAULT_AUTO_CHECKPOINT_INTERVAL,
};
pub use cleanup::{
    check_zjj_exit_code, cleanup_task, create_cleanup_timer, log_cleanup_results, parse_zjj_json,
    run_zjj_clean, verify_zjj_exists, CleanedSession, CleanupConfig, CleanupError, CleanupResult,
    ZjjCleanOutput,
};
pub use engine::{EngineConfig, WorkflowEngine};
pub use error::{Error, Result};
pub use handler::{
    AsyncFnHandler, ChainHandler, FailingHandler, FnHandler, HandlerChain, HandlerRegistry,
    NoOpHandler, PhaseHandler,
};
pub use storage::{InMemoryStorage, WorkflowStorage};
pub use types::{
    Checkpoint, Journal, JournalEntry, Phase, PhaseContext, PhaseId, PhaseOutput, Workflow,
    WorkflowId, WorkflowResult, WorkflowState,
};

// Test module for Arc wrapping verification
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_journal_entry_phase_completed_arc() {
        let phase_id = PhaseId::new();
        let output_data = vec![1, 2, 3, 4, 5];

        // Create a PhaseCompleted journal entry with Arc output
        let entry = JournalEntry::phase_completed(phase_id, "test_phase", output_data.clone());

        match entry {
            JournalEntry::PhaseCompleted { output, .. } => {
                // Verify the output is an Arc
                let arc_data: Arc<Vec<u8>> = output.clone();

                // Verify the data is the same
                assert_eq!(*arc_data, output_data);

                // Verify we can clone the Arc cheaply
                let cloned_output = output.clone();
                assert_eq!(*cloned_output, output_data);

                println!("✓ JournalEntry::PhaseCompleted correctly uses Arc<Vec<u8>>");
            }
            _ => unreachable!("Expected PhaseCompleted variant in test"),
        }
    }

    #[test]
    fn test_phase_output_arc() {
        let data = vec![1, 2, 3, 4, 5];

        // Create a PhaseOutput with Arc data
        let output = PhaseOutput::success(data.clone());

        // Verify the data is an Arc
        let arc_data: Arc<Vec<u8>> = output.data.clone();

        // Verify the data is the same
        assert_eq!(*arc_data, data);

        // Verify we can clone the Arc cheaply
        let cloned_data = output.data.clone();
        assert_eq!(*cloned_data, data);

        println!("✓ PhaseOutput correctly uses Arc<Vec<u8>>");
    }

    #[test]
    fn test_arc_performance() {
        let data = vec![1u8; 1024]; // 1KB of data

        // Test Arc cloning performance
        let start = std::time::Instant::now();

        let arc_data = Arc::new(data);
        for _ in 0..1000 {
            let _cloned = arc_data.clone();
        }

        let duration = start.elapsed();
        println!("✓ Arc cloning of 1KB data 1000 times took: {:?}", duration);

        // This should be very fast (just pointer copying)
        assert!(duration.as_micros() < 1000, "Arc cloning should be fast");
    }
}
