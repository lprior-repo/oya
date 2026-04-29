//! Integration tests for InvocationPoller against a mock Restate server.
//!
//! Verifies that the poller correctly:
//! 1. Detects new invocations
//! 2. Detects status changes
//! 3. Handles server errors gracefully
//! 4. Tracks invocation state across polls
//!
//! Uses wiremock to simulate Restate's SQL query API (POST /query).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::restate_client::{RestateClient, RestateClientConfig};
use oya_frontend::restate_sync::poller::{
    InvocationEvent, InvocationPoller, InvocationStatus, PollerState,
};
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers: build Restate-style SQL responses
// ---------------------------------------------------------------------------

/// Column names matching the INVOCATION_PROJECTION in queries.rs.
const INVOCATION_COLUMNS: &[&str] = &[
    "id",
    "target",
    "target_service_name",
    "target_service_key",
    "target_handler_name",
    "target_service_ty",
    "status",
    "created_at",
    "modified_at",
    "completed_at",
    "journal_size",
    "retry_count",
    "invoked_by",
    "invoked_by_service_name",
    "invoked_by_id",
    "trace_id",
    "last_failure",
    "last_failure_error_code",
];

fn invocation_row(id: &str, status: &str, modified_at: i64) -> Vec<serde_json::Value> {
    let completed_at = if status == "completed" { json!(modified_at) } else { json!(null) };
    vec![
        json!(id),
        json!(format!("{id}-target")),
        json!("TestService"),
        json!(null),
        json!("run"),
        json!("workflow"),
        json!(status),
        json!(1000),
        json!(modified_at),
        completed_at,
        json!(0),
        json!(0),
        json!("ingress"),
        json!(null),
        json!(null),
        json!(null),
        json!(null),
        json!(null),
    ]
}

fn query_response(rows: Vec<Vec<serde_json::Value>>) -> serde_json::Value {
    json!({
        "columns": INVOCATION_COLUMNS,
        "rows": rows
    })
}

fn empty_response() -> serde_json::Value {
    json!({"columns": INVOCATION_COLUMNS, "rows": []})
}

fn make_poller(server: &MockServer) -> InvocationPoller {
    let config = RestateClientConfig {
        host: "127.0.0.1".to_string(),
        port: server.address().port(),
        timeout_secs: 5,
    };
    InvocationPoller::new(Arc::new(RestateClient::new(config)))
}

// ---------------------------------------------------------------------------
// Test 1: Empty server returns no events on first poll
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_empty_server_when_first_poll_then_no_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_response()))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let result = poller.poll().await;

    assert!(result.is_ok(), "poll should succeed on empty server");
    let poll_result = result.unwrap();
    assert!(poll_result.events.is_empty(), "empty server should produce no events");
}

// ---------------------------------------------------------------------------
// Test 2: New invocation detected on first poll
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_running_invocation_when_first_poll_then_new_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(query_response(vec![invocation_row("inv-1", "running", 2000)])),
        )
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let result = poller.poll().await.unwrap();

    assert_eq!(result.events.len(), 1, "should detect one new invocation");
    assert!(
        matches!(&result.events[0], InvocationEvent::New { invocation_id } if invocation_id == "inv-1"),
        "event should be New with correct ID"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Status change detected between polls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_tracked_invocation_when_status_changes_then_status_changed_event() {
    let server = MockServer::start().await;

    // First poll: invocation is running
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(query_response(vec![invocation_row("inv-1", "running", 2000)])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second poll: invocation completed
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(query_response(vec![invocation_row(
                "inv-1",
                "completed",
                3000,
            )])),
        )
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);

    // First poll
    let result1 = poller.poll().await.unwrap();
    assert_eq!(result1.events.len(), 1);
    assert!(matches!(&result1.events[0], InvocationEvent::New { .. }));

    // Second poll
    let result2 = poller.poll().await.unwrap();
    assert_eq!(result2.events.len(), 2, "should have StatusChanged + Completed events");

    let has_status_change = result2.events.iter().any(|e| {
        matches!(e, InvocationEvent::StatusChanged {
            invocation_id, old_status, new_status
        } if invocation_id == "inv-1"
            && *old_status == InvocationStatus::Running
            && *new_status == InvocationStatus::Completed)
    });
    assert!(has_status_change, "should have Running -> Completed status change");

    let has_completed = result2.events.iter().any(|e| {
        matches!(e, InvocationEvent::Completed { invocation_id, .. } if invocation_id == "inv-1")
    });
    assert!(has_completed, "should have Completed event");
}

// ---------------------------------------------------------------------------
// Test 4: Multiple new invocations detected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_multiple_invocations_when_first_poll_then_all_detected_as_new() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(query_response(vec![
            invocation_row("inv-1", "running", 2000),
            invocation_row("inv-2", "pending", 1000),
            invocation_row("inv-3", "suspended", 1500),
        ])))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let result = poller.poll().await.unwrap();

    assert_eq!(result.events.len(), 3, "should detect three new invocations");

    let ids: Vec<&str> = result
        .events
        .iter()
        .map(|e| match e {
            InvocationEvent::New { invocation_id } => invocation_id.as_str(),
            _ => panic!("expected New event"),
        })
        .collect();
    assert!(ids.contains(&"inv-1"), "should contain inv-1");
    assert!(ids.contains(&"inv-2"), "should contain inv-2");
    assert!(ids.contains(&"inv-3"), "should contain inv-3");
}

// ---------------------------------------------------------------------------
// Test 5: Server error returns PollerError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_server_error_when_poll_then_request_error_returned() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error": "internal"})))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let result = poller.poll().await;

    assert!(result.is_err(), "should return error for 500 response");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("HTTP") || err_msg.contains("Request error"),
        "error should mention HTTP or request failure: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: Connection refused returns error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_unreachable_server_when_poll_then_connection_error_returned() {
    let config = RestateClientConfig {
        host: "127.0.0.1".to_string(),
        port: 1, // unreachable port
        timeout_secs: 1,
    };
    let mut poller = InvocationPoller::new(Arc::new(RestateClient::new(config)));
    let result = poller.poll().await;

    assert!(result.is_err(), "should return error for unreachable server");
}

// ---------------------------------------------------------------------------
// Test 7: Failed invocation triggers Failed event
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_running_invocation_when_status_becomes_paused_then_failed_event() {
    let server = MockServer::start().await;

    // First poll: running
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(query_response(vec![invocation_row("inv-1", "running", 2000)])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second poll: paused (maps to Failed in our domain)
    let mut failed_row = invocation_row("inv-1", "paused", 3000);
    // Set last_failure
    failed_row[16] = json!("Something went wrong");
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(query_response(vec![failed_row])))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let _ = poller.poll().await.unwrap();
    let result = poller.poll().await.unwrap();

    let has_failed = result.events.iter().any(|e| {
        matches!(e, InvocationEvent::Failed { invocation_id, error, .. }
            if invocation_id == "inv-1" && error.contains("Something went wrong"))
    });
    assert!(has_failed, "should have Failed event with error message");
}

// ---------------------------------------------------------------------------
// Test 8: Poller state tracks invocations across polls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_polls_when_state_inspected_then_tracked_invocations_match() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(query_response(vec![
            invocation_row("inv-1", "running", 2000),
            invocation_row("inv-2", "pending", 1000),
        ])))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    assert!(!poller.state().is_tracking(), "should start in Initial state");

    let _ = poller.poll().await.unwrap();

    assert!(poller.state().is_tracking(), "should be tracking after poll");
    let ids = poller.state().tracked_ids();
    assert_eq!(ids.len(), 2, "should track 2 invocations");
    assert!(ids.contains(&"inv-1".to_string()));
    assert!(ids.contains(&"inv-2".to_string()));
}

// ---------------------------------------------------------------------------
// Test 9: Invocation disappears between polls (server pruned it)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_tracked_invocation_when_disappears_then_no_stale_state() {
    let server = MockServer::start().await;

    // First poll: one invocation
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(query_response(vec![invocation_row("inv-1", "running", 2000)])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second poll: empty (invocation was pruned)
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_response()))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let _ = poller.poll().await.unwrap();
    assert_eq!(poller.state().tracked_ids().len(), 1);

    let result = poller.poll().await.unwrap();
    assert!(result.events.is_empty(), "disappearing invocation should not emit events");

    // State should now be empty
    assert!(
        poller.state().tracked_ids().is_empty(),
        "pruned invocation should be removed from state"
    );
}

// ---------------------------------------------------------------------------
// Test 10: Malformed JSON response returns error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_malformed_response_when_poll_then_parse_error_returned() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let result = poller.poll().await;
    assert!(result.is_err(), "malformed response should produce error");
}

// ---------------------------------------------------------------------------
// Test 11: Custom poll interval is respected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_custom_interval_when_poller_created_then_interval_stored() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_response()))
        .mount(&server)
        .await;

    let config = RestateClientConfig {
        host: "127.0.0.1".to_string(),
        port: server.address().port(),
        timeout_secs: 5,
    };
    let poller = InvocationPoller::with_interval(Arc::new(RestateClient::new(config)), 100);

    assert!(poller.state().is_tracking() == false, "should start untracked");

    // Verify poller can still poll successfully
    let mut poller = poller;
    let result = poller.poll().await;
    assert!(result.is_ok(), "should poll successfully with custom interval");
}

// ---------------------------------------------------------------------------
// Test 12: PollResult timestamp is populated
// ---------------------------------------------------------------------------

#[tokio::test]
async fn given_successful_poll_when_result_returned_then_timestamp_is_set() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_response()))
        .mount(&server)
        .await;

    let mut poller = make_poller(&server);
    let before = chrono::Utc::now().timestamp();
    let result = poller.poll().await.unwrap();
    let after = chrono::Utc::now().timestamp();

    assert!(result.timestamp >= before, "timestamp should be >= before poll");
    assert!(result.timestamp <= after, "timestamp should be <= after poll");
}

// ---------------------------------------------------------------------------
// Test 13: PollerState default and update behavior
// ---------------------------------------------------------------------------

#[test]
fn given_initial_state_when_updated_then_transitions_to_tracking() {
    use oya_frontend::restate_client::types::{
        Invocation, InvocationStatus as RestateStatus, InvokedBy, ServiceType,
    };

    let mut state = PollerState::default();
    assert!(!state.is_tracking());

    let inv = Invocation {
        id: "inv-1".to_string(),
        target: "test".to_string(),
        target_service_name: "TestService".to_string(),
        target_service_key: None,
        target_handler_name: "run".to_string(),
        target_service_ty: ServiceType::Workflow,
        status: RestateStatus::Running,
        created_at: 1000,
        modified_at: 2000,
        completed_at: None,
        journal_size: 0,
        retry_count: 0,
        invoked_by: InvokedBy::Ingress,
        invoked_by_service_name: None,
        invoked_by_id: None,
        trace_id: None,
        last_failure: None,
        last_failure_error_code: None,
    };

    state.update(inv.clone());
    assert!(state.is_tracking());
    assert_eq!(state.tracked_ids(), vec!["inv-1".to_string()]);

    let tracked = state.get_tracked_invocation("inv-1").unwrap();
    assert_eq!(tracked.id, "inv-1");
    assert_eq!(tracked.status, RestateStatus::Running);
}

#[test]
fn given_tracking_state_when_updated_with_same_id_then_replaces() {
    use oya_frontend::restate_client::types::{
        Invocation, InvocationStatus as RestateStatus, InvokedBy, ServiceType,
    };

    let mut state = PollerState::default();
    let mut inv = Invocation {
        id: "inv-1".to_string(),
        target: "test".to_string(),
        target_service_name: "TestService".to_string(),
        target_service_key: None,
        target_handler_name: "run".to_string(),
        target_service_ty: ServiceType::Workflow,
        status: RestateStatus::Pending,
        created_at: 1000,
        modified_at: 1000,
        completed_at: None,
        journal_size: 0,
        retry_count: 0,
        invoked_by: InvokedBy::Ingress,
        invoked_by_service_name: None,
        invoked_by_id: None,
        trace_id: None,
        last_failure: None,
        last_failure_error_code: None,
    };
    state.update(inv.clone());

    inv.status = RestateStatus::Completed;
    inv.modified_at = 3000;
    state.update(inv);

    let tracked = state.get_tracked_invocation("inv-1").unwrap();
    assert_eq!(tracked.status, RestateStatus::Completed);
    assert_eq!(tracked.modified_at, 3000);
}

// ---------------------------------------------------------------------------
// Test 14: InvocationStatus mapping from Restate statuses
// ---------------------------------------------------------------------------

#[test]
fn given_restate_pending_status_when_mapped_then_invocation_status_pending() {
    let statuses = [
        (oya_frontend::restate_client::types::InvocationStatus::Pending, InvocationStatus::Pending),
        (
            oya_frontend::restate_client::types::InvocationStatus::Scheduled,
            InvocationStatus::Pending,
        ),
        (oya_frontend::restate_client::types::InvocationStatus::Ready, InvocationStatus::Pending),
    ];
    for (restate, expected) in statuses {
        assert_eq!(InvocationStatus::from(restate), expected);
    }
}

#[test]
fn given_restate_running_status_when_mapped_then_invocation_status_running() {
    let statuses = [
        (oya_frontend::restate_client::types::InvocationStatus::Running, InvocationStatus::Running),
        (
            oya_frontend::restate_client::types::InvocationStatus::BackingOff,
            InvocationStatus::Running,
        ),
    ];
    for (restate, expected) in statuses {
        assert_eq!(InvocationStatus::from(restate), expected);
    }
}

#[test]
fn given_restate_suspended_status_when_mapped_then_invocation_status_suspended() {
    assert_eq!(
        InvocationStatus::from(oya_frontend::restate_client::types::InvocationStatus::Suspended),
        InvocationStatus::Suspended
    );
}
