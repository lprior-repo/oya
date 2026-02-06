//! Agent swarm error types.

use std::fmt;

/// Errors that can occur in agent swarm operations.
#[derive(Debug)]
pub enum AgentSwarmError {
    /// Agent not found in the pool.
    AgentNotFound {
        /// The agent ID that was not found
        agent_id: String,
    },

    /// Agent is already registered.
    AgentAlreadyRegistered {
        /// The agent ID that already exists
        agent_id: String,
    },

    /// Agent is not available for work.
    AgentUnavailable {
        /// The agent ID that is unavailable
        agent_id: String,
        /// Reason for unavailability
        reason: String,
    },

    /// No agents available in the pool.
    NoAgentsAvailable,

    /// Agent health check failed.
    HealthCheckFailed {
        /// The agent ID that failed health check
        agent_id: String,
        /// Error details
        reason: String,
    },

    /// Agent heartbeat timeout.
    HeartbeatTimeout {
        /// The agent ID that timed out
        agent_id: String,
        /// Duration since last heartbeat in milliseconds
        last_heartbeat_ms: u64,
    },

    /// Pool capacity exceeded.
    PoolCapacityExceeded {
        /// Current pool size
        current: usize,
        /// Maximum pool capacity
        max: usize,
    },

    /// Bead assignment failed.
    AssignmentFailed {
        /// The bead ID that couldn't be assigned
        bead_id: String,
        /// Reason for failure
        reason: String,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for AgentSwarmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentNotFound { agent_id } => {
                write!(f, "agent not found: {}", agent_id)
            }
            Self::AgentAlreadyRegistered { agent_id } => {
                write!(f, "agent already registered: {}", agent_id)
            }
            Self::AgentUnavailable { agent_id, reason } => {
                write!(f, "agent {} unavailable: {}", agent_id, reason)
            }
            Self::NoAgentsAvailable => {
                write!(f, "no agents available in pool")
            }
            Self::HealthCheckFailed { agent_id, reason } => {
                write!(f, "health check failed for agent {}: {}", agent_id, reason)
            }
            Self::HeartbeatTimeout {
                agent_id,
                last_heartbeat_ms,
            } => {
                write!(
                    f,
                    "heartbeat timeout for agent {}: last heartbeat {}ms ago",
                    agent_id, last_heartbeat_ms
                )
            }
            Self::PoolCapacityExceeded { current, max } => {
                write!(f, "pool capacity exceeded: {}/{}", current, max)
            }
            Self::AssignmentFailed { bead_id, reason } => {
                write!(f, "failed to assign bead {}: {}", bead_id, reason)
            }
            Self::Internal { message } => {
                write!(f, "internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for AgentSwarmError {}

impl AgentSwarmError {
    /// Create an agent not found error.
    #[must_use]
    pub fn agent_not_found(agent_id: impl Into<String>) -> Self {
        Self::AgentNotFound {
            agent_id: agent_id.into(),
        }
    }

    /// Create an agent already registered error.
    #[must_use]
    pub fn already_registered(agent_id: impl Into<String>) -> Self {
        Self::AgentAlreadyRegistered {
            agent_id: agent_id.into(),
        }
    }

    /// Create an agent unavailable error.
    #[must_use]
    pub fn unavailable(agent_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AgentUnavailable {
            agent_id: agent_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a health check failed error.
    #[must_use]
    pub fn health_check_failed(agent_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::HealthCheckFailed {
            agent_id: agent_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a heartbeat timeout error.
    #[must_use]
    pub fn heartbeat_timeout(agent_id: impl Into<String>, last_heartbeat_ms: u64) -> Self {
        Self::HeartbeatTimeout {
            agent_id: agent_id.into(),
            last_heartbeat_ms,
        }
    }

    /// Create an assignment failed error.
    #[must_use]
    pub fn assignment_failed(bead_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AssignmentFailed {
            bead_id: bead_id.into(),
            reason: reason.into(),
        }
    }

    /// Create an internal error.
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Check if error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NoAgentsAvailable | Self::AgentUnavailable { .. } | Self::HeartbeatTimeout { .. }
        )
    }
}

/// Result type for agent swarm operations.
pub type AgentSwarmResult<T> = Result<T, AgentSwarmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AgentSwarmError::agent_not_found("agent-1");
        assert!(err.to_string().contains("agent-1"));

        let err = AgentSwarmError::NoAgentsAvailable;
        assert!(err.to_string().contains("no agents"));
    }

    #[test]
    fn test_is_retryable() {
        assert!(AgentSwarmError::NoAgentsAvailable.is_retryable());
        assert!(AgentSwarmError::unavailable("a", "busy").is_retryable());
        assert!(AgentSwarmError::heartbeat_timeout("a", 5000).is_retryable());

        assert!(!AgentSwarmError::agent_not_found("a").is_retryable());
        assert!(!AgentSwarmError::already_registered("a").is_retryable());
    }
}
