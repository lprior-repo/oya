//! REST API integration tests

use axum_test::TestServer;
use http::StatusCode;
use oya_web::{routes, actors::{mock_scheduler, mock_state_manager, AppState}};
use std::sync::Arc;
use serde_json::Value;

fn create_test_server() -> TestServer {
    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
    };

    let app = routes::create_router()
        .with_state(state);

    TestServer::new(app).expect("Failed to create test server")
}

#[tokio::test]
async fn test_health_check_returns_ok() {
    let server = create_test_server();

    let response = server
        .get("/api/health")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_create_workflow_returns_201() {
    let server = create_test_server();

    let payload = serde_json::json!({
        "bead_spec": "test workflow spec"
    });

    let response = server
        .post("/api/workflows")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let body: Value = response.json();
    assert!(body["bead_id"].is_string());
    assert!(body["bead_id"].as_str().unwrap().len() == 26);
}

#[tokio::test]
async fn test_get_bead_status_returns_200() {
    let server = create_test_server();

    let response = server
        .get("/api/beads/01ARZ3NDEKTSV4RRFFQ69G5FAV")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["id"], "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["phase"], "initializing");
    assert!(body["events"].is_array());
    assert!(body["created_at"].is_string());
    assert!(body["updated_at"].is_string());
}

#[tokio::test]
async fn test_get_bead_status_invalid_ulid_returns_400() {
    let server = create_test_server();

    let response = server
        .get("/api/beads/invalid-ulid")
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
}

#[tokio::test]
async fn test_cancel_bead_returns_200() {
    let server = create_test_server();

    let response = server
        .post("/api/beads/01ARZ3NDEKTSV4RRFFQ69G5FAV/cancel")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_cancel_bead_invalid_ulid_returns_400() {
    let server = create_test_server();

    let response = server
        .post("/api/beads/invalid/cancel")
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
}

#[tokio::test]
async fn test_create_workflow_without_required_field_fails() {
    let server = create_test_server();

    let payload = serde_json::json!({});

    let response = server
        .post("/api/workflows")
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert_eq!(body["status"], 400);
}

#[tokio::test]
async fn test_nonexistent_route_returns_404() {
    let server = create_test_server();

    let response = server
        .get("/api/nonexistent")
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_cors_headers_present() {
    let server = create_test_server();

    let response = server
        .get("/api/health")
        .add_header("Origin", "http://localhost:3000")
        .await;

    assert!(response.status_code().is_success());

    let cors_header = response.headers().get("access-control-allow-origin");
    assert!(cors_header.is_some());
}
