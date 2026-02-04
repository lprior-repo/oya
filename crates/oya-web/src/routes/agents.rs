//! Agents endpoint - GET /api/agents
//!
//! This module provides an endpoint to list all agents with their
//! current status and associated bead (if any).

use crate::actors::AppState;
use crate::error::{AppError, Result};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};

/// Agent summary for API responses
#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub id: String,
    pub status: String,
    pub current_bead: Option<String>,
    pub health_score: f64,
    pub uptime_secs: u64,
    pub capabilities: Vec<String>,
}

/// Response for listing agents
#[derive(Debug, Serialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentSummary>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct SpawnAgentsRequest {
    pub count: usize,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SpawnAgentsResponse {
    pub agent_ids: Vec<String>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ScaleAgentsRequest {
    pub target: usize,
}

#[derive(Debug, Serialize)]
pub struct ScaleAgentsResponse {
    pub previous: usize,
    pub total: usize,
    pub spawned: Vec<String>,
    pub terminated: Vec<String>,
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
pub async fn list_agents(State(state): State<AppState>) -> Result<Json<ListAgentsResponse>> {
    let agents = state.agent_service.list_agents().await;

    if agents.is_empty() {
        return Err(AppError::NotFound("No agents found".to_string()));
    }

    let total = agents.len();
    let agents = agents
        .into_iter()
        .map(|agent| AgentSummary {
            id: agent.id,
            status: agent.state.to_string(),
            current_bead: agent.current_bead,
            health_score: agent.health_score,
            uptime_secs: agent.uptime_secs,
            capabilities: agent.capabilities,
        })
        .collect::<Vec<_>>();

    Ok(Json(ListAgentsResponse { agents, total }))
}

/// POST /api/agents/spawn - Spawn new agents
pub async fn spawn_agents(
    State(state): State<AppState>,
    Json(payload): Json<SpawnAgentsRequest>,
) -> Result<Json<SpawnAgentsResponse>> {
    let result = state
        .agent_service
        .spawn_agents(payload.count, payload.capabilities)
        .await?;

    Ok(Json(SpawnAgentsResponse {
        agent_ids: result.agent_ids,
        total: result.total,
    }))
}

/// POST /api/agents/scale - Scale agent pool to target size
pub async fn scale_agents(
    State(state): State<AppState>,
    Json(payload): Json<ScaleAgentsRequest>,
) -> Result<Json<ScaleAgentsResponse>> {
    let result = state.agent_service.scale_to(payload.target).await?;
    Ok(Json(ScaleAgentsResponse {
        previous: result.previous,
        total: result.total,
        spawned: result.spawned,
        terminated: result.terminated,
    }))
}
