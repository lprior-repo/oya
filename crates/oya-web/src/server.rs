//! Server setup with Tower middleware

use super::actors::{AppState, BroadcastEvent, mock_scheduler, mock_state_manager};
use super::routes;
use axum::{Router, routing::get_service};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;

/// Run the axum server with Tower middleware
pub async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app();

    let listener = TcpListener::bind(addr).await?;
    info!("OYA Web Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the axum application with middleware
/// Serves both the Leptos WASM frontend AND the API
fn create_app() -> Router {
    // Create broadcast channel for WebSocket events
    // Capacity of 100 allows clients to lag behind without blocking the sender
    let (broadcast_tx, _) = broadcast::channel::<BroadcastEvent>(100);

    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
        broadcast_tx,
    };

    let static_files =
        get_service(ServeDir::new("crates/oya-ui/dist").append_index_html_on_directories(true));

    // Combine API routes and static file serving
    Router::new()
        // API routes under /api prefix
        .nest("/api", routes::create_router().with_state(state))
        // Serve static frontend files for all other routes
        .fallback_service(static_files)
        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}
