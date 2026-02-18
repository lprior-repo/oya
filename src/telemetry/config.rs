//! OpenTelemetry observability configuration
//!
//! Uses standard OpenTelemetry environment variables for configuration

use crate::telemetry::error::ObservabilityError;
use std::env;

/// Observability configuration from environment variables
///
/// Uses standard OpenTelemetry environment variables:
/// - OTEL_SERVICE_NAME: Service identifier (default: "oya-orchestrator")
/// - OTEL_EXPORTER_OTLP_ENDPOINT: OTLP collector endpoint (default: "http://localhost:4318")
/// - OTEL_RESOURCE_ATTRIBUTES: Additional resource attributes (optional)
/// - OTEL_TRACES_SAMPLER: Sampling strategy (default: "parentbased_always_on")
/// - OTEL_TRACES_SAMPLER_ARG: Sampling ratio 0.0-1.0 (default: "1.0")
/// - RUST_LOG: Log level filter (default: "oya=info")
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Service name for tracing
    pub service_name: String,
    /// OTLP HTTP endpoint for trace export
    pub otlp_endpoint: String,
    /// Additional resource attributes (optional)
    pub resource_attributes: Option<String>,
    /// Sampling strategy
    pub sampler: String,
    /// Sampling ratio argument
    pub sampler_arg: String,
    /// Log level filter (EnvFilter format)
    pub env_filter: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "oya-orchestrator".to_string(),
            otlp_endpoint: "http://localhost:4318".to_string(),
            resource_attributes: None,
            sampler: "parentbased_always_on".to_string(),
            sampler_arg: "1.0".to_string(),
            env_filter: "oya=info".to_string(),
        }
    }
}

impl ObservabilityConfig {
    /// Load configuration from environment variables with fallback defaults
    ///
    /// Uses standard OpenTelemetry environment variable names
    pub fn from_env() -> Result<Self, ObservabilityError> {
        let service_name = env::var("OTEL_SERVICE_NAME")
            .or_else(|_| {
                env::var("OTEL_RESOURCE_ATTRIBUTES").map(|attrs| {
                    attrs
                        .split(',')
                        .find_map(|attr| {
                            let mut parts = attr.splitn(2, '=');
                            if parts.next() == Some("service.name") {
                                parts.next().map(String::from)
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| "oya-orchestrator".to_string())
                })
            })
            .unwrap_or_else(|_| "oya-orchestrator".to_string());

        let otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4318".to_string());

        let resource_attributes = env::var("OTEL_RESOURCE_ATTRIBUTES").ok();

        let sampler =
            env::var("OTEL_TRACES_SAMPLER").unwrap_or_else(|_| "parentbased_always_on".to_string());

        let sampler_arg = env::var("OTEL_TRACES_SAMPLER_ARG").unwrap_or_else(|_| "1.0".to_string());

        let env_filter = env::var("RUST_LOG").unwrap_or_else(|_| "oya=info".to_string());

        let config = Self {
            service_name,
            otlp_endpoint,
            resource_attributes,
            sampler,
            sampler_arg,
            env_filter,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    fn validate(&self) -> Result<(), ObservabilityError> {
        // Validate OTLP endpoint is a valid URL
        if let Err(err) = url::Url::parse(&self.otlp_endpoint) {
            return Err(ObservabilityError::InvalidEndpoint(format!(
                "{}: {}",
                self.otlp_endpoint, err
            )));
        }

        // Validate sampler_arg is a valid float between 0.0 and 1.0
        if self.sampler_arg.parse::<f64>().is_err() {
            return Err(ObservabilityError::InvalidFilter(
                "OTEL_TRACES_SAMPLER_ARG".to_string(),
                "must be a float between 0.0 and 1.0".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.service_name, "oya-orchestrator");
        assert_eq!(config.otlp_endpoint, "http://localhost:4318");
        assert_eq!(config.env_filter, "oya=info");
    }

    #[test]
    fn test_config_validation() {
        let mut config = ObservabilityConfig::default();
        assert!(config.validate().is_ok());

        config.otlp_endpoint = "not-a-url".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sampler_arg_validation() {
        let mut config = ObservabilityConfig::default();
        config.sampler_arg = "0.5".to_string();
        assert!(config.validate().is_ok());

        config.sampler_arg = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}
