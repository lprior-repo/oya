//! Workflow endpoints: POST /api/workflows

use super::super::actors::{AppState, SchedulerMessage};
use super::super::error::AppError;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Request payload for creating a workflow
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    #[serde(default)]
    bead_spec: Option<String>,
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
) -> impl IntoResponse {
    match req.bead_spec {
        Some(spec) => {
            let bead_id = Ulid::new();

            match state.scheduler.send(SchedulerMessage::CreateBead { spec }) {
                Ok(_) => (
                    StatusCode::CREATED,
                    Json(CreateWorkflowResponse {
                        bead_id: bead_id.to_string(),
                    }),
                )
                    .into_response(),
                Err(_) => AppError::ServiceUnavailable("Scheduler actor unavailable".to_string())
                    .into_response(),
            }
        }
        None => {
            AppError::BadRequest("Missing required field: bead_spec".to_string()).into_response()
        }
    }
}
