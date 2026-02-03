//! Inter-bead coordination via event sourcing.
//!
//! This crate provides event-sourced coordination between beads. Key features:
//!
//! - **Event types**: Rich event types for bead lifecycle management
//! - **Event store**: Append-only event storage with read and query
//! - **Event bus**: Pub/sub for real-time coordination
//! - **Projections**: Materialized views rebuilt from events
//!
//! # Example
//!
//! ```ignore
//! use oya_events::{
//!     EventBus, InMemoryEventStore, BeadEvent, BeadId, BeadSpec, Complexity,
//!     AllBeadsProjection, ManagedProjection,
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create event store and bus
//!     let store = Arc::new(InMemoryEventStore::new());
//!     let bus = EventBus::new(store.clone());
//!
//!     // Subscribe to events
//!     let mut sub = bus.subscribe();
//!
//!     // Publish a bead creation event
//!     let bead_id = BeadId::new();
//!     let spec = BeadSpec::new("My task").with_complexity(Complexity::Medium);
//!     bus.publish(BeadEvent::created(bead_id, spec)).await.unwrap();
//!
//!     // Receive the event
//!     let event = sub.recv().await.unwrap();
//!     println!("Received: {:?}", event.event_type());
//! }
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub mod bus;
pub mod db;
pub mod durable_store;
pub mod error;
pub mod event;
pub mod projection;
pub mod replay;
pub mod store;
pub mod types;

// Re-export main types
pub use bus::{EventBus, EventBusBuilder, EventPattern, EventSubscription};
pub use durable_store::{connect, ConnectionConfig, DurableEventStore, AppendError};
pub use error::{ConnectionError, Error, Result};
pub use event::BeadEvent;
pub use projection::{
    AllBeadsProjection, AllBeadsState, BeadProjection, ManagedProjection, Projection,
};
pub use replay::{create_tracker, ReplayProgress, ReplayTracker};
pub use store::{EventStore, InMemoryEventStore, TracingEventStore};
pub use types::{
    BeadId, BeadResult, BeadSpec, BeadState, Complexity, EventId, PhaseId, PhaseOutput,
    StateTransition,
};
