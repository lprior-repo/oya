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
pub async fn get_bead_status(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<BeadStatusResponse>> {
    let bead_id = id
        .parse::<Ulid>()
        .map_err(|_| AppError::BadRequest("Invalid bead ID".to_string()))?;

    state
        .state_manager
        .send(StateManagerMessage::QueryBead { id: bead_id })
        .map_err(|_| AppError::ServiceUnavailable("State manager unavailable".to_string()))?;

    Ok(Json(BeadStatusResponse {
        id: bead_id.to_string(),
        status: "pending".to_string(),
        phase: "initializing".to_string(),
        events: vec![],
        created_at: "2026-02-01T00:00:00Z".to_string(),
        updated_at: "2026-02-01T00:00:00Z".to_string(),
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
