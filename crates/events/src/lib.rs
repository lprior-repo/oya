//! Inter-bead coordination via event sourcing.
//!
//! This crate provides event-sourced coordination between beads. Key features:
//!
//! - **Event types**: Rich event types for bead lifecycle management
//! - **Event store**: Append-only event storage with read and query
//! - **Event bus**: Pub/sub for real-time coordination
//! - **Projections**: Materialized views rebuilt from events
//!
//! # Telemetry Initialization
//!
//! Initialize distributed tracing using oya-telemetry crate.
//! Call `init_telemetry_json()` at application startup to enable
//! structured JSON logging for distributed tracing.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod bus;
pub mod db;
pub mod durable_store;
pub mod error;
pub mod event;
pub mod projection;
pub mod replay;
pub mod stage;
pub mod store;
pub mod types;

// Re-export error types
pub use error::{ConnectionError, Error, Result};

// Re-export types
pub use types::{
    BeadId, BeadResult, BeadSpec, BeadState, Complexity, EventId, PhaseId, PhaseOutput,
};

// Re-export event types
pub use event::{BeadEvent, SerializationError, SerializationResult};

// Re-export bus types
pub use bus::{CircuitBreaker, EventBus, EventBusBuilder, EventPattern, EventSubscription};

// Re-export store types
pub use store::{EventStore, InMemoryEventStore};

// Re-export durable store types
pub use durable_store::{
    connect, AppendBatchError, AppendError, ConnectionConfig, DurableEventStore,
};

// Re-export stage types
pub use stage::{
    BeadStateMachine, ExhaustionPolicy, RecursionPolicy, Severity, StageKind, StageTransition,
    StateMachineError, TransitionReason,
};

// Re-export replay types
pub use replay::{
    apply_event, apply_events, create_tracker, is_transient_error, ApplyContext, EventFilter,
    EventLoader, EventSourcedState, LoadError, RecoveryConfig, RecoveryStrategy, ReplayProgress,
    ReplayState, ReplayTracker, RetryPolicy,
};

// Re-export db types
pub use db::{DbError, SurrealDbClient, SurrealDbConfig};

/// Initialize telemetry with JSON logging.
///
/// # Errors
///
/// Returns `Error` if initialization fails.
pub fn init_telemetry_json() -> Result<()> {
    use oya_telemetry::TelemetryConfig;

    let config = TelemetryConfig::new("oya-events")
        .with_json_logging(true)
        .with_log_level(tracing::Level::INFO);

    let _guard = oya_telemetry::init_telemetry(&config)
        .map_err(|e| Error::serialization(format!("telemetry init failed: {e}")))?;

    Ok(())
}
