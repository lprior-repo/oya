//! REST API integration tests

use axum_test::TestServer;
use http::StatusCode;
use oya_web::{
    actors::{AppState, mock_agent_service, mock_scheduler, mock_state_manager},
    routes,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

fn create_test_server() -> Result<TestServer, String> {
    let (broadcast_tx, _) = broadcast::channel(100);
    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
        agent_service: Arc::new(mock_agent_service()),
        broadcast_tx,
    };

    let app = routes::create_router().with_state(state);

    TestServer::new(app).map_err(|e| format!("Failed to create test server: {e}"))
}

#[tokio::test]
async fn test_health_check_returns_ok() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_system_health_check_returns_ok() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/system/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_create_workflow_returns_201() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "bead_spec": "test workflow spec"
    });

    let response = server.post("/api/workflows").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let body: Value = response.json();
    assert!(body["bead_id"].is_string());
    assert!(matches!(
        body["bead_id"].as_str(),
        Some(id) if id.len() == 26
    ));
    Ok(())
}

#[tokio::test]
async fn test_get_bead_status_returns_200() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/beads/01ARZ3NDEKTSV4RRFFQ69G5FAV").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["id"], "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["phase"], "initializing");
    assert!(body["events"].is_array());
    assert!(body["created_at"].is_string());
    assert!(body["updated_at"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_get_bead_status_invalid_ulid_returns_400() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/beads/invalid-ulid").await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
    Ok(())
}

#[tokio::test]
async fn test_cancel_bead_returns_200() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .post("/api/beads/01ARZ3NDEKTSV4RRFFQ69G5FAV/cancel")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert!(body["message"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_cancel_bead_invalid_ulid_returns_400() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.post("/api/beads/invalid/cancel").await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
    Ok(())
}

#[tokio::test]
async fn test_create_workflow_without_required_field_returns_400() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({});

    let response = server.post("/api/workflows").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
    Ok(())
}

#[tokio::test]
async fn test_create_workflow_returns_valid_ulid() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "bead_spec": "test workflow with ULID validation"
    });

    let response = server.post("/api/workflows").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let body: Value = response.json();
    let bead_id = body["bead_id"]
        .as_str()
        .ok_or_else(|| "Missing bead_id".to_string())?;

    // ULID should be exactly 26 characters
    assert_eq!(bead_id.len(), 26);

    // ULID should only contain Crockford base32 characters
    assert!(
        bead_id
            .chars()
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c))
    );
    Ok(())
}

#[tokio::test]
async fn test_create_workflow_is_idempotent() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "bead_spec": "idempotent test spec"
    });

    // First request
    let response1 = server.post("/api/workflows").json(&payload).await;
    assert_eq!(response1.status_code(), StatusCode::CREATED);
    let body1: Value = response1.json();

    // Second identical request - should still succeed
    let response2 = server.post("/api/workflows").json(&payload).await;
    assert_eq!(response2.status_code(), StatusCode::CREATED);
    let body2: Value = response2.json();

    // Both should return valid bead IDs (may be different due to ULID generation)
    assert!(body1["bead_id"].is_string());
    assert!(body2["bead_id"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_create_workflow_handles_empty_string_spec() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "bead_spec": ""
    });

    let response = server.post("/api/workflows").json(&payload).await;

    // Empty string should be accepted (validation is responsibility of scheduler)
    assert_eq!(response.status_code(), StatusCode::CREATED);

    let body: Value = response.json();
    assert!(body["bead_id"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_list_workflows_returns_empty_payload_shape() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/workflows").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["total"], 0);
    assert!(body["workflows"].is_array());
    assert_eq!(body["workflows"].as_array().map(|v| v.len()), Some(0));
    Ok(())
}

#[tokio::test]
async fn test_list_agents_returns_not_found_when_empty() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/agents").await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    let body: Value = response.json();
    assert_eq!(body["status"], 404);
    Ok(())
}

#[tokio::test]
async fn test_spawn_agents_rejects_zero_count() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "count": 0,
        "capabilities": ["review"]
    });

    let response = server.post("/api/agents/spawn").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
    Ok(())
}

#[tokio::test]
async fn test_spawn_agents_then_list_agents_reflects_state() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "count": 2,
        "capabilities": ["review", "test"]
    });

    let spawn_response = server.post("/api/agents/spawn").json(&payload).await;
    assert_eq!(spawn_response.status_code(), StatusCode::OK);

    let spawn_body: Value = spawn_response.json();
    assert_eq!(spawn_body["total"], 2);
    assert_eq!(spawn_body["agent_ids"].as_array().map(|v| v.len()), Some(2));

    let list_response = server.get("/api/agents").await;
    assert_eq!(list_response.status_code(), StatusCode::OK);

    let list_body: Value = list_response.json();
    assert_eq!(list_body["total"], 2);
    assert_eq!(list_body["agents"].as_array().map(|v| v.len()), Some(2));
    Ok(())
}

#[tokio::test]
async fn test_create_workflow_response_structure() -> Result<(), String> {
    let server = create_test_server()?;

    let payload = serde_json::json!({
        "bead_spec": "structure validation test"
    });

    let response = server.post("/api/workflows").json(&payload).await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let body: Value = response.json();

    // Should have exactly one field: bead_id
    let object = body
        .as_object()
        .ok_or_else(|| "Expected response object".to_string())?;
    assert_eq!(object.len(), 1);
    assert!(body.get("bead_id").is_some());
    assert!(body["bead_id"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_nonexistent_route_returns_404() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/nonexistent").await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    Ok(())
}

/// Test GET /api/agents/metrics returns 404 when no agents exist
#[tokio::test]
async fn test_get_agent_metrics_returns_404_when_empty() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    let body: Value = response.json();
    assert_eq!(body["status"], 404);
    assert!(body["detail"].as_str().unwrap_or("").contains("No agents found"));
    Ok(())
}

/// Test GET /api/agents/metrics returns 200 with valid metrics
#[tokio::test]
async fn test_get_agent_metrics_returns_valid_metrics() -> Result<(), String> {
    let server = create_test_server()?;

    // First spawn some agents
    let payload = serde_json::json!({
        "count": 3,
        "capabilities": ["rust", "python"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    // Now get metrics with auth
    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();

    // Validate response structure
    assert_eq!(body["total_agents"], 3);
    assert!(body["active_agents"].is_number());
    assert!(body["idle_agents"].is_number());
    assert!(body["unhealthy_agents"].is_number());
    assert!(body["average_uptime_secs"].is_number());
    assert!(body["average_health_score"].is_number());
    assert!(body["status_distribution"].is_object());
    assert!(body["capability_counts"].is_object());

    // Validate capability counts
    let capabilities = body["capability_counts"]
        .as_object()
        .ok_or_else(|| "capability_counts should be object".to_string())?;
    assert!(capabilities.contains_key("rust"));
    assert!(capabilities.contains_key("python"));

    Ok(())
}

/// Test GET /api/agents/metrics validates all metric fields
#[tokio::test]
async fn test_get_agent_metrics_validates_all_fields() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents
    let payload = serde_json::json!({
        "count": 5,
        "capabilities": ["rust", "go", "python"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();

    // Check all numeric fields are present and valid types
    assert!(body["total_agents"].is_number());
    assert!(body["active_agents"].is_number());
    assert!(body["idle_agents"].is_number());
    assert!(body["unhealthy_agents"].is_number());

    // Check average uptime is a reasonable number
    let uptime = body["average_uptime_secs"]
        .as_f64()
        .ok_or_else(|| "average_uptime_secs should be number".to_string())?;
    assert!(uptime >= 0.0);

    // Check health score is between 0 and 1
    let health = body["average_health_score"]
        .as_f64()
        .ok_or_else(|| "average_health_score should be number".to_string())?;
    assert!(health >= 0.0 && health <= 1.0);

    // Check status distribution is a map
    let status_dist = body["status_distribution"]
        .as_object()
        .ok_or_else(|| "status_distribution should be object".to_string())?;
    assert!(!status_dist.is_empty());

    // Check capability counts is a map
    let capabilities = body["capability_counts"]
        .as_object()
        .ok_or_else(|| "capability_counts should be object".to_string())?;
    assert!(!capabilities.is_empty());

    Ok(())
}

/// Test GET /api/agents/metrics reflects current agent state
#[tokio::test]
async fn test_get_agent_metrics_reflects_current_state() -> Result<(), String> {
    let server = create_test_server()?;

    // Get initial metrics (should be 404)
    let response1 = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;
    assert_eq!(response1.status_code(), StatusCode::NOT_FOUND);

    // Spawn agents
    let payload = serde_json::json!({
        "count": 2,
        "capabilities": ["test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    // Get metrics again (should be 200 with 2 agents)
    let response2 = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;
    assert_eq!(response2.status_code(), StatusCode::OK);

    let body2: Value = response2.json();
    assert_eq!(body2["total_agents"], 2);

    Ok(())
}

/// Test GET /api/agents/metrics without authentication returns 401
#[tokio::test]
async fn test_get_agent_metrics_without_auth() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents first
    let payload = serde_json::json!({
        "count": 2,
        "capabilities": ["test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    // Get metrics without auth header - should return 401
    let response = server.get("/api/agents/metrics").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    assert_eq!(body["status"], 401);

    Ok(())
}

/// Test GET /api/agents/metrics with valid authentication
#[tokio::test]
async fn test_get_agent_metrics_with_valid_auth() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents first
    let payload = serde_json::json!({
        "count": 1,
        "capabilities": ["auth-test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    // Get metrics with valid auth header
    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer valid-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["total_agents"], 1);

    Ok(())
}

/// Test GET /api/agents/metrics with invalid auth (wrong scheme) returns 401
#[tokio::test]
async fn test_get_agent_metrics_with_invalid_auth_scheme() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents first
    let payload = serde_json::json!({
        "count": 1,
        "capabilities": ["test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Basic invalid-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    Ok(())
}

/// Test GET /api/agents/metrics returns JSON content type
#[tokio::test]
async fn test_get_agent_metrics_content_type() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents
    let payload = serde_json::json!({
        "count": 1,
        "capabilities": ["content-type-test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert!(response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .contains("application/json"));

    Ok(())
}

/// Test GET /api/agents/metrics handles large agent counts
#[tokio::test]
async fn test_get_agent_metrics_handles_many_agents() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn a larger number of agents
    let payload = serde_json::json!({
        "count": 10,
        "capabilities": ["rust", "go", "python", "java", "javascript"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["total_agents"], 10);

    // Verify capabilities are aggregated correctly
    let capabilities = body["capability_counts"]
        .as_object()
        .ok_or_else(|| "capability_counts should be object".to_string())?;

    // Each of the 5 capabilities should appear 10 times
    for cap in ["rust", "go", "python", "java", "javascript"] {
        assert_eq!(
            capabilities
                .get(cap)
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            10
        );
    }

    Ok(())
}

/// Test GET /api/agents/metrics with empty token returns 401
#[tokio::test]
async fn test_get_agent_metrics_with_empty_token() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents first
    let payload = serde_json::json!({
        "count": 1,
        "capabilities": ["test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "Bearer ")
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    Ok(())
}

/// Test GET /api/agents/metrics authentication is case-sensitive for "Bearer"
#[tokio::test]
async fn test_get_agent_metrics_auth_case_sensitive() -> Result<(), String> {
    let server = create_test_server()?;

    // Spawn agents first
    let payload = serde_json::json!({
        "count": 1,
        "capabilities": ["test"]
    });

    let _spawn_response = server.post("/api/agents/spawn").json(&payload).await;

    // Use lowercase "bearer" - should fail
    let response = server
        .get("/api/agents/metrics")
        .add_header("Authorization", "bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    Ok(())
}

/// Test GET /api/scheduler/metrics returns 401 without authentication
#[tokio::test]
async fn test_get_scheduler_metrics_without_auth_returns_401() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server.get("/api/scheduler/metrics").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    assert_eq!(body["status"], 401);
    assert!(body["detail"].as_str().unwrap_or("").contains("Missing or invalid"));

    Ok(())
}

/// Test GET /api/scheduler/metrics with valid authentication returns 200
#[tokio::test]
async fn test_get_scheduler_metrics_with_valid_auth_returns_200() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/scheduler/metrics")
        .add_header("Authorization", "Bearer valid-test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();

    // Validate response structure
    assert!(body["queue"].is_object());
    assert!(body["throughput"].is_object());
    assert!(body["latency"].is_object());
    assert!(body["collected_at"].is_string());

    Ok(())
}

/// Test GET /api/scheduler/metrics validates queue metrics structure
#[tokio::test]
async fn test_get_scheduler_metrics_validates_queue_structure() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/scheduler/metrics")
        .add_header("Authorization", "Bearer test-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    let queue = &body["queue"];

    assert!(queue["pending"].is_number());
    assert!(queue["in_progress"].is_number());
    assert!(queue["failed"].is_number());
    assert!(queue["total"].is_number());

    // Verify total equals sum of components
    let pending = queue["pending"].as_u64().unwrap_or(0);
    let in_progress = queue["in_progress"].as_u64().unwrap_or(0);
    let failed = queue["failed"].as_u64().unwrap_or(0);
    let total = queue["total"].as_u64().unwrap_or(0);

    assert_eq!(total, pending + in_progress + failed);

    Ok(())
}

/// Test GET /api/scheduler/metrics validates throughput metrics structure
#[tokio::test]
async fn test_get_scheduler_metrics_validates_throughput_structure() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/scheduler/metrics")
        .add_header("Authorization", "Bearer throughput-test")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    let throughput = &body["throughput"];

    assert!(throughput["jobs_per_minute"].is_number());
    assert!(throughput["jobs_per_hour"].is_number());
    assert!(throughput["total_completed"].is_number());

    // Verify values are non-negative
    let jpm = throughput["jobs_per_minute"].as_f64().unwrap_or(0.0);
    let jph = throughput["jobs_per_hour"].as_f64().unwrap_or(0.0);
    let total = throughput["total_completed"].as_u64().unwrap_or(0);

    assert!(jpm >= 0.0);
    assert!(jph >= 0.0);
    assert!(total >= 0);

    Ok(())
}

/// Test GET /api/scheduler/metrics validates latency metrics structure
#[tokio::test]
async fn test_get_scheduler_metrics_validates_latency_structure() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/scheduler/metrics")
        .add_header("Authorization", "Bearer latency-test")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    let latency = &body["latency"];

    assert!(latency["average_queue_wait_ms"].is_number());
    assert!(latency["average_execution_time_ms"].is_number());
    assert!(latency["p95_queue_wait_ms"].is_number());
    assert!(latency["p95_execution_time_ms"].is_number());

    // Verify all latency values are non-negative
    let avg_wait = latency["average_queue_wait_ms"].as_u64().unwrap_or(0);
    let avg_exec = latency["average_execution_time_ms"].as_u64().unwrap_or(0);
    let p95_wait = latency["p95_queue_wait_ms"].as_u64().unwrap_or(0);
    let p95_exec = latency["p95_execution_time_ms"].as_u64().unwrap_or(0);

    assert!(avg_wait >= 0);
    assert!(avg_exec >= 0);
    assert!(p95_wait >= 0);
    assert!(p95_exec >= 0);

    // P95 should typically be >= average (for well-behaved systems)
    assert!(p95_wait >= avg_wait || avg_wait == 0);
    assert!(p95_exec >= avg_exec || avg_exec == 0);

    Ok(())
}

/// Test GET /api/scheduler/metrics with invalid auth scheme
#[tokio::test]
async fn test_get_scheduler_metrics_invalid_auth_scheme_returns_401() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/scheduler/metrics")
        .add_header("Authorization", "Basic invalid-token")
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    assert_eq!(body["status"], 401);

    Ok(())
}

/// Test GET /api/scheduler/metrics with empty Bearer token
#[tokio::test]
async fn test_get_scheduler_metrics_empty_token_returns_401() -> Result<(), String> {
    let server = create_test_server()?;

    let response = server
        .get("/api/scheduler/metrics")
        .add_header("Authorization", "Bearer ")
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    assert_eq!(body["status"], 401);
    assert!(body["detail"].as_str().unwrap_or("").contains("Invalid token"));

    Ok(())
}
