#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OyaConfig {
    pub model: String,
}

impl Default for OyaConfig {
    fn default() -> Self {
        Self { model: "zai-coding-plan/glm-5".to_string() }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    FileNotFound(String),
    #[error("config parse error: {0}")]
    ParseError(String),
    #[error("config field invalid: {0}")]
    InvalidField(&'static str),
}

/// Parse a model name from YAML config content.
///
/// # Errors
///
/// Returns [`ConfigError::ParseError`] if the YAML is malformed.
/// Returns [`ConfigError::InvalidField`] if the model field is empty.
pub fn parse_model_from_yaml(content: &str) -> Result<String, ConfigError> {
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

    let model = yaml
        .get("model")
        .and_then(|v| v.as_str())
        .map_or_else(|| "zai-coding-plan/glm-5".to_string(), String::from);

    if model.trim().is_empty() {
        return Err(ConfigError::InvalidField("model"));
    }

    Ok(model)
}

/// Return the canonical path to the OYA config file within a repo root.
#[must_use]
pub fn config_path(repo_root: &Path) -> PathBuf {
    repo_root.join("oya.yaml")
}

/// Load config from an explicit file path.
///
/// # Errors
///
/// Returns [`ConfigError::ParseError`] if the file cannot be read or parsed.
/// Returns [`ConfigError::InvalidField`] if the model field is invalid.
pub fn load_config_from_path(path: &Path) -> Result<OyaConfig, ConfigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::ParseError(format!("failed to read {}: {e}", path.display())))?;

    let model = parse_model_from_yaml(&content)?;
    Ok(OyaConfig { model })
}

/// Load config from a repo root directory, falling back to defaults if no config file exists.
///
/// # Errors
///
/// Returns [`ConfigError::ParseError`] if the config file is present but cannot be read or parsed.
/// Returns [`ConfigError::InvalidField`] if a config field is invalid.
pub fn load_config(repo_root: &Path) -> Result<OyaConfig, ConfigError> {
    let path = config_path(repo_root);
    if !path.exists() {
        return Ok(OyaConfig::default());
    }
    load_config_from_path(&path)
}

/// Load config from the current working directory.
///
/// # Errors
///
/// Returns [`ConfigError::ParseError`] if the current directory cannot be determined or the
/// config file cannot be read or parsed.
/// Returns [`ConfigError::InvalidField`] if a config field is invalid.
pub fn load_config_from_current_dir() -> Result<OyaConfig, ConfigError> {
    let current = std::env::current_dir()
        .map_err(|e| ConfigError::ParseError(format!("failed to get current dir: {e}")))?;
    load_config(&current)
}
