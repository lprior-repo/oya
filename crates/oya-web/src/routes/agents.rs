//! Agents endpoint - GET /api/agents
//!
//! This module provides endpoints to:
//! - List all agents with their current status
//! - Get aggregate metrics for all agents
//! - Spawn new agents
//! - Scale the agent pool

use crate::actors::AppState;
use crate::error::{AppError, Result};
use crate::metrics::AgentMetrics;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Response for agent metrics
#[derive(Debug, Serialize)]
pub struct AgentMetricsResponse {
    pub total_agents: usize,
    pub active_agents: usize,
    pub idle_agents: usize,
    pub unhealthy_agents: usize,
    pub average_uptime_secs: f64,
    pub average_health_score: f64,
    pub status_distribution: HashMap<String, usize>,
    pub capability_counts: HashMap<String, usize>,
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
            status: agent.status,
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

/// GET /api/agents/metrics - Get aggregate agent metrics
///
/// Returns aggregated metrics for all agents in the system including:
/// - Total number of agents
/// - Active vs idle agent counts
/// - Unhealthy agent count
/// - Average uptime across all agents
/// - Average health score
/// - Status distribution
/// - Capability counts
///
/// # Authentication
///
/// Requires Bearer token in Authorization header:
/// ```text
/// Authorization: Bearer <token>
/// ```
///
/// # Arguments
///
/// * `headers` - HTTP headers containing Authorization token
/// * `state` - Application state containing agent service
///
/// # Returns
///
/// `Result<Json<AgentMetricsResponse>>` - Agent metrics or error
///
/// # Errors
///
/// * `AppError::Unauthorized` (401) - Missing or invalid authentication
/// * `AppError::NotFound` (404) - If no agents are found
/// * `AppError::Internal` (500) - If metrics calculation fails
pub async fn get_agent_metrics(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<AgentMetricsResponse>> {
    // Railway track: authenticate -> query agents -> calculate metrics -> respond
    authenticate(headers).and_then(|_| {
        let agents = state.agent_service.list_agents().await;

        if agents.is_empty() {
            return Err(AppError::NotFound("No agents found for metrics calculation".to_string()));
        }

        match AgentMetrics::calculate(&agents) {
            Ok(metrics) => Ok(Json(AgentMetricsResponse {
                total_agents: metrics.total_agents,
                active_agents: metrics.active_agents,
                idle_agents: metrics.idle_agents,
                unhealthy_agents: metrics.unhealthy_agents,
                average_uptime_secs: metrics.average_uptime_secs,
                average_health_score: metrics.average_health_score,
                status_distribution: metrics.status_distribution,
                capability_counts: metrics.capability_counts,
            })),
            Err(e) => Err(AppError::Internal(format!("Failed to calculate metrics: {e}"))),
        }
    })
}

/// Authenticate the request using Bearer token
fn authenticate(headers: HeaderMap) -> Result<()> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| {
            if let Some(token) = auth.strip_prefix("Bearer ") {
                Some(token.to_string())
            } else {
                None
            }
        })
        .filter(|token| !token.is_empty())
        .map(|_| ())
        .ok_or_else(|| AppError::Unauthorized("Missing or invalid Authorization header".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    /// Test authentication with valid token
    #[test]
    fn test_authenticate_valid_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer test-token"),
        );

        let result = authenticate(headers);
        assert!(result.is_ok());
    }

    /// Test authentication fails with missing header
    #[test]
    fn test_authenticate_missing_header() {
        let headers = HeaderMap::new();

        let result = authenticate(headers);
        assert!(result.is_err());
        match result {
            Err(AppError::Unauthorized(msg)) => {
                assert!(msg.contains("Missing or invalid"));
            }
            _ => panic!("Expected Unauthorized error"),
        }
    }

    /// Test authentication fails with invalid scheme
    #[test]
    fn test_authenticate_invalid_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Basic token"));

        let result = authenticate(headers);
        assert!(result.is_err());
    }

    /// Test authentication fails with empty token
    #[test]
    fn test_authenticate_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer "));

        let result = authenticate(headers);
        assert!(result.is_err());
    }
}
