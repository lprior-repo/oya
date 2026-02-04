//! Agents endpoint - GET /api/agents
//!
//! This module provides an endpoint to list all agents with their
//! current status and associated bead (if any).

use crate::actors::{AppState, StateManagerMessage};
use crate::error::{AppError, Result};
use axum::{extract::State, response::Json, routing::get};

/// Agent summary for API responses
#[derive(Debug, serde::Serialize)]
pub struct AgentSummary {
    pub id: String,
    pub status: String,
    pub current_bead: Option<String>,
}

/// Response for listing agents
#[derive(Debug, serde::Serialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentSummary>,
    pub total: usize,
}

/// GET /api/agents - List all agents
///
/// Returns a list of all agents in the system with basic information:
/// - Agent ID
/// - Current status (running, idle, etc.)
/// - Associated bead ID (if currently working on one)
///
/// # Arguments
///
/// * `state` - Application state containing actor handles
///
/// # Returns
///
/// `Result<Json<ListAgentsResponse>>` - List of agents or error
///
/// # Errors
///
/// * `AppError::ServiceUnavailable` - If state manager is unavailable
/// * `AppError::Internal` - If state manager fails to respond
/// * `AppError::NotFound` - If no agents are found
pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<ListAgentsResponse>> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    state
        .state_manager
        .send(StateManagerMessage::QueryAllAgents { response: tx })
        .map_err(|_| AppError::ServiceUnavailable("State manager unavailable".to_string()))?;

    let agents = rx.await
        .map_err(|_| AppError::Internal("State manager failed to respond".to_string()))?
        .ok_or_else(|| AppError::NotFound("No agents found".to_string()))?;

    Ok(Json(ListAgentsResponse {
        agents,
        total: agents.len(),
    }))
}
