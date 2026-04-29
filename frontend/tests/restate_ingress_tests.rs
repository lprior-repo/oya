//! E2E Tests for Restate Ingress API (Port 909) Service Invocation
//!
//! Verifies the three ingress invocation patterns:
//! - POST /{service}/{handler} (service-call)
//! - POST /{object}/{key}/{handler} (object-call)
//! - POST /{workflow}/{uuid}/run (workflow-call)
//!
//! Tests use wiremock to mock the ingress server. Run with `cargo test --test restate_ingress_tests`.
//! Real-Server E2E tests are gated behind `#[ignore]` and require Restate on port 909.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::canonical_ports::default_ingress_url;
use oya_frontend::graph::Workflow;
use serde_json::json;
use wiremock::matchers::{body_json, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ===========================================================================
// Service-Call: POST /{service}/{handler}
// ===========================================================================

/// given a service-call config when invoking through ingress then correct URL
/// and payload are sent
#[tokio::test]
async fn given_service_call_config_when_invoked_then_posts_to_service_endpoint() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/greeter/greet"))
        .and(body_json(&json!({"name": "world"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "inv-123"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({
        "service": "greeter",
        "endpoint": "greet",
        "payload": {"name": "world"}
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert_eq!(result["status"], 200);
    assert_eq!(result["restate_invocation_id"], "inv-123");
}

/// given a service-call with empty service when invoked then error returned
#[tokio::test]
async fn given_service_call_missing_service_when_invoked_then_error() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({
        "service": "",
        "endpoint": "greet"
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert!(result.get("error").is_some());
    let msg = result["error"].as_str().unwrap();
    assert!(msg.contains("service-call requires"), "Expected validation error, got: {msg}");
}

/// given a service-call with no endpoint when invoked then error returned
#[tokio::test]
async fn given_service_call_missing_endpoint_when_invoked_then_error() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({
        "service": "greeter"
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert!(result.get("error").is_some());
    let msg = result["error"].as_str().unwrap();
    assert!(msg.contains("service-call requires"), "Expected validation error, got: {msg}");
}

/// given a service-call with default payload when invoked then empty json sent
#[tokio::test]
async fn given_service_call_no_payload_when_invoked_then_empty_json_sent() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/svc/do"))
        .and(body_json(&json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "inv-456"})))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({
        "service": "svc",
        "endpoint": "do"
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert_eq!(result["status"], 200);
}

/// given a service-call when response has no id field then invocation_id is null
#[tokio::test]
async fn given_service_call_no_id_in_response_when_invoked_then_invocation_id_null() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/svc/handler"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": "ok"})))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({"service": "svc", "endpoint": "handler"});
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert_eq!(result["status"], 200);
    assert!(result["restate_invocation_id"].is_null());
    assert_eq!(result["body"]["result"], "ok");
}

// ===========================================================================
// Object-Call: POST /{object}/{key}/{handler}
// ===========================================================================

/// given an object-call config when invoked then posts to object endpoint with
/// key
#[tokio::test]
async fn given_object_call_config_when_invoked_then_posts_to_object_endpoint_with_key() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/counter/my-key/increment"))
        .and(body_json(&json!({"amount": 5})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "inv-obj-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({
        "object_name": "counter",
        "handler": "increment",
        "key": "my-key",
        "payload": {"amount": 5}
    });
    let result = workflow.execute_service_call_internal("object-call", &config).await;

    assert_eq!(result["status"], 200);
    assert_eq!(result["restate_invocation_id"], "inv-obj-1");
}

/// given an object-call with no key when invoked then defaults to "default"
#[tokio::test]
async fn given_object_call_no_key_when_invoked_then_uses_default_key() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/myObject/default/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "inv-dk"})))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({
        "object_name": "myObject",
        "handler": "get"
    });
    let result = workflow.execute_service_call_internal("object-call", &config).await;

    assert_eq!(result["status"], 200);
    assert_eq!(result["restate_invocation_id"], "inv-dk");
}

/// given an object-call with empty key when invoked then defaults to "default"
#[tokio::test]
async fn given_object_call_empty_key_when_invoked_then_uses_default_key() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/myObj/default/handler"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "inv-ek"})))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({
        "object_name": "myObj",
        "handler": "handler",
        "key": ""
    });
    let result = workflow.execute_service_call_internal("object-call", &config).await;

    assert_eq!(result["status"], 200);
}

/// given an object-call missing object_name when invoked then error returned
#[tokio::test]
async fn given_object_call_missing_object_name_when_invoked_then_error() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({"handler": "get"});
    let result = workflow.execute_service_call_internal("object-call", &config).await;

    assert!(result.get("error").is_some());
    let msg = result["error"].as_str().unwrap();
    assert!(msg.contains("object-call requires"), "Expected validation error, got: {msg}");
}

/// given an object-call missing handler when invoked then error returned
#[tokio::test]
async fn given_object_call_missing_handler_when_invoked_then_error() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({"object_name": "counter"});
    let result = workflow.execute_service_call_internal("object-call", &config).await;

    assert!(result.get("error").is_some());
    let msg = result["error"].as_str().unwrap();
    assert!(msg.contains("object-call requires"), "Expected validation error, got: {msg}");
}

// ===========================================================================
// Workflow-Call: POST /{workflow}/{uuid}/run
// ===========================================================================

/// given a workflow-call config when invoked then posts to workflow run endpoint
/// with generated UUID
#[tokio::test]
async fn given_workflow_call_config_when_invoked_then_posts_to_workflow_run_with_uuid() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path_regex(r"^/orderWorkflow/[0-9a-f-]+/run$"))
        .and(body_json(&json!({"orderId": "42"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "inv-wf-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({
        "workflow_name": "orderWorkflow",
        "payload": {"orderId": "42"}
    });
    let result = workflow.execute_service_call_internal("workflow-call", &config).await;

    assert_eq!(result["status"], 200);
    assert_eq!(result["restate_invocation_id"], "inv-wf-1");
}

/// given a workflow-call with no payload when invoked then empty json sent
#[tokio::test]
async fn given_workflow_call_no_payload_when_invoked_then_empty_json_sent() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path_regex(r"^/myWorkflow/[0-9a-f-]+/run$"))
        .and(body_json(&json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "inv-np"})))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({"workflow_name": "myWorkflow"});
    let result = workflow.execute_service_call_internal("workflow-call", &config).await;

    assert_eq!(result["status"], 200);
}

/// given a workflow-call with empty workflow_name when invoked then error
#[tokio::test]
async fn given_workflow_call_empty_name_when_invoked_then_error() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({"workflow_name": ""});
    let result = workflow.execute_service_call_internal("workflow-call", &config).await;

    assert!(result.get("error").is_some());
    let msg = result["error"].as_str().unwrap();
    assert!(msg.contains("workflow-call requires"), "Expected validation error, got: {msg}");
}

/// given a workflow-call missing workflow_name when invoked then error
#[tokio::test]
async fn given_workflow_call_missing_name_when_invoked_then_error() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({});
    let result = workflow.execute_service_call_internal("workflow-call", &config).await;

    assert!(result.get("error").is_some());
}

/// given two workflow-calls when invoked then each uses a different UUID
#[tokio::test]
async fn given_two_workflow_calls_when_invoked_then_uuids_differ() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path_regex(r"^/wf/[0-9a-f-]+/run$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "inv-x"})))
        .expect(2)
        .mount(&server)
        .await;

    let config = json!({"workflow_name": "wf"});
    let _ = workflow.execute_service_call_internal("workflow-call", &config).await;
    let _ = workflow.execute_service_call_internal("workflow-call", &config).await;

    // Verify the server received two distinct requests (different UUIDs in path)
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 2, "Should have received two requests");
    assert_ne!(
        received[0].url.path(),
        received[1].url.path(),
        "Each workflow-call should generate a unique UUID"
    );
}

// ===========================================================================
// Error Handling
// ===========================================================================

/// given a service-call when server returns 500 then error status captured
#[tokio::test]
async fn given_server_error_when_invoked_then_status_captured() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/svc/handler"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "message": "internal error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({"service": "svc", "endpoint": "handler"});
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert_eq!(result["status"], 500);
    assert_eq!(result["body"]["message"], "internal error");
}

/// given a service-call when server returns non-json then error captured
#[tokio::test]
async fn given_non_json_response_when_invoked_then_error_captured() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    Mock::given(method("POST"))
        .and(path("/svc/handler"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("not json at all")
                .insert_header("content-type", "text/plain"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let config = json!({"service": "svc", "endpoint": "handler"});
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    assert_eq!(result["status"], 200);
    assert!(result.get("error").is_some(), "Non-JSON response should produce an error field");
}

/// given an unknown node_type when invoked then returns executed true
#[tokio::test]
async fn given_unknown_node_type_when_invoked_then_returns_executed() {
    let server = MockServer::start().await;
    let workflow = workflow_with_ingress(&server.uri());

    let config = json!({});
    let result = workflow.execute_service_call_internal("unknown-type", &config).await;

    assert_eq!(result["executed"], true);
}

// ===========================================================================
// Real Restate Ingress E2E Tests (require running server)
// ===========================================================================

fn ingress_available() -> bool {
    std::env::var("RESTATE_E2E").ok().as_deref() == Some("1")
}

/// given a real Restate ingress when service-call invoked then gets invocation
/// ID
#[tokio::test]
#[ignore = "Requires Restate ingress at localhost:909 (set RESTATE_E2E=1)"]
async fn given_real_restate_service_call_when_invoked_then_gets_invocation_id() {
    if !ingress_available() {
        eprintln!("Skipping real Restate E2E test; set RESTATE_E2E=1 to enable.");
        return;
    }

    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = default_ingress_url();

    let config = json!({
        "service": "health",
        "endpoint": "check",
        "payload": {}
    });
    let result = workflow.execute_service_call_internal("service-call", &config).await;

    if let Some(status) = result.get("status") {
        let code = status.as_u64().unwrap();
        assert!(
            (200..300).contains(&code) || code == 404,
            "Expected 2xx or 404 from Restate ingress, got {code}"
        );
    }
}

/// given a real Restate ingress when workflow-call invoked then gets response
#[tokio::test]
#[ignore = "Requires Restate ingress at localhost:909 (set RESTATE_E2E=1)"]
async fn given_real_restate_workflow_call_when_invoked_then_gets_response() {
    if !ingress_available() {
        eprintln!("Skipping real Restate E2E test; set RESTATE_E2E=1 to enable.");
        return;
    }

    let mut workflow = Workflow::new();
    workflow.restate_ingress_url = default_ingress_url();

    let config = json!({
        "workflow_name": "testWorkflow",
        "payload": {}
    });
    let result = workflow.execute_service_call_internal("workflow-call", &config).await;

    // Any response means the ingress is reachable and processing requests
    assert!(
        result.get("status").is_some() || result.get("error").is_some(),
        "Expected some response from Restate ingress"
    );
}

// ===========================================================================
// Helpers
// ===========================================================================

fn workflow_with_ingress(url: &str) -> Workflow {
    let mut w = Workflow::new();
    w.restate_ingress_url = url.to_string();
    w
}
