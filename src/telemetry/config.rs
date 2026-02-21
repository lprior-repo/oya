//! OpenTelemetry observability configuration
//!
//! Uses standard OpenTelemetry environment variables for configuration.
//! All defaults are centralized in the Default impl - no hardcoded fallbacks elsewhere.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use crate::telemetry::error::ObservabilityError;
use std::env;

/// Default OTLP endpoint for local development
const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4318";

/// Default service name
const DEFAULT_SERVICE_NAME: &str = "oya-orchestrator";

/// Default sampling strategy
const DEFAULT_SAMPLER: &str = "parentbased_always_on";

/// Default sampling ratio
const DEFAULT_SAMPLER_ARG: &str = "1.0";

/// Default log filter
const DEFAULT_ENV_FILTER: &str = "oya=info";

/// Observability configuration from environment variables
///
/// Uses standard OpenTelemetry environment variables:
/// - `OTEL_SERVICE_NAME`: Service identifier (default: `oya-orchestrator`)
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP collector endpoint (default: <http://localhost:4318>)
/// - `OTEL_RESOURCE_ATTRIBUTES`: Additional resource attributes (optional)
/// - `OTEL_TRACES_SAMPLER`: Sampling strategy (default: `parentbased_always_on`)
/// - `OTEL_TRACES_SAMPLER_ARG`: Sampling ratio 0.0-1.0 (default: `1.0`)
/// - `RUST_LOG`: Log level filter (default: `oya=info`)
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
    /// Log level filter (`EnvFilter` format)
    pub env_filter: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: DEFAULT_SERVICE_NAME.to_string(),
            otlp_endpoint: DEFAULT_OTLP_ENDPOINT.to_string(),
            resource_attributes: None,
            sampler: DEFAULT_SAMPLER.to_string(),
            sampler_arg: DEFAULT_SAMPLER_ARG.to_string(),
            env_filter: DEFAULT_ENV_FILTER.to_string(),
        }
    }
}

impl ObservabilityConfig {
    /// Load configuration from environment variables with fallback defaults
    ///
    /// Uses standard OpenTelemetry environment variable names.
    /// Defaults are sourced from the Default impl via constants.
    ///
    /// # Errors
    ///
    /// Returns `ObservabilityError::InvalidEndpoint` if the OTLP endpoint
    /// is not a valid URL.
    ///
    /// Returns `ObservabilityError::InvalidFilter` if the sampler arg
    /// is not a valid float.
    pub fn from_env() -> Result<Self, ObservabilityError> {
        let defaults = Self::default();

        let service_name = Self::resolve_service_name(&defaults);

        let otlp_endpoint =
            env::var("OTEL_EXPORTER_OTLP_ENDPOINT").map_or(defaults.otlp_endpoint, |v| v);

        let resource_attributes = env::var("OTEL_RESOURCE_ATTRIBUTES").ok();

        let sampler = env::var("OTEL_TRACES_SAMPLER").map_or(defaults.sampler, |v| v);

        let sampler_arg = env::var("OTEL_TRACES_SAMPLER_ARG").map_or(defaults.sampler_arg, |v| v);

        let env_filter = env::var("RUST_LOG").map_or(defaults.env_filter, |v| v);

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

    /// Resolve service name from env vars or defaults
    ///
    /// Priority order:
    /// 1. `OTEL_SERVICE_NAME` env var
    /// 2. service.name attribute in `OTEL_RESOURCE_ATTRIBUTES`
    /// 3. Default value
    fn resolve_service_name(defaults: &Self) -> String {
        // Try OTEL_SERVICE_NAME first
        if let Ok(name) = env::var("OTEL_SERVICE_NAME") {
            return name;
        }

        // Try extracting from OTEL_RESOURCE_ATTRIBUTES
        env::var("OTEL_RESOURCE_ATTRIBUTES")
            .ok()
            .and_then(|attrs| Self::extract_service_name_from_attrs(&attrs))
            .map_or(defaults.service_name.clone(), |name| name)
    }

    /// Extract service.name from resource attributes string
    ///
    /// Attributes format: "key1=value1,key2=value2"
    fn extract_service_name_from_attrs(attrs: &str) -> Option<String> {
        attrs.split(',').find_map(|attr| {
            let mut parts = attr.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("service.name"), Some(value)) => Some(value.to_string()),
                _ => None,
            }
        })
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
    use std::env;

    /// Helper to safely set and restore env vars
    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var(key).ok();
            env::set_var(key, value);
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = env::var(key).ok();
            env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => env::set_var(self.key, val),
                None => env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.service_name, "oya-orchestrator");
        assert_eq!(config.otlp_endpoint, "http://localhost:4318");
        assert_eq!(config.env_filter, "oya=info");
    }

    #[test]
    fn test_config_validation() {
        let config = ObservabilityConfig::default();
        assert!(config.validate().is_ok());

        let mut bad_config = config.clone();
        bad_config.otlp_endpoint = "not-a-url".to_string();
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_sampler_arg_validation() {
        let mut config = ObservabilityConfig::default();
        config.sampler_arg = "0.5".to_string();
        assert!(config.validate().is_ok());

        config.sampler_arg = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_endpoint_from_env_used() {
        // RED TEST: Verify custom endpoint from env var is used
        let _guard = EnvGuard::set("OTEL_EXPORTER_OTLP_ENDPOINT", "http://custom-collector:4317");

        let config =
            ObservabilityConfig::from_env().expect("from_env should succeed with valid endpoint");

        assert_eq!(
            config.otlp_endpoint, "http://custom-collector:4317",
            "Custom endpoint from env should be used, not hardcoded default"
        );
    }

    #[test]
    fn test_default_endpoint_when_env_not_set() {
        // Ensure env var is not set
        let _guard = EnvGuard::remove("OTEL_EXPORTER_OTLP_ENDPOINT");

        let config =
            ObservabilityConfig::from_env().expect("from_env should succeed with defaults");

        assert_eq!(
            config.otlp_endpoint, "http://localhost:4318",
            "Default endpoint should be used when env var not set"
        );
    }

    #[test]
    fn test_invalid_endpoint_handled_gracefully() {
        // RED TEST: Invalid URL should return error, not panic
        let _guard = EnvGuard::set("OTEL_EXPORTER_OTLP_ENDPOINT", "not-a-valid-url");

        let result = ObservabilityConfig::from_env();

        assert!(result.is_err(), "Invalid endpoint should return error, not panic or succeed");

        let err = result.expect_err("Should be error");
        assert!(
            err.to_string().contains("Invalid OTLP endpoint"),
            "Error should mention invalid endpoint, got: {err}"
        );
    }

    #[test]
    fn test_custom_service_name_from_env() {
        let _guard = EnvGuard::set("OTEL_SERVICE_NAME", "custom-oya-service");

        let config = ObservabilityConfig::from_env().expect("from_env should succeed");

        assert_eq!(
            config.service_name, "custom-oya-service",
            "Custom service name from env should be used"
        );
    }

    #[test]
    fn test_custom_env_filter_from_rust_log() {
        let _guard = EnvGuard::set("RUST_LOG", "oya=debug,other=trace");

        let config = ObservabilityConfig::from_env().expect("from_env should succeed");

        assert_eq!(
            config.env_filter, "oya=debug,other=trace",
            "Custom env filter from RUST_LOG should be used"
        );
    }
}
