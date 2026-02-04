//! Agent API client for CLI.

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use oya_pipeline::{Error, Result};

const DEFAULT_SERVER: &str = "http://localhost:3000";

#[derive(Debug, Clone)]
pub struct AgentApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgentApiClient {
    pub fn new(server: Option<&str>) -> Self {
        let base_url = server.map_or_else(|| DEFAULT_SERVER.to_string(), normalize_server);
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn spawn(&self, count: usize) -> Result<SpawnAgentsResponse> {
        let url = format!("{}/api/agents/spawn", self.base_url);
        let payload = SpawnAgentsRequest { count };
        let response = self.client.post(url).json(&payload).send().await?;
        parse_response(response).await
    }

    pub async fn scale(&self, target: usize) -> Result<ScaleAgentsResponse> {
        let url = format!("{}/api/agents/scale", self.base_url);
        let payload = ScaleAgentsRequest { target };
        let response = self.client.post(url).json(&payload).send().await?;
        parse_response(response).await
    }

    pub async fn list(&self) -> Result<ListAgentsResponse> {
        let url = format!("{}/api/agents", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_response(response).await
    }
}

#[derive(Debug, Serialize)]
struct SpawnAgentsRequest {
    count: usize,
}

#[derive(Debug, Serialize)]
struct ScaleAgentsRequest {
    target: usize,
}

#[derive(Debug, Deserialize)]
pub struct SpawnAgentsResponse {
    pub agent_ids: Vec<String>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ScaleAgentsResponse {
    pub previous: usize,
    pub total: usize,
    pub spawned: Vec<String>,
    pub terminated: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentSummary>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct AgentSummary {
    pub id: String,
    pub status: String,
    pub current_bead: Option<String>,
    pub health_score: f64,
    pub uptime_secs: u64,
    pub capabilities: Vec<String>,
}

fn normalize_server(server: &str) -> String {
    server.trim_end_matches('/').to_string()
}

async fn parse_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(Error::InvalidRecord {
            reason: format!("API error {}: {}", status, body),
        });
    }

    serde_json::from_str(&body).map_err(|err| Error::InvalidRecord {
        reason: format!("Invalid API response: {}", err),
    })
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::InvalidRecord {
            reason: format!("HTTP request failed: {}", err),
        }
    }
}
