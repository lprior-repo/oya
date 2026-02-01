//! OpenCode client for executing AI prompts.
//!
//! This module provides the `OpencodeClient` for interacting with the opencode CLI
//! or compatible AI execution services.

use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

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

    /// Execute a prompt via the opencode CLI.
    async fn execute_via_cli(&self, prompt: &str) -> Result<ExecutionResult> {
        let opencode_path = &self.config.cli_path;

        // Build command arguments
        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "-f".to_string(),
            "json".to_string(),
            "-q".to_string(), // Quiet mode (no spinner)
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
    #[allow(dead_code)]
    async fn execute_via_api(&self, prompt: &str) -> Result<ExecutionResult> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| Error::config_error("No base URL configured for API mode"))?;

        let url = base_url
            .join("/api/execute")
            .map_err(|e| Error::config_error(format!("Invalid API URL: {e}")))?;

        let response = self
            .http_client
            .post(url)
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::execution_failed(format!(
                "API returned {status}: {body}"
            )));
        }

        let result: ExecutionResult = response.json().await?;
        Ok(result)
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
    let mut args = vec!["-p".to_string(), prompt.to_string(), "-q".to_string()];

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
}
