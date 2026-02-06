//! Tests for CORS middleware configuration

use axum_test::TestServer;
use oya_web::actors::{AppState, mock_agent_service, mock_scheduler, mock_state_manager};
use oya_web::routes;
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_cors_headers_added() -> Result<(), String> {
    let (broadcast_tx, _) = broadcast::channel(100);
    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
        agent_service: Arc::new(mock_agent_service()),
        broadcast_tx,
    };

    let app = routes::create_router().with_state(state);

    let server = TestServer::new(app).map_err(|e| format!("Failed to create test server: {e}"))?;

    let response = server
        .get("/api/health")
        .add_header("Origin", "http://localhost:3000")
        .await;

    assert!(response.status_code().is_success());
    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "Missing Access-Control-Allow-Origin header".to_string())?;

    assert_eq!(allow_origin, "*");
    Ok(())
}
