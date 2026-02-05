//! Workflow endpoints: POST /api/workflows, GET /api/workflows
//!
//! This module handles workflow creation and listing operations.
//!
//! ## Endpoints
//!
//! - `POST /api/workflows` - Create a new workflow/bead
//! - `GET /api/workflows` - List all workflows

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
///
/// Railway-Oriented Programming approach:
/// 1. Validate input (bead_spec required)
/// 2. Generate unique ULID
/// 3. Send message to scheduler
/// 4. Return created response with bead_id
///
/// All error paths use proper Result types with ? operator
pub async fn create_workflow(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkflowRequest>,
) -> impl IntoResponse {
    // Railway track: validate -> generate ID -> schedule -> respond
    let result = validate_request(&req)
        .and_then(|spec| {
            let bead_id = Ulid::new();
            schedule_bead(&state, bead_id, spec).map(|_| bead_id)
        })
        .map(|bead_id| {
            (
                StatusCode::CREATED,
                Json(CreateWorkflowResponse {
                    bead_id: bead_id.to_string(),
                }),
            )
        });

    match result {
        Ok(response) => response.into_response(),
        Err(err) => err.into_response(),
    }
}

/// Validate the create workflow request
fn validate_request(req: &CreateWorkflowRequest) -> Result<String, AppError> {
    req.bead_spec
        .as_ref()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing required field: bead_spec".to_string()))
}

/// Schedule the bead creation with the scheduler actor
fn schedule_bead(state: &AppState, bead_id: Ulid, spec: String) -> Result<(), AppError> {
    state
        .scheduler
        .send(SchedulerMessage::CreateBead { id: bead_id, spec })
        .map_err(|_| AppError::ServiceUnavailable("Scheduler actor unavailable".to_string()))
}

/// Response for listing workflows
#[derive(Debug, Serialize)]
pub struct ListWorkflowsResponse {
    workflows: Vec<WorkflowSummary>,
    total: usize,
}

/// Summary of a workflow
#[derive(Debug, Serialize)]
pub struct WorkflowSummary {
    bead_id: String,
    status: String,
    created_at: String,
}

/// GET /api/workflows - List all workflows
///
/// Returns a paginated list of all workflows in the system.
/// This is a placeholder implementation that will be replaced with
/// actual state querying in future beads.
///
/// # Returns
///
/// `Result<Json<ListWorkflowsResponse>>` - List of workflow summaries
pub async fn list_workflows(
    State(_state): State<AppState>,
) -> Result<Json<ListWorkflowsResponse>, AppError> {
    // Placeholder: return empty list
    // Future implementation will query StateManager for all workflows
    Ok(Json(ListWorkflowsResponse {
        workflows: vec![],
        total: 0,
    }))
}
