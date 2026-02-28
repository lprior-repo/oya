#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use reqwest::Client;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::Deserialize;

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
    let response = Client::new().post(url).json(&request).send().await?;
    let response = ensure_success(response).await?;
    response.json().await.map_err(Into::into)
}

pub async fn call_restate_root_json<T: serde::Serialize, R: DeserializeOwned>(
    ingress: &str,
    service: &str,
    handler: &str,
    request: T,
) -> anyhow::Result<R> {
    let url = format!("{ingress}/{service}/{handler}");
    let response = Client::new().post(url).json(&request).send().await?;
    let response = ensure_success(response).await?;
    response.json().await.map_err(Into::into)
}

async fn ensure_success(response: Response) -> anyhow::Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let url = response.url().to_owned();
    let body = response.text().await.unwrap_or_default();
    Err(anyhow::anyhow!(format_http_error(status, &url, &body)))
}

pub fn format_http_error(status: reqwest::StatusCode, url: &reqwest::Url, body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        format!("HTTP status {} for url ({url})", status)
    } else {
        format!("HTTP status {} for url ({url}): {trimmed}", status)
    }
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
        None => Err(anyhow::anyhow!("br ready --json returned no JSON payload")),
    }
}

pub fn parse_json_payload(raw: &str) -> anyhow::Result<serde_json::Value> {
    let object_idx = raw.find('{');
    let array_idx = raw.find('[');
    let start = match (object_idx, array_idx) {
        (Some(o), Some(a)) => o.min(a),
        (Some(o), None) => o,
        (None, Some(a)) => a,
        (None, None) => {
            return Err(anyhow::anyhow!("command returned no JSON payload to parse"));
        }
    };
    serde_json::from_str(&raw[start..]).map_err(Into::into)
}

pub async fn run_capture_command(args: &[&str]) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("br")
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run br: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("br failed: {}", stderr.trim()));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("br output was not UTF-8: {error}"))
}

pub async fn run_simple_command(args: &[&str]) -> anyhow::Result<()> {
    let output = tokio::process::Command::new("br")
        .args(args)
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run br: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("br failed: {}", stderr.trim()))
    }
}
