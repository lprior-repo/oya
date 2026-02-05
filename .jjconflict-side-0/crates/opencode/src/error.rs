//! Error types for the opencode crate.

use thiserror::Error;

/// Result type for opencode operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during opencode operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to connect to opencode service.
    #[error("connection failed: {reason}")]
    ConnectionFailed { reason: String },

    /// Request to opencode timed out.
    #[error("request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Invalid response from opencode.
    #[error("invalid response: {reason}")]
    InvalidResponse { reason: String },

    /// Execution failed.
    #[error("execution failed: {reason}")]
    ExecutionFailed { reason: String },

    /// Streaming error.
    #[error("stream error: {reason}")]
    StreamError { reason: String },

    /// Configuration error.
    #[error("configuration error: {reason}")]
    ConfigError { reason: String },

    /// Phase execution error.
    #[error("phase '{phase}' failed: {reason}")]
    PhaseFailed { phase: String, reason: String },

    /// Prompt generation error.
    #[error("failed to generate prompt: {reason}")]
    PromptGenerationFailed { reason: String },

    /// HTTP error from reqwest.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// URL parse error.
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Core error.
    #[error("core error: {0}")]
    Core(#[from] oya_core::Error),
}

impl Error {
    /// Create a connection failed error.
    pub fn connection_failed(reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            reason: reason.into(),
        }
    }

    /// Create a timeout error.
    pub const fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Create an invalid response error.
    pub fn invalid_response(reason: impl Into<String>) -> Self {
        Self::InvalidResponse {
            reason: reason.into(),
        }
    }

    /// Create an execution failed error.
    pub fn execution_failed(reason: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            reason: reason.into(),
        }
    }

    /// Create a stream error.
    pub fn stream_error(reason: impl Into<String>) -> Self {
        Self::StreamError {
            reason: reason.into(),
        }
    }

    /// Create a config error.
    pub fn config_error(reason: impl Into<String>) -> Self {
        Self::ConfigError {
            reason: reason.into(),
        }
    }

    /// Create a phase failed error.
    pub fn phase_failed(phase: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PhaseFailed {
            phase: phase.into(),
            reason: reason.into(),
        }
    }

    /// Create a prompt generation failed error.
    pub fn prompt_generation_failed(reason: impl Into<String>) -> Self {
        Self::PromptGenerationFailed {
            reason: reason.into(),
        }
    }

    /// Check if this error is retryable.
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionFailed { .. } | Self::Timeout { .. } | Self::Http(_)
        )
    }
}
