//! Agent metrics endpoint tests
//!
//! Tests for GET /api/agents/metrics endpoint that returns agent metrics information.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![allow(clippy::expect_used)] // Tests are allowed to use expect

use axum::{body::Body, http::StatusCode};
use http_body_util::BodyExt;
use oya_web::{AgentMetricsResponse, ServerConfig, create_router};
use serde_json::Value;
use tower::ServiceExt;

/// Test helper to make requests and parse JSON responses
async fn get_json(path: &str) -> (StatusCode, Value) {
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    let status = response.status();

    let body = response.into_body();
    let body_bytes = body
        .collect()
        .await
        .expect("Failed to collect body")
        .to_bytes();

    let json: Value = serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    (status, json)
}

#[tokio::test]
async fn test_returns_agent_metrics_when_endpoint_is_called() {
    // Given: The server is running with the metrics endpoint configured
    // When: A GET request is made to /api/agents/metrics
    let (status, json) = get_json("/api/agents/metrics").await;

    // Then: The response should be successful (200 OK)
    assert_eq!(
        status,
        StatusCode::OK,
        "Metrics endpoint should return 200 OK"
    );

    // And: The response should contain agent_id field
    assert!(
        json.get("agent_id").is_some(),
        "Response should include agent_id field"
    );
}

#[tokio::test]
async fn test_includes_agent_id_field_when_metrics_is_requested() {
    // Given: The metrics endpoint exists
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: The agent_id field should be a string
    let agent_id = json
        .get("agent_id")
        .and_then(|v| v.as_str())
        .expect("Agent ID should be a string");

    // And: Agent ID should not be empty
    assert!(!agent_id.is_empty(), "Agent ID should not be empty");
}

#[tokio::test]
async fn test_includes_state_field_when_metrics_is_requested() {
    // Given: The agent has a state
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: State field should be present
    let state = json
        .get("state")
        .and_then(|v| v.as_str())
        .expect("State should be a string");

    // And: State should be one of expected values
    assert!(
        matches!(state, "idle" | "working" | "unhealthy"),
        "State should be idle, working, or unhealthy"
    );
}

#[tokio::test]
async fn test_includes_uptime_field_when_metrics_is_requested() {
    // Given: The agent has been running
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: Uptime should be present
    let _uptime = json
        .get("uptime_secs")
        .and_then(|v| v.as_u64())
        .expect("Uptime should be a number");

    // And: Uptime is a u64 which is always non-negative
    // No comparison needed as u64 >= 0 is always true
}

#[tokio::test]
async fn test_includes_beads_completed_field_when_metrics_is_requested() {
    // Given: The agent has completed beads
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: Beads completed should be present
    let _beads_completed = json
        .get("beads_completed")
        .and_then(|v| v.as_u64())
        .expect("Beads completed should be a number");

    // And: beads_completed is a u64 which is always non-negative
    // No comparison needed as u64 >= 0 is always true
}

#[tokio::test]
async fn test_includes_operations_executed_field_when_metrics_is_requested() {
    // Given: The agent has executed operations
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: Operations executed should be present
    let _operations = json
        .get("operations_executed")
        .and_then(|v| v.as_u64())
        .expect("Operations executed should be a number");

    // And: operations is a u64 which is always non-negative
    // No comparison needed as u64 >= 0 is always true
}

#[tokio::test]
async fn test_includes_health_score_field_when_metrics_is_requested() {
    // Given: The agent has a health score
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: Health score should be present
    let health_score = json
        .get("health_score")
        .and_then(|v| v.as_f64())
        .expect("Health score should be a number");

    // And: Should be between 0.0 and 1.0
    assert!(
        (0.0..=1.0).contains(&health_score),
        "Health score should be between 0.0 and 1.0"
    );
}

#[tokio::test]
async fn test_includes_last_heartbeat_field_when_metrics_is_requested() {
    // Given: The agent sends heartbeats
    // When: Metrics are requested
    let (_status, json) = get_json("/api/agents/metrics").await;

    // Then: Last heartbeat should be present
    let last_heartbeat = json
        .get("last_heartbeat")
        .and_then(|v| v.as_str())
        .expect("Last heartbeat should be a string");

    // And: Should not be empty
    assert!(
        !last_heartbeat.is_empty(),
        "Last heartbeat should not be empty"
    );
}

#[tokio::test]
async fn test_returns_valid_json_content_type_when_metrics_is_requested() {
    // Given: The metrics endpoint exists
    // When: A request is made to /api/agents/metrics
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/agents/metrics")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    // Then: The Content-Type header should be application/json
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .expect("Content-Type header should be present");

    assert!(
        content_type.contains("application/json"),
        "Content-Type should be application/json, got: {content_type}"
    );
}

#[tokio::test]
async fn test_handles_cors_headers_when_metrics_is_requested_from_browser() {
    // Given: A browser making a cross-origin request
    // When: Metrics are requested
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/agents/metrics")
                .header("Origin", "tauri://localhost")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    // Then: CORS headers should be present
    let headers = response.headers();
    assert!(
        headers.contains_key("access-control-allow-origin"),
        "CORS origin header should be present"
    );

    // And: Response should be successful
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_response_size_is_reasonable() {
    // Given: The metrics endpoint returns agent information
    // When: A request is made
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/agents/metrics")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    // Then: Response size should be reasonable (< 10KB)
    let body = response.into_body();
    let body_bytes = body
        .collect()
        .await
        .expect("Failed to collect body")
        .to_bytes();

    assert!(
        body_bytes.len() < 10240,
        "Response size should be reasonable (< 10KB), got: {} bytes",
        body_bytes.len()
    );
}

#[tokio::test]
async fn test_metrics_response_struct_matches_json_schema() {
    // Given: An AgentMetricsResponse struct
    // When: It's serialized to JSON
    let metrics = AgentMetricsResponse {
        agent_id: "test-agent".to_string(),
        state: "idle".to_string(),
        uptime_secs: 3600,
        beads_completed: 42,
        operations_executed: 1337,
        health_score: 0.95,
        last_heartbeat: "2026-02-08T04:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&metrics).expect("Serialization should succeed");

    // Then: All fields should be present
    assert!(json.contains("agent_id"), "Should contain agent_id");
    assert!(json.contains("state"), "Should contain state");
    assert!(json.contains("uptime_secs"), "Should contain uptime_secs");
    assert!(
        json.contains("beads_completed"),
        "Should contain beads_completed"
    );
    assert!(
        json.contains("operations_executed"),
        "Should contain operations_executed"
    );
    assert!(json.contains("health_score"), "Should contain health_score");
    assert!(
        json.contains("last_heartbeat"),
        "Should contain last_heartbeat"
    );
}
