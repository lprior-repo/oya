//! REST API routes

use super::actors::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub mod beads;
pub mod health;
pub mod websocket;
pub mod workflows;

/// Create the main API router
/// Routes are mounted under /api prefix in server.rs, so paths are relative
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/workflows", post(workflows::create_workflow))
        .route("/beads/{id}", get(beads::get_bead_status))
        .route("/beads/{id}/cancel", post(beads::cancel_bead))
        .route("/beads/{id}/retry", post(beads::retry_bead))
        .route("/ws", get(websocket::websocket_handler))
}
