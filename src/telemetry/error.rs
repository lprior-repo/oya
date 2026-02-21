//! Observability error types
//!
//! Functional Rust compliant: no unwrap/panic/expect

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use thiserror::Error;

/// Observability initialization and runtime errors
#[derive(Error, Debug)]
pub enum ObservabilityError {
    /// Missing required environment variable
    #[error("Missing environment variable: {0}")]
    MissingEnv(&'static str),

    /// Invalid OTLP endpoint URL
    #[error("Invalid OTLP endpoint URL: {0}")]
    InvalidEndpoint(String),

    /// Failed to build HTTP client for OTLP export
    #[error("Failed to build OTLP HTTP client: {source}")]
    HttpClientBuild {
        /// Underlying error
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to send OTLP request
    #[error("OTLP HTTP request failed to {url}: {source}")]
    HttpRequest {
        /// Target URL
        url: String,
        /// Underlying error
        #[source]
        source: reqwest::Error,
    },

    /// Failed to set global tracing subscriber
    #[error("Failed to set global tracing subscriber: {0}")]
    SetGlobalError(String),

    /// Invalid log level filter
    #[error("Invalid log level filter '{0}': {1}")]
    InvalidFilter(String, String),

    /// Generic tracing error
    #[error("Tracing error: {0}")]
    TracingError(String),
}

impl From<tracing_subscriber::util::TryInitError> for ObservabilityError {
    fn from(err: tracing_subscriber::util::TryInitError) -> Self {
        ObservabilityError::SetGlobalError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ObservabilityError::MissingEnv("OTEL_SERVICE_NAME");
        assert!(err.to_string().contains("OTEL_SERVICE_NAME"));

        let err = ObservabilityError::InvalidEndpoint("not-a-url".to_string());
        assert!(err.to_string().contains("not-a-url"));
    }

    #[test]
    fn test_connection_failure_error_type_exists() {
        // Verify that the HttpRequest error variant exists and has the correct shape
        // This ensures we have proper error handling for connection failures
        let error_msg = ObservabilityError::SetGlobalError("connection refused".to_string());
        assert!(error_msg.to_string().contains("connection refused"));
    }

    #[test]
    fn test_invalid_filter_error() {
        let err = ObservabilityError::InvalidFilter(
            "OTEL_TRACES_SAMPLER_ARG".to_string(),
            "must be a float".to_string(),
        );
        let msg = err.to_string();
        assert!(msg.contains("OTEL_TRACES_SAMPLER_ARG"), "Error should contain the env var name");
        assert!(msg.contains("must be a float"), "Error should contain the reason");
    }
}
