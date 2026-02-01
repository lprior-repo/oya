//! REST API routes

use super::actors::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub mod beads;
pub mod health;
pub mod workflows;

/// Create the main API router
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/api/workflows", post(workflows::create_workflow))
        .route("/api/beads/:id", get(beads::get_bead_status))
        .route("/api/beads/:id/cancel", post(beads::cancel_bead))
        .route("/api/health", get(health::health_check))
}
