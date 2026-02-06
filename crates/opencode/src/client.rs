//! OpenCode client for executing AI prompts.
//!
//! This module provides the `OpencodeClient` for interacting with the opencode CLI
//! or compatible AI execution services.

use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use url::Url;

use crate::config::OpencodeConfig;
use crate::error::{Error, Result};
use crate::types::{ExecutionResult, StreamChunk};

/// Client for executing prompts via opencode.
#[derive(Debug, Clone)]
pub struct OpencodeClient {
    /// Configuration for the client.
    config: Arc<OpencodeConfig>,
    /// HTTP client for API-based backends.
    http_client: reqwest::Client,
}

impl OpencodeClient {
    /// Create a new OpencodeClient with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(OpencodeConfig::default())
    }

    /// Create a new OpencodeClient with custom configuration.
    pub fn with_config(config: OpencodeConfig) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| Error::connection_failed(e.to_string()))?;

        Ok(Self {
            config: Arc::new(config),
            http_client,
        })
    }

    /// Create a new OpencodeClient with a base URL for API mode.
    pub fn with_url(base_url: Url) -> Result<Self> {
        let config = OpencodeConfig {
            base_url: Some(base_url),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Execute a prompt and return the result.
    ///
    /// This method runs the opencode CLI in non-interactive mode and
    /// captures the output.
    pub async fn execute(&self, prompt: &str) -> Result<ExecutionResult> {
        info!(prompt_len = prompt.len(), "Executing prompt");
        let start = Instant::now();

        // Use CLI execution by default
        let result = self.execute_via_cli(prompt).await?;

        let duration = start.elapsed();
        debug!(duration_ms = duration.as_millis(), "Execution complete");

        Ok(result.with_duration(duration))
    }

    /// Execute a prompt and stream the results.
    ///
    /// Returns a stream of `StreamChunk` items as they are produced.
    pub async fn stream(
        &self,
        prompt: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        info!(prompt_len = prompt.len(), "Starting streaming execution");

        let (tx, rx) = mpsc::channel::<Result<StreamChunk>>(100);
        let config = self.config.clone();
        let prompt = prompt.to_string();

        // Spawn a task to run the CLI and send chunks
        tokio::spawn(async move {
            if let Err(e) = stream_cli_output(&config, &prompt, tx.clone()).await {
                let _ = tx.send(Err(Error::stream_error(e.to_string()))).await;
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    /// Execute a prompt and stream results as SSE-formatted output.
    ///
    /// Returns a stream of SSE-formatted strings.
    /// Each message is formatted according to SSE protocol:
    /// - Content-Type: text/event-stream
    /// - Messages separated by double newlines
    /// - Fields: event, data, id, retry
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use oya_opencode::OpencodeClient;
    ///
    /// let client = OpencodeClient::new().unwrap();
    /// let mut stream = client.stream_sse("Hello, world!").await?;
    ///
    /// while let Some(result) = stream.next().await {
    ///     let sse_message = result?;
    ///     println!("{}", sse_message);
    /// }
    /// ```
    pub async fn stream_sse(
        &self,
        prompt: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // Call existing stream() method to get Stream<StreamChunk>
        let stream_chunk_stream = self.stream(prompt).await?;

        // Pipe through SSE formatter
        Ok(Box::pin(tokio_stream::StreamExt::map(
            stream_chunk_stream,
            move |result| {
                result.and_then(|chunk| {
                    crate::sse::SseFormatter::new()
                        .format_chunk(chunk)
                        .map_err(Error::stream_error)
                })
            },
        )))
    }

    /// Execute a prompt via the opencode CLI.
    async fn execute_via_cli(&self, prompt: &str) -> Result<ExecutionResult> {
        let opencode_path = &self.config.cli_path;

        // Build command arguments - opencode run subcommand with JSON output
        let mut args = vec![
            "run".to_string(),
            prompt.to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];

        // Add working directory if specified
        if let Some(ref cwd) = self.config.working_directory {
            args.push("-c".to_string());
            args.push(cwd.to_string_lossy().to_string());
        }

        debug!(cli = %opencode_path, args = ?args, "Running opencode CLI");

        let output = Command::new(opencode_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::execution_failed(format!("Failed to spawn opencode: {e}")))?
            .wait_with_output()
            .await
            .map_err(|e| Error::execution_failed(format!("Failed to wait for opencode: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::execution_failed(format!(
                "opencode exited with status {}: {}",
                output.status, stderr
            )));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_cli_json_output(&stdout)
    }

    /// Execute via HTTP API (for remote opencode servers).
    ///
    /// Makes a POST request to the /execute endpoint with the prompt in JSON format.
    /// Handles response parsing and error handling.
    pub async fn execute_via_api(&self, prompt: &str) -> Result<ExecutionResult> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| Error::config_error("No base URL configured for API mode"))?;

        let url = base_url
            .join("/execute")
            .map_err(|e| Error::config_error(format!("Invalid API URL: {e}")))?;

        self.execute_with_retry(url, prompt).await
    }

    /// Execute request with exponential backoff retry logic.
    ///
    /// Retries on retryable errors with exponential backoff:
    /// - Retry 1: wait 1s
    /// - Retry 2: wait 2s
    /// - Retry 3: wait 4s
    ///
    /// Max retries: 3
    async fn execute_with_retry(&self, url: Url, prompt: &str) -> Result<ExecutionResult> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 1_000;

        let mut attempt = 0u32;

        loop {
            attempt += 1;

            match self.execute_request(&url, prompt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt >= MAX_RETRIES || !self.should_retry(&e) {
                        return Err(e);
                    }

                    let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt - 1);
                    info!(
                        attempt,
                        delay_ms, "Retrying after error (attempt {}/{})", attempt, MAX_RETRIES
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    /// Execute a single request attempt.
    async fn execute_request(&self, url: &Url, prompt: &str) -> Result<ExecutionResult> {
        let response = self
            .http_client
            .post(url.as_ref())
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();

            if status.is_server_error() {
                return Err(Error::execution_failed(format!(
                    "API returned server error {status}"
                )));
            }

            let body = response
                .text()
                .await
                .map_err(|e| Error::execution_failed(format!("Failed to read error body: {e}")))?;
            return Err(Error::execution_failed(format!(
                "API returned {status}: {body}"
            )));
        }

        let result: ExecutionResult = response.json().await?;
        Ok(result)
    }

    /// Check if error should be retried.
    fn should_retry(&self, error: &Error) -> bool {
        error.is_retryable()
    }

    /// Check if opencode is available.
    pub async fn is_available(&self) -> bool {
        let result = Command::new(&self.config.cli_path)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        result.is_ok_and(|status| status.success())
    }

    /// Get the version of opencode.
    pub async fn version(&self) -> Result<String> {
        let output = Command::new(&self.config.cli_path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| Error::execution_failed(format!("Failed to get version: {e}")))?;

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    }

    /// Perform a health check on the opencode API.
    ///
    /// Makes a GET request to the /health endpoint.
    /// Returns Ok(true) if the API is healthy (HTTP 200), Ok(false) if unhealthy (non-200 status).
    /// Returns Err if the base URL is not configured or the request fails.
    pub async fn health_check(&self) -> Result<bool> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| Error::config_error("No base URL configured for API mode"))?;

        let url = base_url
            .join("/health")
            .map_err(|e| Error::config_error(format!("Invalid API URL: {e}")))?;

        let response = self
            .http_client
            .get(url.as_ref())
            .send()
            .await
            .map_err(|e| Error::connection_failed(format!("Health check failed: {e}")))?;

        Ok(response.status().is_success())
    }
}

impl Default for OpencodeClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            config: Arc::new(OpencodeConfig::default()),
            http_client: reqwest::Client::new(),
        })
    }
}

/// Stream CLI output as chunks.
async fn stream_cli_output(
    config: &OpencodeConfig,
    prompt: &str,
    tx: mpsc::Sender<Result<StreamChunk>>,
) -> Result<()> {
    let mut args = vec![
        "run".to_string(),
        prompt.to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];

    if let Some(ref cwd) = config.working_directory {
        args.push("-c".to_string());
        args.push(cwd.to_string_lossy().to_string());
    }

    let mut child = Command::new(&config.cli_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::execution_failed(format!("Failed to spawn opencode: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::stream_error("Failed to capture stdout"))?;

    let mut reader = BufReader::new(stdout).lines();
    let mut sequence = 0u64;

    while let Ok(Some(line)) = reader.next_line().await {
        sequence += 1;
        let chunk = StreamChunk::text(line, sequence);
        if tx.send(Ok(chunk)).await.is_err() {
            warn!("Stream receiver dropped");
            break;
        }
    }

    // Wait for process to complete
    let status = child
        .wait()
        .await
        .map_err(|e| Error::execution_failed(format!("Failed to wait for opencode: {e}")))?;

    // Send final chunk
    let final_chunk = if status.success() {
        StreamChunk::final_chunk("", sequence + 1)
    } else {
        StreamChunk::error(
            format!("Process exited with status: {status}"),
            sequence + 1,
        )
    };

    let _ = tx.send(Ok(final_chunk)).await;

    Ok(())
}

/// Parse JSON output from the opencode CLI.
fn parse_cli_json_output(output: &str) -> Result<ExecutionResult> {
    // Try to parse as JSON first
    if let Ok(result) = serde_json::from_str::<ExecutionResult>(output) {
        return Ok(result);
    }

    // If not valid JSON, treat the output as plain text
    // This handles the case where opencode outputs text directly
    if output.trim().is_empty() {
        return Ok(ExecutionResult::success(""));
    }

    // Check if it looks like an error
    let lower = output.to_lowercase();
    if lower.contains("error") || lower.contains("failed") {
        Ok(ExecutionResult::failure(output.trim().to_string()))
    } else {
        Ok(ExecutionResult::success(output.trim().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json_string, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_client_default() {
        let client = OpencodeClient::default();
        assert!(client.config.cli_path.contains("opencode"));
    }

    #[test]
    fn test_parse_json_output_success() {
        let json = r#"{"success":true,"output":"Hello","modified_files":[],"commands_executed":[],"tokens_used":{"prompt_tokens":0,"completion_tokens":0,"total_tokens":0},"duration":0}"#;
        let result = parse_cli_json_output(json);
        assert!(result.is_ok());
        assert!(result.as_ref().map(|r| r.success).unwrap_or(false));
    }

    #[test]
    fn test_parse_plain_text_output() {
        let output = "This is plain text output";
        let result = parse_cli_json_output(output);
        assert!(result.is_ok());
        let result = result.ok();
        assert!(result.as_ref().map(|r| r.success).unwrap_or(false));
        assert_eq!(
            result.map(|r| r.output),
            Some("This is plain text output".to_string())
        );
    }

    #[test]
    fn test_parse_error_output() {
        let output = "Error: something went wrong";
        let result = parse_cli_json_output(output);
        assert!(result.is_ok());
        let result = result.ok();
        assert!(!result.as_ref().map(|r| r.success).unwrap_or(true));
    }

    #[tokio::test]
    async fn test_execute_via_api_success() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;

        let body = serde_json::to_string(&serde_json::json!({"prompt": "test prompt"}))?;

        Mock::given(method("POST"))
            .and(path("/execute"))
            .and(body_json_string(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "output": "Test response",
                "modified_files": [],
                "commands_executed": [],
                "tokens_used": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                },
                "duration": 1000,
                "metadata": {}
            })))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().parse::<Url>()?;
        let client = OpencodeClient::with_url(base_url)?;

        let result = client.execute_via_api("test prompt").await?;

        assert!(result.success);
        assert_eq!(result.output, "Test response");
        assert_eq!(result.tokens_used.total_tokens, 30);
        Ok(())
    }

    #[tokio::test]
    async fn test_execute_via_api_error_response()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/execute"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().parse::<Url>()?;
        let client = OpencodeClient::with_url(base_url)?;

        let result = client.execute_via_api("test prompt").await;

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.to_string().contains("500"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_execute_via_api_timeout() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/execute"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(10)))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().parse::<Url>()?;
        let config = crate::config::OpencodeConfig {
            base_url: Some(base_url),
            timeout: std::time::Duration::from_millis(100),
            ..Default::default()
        };
        let client = OpencodeClient::with_config(config)?;

        let result = client.execute_via_api("test prompt").await;

        assert!(result.is_err());
        if let Err(error) = result {
            let message = error.to_string();
            assert!(
                message.contains("timeout")
                    || message.contains("deadline")
                    || message.contains("send request")
                    || message.contains("HTTP error"),
                "unexpected error message: {message}"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_execute_via_api_no_base_url()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let client = OpencodeClient::new()?;

        let result = client.execute_via_api("test prompt").await;

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.to_string().contains("base URL"));
        }

        Ok(())
    }

    #[test]
    fn test_should_retry_on_connection_failed() {
        let client = OpencodeClient::default();
        let error = Error::connection_failed("test");
        assert!(client.should_retry(&error));
    }

    #[test]
    fn test_should_retry_on_timeout() {
        let client = OpencodeClient::default();
        let error = Error::timeout(5000);
        assert!(client.should_retry(&error));
    }

    #[test]
    fn test_should_retry_on_http_error() {
        let client = OpencodeClient::default();
        let error = Error::timeout(5000);
        assert!(client.should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_on_invalid_response() {
        let client = OpencodeClient::default();
        let error = Error::invalid_response("bad data");
        assert!(!client.should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_on_config_error() {
        let client = OpencodeClient::default();
        let error = Error::config_error("bad config");
        assert!(!client.should_retry(&error));
    }

    #[tokio::test]
    async fn test_health_check_success() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok"
            })))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().parse::<Url>()?;
        let client = OpencodeClient::with_url(base_url)?;

        let result = client.health_check().await?;

        assert!(result);
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check_timeout() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(10)))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().parse::<Url>()?;
        let config = crate::config::OpencodeConfig {
            base_url: Some(base_url),
            timeout: std::time::Duration::from_secs(5),
            ..Default::default()
        };
        let client = OpencodeClient::with_config(config)?;

        let result = client.health_check().await;

        assert!(result.is_err());
        if let Err(error) = result {
            let message = error.to_string();
            assert!(
                message.contains("timeout")
                    || message.contains("deadline")
                    || message.contains("timed out"),
                "unexpected error message: {message}"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_health_check_non_200_response() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().parse::<Url>()?;
        let client = OpencodeClient::with_url(base_url)?;

        let result = client.health_check().await?;

        assert!(!result);
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check_no_base_url() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let client = OpencodeClient::new()?;

        let result = client.health_check().await;

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.to_string().contains("base URL"));
        }

        Ok(())
    }
}
