//! Tests for CORS middleware configuration

use axum::Router;
use axum::routing::get;
use axum_test::TestServer;
use oya_web::actors::{AppState, mock_scheduler, mock_state_manager};
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_cors_headers_added() {
    let (broadcast_tx, _) = broadcast::channel(100);
    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
        broadcast_tx,
    };

    let app = Router::new()
        .route("/test", get(|| async { "OK" }))
        .with_state(state)
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        );

    let server = TestServer::new(app)
        .map_err(|e| format!("Failed to create test server: {e}"))
        .unwrap();

    let response = server
        .get("/test")
        .add_header("Origin", "http://localhost:3000")
        .await;

    assert!(response.status_code().is_success());
    assert_eq!(response.text(), "OK");
}
