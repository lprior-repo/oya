//! Bead endpoints: GET /api/beads/:id, POST /api/beads/:id/cancel

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
