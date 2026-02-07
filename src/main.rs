//! # OYA - Main CLI Entry Point
//!
//! This is the main entry point for the OYA SDLC system.
//!
//! ## CLI Interface
//!
//! The binary provides a command-line interface for managing tasks:
//! - `oya new -s <slug>` - Create a new task
//! - `oya list` - List all tasks
//! - `oya show -s <slug>` - Show task details
//! - `oya stage -s <slug> --stage <name>` - Run a pipeline stage
//! - `oya approve -s <slug>` - Approve task for deployment
//! - `oya agents` - Manage agent pool
//!
//! ## Daemon Mode
//!
//! Some commands may start the full OYA daemon with database and event bus.

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

mod cli;
mod commands;

use cli::Cli;

/// Main entry point for OYA.
///
/// Parses CLI commands and dispatches to appropriate handlers.
#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize tracing (respecting verbosity flags)
    init_tracing_from_cli(&cli);

    // Execute the command
    match commands::execute_command(cli.command).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Initialize tracing subscriber with environment filter.
fn init_tracing_from_cli(cli: &Cli) {
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else if cli.quiet {
        EnvFilter::new("warn")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize SurrealDB connection.
///
/// This is used by commands that need database access.
pub async fn init_database() -> Result<oya_events::db::SurrealDbClient> {
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
pub fn init_event_bus() -> Arc<EventBus> {
    let event_store = Arc::new(InMemoryEventStore::new());
    Arc::new(EventBus::new(event_store))
}

/// Wait for shutdown signal (Ctrl+C).
pub async fn wait_for_shutdown() {
    match signal::ctrl_c().await {
        Ok(()) => info!("Received Ctrl+C, initiating graceful shutdown"),
        Err(err) => error!("Failed to listen for shutdown signal: {}", err),
    }
}
