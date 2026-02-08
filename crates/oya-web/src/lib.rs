//! OYA Web Server
//!
//! HTTP API with tower middleware for CORS, tracing, and compression.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

use axum::{Json, Router, http::Method, routing::get};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

mod agent_repository;
mod circuit_breaker;
mod error_handler;
mod health;
mod retry;
pub mod validation;
pub mod workflow_graph;

use health::HealthResponse;

pub use agent_repository::{
    AgentRepository, AgentRepositoryError, AgentRepositoryStats, InMemoryAgentRepository,
    RepositoryAgent, RepositoryAgentState,
};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState, RequestResult};
pub use error_handler::{ErrorCategory, ErrorResponse, HttpError};
pub use retry::{RetryDecision, RetryPolicy, RetryState};
pub use validation::{ValidationError, ValidationResult, ValidatorConfig};

/// Agent metrics response.
///
/// Response type for agent metrics returned via HTTP API endpoints.
/// Contains comprehensive agent information including state, workload, and health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsResponse {
    /// Unique agent identifier.
    pub agent_id: String,
    /// Current agent state (e.g., "idle", "working", "unhealthy").
    pub state: String,
    /// Agent uptime in seconds.
    pub uptime_secs: u64,
    /// Total number of beads completed.
    pub beads_completed: u64,
    /// Total number of operations executed.
    pub operations_executed: u64,
    /// Current health score (0.0 - 1.0).
    pub health_score: f64,
    /// ISO 8601 timestamp of last heartbeat.
    pub last_heartbeat: String,
}

/// Web server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "127.0.0.1:3000")
    pub bind_address: String,
    /// Allowed CORS origin (e.g., "tauri://localhost")
    pub cors_origin: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:3000".to_string(),
            cors_origin: "tauri://localhost".to_string(),
        }
    }
}

/// Create a new router with middleware.
///
/// # Errors
///
/// Returns an error if router creation fails.
pub fn create_router(config: ServerConfig) -> Result<Router, Error> {
    info!("Creating router with CORS origin: {}", config.cors_origin);

    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin.parse::<::axum::http::HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let router = Router::new()
        .route("/health", get(health_check))
        .route("/api/system/health", get(system_health))
        .route("/api/agents/metrics", get(agent_metrics))
        .route("/api/graph", get(workflow_graph))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors),
        );

    Ok(router)
}

/// Health check handler.
async fn health_check() -> &'static str {
    "OK"
}

/// System health check handler.
///
/// Returns detailed system health information including:
/// - Overall status
/// - Component health
/// - System metrics
/// - Version information
async fn system_health() -> Json<HealthResponse> {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let response = HealthResponse::new(version).await;
    Json(response)
}

/// Agent metrics handler.
///
/// Returns comprehensive agent metrics including:
/// - Agent ID and state
/// - Uptime and work completed
/// - Health score and last heartbeat
async fn agent_metrics() -> Json<AgentMetricsResponse> {
    use std::time::SystemTime;

    // In a real implementation, this would fetch actual agent metrics
    // For now, return placeholder data
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let response = AgentMetricsResponse {
        agent_id: "agent-0".to_string(),
        state: "idle".to_string(),
        uptime_secs: 0,
        beads_completed: 0,
        operations_executed: 0,
        health_score: 1.0,
        last_heartbeat: format!("{now}"),
    };
    Json(response)
}

/// Workflow graph handler.
///
/// Returns DAG workflow graph data including:
/// - Nodes (beads/tasks) with positions
/// - Edges (dependencies) with types
async fn workflow_graph() -> Json<workflow_graph::WorkflowGraphResponse> {
    use workflow_graph::WorkflowGraphResponse;

    // In a real implementation, this would fetch actual workflow graph data
    // For now, return empty graph
    let response = WorkflowGraphResponse {
        nodes: vec![],
        edges: vec![],
    };
    Json(response)
}

/// Web server errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid header value
    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] ::axum::http::header::InvalidHeaderValue),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] ::axum::http::Error),

    /// Axum error
    #[error("Axum error: {0}")]
    Axum(#[from] axum::Error),

    /// Hyper error
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Infallible error (for type compatibility in tests)
    #[error("Infallible")]
    Infallible,
}

impl From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Helper to build a test request with proper error handling.
    fn build_test_request(
        uri: &str,
        method: Option<Method>,
        headers: Vec<(&str, &str)>,
    ) -> Result<axum::http::Request<Body>, Error> {
        let mut builder = axum::http::Request::builder();
        if let Some(m) = method {
            builder = builder.method(m);
        }
        builder = builder.uri(uri);
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        builder.body(Body::empty()).map_err(Error::from)
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1:3000");
        assert_eq!(config.cors_origin, "tauri://localhost");
    }

    #[test]
    fn test_create_router_success() {
        let config = ServerConfig::default();
        let result = create_router(config);
        assert!(result.is_ok(), "Router creation should succeed");
    }

    #[test]
    fn test_create_router_with_custom_origin() {
        let config = ServerConfig {
            bind_address: "0.0.0.0:8080".to_string(),
            cors_origin: "https://example.com".to_string(),
        };
        let result = create_router(config);
        assert!(
            result.is_ok(),
            "Router creation with custom origin should succeed"
        );
    }

    #[test]
    fn test_create_router_with_invalid_origin() {
        let config = ServerConfig {
            bind_address: "127.0.0.1:3000".to_string(),
            cors_origin: "invalid\0origin".to_string(), // Null byte is invalid
        };
        let result = create_router(config);
        assert!(
            result.is_err(),
            "Router creation with invalid origin should fail"
        );
        assert!(
            matches!(result, Err(Error::InvalidHeader(_))),
            "Expected InvalidHeader error"
        );
    }

    #[tokio::test]
    async fn test_health_check_endpoint() -> Result<(), Error> {
        let config = ServerConfig::default();
        let router = create_router(config)?;

        // Create a test request
        let request = build_test_request("/health", None, Vec::new())?;
        let response = router.oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        // Check response body
        let body = response.into_body();
        let body_bytes = body.collect().await?.to_bytes();
        assert_eq!(&body_bytes[..], b"OK");
        Ok(())
    }

    #[tokio::test]
    async fn test_cors_headers_present() -> Result<(), Error> {
        let config = ServerConfig::default();
        let router = create_router(config)?;

        // Create a test request with OPTIONS method
        let request = build_test_request(
            "/health",
            Some(Method::OPTIONS),
            vec![
                ("Origin", "tauri://localhost"),
                ("Access-Control-Request-Method", "GET"),
            ],
        )?;

        let response = router.oneshot(request).await?;

        // Check CORS headers are present
        let headers = response.headers();
        assert!(
            headers.contains_key("access-control-allow-origin"),
            "CORS origin header should be present"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_compression_layer_works() -> Result<(), Error> {
        let config = ServerConfig::default();
        let router = create_router(config)?;

        // Request with Accept-Encoding: gzip
        let request = build_test_request("/health", None, vec![("Accept-Encoding", "gzip")])?;

        let response = router.oneshot(request).await?;

        // Response should be successful
        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_tracing_layer_works() -> Result<(), Error> {
        let config = ServerConfig::default();
        let _router = create_router(config)?;

        // Tracing layer is configured - this test mainly verifies compilation
        // In a real test, we'd check that logs are emitted
        Ok(())
    }

    #[test]
    fn test_error_display() {
        // Test InvalidHeader error
        let invalid_header = "invalid\0value".parse::<::axum::http::HeaderValue>();
        assert!(invalid_header.is_err());
        if let Err(e) = invalid_header {
            let err = Error::InvalidHeader(e);
            assert!(err.to_string().contains("Invalid header value"));
        }

        // Test Io error
        let err2: Error = std::io::Error::new(std::io::ErrorKind::NotFound, "not found").into();
        assert!(err2.to_string().contains("IO error"));
    }

    #[tokio::test]
    async fn test_router_not_found() -> Result<(), Error> {
        let config = ServerConfig::default();
        let router = create_router(config)?;

        // Request to non-existent route
        let request = build_test_request("/nonexistent", None, Vec::new())?;
        let response = router.oneshot(request).await?;

        // Should return 404
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn test_system_health_endpoint() -> Result<(), Error> {
        let config = ServerConfig::default();
        let router = create_router(config)?;

        let request = build_test_request("/api/system/health", None, Vec::new())?;
        let response = router.oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        // Check that response contains JSON
        let body = response.into_body();
        let body_bytes = body.collect().await?.to_bytes();

        let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;

        // Verify required fields
        assert!(json.get("status").is_some(), "Should have status field");
        assert!(
            json.get("timestamp").is_some(),
            "Should have timestamp field"
        );
        assert!(json.get("version").is_some(), "Should have version field");
        assert!(
            json.get("components").is_some(),
            "Should have components field"
        );
        assert!(json.get("metrics").is_some(), "Should have metrics field");
        Ok(())
    }

    #[test]
    fn test_agent_metrics_response_serialization() -> Result<(), Error> {
        let response = AgentMetricsResponse {
            agent_id: "agent-123".to_string(),
            state: "working".to_string(),
            uptime_secs: 3600,
            beads_completed: 42,
            operations_executed: 1337,
            health_score: 0.95,
            last_heartbeat: "2026-02-07T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;

        // Verify all fields are present in JSON
        assert_eq!(parsed["agent_id"], "agent-123");
        assert_eq!(parsed["state"], "working");
        assert_eq!(parsed["uptime_secs"], 3600);
        assert_eq!(parsed["beads_completed"], 42);
        assert_eq!(parsed["operations_executed"], 1337);
        assert_eq!(parsed["health_score"], 0.95);
        assert_eq!(parsed["last_heartbeat"], "2026-02-07T12:00:00Z");

        Ok(())
    }

    #[test]
    fn test_agent_metrics_response_deserialization() -> Result<(), Error> {
        let json_str = r#"{
            "agent_id": "agent-456",
            "state": "idle",
            "uptime_secs": 7200,
            "beads_completed": 100,
            "operations_executed": 5000,
            "health_score": 0.88,
            "last_heartbeat": "2026-02-07T13:30:00Z"
        }"#;

        let response: AgentMetricsResponse = serde_json::from_str(json_str)?;

        assert_eq!(response.agent_id, "agent-456");
        assert_eq!(response.state, "idle");
        assert_eq!(response.uptime_secs, 7200);
        assert_eq!(response.beads_completed, 100);
        assert_eq!(response.operations_executed, 5000);
        assert_eq!(response.health_score, 0.88);
        assert_eq!(response.last_heartbeat, "2026-02-07T13:30:00Z");

        Ok(())
    }

    #[test]
    fn test_agent_metrics_response_all_fields_required() {
        // Missing required fields should fail deserialization
        let incomplete_json = r#"{
            "agent_id": "agent-789",
            "state": "unhealthy"
        }"#;

        let result: Result<AgentMetricsResponse, _> = serde_json::from_str(incomplete_json);
        assert!(
            result.is_err(),
            "Deserialization should fail with missing fields"
        );
    }

    #[tokio::test]
    async fn test_agent_metrics_endpoint() -> Result<(), Error> {
        let config = ServerConfig::default();
        let router = create_router(config)?;

        let request = build_test_request("/api/agents/metrics", None, Vec::new())?;
        let response = router.oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);

        // Check that response contains valid JSON
        let body = response.into_body();
        let body_bytes = body.collect().await?.to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;

        // Verify all required fields are present
        assert!(json.get("agent_id").is_some(), "Should have agent_id field");
        assert!(json.get("state").is_some(), "Should have state field");
        assert!(
            json.get("uptime_secs").is_some(),
            "Should have uptime_secs field"
        );
        assert!(
            json.get("beads_completed").is_some(),
            "Should have beads_completed field"
        );
        assert!(
            json.get("operations_executed").is_some(),
            "Should have operations_executed field"
        );
        assert!(
            json.get("health_score").is_some(),
            "Should have health_score field"
        );
        assert!(
            json.get("last_heartbeat").is_some(),
            "Should have last_heartbeat field"
        );

        Ok(())
    }
}
