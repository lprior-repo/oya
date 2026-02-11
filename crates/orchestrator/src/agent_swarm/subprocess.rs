//! AI Agent subprocess management for spawning and managing Claude Code CLI sessions.
//!
//! This module provides subprocess spawning foundation for executing AI agents
//! as isolated processes within the OYA orchestrator.
//!
//! # Architecture
//!
//! - [`AgentConfig`]: Configuration for spawning agent subprocesses
//! - [`SubprocessHandle`]: Active subprocess handle for lifecycle management
//! - [`AgentOutput`]: Captured output from agent execution
//! - [`AgentError`]: Error types for subprocess operations
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::agent_swarm::{AgentConfig, AgentExecutor};
//! use oya_events::{BeadId, StageKind};
//!
//! let config = AgentConfig::new(
//!     PathBuf::from("/usr/bin/claude-code"),
//!     PathBuf::from("/project"),
//! );
//!
//! let executor = AgentExecutor::new(config);
//! let output = executor.execute_stage(bead_id, StageKind::Implement, "prompt", Duration::from_secs(60)).await?;
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

/// Errors from agent subprocess operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// Failed to spawn subprocess
    #[error("Failed to spawn agent subprocess: {message}")]
    SpawnFailed { message: String },

    /// Agent process timed out
    #[error("Agent timeout after {timeout:?}")]
    Timeout { timeout: std::time::Duration },

    /// Agent crashed with exit code
    #[error("Agent crashed with exit code {exit_code}: {stderr}")]
    Crash { exit_code: i32, stderr: String },

    /// IO error during subprocess operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 decoding error
    #[error("UTF-8 decoding error: {0}")]
    InvalidUtf8(String),

    /// Agent executable not found
    #[error("Agent executable not found: {path}")]
    ExecutableNotFound { path: std::path::PathBuf },

    /// Subprocess stdin not available
    #[error("Subprocess stdin not available")]
    StdinUnavailable,

    /// Subprocess stdout not available
    #[error("Subprocess stdout not available")]
    StdoutUnavailable,

    /// No active subprocess
    #[error("No active subprocess")]
    NoActiveSubprocess,
}

/// Result type for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    // Import types from sibling modules
    use crate::agent_swarm::config::{AgentConfig, AgentConfigBuilder};

    #[test]
    fn test_agent_config_creation() {
        let executable = PathBuf::from("/usr/bin/echo");
        let working_dir = PathBuf::from("/tmp");

        let config = AgentConfig::new(executable.clone(), working_dir.clone());

        assert_eq!(config.executable(), &executable);
        assert_eq!(config.working_dir(), &working_dir);
        assert_eq!(config.default_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_agent_config_builder() {
        let result = AgentConfigBuilder::new()
            .executable(PathBuf::from("/usr/bin/env"))
            .working_dir(PathBuf::from("/project"))
            .default_timeout(Duration::from_secs(120))
            .build();

        assert!(result.is_ok(), "builder should succeed");
        let config = result.unwrap();
        assert_eq!(config.executable(), &PathBuf::from("/usr/bin/env"));
        assert_eq!(config.working_dir(), &PathBuf::from("/project"));
        assert_eq!(config.default_timeout(), Duration::from_secs(120));
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::Timeout {
            timeout: Duration::from_secs(30),
        };
        assert!(err.to_string().contains("timeout"));
        assert!(err.to_string().contains("30s"));
    }

    #[test]
    fn test_agent_error_executable_not_found() {
        let path = PathBuf::from("/nonexistent/path");
        let err = AgentError::ExecutableNotFound { path: path.clone() };
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn test_agent_error_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let agent_err: AgentError = io_err.into();
        assert!(matches!(agent_err, AgentError::Io(_)));
    }
}
