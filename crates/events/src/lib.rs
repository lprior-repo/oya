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

pub use error::{Error, Result};

pub use durable_store::{connect, ConnectionConfig, DurableEventStore};

pub use replay::{apply_event, apply_events, ApplyContext, EventSourcedState};

pub use bus::{EventBus, EventBusBuilder, EventPattern, EventSubscription};
pub use event::BeadEvent;
pub use stage::{BeadStateMachine, ExhaustionPolicy, RecursionPolicy, Severity, StageKind, StageTransition, StateMachineError, TransitionReason};
pub use types::{
    BeadId, BeadResult, BeadSpec, BeadState, Complexity, EventId, PhaseId, PhaseOutput,
};

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
