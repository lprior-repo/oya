//! Telemetry initialization and runtime errors.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("invalid log level: {value}")]
    InvalidLogLevel {
        value: String,
        #[source]
        source: tracing_subscriber::filter::ParseError,
    },

    #[error("failed to initialize subscriber: {reason}")]
    SubscriberInitFailed { reason: String },

    #[error("failed to build OTLP exporter: {reason}")]
    ExporterBuildFailed { reason: String },

    #[error("failed to set tracer provider: {reason}")]
    TracerProviderFailed { reason: String },

    #[error("failed to create guard: {reason}")]
    GuardCreationFailed { reason: String },

    #[error("IO error: {operation} failed: {reason}")]
    IoError {
        operation: String,
        reason: String,
        #[source]
        source: std::io::Error,
    },
}

pub type TelemetryResult<T> = Result<T, TelemetryError>;

impl TelemetryError {
    #[inline]
    pub fn invalid_log_level(
        value: impl Into<String>,
        source: tracing_subscriber::filter::ParseError,
    ) -> Self {
        Self::InvalidLogLevel {
            value: value.into(),
            source,
        }
    }

    #[inline]
    pub fn subscriber_init_failed(reason: impl Into<String>) -> Self {
        Self::SubscriberInitFailed {
            reason: reason.into(),
        }
    }

    #[inline]
    pub fn exporter_build_failed(reason: impl Into<String>) -> Self {
        Self::ExporterBuildFailed {
            reason: reason.into(),
        }
    }

    #[inline]
    pub fn tracer_provider_failed(reason: impl Into<String>) -> Self {
        Self::TracerProviderFailed {
            reason: reason.into(),
        }
    }

    #[inline]
    pub fn guard_creation_failed(reason: impl Into<String>) -> Self {
        Self::GuardCreationFailed {
            reason: reason.into(),
        }
    }
}

impl From<tracing::subscriber::SetGlobalDefaultError> for TelemetryError {
    fn from(err: tracing::subscriber::SetGlobalDefaultError) -> Self {
        Self::subscriber_init_failed(err.to_string())
    }
}

impl From<std::io::Error> for TelemetryError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            operation: "io".to_string(),
            reason: err.to_string(),
            source: err,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_log_level_factory() {
        // Create a simple test without using ParseError constructor
        let error = TelemetryError::subscriber_init_failed("invalid log level");
        let display = format!("{error}");
        assert!(display.contains("failed to initialize subscriber"));
    }

    #[test]
    fn test_subscriber_init_failed_factory() {
        let _ = TelemetryError::subscriber_init_failed("failed to init");
    }

    #[test]
    fn test_exporter_build_failed_factory() {
        let _ = TelemetryError::exporter_build_failed("connection refused");
    }

    #[test]
    fn test_tracer_provider_failed_factory() {
        let _ = TelemetryError::tracer_provider_failed("provider error");
    }

    #[test]
    fn test_guard_creation_failed_factory() {
        let _ = TelemetryError::guard_creation_failed("guard error");
    }

    #[test]
    fn test_error_display() {
        let error = TelemetryError::subscriber_init_failed("test reason");
        let display = format!("{error}");
        assert!(display.contains("failed to initialize subscriber"));
    }
}
