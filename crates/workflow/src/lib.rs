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
    start_auto_checkpoint, AutoCheckpointConfig, AutoCheckpointTimer, CheckpointId,
    CheckpointMetadata, CheckpointStorage, RestoreError, RestoreResult, StateProvider,
    StorageStats, DEFAULT_AUTO_CHECKPOINT_INTERVAL,
};
pub use engine::{EngineConfig, WorkflowEngine};
pub use error::{Error, Result};
pub use handler::{
    AsyncFnHandler, ChainHandler, FailingHandler, FnHandler, HandlerRegistry, NoOpHandler,
    PhaseHandler,
};
pub use storage::{InMemoryStorage, WorkflowStorage};
pub use types::{
    Checkpoint, Journal, JournalEntry, Phase, PhaseContext, PhaseId, PhaseOutput, Workflow,
    WorkflowId, WorkflowResult, WorkflowState,
};
