//! Workflow endpoints: POST /api/workflows

use super::super::actors::{AppState, SchedulerMessage};
use super::super::error::{AppError, Result};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Request payload for creating a workflow
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    bead_spec: String,
}

/// Response for creating a workflow
#[derive(Debug, Serialize)]
pub struct CreateWorkflowResponse {
    bead_id: String,
}

/// POST /api/workflows - Create a new workflow/bead
pub async fn create_workflow(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<Json<CreateWorkflowResponse>> {
    let bead_id = Ulid::new();

    state
        .scheduler
        .send(SchedulerMessage::CreateBead {
            spec: req.bead_spec,
        })
        .map_err(|_| AppError::ServiceUnavailable("Scheduler actor unavailable".to_string()))?;

    Ok(Json(CreateWorkflowResponse {
        bead_id: bead_id.to_string(),
    }))
}
