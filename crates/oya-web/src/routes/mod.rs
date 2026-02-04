//! REST API routes
//!
//! This module defines the main API router with all REST endpoints.
//! All routes are mounted under `/api` prefix in server.rs.
//!
//! ## Route Structure
//!
//! - `GET /api/health` - Health check endpoint
//! - `GET /api/system/health` - System health check endpoint
//! - `POST /api/workflows` - Create a new workflow/bead
//! - `GET /api/workflows` - List all workflows
//! - `GET /api/beads/{id}` - Query bead status by ID
//! - `POST /api/beads/{id}/cancel` - Cancel a running bead
//! - `POST /api/beads/{id}/retry` - Retry a failed bead
//! - `GET /api/beads` - List all beads
//!
//! ## Design Principles
//!
//! - Zero unwraps, zero panics (enforced by clippy lints)
//! - Result-based error handling with RFC 7807 Problem Details
//! - Railway-Oriented Programming for request processing
//! - Functional composition over imperative control flow
//! - Actor message passing for state management

use super::actors::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub mod agents;
pub mod beads;
pub mod health;
pub mod websocket;
pub mod workflows;

/// Create the main API router
///
/// All routes are relative to `/api` prefix (applied in server.rs).
/// This function assembles all route modules into a single router.
///
/// # Returns
///
/// A configured `Router<AppState>` with all REST endpoints registered.
///
/// # Example
///
/// ```ignore
/// use oya_web::routes;
/// use oya_web::actors::AppState;
///
/// let state = AppState::new(/* ... */);
/// let router = routes::create_router().with_state(state);
/// ```
pub fn create_router() -> Router<AppState> {
    let api_routes = Router::new()
        // Health check
        .route("/health", get(health::health_check))
        .route("/system/health", get(health::system_health_check))
        // Workflow endpoints
        .route("/workflows", post(workflows::create_workflow))
        .route("/workflows", get(workflows::list_workflows))
        // Bead endpoints
        .route("/beads", get(beads::list_beads))
        .route("/beads/{id}", get(beads::get_bead_status))
        .route("/beads/{id}/cancel", post(beads::cancel_bead))
        .route("/beads/{id}/retry", post(beads::retry_bead))
        // Agents endpoint
        .route("/agents", get(agents::list_agents))
        .route("/agents/spawn", post(agents::spawn_agents))
        .route("/agents/scale", post(agents::scale_agents));

    Router::new()
        .nest("/api", api_routes)
        .route("/ws", get(websocket::websocket_handler))
}
