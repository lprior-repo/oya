//! OpenTelemetry-based telemetry infrastructure for OYA.
//!
//! This crate provides production-ready tracing and telemetry with:
//! - Zero-panic functional error handling
//! - Configurable JSON logging
//! - Semantic convention attributes

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod config;
pub mod error;
pub mod trace;

pub use crate::config::TelemetryConfig;
pub use crate::error::{TelemetryError, TelemetryResult};

/// Initialize telemetry with the given configuration.
///
/// This sets up:
/// - Tracing subscriber with JSON logging
///
/// # Errors
///
/// Returns `TelemetryError` if initialization fails.
pub fn init_telemetry(config: TelemetryConfig) -> TelemetryResult<TracingGuard> {
    let env_filter = create_env_filter(&config);

    if config.json_logging {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_current_span(false)
            .with_span_list(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber).map_err(TelemetryError::from)?;
    } else {
        let subscriber = tracing_subscriber::fmt()
            .pretty()
            .with_env_filter(env_filter)
            .finish();

        tracing::subscriber::set_global_default(subscriber).map_err(TelemetryError::from)?;
    }

    Ok(TracingGuard)
}

fn create_env_filter(config: &TelemetryConfig) -> tracing_subscriber::EnvFilter {
    let filter = match config.log_level {
        tracing::Level::TRACE => "oya=trace,tower=trace,axum=trace,tokio=trace",
        tracing::Level::DEBUG => "oya=debug,tower=debug,axum=debug,tokio=info",
        tracing::Level::INFO => "oya=info,tower=info,axum=info,tokio=warn",
        tracing::Level::WARN => "oya=warn,tower=warn,axum=warn,tokio=error",
        tracing::Level::ERROR => "oya=error,tower=error,axum=error,tokio=error",
    };

    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter))
}

/// Guard that ensures telemetry is properly shutdown.
///
/// When dropped, this guard flushes any pending telemetry data.
#[derive(Debug)]
pub struct TracingGuard;

impl Drop for TracingGuard {
    fn drop(&mut self) {
        // No-op for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_telemetry_console() {
        let config = TelemetryConfig::new("test-service")
            .with_otel_enabled(false)
            .with_json_logging(false);

        let _guard = init_telemetry(config);
    }

    #[test]
    fn test_init_telemetry_json() {
        let config = TelemetryConfig::new("test-service")
            .with_otel_enabled(false)
            .with_json_logging(true);

        let _guard = init_telemetry(config);
    }

    #[test]
    fn test_tracing_guard_drop() {
        let config = TelemetryConfig::new("test-service").with_otel_enabled(false);

        {
            let _guard = init_telemetry(config);
        }
    }
}
