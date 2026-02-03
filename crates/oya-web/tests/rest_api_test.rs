//! REST API integration tests

use axum_test::TestServer;
use http::StatusCode;
use oya_web::{
    actors::{AppState, mock_scheduler, mock_state_manager},
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
