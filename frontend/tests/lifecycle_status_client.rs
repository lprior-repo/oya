#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oya_contracts::{
    CompensationDiagnostic, LifecycleGateSnapshot, LifecycleStatusSnapshot, LifecycleStepSnapshot,
};
use oya_frontend::restate_client::{LifecycleStatusClient, LifecycleStatusClientConfig};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn lifecycle_status_client_parses_status_snapshot() {
    let server = MockServer::start().await;
    let snapshot = lifecycle_snapshot();

    Mock::given(method("POST"))
        .and(path("/OyaService/get_lifecycle"))
        .and(body_json(&json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(&snapshot))
        .expect(1)
        .mount(&server)
        .await;

    let client = lifecycle_client(&server.uri());
    let status = client.get_lifecycle().await.expect("status snapshot");

    assert_eq!(status, snapshot);
}

#[tokio::test]
async fn lifecycle_status_client_parses_unavailable_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/OyaService/get_lifecycle"))
        .respond_with(
            ResponseTemplate::new(503).set_body_json(json!({"message": "OyaService unavailable"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = lifecycle_client(&server.uri());
    let error = client.get_lifecycle().await.expect_err("unavailable error");

    assert!(error.to_string().contains("HTTP 503"));
    assert!(error.to_string().contains("OyaService unavailable"));
}

fn lifecycle_client(base_url: &str) -> LifecycleStatusClient {
    LifecycleStatusClient::new(LifecycleStatusClientConfig {
        ingress_url: base_url.to_owned(),
        timeout_secs: 10,
    })
}

fn lifecycle_snapshot() -> LifecycleStatusSnapshot {
    LifecycleStatusSnapshot {
        bead_id: Some("demo".to_owned()),
        steps: vec![LifecycleStepSnapshot {
            step: "verify".to_owned(),
            status: "succeeded".to_owned(),
            message: Some("green".to_owned()),
            details: None,
            started_at: Some("2026-04-29T00:00:00Z".to_owned()),
            finished_at: Some("2026-04-29T00:00:01Z".to_owned()),
            duration_ms: Some(1_000),
        }],
        gates: vec![LifecycleGateSnapshot {
            gate_id: "fmt".to_owned(),
            status: "passed".to_owned(),
            message: None,
        }],
        discipline_gates: Vec::new(),
        state: None,
        pr_url: None,
        done: true,
        success: Some(true),
        message: Some("completed".to_owned()),
        compensation_diagnostics: vec![CompensationDiagnostic {
            compensation_type: "none".to_owned(),
            target: "demo".to_owned(),
            success: true,
            error: None,
        }],
    }
}
