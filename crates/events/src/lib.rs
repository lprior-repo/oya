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

/// Initialize telemetry with JSON logging.
///
/// # Errors
///
/// Returns `Error` if initialization fails.
pub fn init_telemetry_json() -> Result<(), Box<dyn std::error::Error>> {
    use oya_telemetry::{TelemetryConfig, TracingGuard};

    let config = TelemetryConfig::new("oya-events")
        .with_json_logging(true)
        .with_log_level(tracing::Level::INFO);

    let _guard = oya_telemetry::init_telemetry(config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}

/// Example with telemetry initialization
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize telemetry first
///     init_telemetry_json()?;
///     
///     let store = Arc::new(InMemoryEventStore::new());
///     let bus = EventBus::new(store.clone());
///     
///     let mut sub = bus.subscribe();
///     
///     let bead_id = BeadId::new();
///     let spec = BeadSpec::new("My task").with_complexity(Complexity::Medium);
///     bus.publish(BeadEvent::created(bead_id, spec)).await?;
///     
///     let event = sub.recv().await?;
///     tracing::info!("Received: {:?}", event.event_type());
///     Ok(())
/// }
/// ```
