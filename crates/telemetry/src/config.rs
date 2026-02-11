//! Telemetry configuration types.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]

use std::env;
use tracing::Level;

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub otlp_endpoint: Option<String>,
    pub log_level: Level,
    pub json_logging: bool,
    pub otel_enabled: bool,
}

impl Default for TelemetryConfig {
    #[inline]
    fn default() -> Self {
        Self {
            service_name: env!("CARGO_PKG_NAME").to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: env::var("OYA_ENV").unwrap_or_else(|_| "development".to_string()),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            log_level: Level::INFO,
            json_logging: true,
            otel_enabled: true,
        }
    }
}

impl TelemetryConfig {
    #[must_use]
    #[inline]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Self::default()
        }
    }

    #[must_use]
    #[inline]
    pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    #[must_use]
    #[inline]
    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = environment.into();
        self
    }

    #[must_use]
    #[inline]
    pub fn with_otlp_endpoint(mut self, endpoint: Option<impl Into<String>>) -> Self {
        self.otlp_endpoint = endpoint.map(Into::into);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_log_level(mut self, level: Level) -> Self {
        self.log_level = level;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_json_logging(mut self, enabled: bool) -> Self {
        self.json_logging = enabled;
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_otel_enabled(mut self, enabled: bool) -> Self {
        self.otel_enabled = enabled;
        self
    }

    #[must_use]
    #[inline]
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(value) = env::var("OYA_SERVICE_NAME") {
            config.service_name = value;
        }

        if let Ok(value) = env::var("OYA_SERVICE_VERSION") {
            config.service_version = value;
        }

        if let Ok(value) = env::var("OYA_ENV") {
            config.environment = value;
        }

        if let Ok(endpoint) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            config.otlp_endpoint = Some(endpoint);
        }

        if let Ok(level_str) = env::var("RUST_LOG") {
            if let Ok(level) = level_str.parse::<Level>() {
                config.log_level = level;
            }
        }

        if let Ok(json_logging) = env::var("OYA_JSON_LOGGING") {
            config.json_logging = json_logging == "true" || json_logging == "1";
        }

        if let Ok(otel_enabled) = env::var("OYA_OTEL_ENABLED") {
            config.otel_enabled = otel_enabled == "true" || otel_enabled == "1";
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, env!("CARGO_PKG_NAME"));
        assert_eq!(config.service_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(config.environment, "development");
        assert!(config.json_logging);
        assert!(config.otel_enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = TelemetryConfig::new("test-service")
            .with_service_version("1.0.0")
            .with_environment("production")
            .with_otlp_endpoint(Some("http://localhost:4317"))
            .with_log_level(Level::DEBUG)
            .with_json_logging(false)
            .with_otel_enabled(false);

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "1.0.0");
        assert_eq!(config.environment, "production");
        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert_eq!(config.log_level, Level::DEBUG);
        assert!(!config.json_logging);
        assert!(!config.otel_enabled);
    }
}
