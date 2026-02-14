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

#[cfg(test)]
mod tests {
    use crate::agent_swarm::config::{AgentConfig, AgentConfigBuilder};
    use oya_events::StageKind;
    use std::path::PathBuf;
    use std::time::Duration;

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
    fn test_agent_config_builder() -> Result<(), crate::agent_swarm::error::AgentSwarmError> {
        let config = AgentConfigBuilder::new()
            .executable(PathBuf::from("/usr/bin/env"))
            .working_dir(PathBuf::from("/project"))
            .default_timeout(Duration::from_secs(120))
            .build()?;

        assert_eq!(config.executable(), &PathBuf::from("/usr/bin/env"));
        assert_eq!(config.working_dir(), &PathBuf::from("/project"));
        assert_eq!(config.default_timeout(), Duration::from_secs(120));
        Ok(())
    }

    #[test]
    fn test_agent_error_display() {
        let err = crate::agent_swarm::error::AgentSwarmError::Timeout {
            timeout: Duration::from_secs(30),
        };
        assert!(err.to_string().contains("timeout"));
        assert!(err.to_string().contains("30s"));
    }

    #[test]
    fn test_agent_error_executable_not_found() {
        let path = PathBuf::from("/nonexistent/path");
        let err =
            crate::agent_swarm::error::AgentSwarmError::ExecutableNotFound { path: path.clone() };
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn test_agent_error_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let agent_err: crate::agent_swarm::error::AgentSwarmError = io_err.into();
        assert!(matches!(
            agent_err,
            crate::agent_swarm::error::AgentSwarmError::Io(_)
        ));
    }
}
