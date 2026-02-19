#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;
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

pub fn parse_model_from_yaml(content: &str) -> Result<String, ConfigError> {
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

    let model = yaml
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| "zai-coding-plan/glm-5".to_string());

    if model.trim().is_empty() {
        return Err(ConfigError::InvalidField("model"));
    }

    Ok(model)
}

pub fn config_path(repo_root: &PathBuf) -> PathBuf {
    repo_root.join("oya.yaml")
}

pub fn load_config_from_path(path: &PathBuf) -> Result<OyaConfig, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        ConfigError::ParseError(format!("failed to read {}: {}", path.display(), e))
    })?;

    let model = parse_model_from_yaml(&content)?;
    Ok(OyaConfig { model })
}

pub fn load_config(repo_root: &PathBuf) -> Result<OyaConfig, ConfigError> {
    let path = config_path(repo_root);
    if !path.exists() {
        return Ok(OyaConfig::default());
    }
    load_config_from_path(&path)
}

pub fn load_config_from_current_dir() -> Result<OyaConfig, ConfigError> {
    let current = std::env::current_dir()
        .map_err(|e| ConfigError::ParseError(format!("failed to get current dir: {}", e)))?;
    load_config(&current)
}
