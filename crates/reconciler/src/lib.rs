//! K8s-style reconciliation loop for bead management.
//!
//! This crate implements a reconciliation pattern inspired by Kubernetes:
//!
//! - **Desired State**: Declare what the system should look like
//! - **Actual State**: Computed from events/projections
//! - **Diff**: Compare desired vs actual
//! - **Actions**: Generate and execute actions to converge
//!
//! # Key Concepts
//!
//! ## Reconciliation
//!
//! The reconciler periodically:
//! 1. Gets the desired state (what beads should exist)
//! 2. Computes actual state (from event projections)
//! 3. Generates actions to close the gap
//! 4. Executes actions
//!
//! ## Actions
//!
//! - `CreateBead` - Create a new bead
//! - `StartBead` - Start a scheduled bead
//! - `StopBead` - Stop a running bead
//! - `RetryBead` - Retry a failed bead
//! - `ScheduleBead` - Schedule a pending bead
//!
//! # Example
//!
//! ```ignore
//! use oya_reconciler::{
//!     Reconciler, ReconcilerConfig, ReconciliationLoop, LoopConfig,
//!     DesiredState, InMemoryDesiredStateProvider,
//! };
//! use oya_events::{EventBus, InMemoryEventStore, AllBeadsProjection, ManagedProjection};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let store = Arc::new(InMemoryEventStore::new());
//!     let bus = Arc::new(EventBus::new(store));
//!     let reconciler = Arc::new(Reconciler::with_event_executor(
//!         bus.clone(),
//!         ReconcilerConfig::default(),
//!     ));
//!
//!     let desired = Arc::new(InMemoryDesiredStateProvider::new(DesiredState::new()));
//!     let projection = Arc::new(ManagedProjection::new(AllBeadsProjection::new()));
//!
//!     let mut loop_runner = ReconciliationLoop::new(
//!         reconciler,
//!         desired,
//!         projection,
//!         LoopConfig::default(),
//!     );
//!
//!     // Run until stopped
//!     // loop_runner.run().await;
//! }
//! ```

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub mod error;
pub mod r#loop;
pub mod reconciler;
pub mod types;

// Re-export main types
pub use error::{Error, Result};
pub use r#loop::{
    DesiredStateProvider, InMemoryDesiredStateProvider, LoopConfig, LoopStopper,
    ReconciliationLoop,
};
pub use reconciler::{ActionExecutor, EventActionExecutor, Reconciler, ReconcilerBuilder, ReconcilerConfig};
pub use types::{ActualState, DesiredState, ReconcileAction, ReconcileResult};
