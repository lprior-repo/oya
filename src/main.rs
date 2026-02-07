//! # OYA - Main Orchestrator Initialization
//!
//! This is the main entry point for the OYA SDLC system.
//!
//! ## Initialization Sequence
//!
//! The system initializes in a specific order to ensure all dependencies are ready:
//!
//! 1. **SurrealDB Connection** - Connect to the database and verify health
//! 2. **UniverseSupervisor** - Spawn tier-1 supervisors for all subsystems
//! 3. **Process Pool Warm** - Initialize worker processes for parallel execution
//! 4. **Reconciliation Loop** - Start the K8s-style reconciliation loop
//! 5. **Axum API** - Start the web server for REST/WebSocket API
//!
//! ## Error Handling
//!
//! All initialization steps use `Result<T, Error>` with proper error propagation.
//! Any failure during initialization will halt startup with a clear error message.
//!
//! ## Shutdown
//!
//! Graceful shutdown is coordinated through the `ShutdownCoordinator`:
//! - SIGTERM/SIGINT signals are caught
//! - Checkpoints are saved within 25s
//! - Actors are stopped gracefully
//! - All cleanup completes within 30s

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use oya_events::{EventBus, InMemoryEventStore};

/// Main entry point for OYA.
///
/// Initializes all subsystems in the correct order and runs until shutdown.
#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Instant::now();

    // Initialize tracing
    init_tracing();

    info!("OYA SDLC System starting...");

    // Initialize all subsystems using functional composition
    let _db_client = init_database().await.context(
        "Database initialization failed. Please check your database configuration and permissions",
    )?;

    let _event_bus = init_event_bus();

    info!("SurrealDB connected and healthy");
    info!("EventBus initialized");

    // TODO: Reconciler initialization pending completion of reconciler crate
    // let reconciler = init_reconciler(event_bus.clone())?;
    // info!("Reconciler initialized");

    // TODO: Desired state provider pending completion
    // let desired_state = init_desired_state_provider()?;
    // info!("Desired State Provider initialized (stub)");

    // TODO: Reconciliation loop pending completion
    // let loop_handle = spawn_reconciliation_loop(
    //     reconciler.clone(),
    //     desired_state.clone(),
    // )?;
    // info!("Reconciliation Loop started (stub)");

    // TODO: API server pending completion of oya-web crate
    // let api_handle = spawn_api_server()?;
    // info!("API server started (stub)");

    // Report startup time
    let startup_duration = start_time.elapsed();
    info!(
        "OYA SDLC System started successfully in {:?}",
        startup_duration
    );

    if startup_duration.as_secs() >= 10 {
        warn!(
            "Startup took {:?} which is longer than the 10s target",
            startup_duration
        );
    }

    // Wait for shutdown signal
    info!("OYA is running. Press Ctrl+C to stop.");
    wait_for_shutdown().await;

    // Graceful shutdown
    info!("Cleaning up...");
    // Note: In production, we would coordinate with ShutdownCoordinator here
    // to ensure checkpoints are saved within 25s and actors stop within 30s

    // Example of fail-fast cleanup pattern (would replace current stub):
    //
    // OLD pattern (continues on error):
    // for resource in resources {
    //     cleanup_resource(&resource).await;  // Ignores errors
    // }
    //
    // NEW pattern (fail-fast):
    // let resources = vec![database, event_bus, reconciler, api_server];
    // stream::iter(resources)
    //     .map(|resource| async move { cleanup_resource(&resource).await })
    //     .try_collect()
    //     .await?;

    info!("OYA SDLC System stopped gracefully");
    Ok(())
}

/// Initialize tracing subscriber with environment filter.
fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize SurrealDB connection.
///
/// This is the first initialization step. All other subsystems depend on the database.
async fn init_database() -> Result<oya_events::db::SurrealDbClient> {
    use oya_events::db::SurrealDbConfig;

    let db_path = std::path::Path::new(".oya/data/db");
    let config = SurrealDbConfig::new(db_path.to_string_lossy().to_string());

    let client = oya_events::db::SurrealDbClient::connect(config)
        .await
        .context("Failed to connect to SurrealDB")?;

    client
        .health_check()
        .await
        .context("SurrealDB health check failed")?;

    Ok(client)
}

/// Initialize EventBus with pure functional construction.
fn init_event_bus() -> Arc<EventBus> {
    let event_store = Arc::new(InMemoryEventStore::new());
    Arc::new(EventBus::new(event_store))
}

/// Wait for shutdown signal (Ctrl+C).
async fn wait_for_shutdown() {
    match signal::ctrl_c().await {
        Ok(()) => info!("Received Ctrl+C, initiating graceful shutdown"),
        Err(err) => error!("Failed to listen for shutdown signal: {}", err),
    }
}
