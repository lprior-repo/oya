//! Integration tests for Restate Admin API (port 9070).
//!
//! Verifies:
//! 1. Admin API connectivity (health check, SQL queries)
//! 2. Service discovery and deployment
//! 3. Invocation creation and tracking in sys_invocation
//! 4. Invocation mutation operations (pause, resume, cancel, kill, purge)
//!
//! Requirements:
//!   - restate-server binary at ~/bin/restate-server
//!   - No other process on ports 9070/909
//!
//! Run:
//!   cargo test --test restate_admin_api_tests -- --ignored

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::{Child, Command};
use std::time::Duration;

use oya_frontend::canonical_ports::{ADMIN_PORT, INGRESS_PORT};
use oya_frontend::restate_client::client::{RestateClient, RestateClientConfig};
use oya_frontend::restate_client::types::{InvocationFilter, InvocationStatus};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};

const RESTATE_BIN: &str = "/home/lewis/bin/restate-server";
const MOCK_SERVICE_PORT: u16 = 8383;

// ─── Restate Server Management ───────────────────────────────────────────

struct RestateGuard {
    child: Child,
    _tmpdir: tempfile::TempDir,
}

impl Drop for RestateGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn make_admin_client() -> RestateClient {
    RestateClient::new(RestateClientConfig {
        host: "localhost".into(),
        port: ADMIN_PORT,
        timeout_secs: 10,
    })
}

async fn start_restate() -> Result<RestateGuard, Box<dyn std::error::Error>> {
    let tmpdir = tempfile::TempDir::new()?;
    let base_dir = tmpdir.path().to_str().ok_or("temp dir path is not valid UTF-8")?.to_string();

    let child = Command::new(RESTATE_BIN)
        .args(["--base-dir", &base_dir, "--no-logo", "--auto-provision=true"])
        .spawn()?;

    let guard = RestateGuard { child, _tmpdir: tmpdir };

    let client = make_admin_client();
    let ready = timeout(Duration::from_secs(30), async {
        loop {
            sleep(Duration::from_secs(1)).await;
            if matches!(client.health_check().await, Ok(true)) {
                break;
            }
        }
    })
    .await;

    if ready.is_err() {
        return Err("Restate server did not become ready within 30s".into());
    }

    Ok(guard)
}

// ─── Mock HTTP Service ───────────────────────────────────────────────────

/// Service discovery response for TestEcho with two handlers:
/// - "echo": instant response for basic flow testing
/// - "slow": 30s delay for mutation (pause/cancel/kill) testing
const DISCOVERY_JSON: &str = r#"{"services":[{"name":"TestEcho","ty":0,"handlers":[{"name":"echo","ty":0,"input":{"name":"default","ty":0,"contentType":"application/json"},"output":{"name":"default","ty":0,"contentType":"application/json"}},{"name":"slow","ty":0,"input":{"name":"default","ty":0,"contentType":"application/json"},"output":{"name":"default","ty":0,"contentType":"application/json"}}]}]}"#;

async fn start_mock_service() -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", MOCK_SERVICE_PORT)).await?;

    let handle = tokio::spawn(async move {
        loop {
            let stream = match listener.accept().await {
                Ok((s, _)) => s,
                Err(_) => continue,
            };
            tokio::spawn(handle_mock_request(stream));
        }
    });

    Ok(handle)
}

async fn handle_mock_request(mut stream: tokio::net::TcpStream) {
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf).await {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };
    let request = String::from_utf8_lossy(&buf[..n]);

    let response = if request.starts_with("GET /discover")
        || request.starts_with("GET /.well-known/restate")
    {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            DISCOVERY_JSON.len(),
            DISCOVERY_JSON
        )
    } else if request.starts_with("POST") {
        if request.contains("/slow") {
            // Keep invocation active for mutation testing
            sleep(Duration::from_secs(30)).await;
        }
        let body = r#"{"result":"echo"}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
    };

    let _ = stream.write_all(response.as_bytes()).await;
}

async fn deploy_to_restate() -> Result<(), Box<dyn std::error::Error>> {
    let http = reqwest::Client::new();
    let url = format!("http://localhost:{ADMIN_PORT}/deployments");
    let resp = http
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "uri": format!("http://localhost:{MOCK_SERVICE_PORT}")
        }))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Deployment failed: {status} - {text}").into());
    }

    Ok(())
}

async fn invoke_handler(
    service: &str,
    handler: &str,
    payload: &serde_json::Value,
) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    let http = reqwest::Client::new();
    let url = format!("http://localhost:{INGRESS_PORT}/{service}/{handler}");
    let resp = http.post(&url).json(payload).timeout(Duration::from_secs(10)).send().await?;
    Ok(resp)
}

/// Wait up to `max_secs` for an invocation matching `pred`, returning its ID.
async fn wait_for_invocation<F>(
    client: &RestateClient,
    max_secs: u64,
    pred: F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&oya_frontend::restate_client::types::Invocation) -> bool,
{
    let result = timeout(Duration::from_secs(max_secs), async {
        loop {
            sleep(Duration::from_millis(500)).await;
            let invocations =
                client.list_invocations(InvocationFilter::All).await.unwrap_or_default();
            if let Some(inv) = invocations.iter().find(|inv| pred(inv)) {
                return inv.id.clone();
            }
        }
    })
    .await;

    match result {
        Ok(id) => Ok(id),
        Err(_) => Err("No matching invocation appeared within timeout".into()),
    }
}

// ─── Phase 1: Admin API Connectivity ─────────────────────────────────────

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909 free"]
async fn admin_api_health_check() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let client = make_admin_client();

    let healthy = client.health_check().await?;
    assert!(healthy, "Restate admin API should report healthy");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909 free"]
async fn admin_api_sql_query() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let client = make_admin_client();

    let result = client.query("SELECT 1").await?;
    assert!(!result.rows.is_empty(), "SELECT 1 should return a row");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909 free"]
async fn admin_api_sys_invocation_empty() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let client = make_admin_client();

    let result = client.query("SELECT * FROM sys_invocation LIMIT 5").await?;
    assert!(result.rows.is_empty(), "sys_invocation should be empty on fresh server");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909 free"]
async fn admin_api_list_services_empty() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let client = make_admin_client();

    let services = client.list_services().await?;
    assert!(services.is_empty(), "No services should be deployed initially");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909 free"]
async fn admin_api_list_deployments_empty() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let client = make_admin_client();

    let deployments = client.list_deployments().await?;
    assert!(deployments.is_empty(), "No deployments should exist initially");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909 free"]
async fn admin_api_list_invocations_empty() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let client = make_admin_client();

    let all = client.list_invocations(InvocationFilter::All).await?;
    assert!(all.is_empty(), "No invocations (All) should exist initially");

    let active = client.list_invocations(InvocationFilter::ActiveOnly).await?;
    assert!(active.is_empty(), "No active invocations should exist initially");

    Ok(())
}

// ─── Phase 2: Service Deployment & Invocation Lifecycle ──────────────────

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909/8383 free"]
async fn deploy_service_registers_in_sys() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let _mock = start_mock_service().await?;
    sleep(Duration::from_millis(500)).await;
    let client = make_admin_client();

    deploy_to_restate().await?;
    sleep(Duration::from_secs(3)).await;

    let services = client.list_services().await?;
    assert!(!services.is_empty(), "Service should be registered after deploy");
    assert!(
        services.iter().any(|s| s.name == "TestEcho"),
        "TestEcho should appear in services: {services:?}"
    );

    let deployments = client.list_deployments().await?;
    assert!(!deployments.is_empty(), "Deployment should exist after deploy");

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909/8383 free"]
async fn invoke_creates_invocation_in_sys_table() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let _mock = start_mock_service().await?;
    sleep(Duration::from_millis(500)).await;
    let client = make_admin_client();

    deploy_to_restate().await?;
    sleep(Duration::from_secs(3)).await;

    let resp = invoke_handler("TestEcho", "echo", &serde_json::json!({"message": "hello"})).await?;
    assert!(resp.status().is_success(), "Echo invocation should succeed: {}", resp.status());

    let inv_id = wait_for_invocation(&client, 10, |inv| inv.target_handler_name == "echo").await?;

    let detail = client.get_invocation(&inv_id).await?;
    assert_eq!(detail.invocation.id, inv_id, "Invocation ID should match");
    assert_eq!(
        detail.invocation.target_service_name, "TestEcho",
        "Service name should be TestEcho"
    );
    assert_eq!(detail.invocation.target_handler_name, "echo", "Handler name should be echo");

    // Journal may have entries depending on Restate version
    let journal = client.get_journal(&inv_id).await?;
    // Journal size is a u32 field — just verify it's present
    let _journal_size = detail.invocation.journal_size;
    let _ = journal.len(); // Just verify we got a result

    Ok(())
}

// ─── Phase 3: Invocation Mutation Operations ─────────────────────────────

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909/8383 free"]
async fn pause_and_resume_invocation() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let _mock = start_mock_service().await?;
    sleep(Duration::from_millis(500)).await;
    let client = make_admin_client();

    deploy_to_restate().await?;
    sleep(Duration::from_secs(3)).await;

    // Invoke the slow handler (30s delay) to keep invocation active
    let _slow_resp =
        invoke_handler("TestEcho", "slow", &serde_json::json!({"delay": true})).await?;

    let inv_id = wait_for_invocation(&client, 10, |inv| {
        inv.target_handler_name == "slow" && inv.status.is_active()
    })
    .await?;

    // Pause
    let result = client.pause_invocation(&inv_id).await?;
    assert!(result.success, "Pause should succeed");
    assert_eq!(result.invocation_id, inv_id, "Invocation ID should match");

    sleep(Duration::from_millis(500)).await;
    let detail = client.get_invocation(&inv_id).await?;
    assert!(
        matches!(detail.invocation.status, InvocationStatus::Paused),
        "Invocation should be paused, got {:?}",
        detail.invocation.status
    );

    // Resume
    let result = client.resume_invocation(&inv_id).await?;
    assert!(result.success, "Resume should succeed");

    // Clean up: cancel the resumed invocation
    sleep(Duration::from_millis(500)).await;
    let _ = client.cancel_invocation(&inv_id).await;

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909/8383 free"]
async fn cancel_invocation() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let _mock = start_mock_service().await?;
    sleep(Duration::from_millis(500)).await;
    let client = make_admin_client();

    deploy_to_restate().await?;
    sleep(Duration::from_secs(3)).await;

    // Invoke slow handler
    let _slow_resp =
        invoke_handler("TestEcho", "slow", &serde_json::json!({"delay": true})).await?;

    let inv_id = wait_for_invocation(&client, 10, |inv| {
        inv.target_handler_name == "slow" && inv.status.is_active()
    })
    .await?;

    let result = client.cancel_invocation(&inv_id).await?;
    assert!(result.success, "Cancel should succeed");
    assert_eq!(result.action, oya_frontend::restate_client::types::InvocationAction::Cancel);

    // Wait for cancellation to take effect
    sleep(Duration::from_secs(2)).await;
    let invocations = client.list_invocations(InvocationFilter::All).await?;
    let cancelled = invocations.iter().find(|inv| inv.id == inv_id);
    if let Some(inv) = cancelled {
        assert!(
            inv.status.is_terminal(),
            "Cancelled invocation should be terminal, got {:?}",
            inv.status
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909/8383 free"]
async fn kill_invocation() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let _mock = start_mock_service().await?;
    sleep(Duration::from_millis(500)).await;
    let client = make_admin_client();

    deploy_to_restate().await?;
    sleep(Duration::from_secs(3)).await;

    // Invoke slow handler
    let _slow_resp =
        invoke_handler("TestEcho", "slow", &serde_json::json!({"delay": true})).await?;

    let inv_id = wait_for_invocation(&client, 10, |inv| {
        inv.target_handler_name == "slow" && inv.status.is_active()
    })
    .await?;

    let result = client.kill_invocation(&inv_id).await?;
    assert!(result.success, "Kill should succeed");
    assert_eq!(result.action, oya_frontend::restate_client::types::InvocationAction::Kill);

    // Wait for kill to take effect
    sleep(Duration::from_secs(2)).await;
    let invocations = client.list_invocations(InvocationFilter::All).await?;
    let killed = invocations.iter().find(|inv| inv.id == inv_id);
    if let Some(inv) = killed {
        assert!(
            inv.status.is_terminal(),
            "Killed invocation should be terminal, got {:?}",
            inv.status
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires restate-server at ~/bin/restate-server and ports 9070/909/8383 free"]
async fn purge_invocation_removes_from_table() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = start_restate().await?;
    let _mock = start_mock_service().await?;
    sleep(Duration::from_millis(500)).await;
    let client = make_admin_client();

    deploy_to_restate().await?;
    sleep(Duration::from_secs(3)).await;

    // Invoke echo handler (completes quickly)
    let _resp =
        invoke_handler("TestEcho", "echo", &serde_json::json!({"message": "purge-test"})).await?;

    let inv_id = wait_for_invocation(&client, 10, |inv| inv.target_handler_name == "echo").await?;

    // Wait for completion
    sleep(Duration::from_secs(2)).await;

    // Purge the invocation
    let result = client.purge_invocation(&inv_id).await?;
    assert!(result.success, "Purge should succeed");
    assert_eq!(result.action, oya_frontend::restate_client::types::InvocationAction::Purge);

    // Verify it's gone from sys_invocation
    sleep(Duration::from_secs(1)).await;
    let remaining = client.list_invocations(InvocationFilter::All).await?;
    assert!(
        remaining.iter().all(|inv| inv.id != inv_id),
        "Purged invocation should no longer appear in sys_invocation"
    );

    Ok(())
}
