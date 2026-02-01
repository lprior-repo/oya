//! Server setup with Tower middleware

use super::actors::{AppState, mock_scheduler, mock_state_manager};
use super::routes;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
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
fn create_app() -> Router {
    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
    };

    routes::create_router()
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}
