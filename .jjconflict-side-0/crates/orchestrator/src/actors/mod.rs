//! Actor-based concurrency for the orchestrator.
//!
//! This module provides ractor-based actors for managing workflow DAGs
//! and bead scheduling with message-passing concurrency and supervision.
//!
//! # Architecture
//!
//! The actor system follows Erlang-inspired patterns:
//! - **Message passing**: Actors communicate via messages, no shared mutable state
//! - **Process isolation**: Each actor owns its state exclusively
//! - **Supervision**: Automatic restart on panic with exponential backoff
//! - **Graceful degradation**: Handle errors, don't crash
//!
//! # Components
//!
//! - `SchedulerActorDef`: The main scheduler actor that manages workflow DAGs
//! - `SchedulerMessage`: Messages for communicating with the scheduler
//! - `ActorError`: Business logic errors returned via RPC replies
//! - `SupervisorConfig`: Configuration for supervision
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::actors::{spawn_scheduler, SchedulerArguments, SchedulerMessage};
//! use ractor::call;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Spawn the scheduler
//!     let args = SchedulerArguments::new();
//!     let scheduler = spawn_scheduler(args).await?;
//!
//!     // Register a workflow (fire-and-forget)
//!     scheduler.send_message(SchedulerMessage::RegisterWorkflow {
//!         workflow_id: "wf-123".to_string(),
//!     })?;
//!
//!     // Query ready beads (request-response)
//!     let ready = call!(scheduler, SchedulerMessage::GetWorkflowReadyBeads {
//!         workflow_id: "wf-123".to_string(),
//!     })?;
//!
//!     println!("Ready beads: {:?}", ready);
//!
//!     // Stop the scheduler
//!     scheduler.stop(None);
//!     Ok(())
//! }
//! ```

pub mod errors;
pub mod messages;
pub mod queue;
pub mod reconciler;
pub mod scheduler;
pub mod storage;
pub mod supervisor;
pub mod universe;
pub mod workflow;

// Re-export main types for convenience
pub use errors::ActorError;
pub use messages::{BeadState, SchedulerMessage, WorkflowStatus};
pub use queue::{QueueActorDef, QueueMessage, QueueState};
pub use reconciler::{ReconcilerActorDef, ReconcilerMessage, ReconcilerState};
pub use scheduler::{SchedulerActorDef, SchedulerArguments, SchedulerState};
pub use storage::{
    EventStoreActorDef, EventStoreMessage, EventStoreState, StateManagerActorDef,
    StateManagerMessage, StateManagerState,
};
pub use supervisor::{
    GenericSupervisableActor, MeltdownStatus, SupervisorActorDef, SupervisorActorState,
    SupervisorArguments, SupervisorConfig, SupervisorMessage, SupervisorState, SupervisorStatus,
    calculate_backoff,
};
pub use universe::{UniverseArguments, UniverseMessage, UniverseState, UniverseSupervisorDef};
pub use workflow::{WorkflowActorDef, WorkflowMessage, WorkflowStateActor};
