//! Bead endpoints: GET /api/beads, GET /api/beads/:id, POST /api/beads/:id/cancel, POST /api/beads/:id/retry
//!
//! This module handles all bead-related operations including querying status,
//! cancellation, retry, and listing.
//!
//! ## Endpoints
//!
//! - `GET /api/beads` - List all beads
//! - `GET /api/beads/:id` - Query bead status by ID
//! - `POST /api/beads/:id/cancel` - Cancel a running bead
//! - `POST /api/beads/:id/retry` - Retry a failed bead

use super::super::actors::{AppState, SchedulerMessage, StateManagerMessage};
use super::super::error::{AppError, Result};
use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::Serialize;
use ulid::Ulid;

/// Response for bead status
#[derive(Debug, Serialize)]
pub struct BeadStatusResponse {
    id: String,
    status: String,
    phase: String,
    events: Vec<String>,
    created_at: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<String>,
}

/// Response for bead cancellation
#[derive(Debug, Serialize)]
pub struct CancelBeadResponse {
    message: String,
}

/// Response for bead retry
#[derive(Debug, Serialize)]
pub struct RetryBeadResponse {
    message: String,
}

/// GET /api/beads/:id - Query bead status
///
/// Retrieves the current status of a bead by its ID.
///
/// # Arguments
///
/// * `id` - The ULID of the bead to query
/// * `state` - Application state containing actor handles
///
/// # Returns
///
/// `Result<Json<BeadStatusResponse>>` - Bead status or error
///
/// # Errors
///
/// * `AppError::BadRequest` - If the bead ID is not a valid ULID
/// * `AppError::NotFound` - If the bead does not exist
/// * `AppError::ServiceUnavailable` - If the state manager is unavailable
/// * `AppError::Internal` - If the state manager fails to respond
pub async fn get_bead_status(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<BeadStatusResponse>> {
    // Parse and validate the bead ID
    let bead_id = id
        .parse::<Ulid>()
        .map_err(|_| AppError::BadRequest("Invalid bead ID format".to_string()))?;

    // Create a oneshot channel for the response
    let (tx, rx) = tokio::sync::oneshot::channel();

    // Send query message to state manager
    state
        .state_manager
        .send(StateManagerMessage::QueryBead {
            id: bead_id,
            response: tx,
        })
        .map_err(|_| AppError::ServiceUnavailable("State manager unavailable".to_string()))?;

    // Wait for response from state manager
    let bead_state = rx
        .await
        .map_err(|_| AppError::Internal("State manager failed to respond".to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Bead {} not found", bead_id)))?;

    // Convert internal BeadState to response
    Ok(Json(BeadStatusResponse {
        id: bead_state.id.to_string(),
        status: bead_state.status,
        phase: bead_state.phase,
        events: bead_state.events,
        created_at: bead_state.created_at,
        updated_at: bead_state.updated_at,
        title: bead_state.title,
        dependencies: bead_state.dependencies,
    }))
}

/// POST /api/beads/:id/cancel - Cancel a bead
pub async fn cancel_bead(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<CancelBeadResponse>> {
    let bead_id = id
        .parse::<Ulid>()
        .map_err(|_| AppError::BadRequest("Invalid bead ID".to_string()))?;

    state
        .scheduler
        .send(SchedulerMessage::CancelBead { id: bead_id })
        .map_err(|_| AppError::ServiceUnavailable("Scheduler unavailable".to_string()))?;

    Ok(Json(CancelBeadResponse {
        message: format!("Bead {} cancellation requested", bead_id),
    }))
}

/// POST /api/beads/:id/retry - Retry a failed bead
pub async fn retry_bead(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RetryBeadResponse>> {
    let bead_id = id
        .parse::<Ulid>()
        .map_err(|_| AppError::BadRequest("Invalid bead ID".to_string()))?;

    state
        .scheduler
        .send(SchedulerMessage::RetryBead { id: bead_id })
        .map_err(|_| AppError::ServiceUnavailable("Scheduler unavailable".to_string()))?;

    Ok(Json(RetryBeadResponse {
        message: format!("Bead {} retry requested", bead_id),
    }))
}

/// Response for listing beads
#[derive(Debug, Serialize)]
pub struct ListBeadsResponse {
    beads: Vec<BeadSummary>,
    total: usize,
}

/// Summary of a bead
#[derive(Debug, Serialize)]
pub struct BeadSummary {
    id: String,
    status: String,
    phase: String,
    created_at: String,
}

/// GET /api/beads - List all beads
///
/// Returns a paginated list of all beads in the system.
/// This is a placeholder implementation that will be replaced with
/// actual state querying in future beads.
///
/// # Returns
///
/// `Result<Json<ListBeadsResponse>>` - List of bead summaries
pub async fn list_beads(State(_state): State<AppState>) -> Result<Json<ListBeadsResponse>> {
    // Placeholder: return empty list
    // Future implementation will query StateManager for all beads
    Ok(Json(ListBeadsResponse {
        beads: vec![],
        total: 0,
    }))
}
