//! Configuration for the opencode client.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

/// Configuration for the OpencodeClient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpencodeConfig {
    /// Path to the opencode CLI executable.
    #[serde(default = "default_cli_path")]
    pub cli_path: String,

    /// Base URL for API mode (optional).
    #[serde(default)]
    pub base_url: Option<Url>,

    /// Working directory for execution.
    #[serde(default)]
    pub working_directory: Option<PathBuf>,

    /// Timeout for requests.
    #[serde(with = "duration_secs", default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum retries for failed requests.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Model to use (e.g., "claude-sonnet-4-20250514").
    #[serde(default)]
    pub model: Option<String>,

    /// Agent mode ("build" or "plan").
    #[serde(default = "default_agent_mode")]
    pub agent_mode: AgentMode,

    /// Enable debug mode.
    #[serde(default)]
    pub debug: bool,
}

impl Default for OpencodeConfig {
    fn default() -> Self {
        Self {
            cli_path: default_cli_path(),
            base_url: None,
            working_directory: None,
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            model: None,
            agent_mode: default_agent_mode(),
            debug: false,
        }
    }
}

impl OpencodeConfig {
    /// Create a new config with the given CLI path.
    pub fn with_cli_path(cli_path: impl Into<String>) -> Self {
        Self {
            cli_path: cli_path.into(),
            ..Default::default()
        }
    }

    /// Create a new config for API mode.
    pub fn with_api(base_url: Url) -> Self {
        Self {
            base_url: Some(base_url),
            ..Default::default()
        }
    }

    /// Set the working directory.
    #[must_use]
    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    /// Set the timeout.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the model.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the agent mode.
    #[must_use]
    pub const fn agent_mode(mut self, mode: AgentMode) -> Self {
        self.agent_mode = mode;
        self
    }

    /// Enable debug mode.
    #[must_use]
    pub const fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(path) = std::env::var("OPENCODE_CLI_PATH") {
            config.cli_path = path;
        }

        if let Ok(url) = std::env::var("OPENCODE_API_URL") {
            if let Ok(parsed) = url.parse() {
                config.base_url = Some(parsed);
            }
        }

        if let Ok(model) = std::env::var("OPENCODE_MODEL") {
            config.model = Some(model);
        }

        if let Ok(mode) = std::env::var("OPENCODE_AGENT_MODE") {
            config.agent_mode = match mode.to_lowercase().as_str() {
                "plan" => AgentMode::Plan,
                _ => AgentMode::Build,
            };
        }

        if std::env::var("OPENCODE_DEBUG").is_ok() {
            config.debug = true;
        }

        config
    }

    /// Load configuration from a file.
    pub fn from_file(path: &std::path::Path) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        // Try JSON first, then TOML
        if path.extension().is_some_and(|e| e == "json") {
            Ok(serde_json::from_str(&content)?)
        } else {
            toml::from_str(&content).map_err(|e| {
                crate::error::Error::config_error(format!("Failed to parse config: {e}"))
            })
        }
    }
}

/// Agent mode for opencode execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Full access agent for development work.
    #[default]
    Build,
    /// Read-only agent for analysis and exploration.
    Plan,
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build => write!(f, "build"),
            Self::Plan => write!(f, "plan"),
        }
    }
}

fn default_cli_path() -> String {
    // Try to find opencode in common locations
    if let Ok(path) = which::which("opencode") {
        return path.to_string_lossy().to_string();
    }
    "opencode".to_string()
}

const fn default_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

const fn default_max_retries() -> u32 {
    3
}

const fn default_agent_mode() -> AgentMode {
    AgentMode::Build
}

/// Serialization helper for Duration as seconds.
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OpencodeConfig::default();
        assert_eq!(config.agent_mode, AgentMode::Build);
        assert_eq!(config.max_retries, 3);
        assert!(!config.debug);
    }

    #[test]
    fn test_config_builder() {
        let config = OpencodeConfig::with_cli_path("/usr/bin/opencode")
            .working_dir("/tmp")
            .timeout(Duration::from_secs(60))
            .model("claude-sonnet-4-20250514")
            .agent_mode(AgentMode::Plan)
            .debug();

        assert_eq!(config.cli_path, "/usr/bin/opencode");
        assert_eq!(config.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.model, Some("claude-sonnet-4-20250514".to_string()));
        assert_eq!(config.agent_mode, AgentMode::Plan);
        assert!(config.debug);
    }

    #[test]
    fn test_agent_mode_display() {
        assert_eq!(AgentMode::Build.to_string(), "build");
        assert_eq!(AgentMode::Plan.to_string(), "plan");
    }
}
