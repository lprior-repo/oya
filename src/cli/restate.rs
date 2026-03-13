#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use reqwest::Client;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

const RESTATE_HTTP_TIMEOUT: Duration = Duration::from_secs(4);
const RESTATE_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const RESTATE_RETRY_DELAY: Duration = Duration::from_millis(150);
const RESTATE_RETRY_ATTEMPTS: u8 = 2;

#[derive(Debug, Deserialize)]
pub struct ReadyIssue {
    pub id: String,
}

pub async fn call_restate_start(
    ingress: &str,
    id: &str,
    request: crate::restate_oya::StartRequest,
) -> anyhow::Result<crate::restate_oya::StartResponse> {
    call_restate_json(ingress, id, "start", request).await
}

pub async fn call_restate_json<T: serde::Serialize>(
    ingress: &str,
    id: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<crate::restate_oya::StartResponse> {
    call_restate_service_json(ingress, "OyaMemory", id, handler, request).await
}

pub async fn call_restate_service_json<T: serde::Serialize>(
    ingress: &str,
    service: &str,
    id: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<crate::restate_oya::StartResponse> {
    let url = format!("{ingress}/{service}/{id}/{handler}");
    let client = restate_http_client()?;
    let mut attempt: u8 = 0;
    loop {
        let response = post_json_with_retry(&client, &url, &request).await?;
        if should_retry_status(response.status()) && attempt < RESTATE_RETRY_ATTEMPTS {
            attempt = attempt.saturating_add(1);
            sleep(RESTATE_RETRY_DELAY).await;
            continue;
        }
        let response = ensure_success(response).await?;
        return response.json().await.map_err(Into::into);
    }
}

pub async fn call_restate_root_json<T: serde::Serialize, R: DeserializeOwned>(
    ingress: &str,
    service: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<R> {
    let url = format!("{ingress}/{service}/{handler}");
    let client = restate_http_client()?;
    let mut attempt: u8 = 0;
    loop {
        let response = post_json_with_retry(&client, &url, &request).await?;
        if should_retry_status(response.status()) && attempt < RESTATE_RETRY_ATTEMPTS {
            attempt = attempt.saturating_add(1);
            sleep(RESTATE_RETRY_DELAY).await;
            continue;
        }
        let response = ensure_success(response).await?;
        return response.json().await.map_err(Into::into);
    }
}

async fn post_json_with_retry<T: serde::Serialize>(
    client: &Client,
    url: &str,
    request: &T,
) -> anyhow::Result<Response> {
    let mut attempt: u8 = 0;
    loop {
        match client.post(url).json(request).send().await {
            Ok(response) => return Ok(response),
            Err(error)
                if attempt < RESTATE_RETRY_ATTEMPTS && is_transient_transport_error(&error) =>
            {
                attempt = attempt.saturating_add(1);
                sleep(RESTATE_RETRY_DELAY).await;
            }
            Err(error) => {
                if let Some(mapped) = map_transport_error(&error) {
                    return Err(anyhow::anyhow!(mapped));
                }
                return Err(error.into());
            }
        }
    }
}

fn should_retry_status(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
            | reqwest::StatusCode::SERVICE_UNAVAILABLE
            | reqwest::StatusCode::NOT_FOUND
    )
}

fn is_transient_transport_error(error: &reqwest::Error) -> bool {
    error.is_connect()
        || error.is_timeout()
        || error.is_request()
        || is_transient_transport_text(&error.to_string())
}

pub fn is_transient_transport_text(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("connection closed before message completed")
        || lowered.contains("connection reset by peer")
        || lowered.contains("broken pipe")
}

fn restate_http_client() -> anyhow::Result<Client> {
    Client::builder()
        .connect_timeout(RESTATE_CONNECT_TIMEOUT)
        .timeout(RESTATE_HTTP_TIMEOUT)
        .build()
        .map_err(|error| anyhow::anyhow!("failed to build HTTP client: {error}"))
}

async fn ensure_success(response: Response) -> anyhow::Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let url = response.url().to_owned();
    let body = response.text().await.unwrap_or_default();
    let normalized = normalize_http_error_body(&body);
    if let Some(mapped) = map_special_error(normalized.as_str(), status) {
        return Err(anyhow::anyhow!(mapped));
    }
    Err(anyhow::anyhow!(format_http_error(status, &url, &normalized)))
}

pub fn format_http_error(status: reqwest::StatusCode, url: &reqwest::Url, body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        format!("HTTP status {} for url ({url})", status)
    } else {
        format!("HTTP status {} for url ({url}): {trimmed}", status)
    }
}

pub fn normalize_http_error_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let simplified = simplify_terminal_message(trimmed);
    if simplified != trimmed {
        return simplified;
    }
    extract_nested_terminal_message(trimmed)
        .map(|message| simplify_terminal_message(&message))
        .or_else(|| extract_top_level_message(trimmed))
        .unwrap_or_else(|| trimmed.to_owned())
}

fn extract_nested_terminal_message(raw: &str) -> Option<String> {
    let outer: serde_json::Value = serde_json::from_str(raw).ok()?;
    let message = outer.get("message")?.as_str()?;
    let nested: serde_json::Value = serde_json::from_str(message).ok()?;
    nested
        .pointer("/error/Terminal/message")
        .and_then(serde_json::Value::as_str)
        .map(std::borrow::ToOwned::to_owned)
}

fn extract_top_level_message(raw: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(raw).ok()?;
    json.get("message").and_then(serde_json::Value::as_str).map(std::borrow::ToOwned::to_owned)
}

fn simplify_terminal_message(message: &str) -> String {
    for (index, ch) in message.char_indices() {
        if ch != '{' {
            continue;
        }
        let parsed: serde_json::Value = match serde_json::from_str(&message[index..]) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(error_message) =
            parsed.pointer("/error/message").and_then(serde_json::Value::as_str)
        {
            return error_message.to_owned();
        }
    }
    message.to_owned()
}

fn map_transport_error(error: &reqwest::Error) -> Option<String> {
    if error.is_connect() {
        Some("unavailable: restate ingress is not reachable".to_owned())
    } else if error.is_timeout() {
        Some("timeout: restate ingress did not respond".to_owned())
    } else if error.is_request() || is_transient_transport_text(&error.to_string()) {
        Some("unavailable: restate ingress closed the connection".to_owned())
    } else {
        None
    }
}

pub fn map_special_error(message: &str, status: reqwest::StatusCode) -> Option<String> {
    if status == reqwest::StatusCode::INTERNAL_SERVER_ERROR
        && message.starts_with("Issue not found:")
    {
        return Some(format!("not_found: {message}"));
    }
    if status == reqwest::StatusCode::NOT_FOUND
        && message.contains("service '")
        && message.contains("not found")
    {
        return Some("unavailable: restate service is not registered yet".to_owned());
    }
    if status == reqwest::StatusCode::INTERNAL_SERVER_ERROR
        && message.contains("internal routing error")
    {
        return Some("unavailable: restate ingress is rebalancing, retry shortly".to_owned());
    }
    None
}

pub async fn pick_ready_bead() -> anyhow::Result<String> {
    let raw = run_capture_command(&["ready", "--json"]).await?;
    let json = extract_json_array(&raw)?;
    let issues: Vec<ReadyIssue> = serde_json::from_str(json)?;
    match issues.first() {
        Some(issue) => Ok(issue.id.clone()),
        None => Err(anyhow::anyhow!("no ready beads found")),
    }
}

fn extract_json_array(raw: &str) -> anyhow::Result<&str> {
    match raw.find('[') {
        Some(index) => Ok(&raw[index..]),
        None => Err(anyhow::anyhow!("bd ready --json returned no JSON payload")),
    }
}

pub fn parse_json_payload(raw: &str) -> anyhow::Result<serde_json::Value> {
    for (index, ch) in raw.char_indices() {
        if ch == '{' || ch == '[' {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw[index..]) {
                return Ok(value);
            }
        }
    }
    Err(anyhow::anyhow!("command returned no JSON payload to parse"))
}

pub async fn run_capture_command(args: &[&str]) -> anyhow::Result<String> {
    run_capture_command_in(args, None).await
}

pub async fn run_capture_command_in(args: &[&str], cwd: Option<&Path>) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("bd")
        .args(args)
        .current_dir(cwd.unwrap_or_else(|| Path::new(".")))
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run bd: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("br failed: {}", stderr.trim()));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("bd output was not UTF-8: {error}"))
}

pub async fn run_simple_command(args: &[&str]) -> anyhow::Result<()> {
    let output = tokio::process::Command::new("bd")
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run bd: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("bd failed: {}", stderr.trim()))
    }
}
