#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! Oya lifecycle status client over Restate ingress.

use crate::canonical_ports::default_ingress_url;
use oya_contracts::LifecycleStatusSnapshot;
use serde_json::Value;
use thiserror::Error;

const LIFECYCLE_STATUS_PATH: &str = "/OyaService/get_lifecycle";

#[derive(Error, Debug)]
pub enum LifecycleStatusError {
    #[error("Lifecycle status unavailable (HTTP {status}): {message}")]
    HttpUnavailable { status: u16, message: String },

    #[error("Lifecycle status unavailable: {0}")]
    ConnectionUnavailable(String),

    #[error("Lifecycle status request timed out")]
    Timeout,

    #[error("Invalid lifecycle status response: {0}")]
    InvalidResponse(String),

    #[error("Lifecycle status request failed: {0}")]
    RequestError(#[from] reqwest::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleStatusClientConfig {
    pub ingress_url: String,
    pub timeout_secs: u64,
}

impl Default for LifecycleStatusClientConfig {
    fn default() -> Self {
        Self { ingress_url: default_ingress_url(), timeout_secs: 10 }
    }
}

#[derive(Debug, Clone)]
pub struct LifecycleStatusClient {
    http_client: reqwest::Client,
    ingress_url: String,
}

impl LifecycleStatusClient {
    #[must_use]
    pub fn new(config: LifecycleStatusClientConfig) -> Self {
        let LifecycleStatusClientConfig { ingress_url, timeout_secs } = config;
        Self {
            http_client: build_http_client(timeout_secs),
            ingress_url: normalize_ingress_url(&ingress_url),
        }
    }

    #[must_use]
    pub fn local() -> Self {
        Self::new(LifecycleStatusClientConfig::default())
    }

    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("{}{}", self.ingress_url, LIFECYCLE_STATUS_PATH)
    }

    /// Fetch the latest Oya lifecycle status snapshot.
    ///
    /// # Errors
    /// Returns a typed error when the service is unavailable, times out, or returns malformed JSON.
    pub async fn get_lifecycle(&self) -> Result<LifecycleStatusSnapshot, LifecycleStatusError> {
        let response = self
            .http_client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(map_request_error)?;

        parse_lifecycle_response(response).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_http_client(timeout_secs: u64) -> reqwest::Client {
    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(timeout_secs));
    match builder.build() {
        Ok(client) => client,
        Err(_) => reqwest::Client::new(),
    }
}

#[cfg(target_arch = "wasm32")]
fn build_http_client(_timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::new()
}

fn normalize_ingress_url(ingress_url: &str) -> String {
    ingress_url.trim_end_matches('/').to_owned()
}

fn map_request_error(error: reqwest::Error) -> LifecycleStatusError {
    if error.is_timeout() {
        LifecycleStatusError::Timeout
    } else if error.is_connect() {
        LifecycleStatusError::ConnectionUnavailable(error.to_string())
    } else {
        LifecycleStatusError::RequestError(error)
    }
}

async fn parse_lifecycle_response(
    response: reqwest::Response,
) -> Result<LifecycleStatusSnapshot, LifecycleStatusError> {
    let status = response.status();
    if status.is_success() {
        return response
            .json::<LifecycleStatusSnapshot>()
            .await
            .map_err(|error| LifecycleStatusError::InvalidResponse(error.to_string()));
    }

    let message = error_message(response, status.as_u16()).await;
    Err(LifecycleStatusError::HttpUnavailable { status: status.as_u16(), message })
}

async fn error_message(response: reqwest::Response, status: u16) -> String {
    match response.text().await {
        Ok(body) => body_message(&body),
        Err(error) => format!("<failed to read response body for HTTP {status}: {error}>"),
    }
}

fn body_message(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(value) => json_message(&value).map_or_else(|| non_empty_body(body), ToOwned::to_owned),
        Err(_) => non_empty_body(body),
    }
}

fn json_message(value: &Value) -> Option<&str> {
    value
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| value.get("error").and_then(Value::as_str))
        .filter(|message| !message.trim().is_empty())
}

fn non_empty_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        "<empty response body>".to_owned()
    } else {
        trimmed.to_owned()
    }
}
