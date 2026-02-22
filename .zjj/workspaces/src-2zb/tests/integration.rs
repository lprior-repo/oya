//! Integration tests with HTTP mocking
//!
//! Uses:
//! - Wiremock for OpenCode API mocking
//! - Can use testcontainers for Restate (see commented example)

use oya::{build_opencode_poll_snapshot, parse_opencode_output, parse_opencode_sse_events};
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

    let snapshot = build_opencode_poll_snapshot(session_status, permission, question).unwrap();

    assert_eq!(snapshot.busy_sessions.len(), 1);
    assert_eq!(snapshot.busy_sessions[0], "session-1");
    assert_eq!(snapshot.pending_permissions, 1);
    assert_eq!(snapshot.pending_questions, 0);
}

/// Example: Restate testcontainers test (requires Docker)
///
/// To run: `cargo test --test integration -- --ignored`
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_with_restate_container() {
    // Placeholder - full implementation would:
    // 1. Start Restate container using restate-sdk-testcontainers
    // 2. Register OyaOrchestrator service
    // 3. Send start request
    // 4. Poll for completion
    // 5. Verify state
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
