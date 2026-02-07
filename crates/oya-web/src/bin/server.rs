//! OYA Web Server Binary
//!
//! Standalone binary to run the OYA web server with Tower middleware.
//! Serves both the Leptos WASM frontend AND the API from a single server.

use anyhow::{Context, Result};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with functional error handling
    init_tracing();

    // Build server configuration
    let addr = parse_server_address().context("Failed to parse server address")?;

    tracing::info!("Starting OYA Full Stack Server");
    tracing::info!("Frontend: http://localhost:3000");
    tracing::info!("API: http://localhost:3000/api");

    // Run server with proper error context
    oya_web::server::run_server(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Server runtime error: {}", e))?;

    Ok(())
}

/// Initialize tracing subscriber with environment-aware filtering.
fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,oya_web=debug,tower_http=debug".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Parse server address from configuration.
///
/// Pure function that returns the server socket address.
fn parse_server_address() -> Result<SocketAddr> {
    Ok(SocketAddr::from(([127, 0, 0, 1], 3000)))
}
