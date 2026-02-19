//! OpenTelemetry observability initialization
//!
//! Provides dual-layer output:
//! 1. JSON logs to stdout (for `OpenObserve` log stream ingestion)
//! 2. OTLP traces to `OpenObserve` trace backend
//!
//! Both layers share the same tracing Registry, ensuring consistent
//! `trace_id`/`span_id` correlation across logs and traces.

pub mod config;
pub mod error;

use crate::telemetry::config::ObservabilityConfig;
use crate::telemetry::error::ObservabilityError;
use opentelemetry::trace::TracerProvider as TracerProviderTrait;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

/// Shutdown guard that flushes spans on drop
///
/// Ensures all pending spans are exported before process exit.
/// Must be kept alive for the duration of the program.
pub struct ShutdownGuard(SdkTracerProvider);

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // Flush any remaining spans
        if let Err(err) = self.0.shutdown() {
            eprintln!("Failed to shutdown tracer provider: {}", err);
        }
    }
}

/// Initialize telemetry with custom configuration
///
/// Creates dual-layer output:
/// - JSON logs to stdout with automatic trace_id/span_id injection
/// - OTLP trace export to OpenObserve
///
/// # Returns
///
/// Returns a `ShutdownGuard`. The guard must be kept alive for the
/// program duration to ensure proper span flushing.
///
/// # Errors
///
/// Returns `ObservabilityError` if:
/// - OTLP endpoint is invalid
/// - HTTP client build fails
/// - Global subscriber already set
///
/// # Example
///
/// ```no_run
/// use oya::telemetry::init_telemetry;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = oya::telemetry::config::ObservabilityConfig::from_env()?;
/// let _shutdown_guard = init_telemetry(&config)?;
///
/// // Application code here...
/// # Ok(())
/// # }
/// ```
pub fn init_telemetry(config: &ObservabilityConfig) -> Result<ShutdownGuard, ObservabilityError> {
    use opentelemetry_otlp::WithExportConfig;

    // Build OTLP exporter with tonic (gRPC)
    let endpoint = config.otlp_endpoint.clone();
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .build()
        .map_err(|err| ObservabilityError::HttpClientBuild { source: Box::new(err) })?;

    eprintln!("[Telemetry] OTLP exporter configured for {} (gRPC)", endpoint);

    // Configure batch processor for efficient export
    let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();

    eprintln!("[Telemetry] Tracer provider created with batch exporter");

    // Create tracer for this service
    let service_name = config.service_name.clone();
    let tracer = provider.tracer(service_name);

    // Build OpenTelemetry tracing layer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());

    // Build JSON log layer for stdout
    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE);

    // Build env filter from RUST_LOG or default
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.env_filter));

    // Compose subscriber with both layers
    let subscriber = Registry::default().with(env_filter).with(json_layer).with(otel_layer);

    // Set global default subscriber
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| ObservabilityError::SetGlobalError(e.to_string()))?;

    Ok(ShutdownGuard(provider))
}

/// Initialize telemetry with default configuration from environment
///
/// Convenience wrapper that reads standard OpenTelemetry environment
/// variables and initializes the telemetry stack.
///
/// # Environment Variables
///
/// - `OTEL_SERVICE_NAME`: Service name (default: "oya-orchestrator")
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP endpoint (default: "http://localhost:4318")
/// - `RUST_LOG`: Log level filter (default: "oya=info")
///
/// # Returns
///
/// Returns a `ShutdownGuard`.
///
/// # Example
///
/// ```no_run
/// use oya::telemetry::init_default;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let _shutdown_guard = init_default()?;
///
/// // Application code here...
/// # Ok(())
/// # }
/// ```
pub fn init_default() -> Result<ShutdownGuard, ObservabilityError> {
    let config = ObservabilityConfig::from_env()?;
    init_telemetry(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_guard_drop() {
        // Create a minimal provider for testing
        let provider = SdkTracerProvider::builder().build();
        let _guard = ShutdownGuard(provider);
        // Guard drops here, flushing provider
    }
}
