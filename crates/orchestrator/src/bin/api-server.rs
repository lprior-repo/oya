//! Orchestrator API server binary.
//!
//! HTTP server for managing beads, agents, and workflows.

use orchestrator::api::ApiState;
use orchestrator::messaging::{MessageRouter, RouterConfig};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    fmt()
        .with_max_level(LevelFilter::INFO)
        .with_target(false)
        .init();

    // Create message router
    let router = Arc::new(MessageRouter::new(RouterConfig::default()));

    // Create API state
    let state = ApiState::new(router.clone());

    // Build router with middleware
    let cors_origin = "tauri://localhost"
        .parse::<axum::http::HeaderValue>()
        .map_err(|e| format!("Invalid CORS origin: {}", e))?;

    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(Any);

    let app = axum::Router::new()
        .route(
            "/api/beads/{id}/cancel",
            axum::routing::post(orchestrator::api::cancel_bead),
        )
        .route("/health", axum::routing::get(|| async { "OK" }))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors),
        );

    // Bind to port
    let bind_address = "0.0.0.0:3000";
    let listener = TcpListener::bind(bind_address).await?;
    info!(
        "Orchestrator API server listening on http://{}",
        bind_address
    );

    // Serve
    axum::serve(listener, app).await?;

    Ok(())
}
