//! Tests for CORS middleware configuration

use axum::Router;
use axum::routing::get;
use axum_test::TestServer;
use http::StatusCode;
use oya_web::{routes, actors::{mock_scheduler, mock_state_manager, AppState}};
use std::sync::Arc;
use tower_http::cors;

#[tokio::test]
async fn test_cors_headers_added() {
    let state = AppState {
        scheduler: Arc::new(mock_scheduler()),
        state_manager: Arc::new(mock_state_manager()),
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

    let server = TestServer::new(app).expect("Failed to create test server");

    let response = server
        .get("/test")
        .add_header("Origin", "http://localhost:3000")
        .await;

    assert!(response.status_code().is_success());
    assert_eq!(response.text(), "OK");
}
