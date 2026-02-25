//! Integration tests with HTTP mocking
//!
//! Uses:
//! - Wiremock for OpenCode API mocking
//! - Restate testcontainers for real Restate integration tests

use oya::{build_opencode_poll_snapshot, parse_opencode_output, parse_opencode_sse_events};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::prelude::*;
use restate_sdk_testcontainers::TestEnvironment;
use serde::{Deserialize, Serialize};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test parsing OpenCode JSON output with Wiremock
#[tokio::test]
async fn test_opencode_json_parsing_with_mock() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    Mock::given(method("POST"))
        .and(path("/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "stdout": "Task completed successfully"
        })))
        .mount(&mock_server)
        .await;

    // In real code, we'd configure opencode to use this URL
    // For now, just test the parsing
    let mock_response = r#"{"stdout": "Task completed successfully"}"#;
    let result = parse_opencode_output(mock_response).unwrap();

    assert_eq!(result.stdout, "Task completed successfully");
}

/// Test OpenCode SSE event parsing with mock server
#[tokio::test]
async fn test_opencode_sse_parsing() {
    let mock_server = MockServer::start().await;

    let sse_body = r#"data: {"event": "start"}

data: {"event": "progress", "percent": 50}

data: {"event": "complete"}

"#;

    Mock::given(method("GET"))
        .and(path("/events"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&mock_server)
        .await;

    // Test parsing
    let events = parse_opencode_sse_events(sse_body, 10).unwrap();
    assert_eq!(events.len(), 3);
}

/// Test poll snapshot building with mock responses
#[tokio::test]
async fn test_poll_snapshot_with_mocks() {
    let session_status = r#"{"session-1": {"type": "busy"}}"#;
    let permission = r#"{"items": [{"id": "perm-1"}]}"#;
    let question = r#"[]"#;

    let snapshot =
        build_opencode_poll_snapshot(session_status, permission, question, 0, 0).unwrap();

    assert_eq!(snapshot.busy_sessions.len(), 1);
    assert_eq!(snapshot.busy_sessions[0], "session-1");
    assert_eq!(snapshot.pending_permissions, 1);
    assert_eq!(snapshot.pending_questions, 0);
}

/// Simple test service for Restate integration testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoResponse {
    pub echo: String,
}

#[restate_sdk::service]
pub trait EchoService {
    async fn echo(request: Json<EchoRequest>) -> Result<Json<EchoResponse>, HandlerError>;
}

pub struct EchoServiceImpl;

impl EchoService for EchoServiceImpl {
    async fn echo(
        &self,
        _ctx: Context<'_>,
        request: Json<EchoRequest>,
    ) -> Result<Json<EchoResponse>, HandlerError> {
        Ok(Json(EchoResponse { echo: request.0.message }))
    }
}

/// Integration test with real Restate container
///
/// Runs a full Restate service and makes actual ingress calls.
/// Requires Docker to be running.
#[tokio::test]
#[ignore = "requires Docker and restate container registration"]
async fn test_with_restate_container() {
    let endpoint = Endpoint::builder().bind(EchoServiceImpl.serve()).build();

    let restate = TestEnvironment::new()
        .with_container_logging()
        .start(endpoint)
        .await
        .expect("Failed to start Restate container");

    let ingress_url = restate.ingress_url();
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/EchoService/echo", ingress_url))
        .header("Content-Type", "application/json")
        .header("idempotency-key", "test-key")
        .json(&EchoRequest { message: "hello from test".to_string() })
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200, "Expected successful response");

    let body: EchoResponse = response.json().await.expect("Failed to parse response");
    assert_eq!(body.echo, "hello from test", "Echo should return the same message");
}

/// Test error handling when OpenCode returns invalid JSON
#[tokio::test]
async fn test_opencode_invalid_json_handling() {
    let invalid_responses = vec![
        "",                    // Empty
        "not json",            // Invalid
        "{}",                  // Missing stdout
        r#"{"stdout": null}"#, // Null stdout
        r#"{"stdout": 123}"#,  // Wrong type
    ];

    for response in invalid_responses {
        let result = parse_opencode_output(response);
        assert!(result.is_err(), "Should fail for invalid response: {:?}", response);
    }
}

/// Test handling of malformed SSE events
#[tokio::test]
async fn test_malformed_sse_handling() {
    let malformed = vec![
        "",               // Empty
        "data:",          // Empty data
        "not data: foo",  // Wrong prefix
        "data: {invalid", // Invalid JSON
    ];

    for input in malformed {
        // Should not panic, may return empty or error
        let _ = parse_opencode_sse_events(input, 10);
    }
}
