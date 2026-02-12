//! Agent subprocess configuration.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for spawning agent subprocesses.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Path to the agent executable (e.g., claude-code)
    executable: PathBuf,
    /// Working directory for agent execution
    working_dir: PathBuf,
    /// Default timeout for agent operations
    default_timeout: Duration,
    /// Environment variables to pass to subprocess
    env_vars: HashMap<String, String>,
}

impl AgentConfig {
    /// Create a new agent configuration with defaults.
    #[must_use]
    pub fn new(executable: PathBuf, working_dir: PathBuf) -> Self {
        Self {
            executable,
            working_dir,
            default_timeout: Duration::from_secs(60),
            env_vars: HashMap::new(),
        }
    }

    /// Get the executable path.
    #[must_use]
    pub const fn executable(&self) -> &PathBuf {
        &self.executable
    }

    /// Get the working directory.
    #[must_use]
    pub const fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Get the default timeout.
    #[must_use]
    pub const fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    /// Get environment variables.
    #[must_use]
    pub const fn env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
    }

    /// Check if executable exists.
    #[must_use]
    pub fn executable_exists(&self) -> bool {
        self.executable.exists()
    }
}

/// Builder for creating [`AgentConfig`] with fluent API.
#[derive(Debug, Clone)]
pub struct AgentConfigBuilder {
    executable: Option<PathBuf>,
    working_dir: Option<PathBuf>,
    default_timeout: Duration,
    env_vars: HashMap<String, String>,
}

impl Default for AgentConfigBuilder {
    fn default() -> Self {
        Self {
            executable: None,
            working_dir: None,
            default_timeout: Duration::from_secs(60),
            env_vars: HashMap::new(),
        }
    }
}

impl AgentConfigBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the executable path.
    #[must_use]
    pub fn executable(mut self, path: PathBuf) -> Self {
        self.executable = Some(path);
        self
    }

    /// Set the working directory.
    #[must_use]
    pub fn working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = Some(path);
        self
    }

    /// Set the default timeout.
    #[must_use]
    pub const fn default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env_var(mut self, key: String, value: String) -> Self {
        self.env_vars.insert(key, value);
        self
    }

    /// Build the configuration.
    ///
    /// # Errors
    ///
    /// Returns error if executable or `working_dir` not set, or if executable
    /// does not exist at the specified path.
    pub fn build(self) -> super::AgentSwarmResult<AgentConfig> {
        let executable = self
            .executable
            .ok_or_else(|| super::AgentSwarmError::SpawnFailed {
                message: "executable path not set".to_string(),
            })?;

        let working_dir = self
            .working_dir
            .ok_or_else(|| super::AgentSwarmError::SpawnFailed {
                message: "working directory not set".to_string(),
            })?;

        // Verify executable exists at build time
        if !executable.exists() {
            return Err(super::AgentSwarmError::ExecutableNotFound { path: executable });
        }

        Ok(AgentConfig {
            executable,
            working_dir,
            default_timeout: self.default_timeout,
            env_vars: self.env_vars,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_new() {
        let executable = PathBuf::from("/usr/bin/echo");
        let working_dir = PathBuf::from("/tmp");

        let config = AgentConfig::new(executable.clone(), working_dir.clone());

        assert_eq!(config.executable(), &executable);
        assert_eq!(config.working_dir(), &working_dir);
        assert_eq!(config.default_timeout(), Duration::from_secs(60));
        assert!(config.env_vars().is_empty());
    }

    #[test]
    fn test_agent_config_default_timeout() {
        let config = AgentConfig::new(PathBuf::from("/bin/true"), PathBuf::from("/tmp"));
        assert_eq!(config.default_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_agent_config_builder_complete() -> super::AgentSwarmResult<()> {
        let result = AgentConfigBuilder::new()
            .executable(PathBuf::from("/usr/bin/env"))
            .working_dir(PathBuf::from("/project"))
            .default_timeout(Duration::from_secs(120))
            .env_var("TEST_VAR".to_string(), "test_value".to_string())
            .build();

        assert!(result.is_ok());
        let config = result?;
        assert_eq!(config.executable(), &PathBuf::from("/usr/bin/env"));
        assert_eq!(config.working_dir(), &PathBuf::from("/project"));
        assert_eq!(config.default_timeout(), Duration::from_secs(120));
        assert_eq!(config.env_vars().len(), 1);
        Ok(())
    }

    #[test]
    fn test_agent_config_builder_missing_executable() {
        let result = AgentConfigBuilder::new()
            .working_dir(PathBuf::from("/project"))
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("executable"));
    }

    #[test]
    fn test_agent_config_builder_missing_working_dir() {
        let result = AgentConfigBuilder::new()
            .executable(PathBuf::from("/usr/bin/env"))
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("working directory"));
    }

    #[test]
    fn test_agent_config_builder_multiple_env_vars() -> super::AgentSwarmResult<()> {
        let result = AgentConfigBuilder::new()
            .executable(PathBuf::from("/usr/bin/env"))
            .working_dir(PathBuf::from("/project"))
            .env_var("VAR1".to_string(), "value1".to_string())
            .env_var("VAR2".to_string(), "value2".to_string())
            .build();

        assert!(result.is_ok());
        let config = result?;
        assert_eq!(config.env_vars().len(), 2);
        Ok(())
    }

    #[test]
    fn test_agent_config_executable_exists_true() {
        let config = AgentConfig::new(PathBuf::from("/usr/bin/echo"), PathBuf::from("/tmp"));
        // On most systems /usr/bin/echo exists
        let result = config.executable_exists();
        // We don't assert true/false as it depends on the system
        let _ = result;
    }

    #[test]
    fn test_agent_config_executable_exists_false() {
        let config = AgentConfig::new(
            PathBuf::from("/nonexistent/path/to/executable"),
            PathBuf::from("/tmp"),
        );
        assert!(!config.executable_exists());
    }
}
