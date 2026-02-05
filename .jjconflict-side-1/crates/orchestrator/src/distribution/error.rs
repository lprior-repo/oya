//! Distribution error types.

use std::fmt;

/// Errors that can occur during task distribution.
#[derive(Debug, Clone)]
pub enum DistributionError {
    /// No beads available for distribution.
    NoBeadsAvailable,

    /// No agents available for assignment.
    NoAgentsAvailable,

    /// Bead not found.
    BeadNotFound {
        /// The bead ID that was not found
        bead_id: String,
    },

    /// Agent not found.
    AgentNotFound {
        /// The agent ID that was not found
        agent_id: String,
    },

    /// Strategy configuration error.
    ConfigurationError {
        /// Error message
        message: String,
    },

    /// Affinity constraint cannot be satisfied.
    AffinityUnsatisfied {
        /// The bead ID
        bead_id: String,
        /// Required capability
        required_capability: String,
    },

    /// Priority calculation failed.
    PriorityError {
        /// The bead ID
        bead_id: String,
        /// Error message
        message: String,
    },

    /// Internal error.
    Internal {
        /// Error message
        message: String,
    },
}

impl fmt::Display for DistributionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBeadsAvailable => write!(f, "no beads available for distribution"),
            Self::NoAgentsAvailable => write!(f, "no agents available for assignment"),
            Self::BeadNotFound { bead_id } => write!(f, "bead not found: {}", bead_id),
            Self::AgentNotFound { agent_id } => write!(f, "agent not found: {}", agent_id),
            Self::ConfigurationError { message } => write!(f, "configuration error: {}", message),
            Self::AffinityUnsatisfied {
                bead_id,
                required_capability,
            } => write!(
                f,
                "affinity unsatisfied for bead {}: requires {}",
                bead_id, required_capability
            ),
            Self::PriorityError { bead_id, message } => {
                write!(f, "priority error for bead {}: {}", bead_id, message)
            }
            Self::Internal { message } => write!(f, "internal error: {}", message),
        }
    }
}

impl std::error::Error for DistributionError {}

impl DistributionError {
    /// Create a bead not found error.
    #[must_use]
    pub fn bead_not_found(bead_id: impl Into<String>) -> Self {
        Self::BeadNotFound {
            bead_id: bead_id.into(),
        }
    }

    /// Create an agent not found error.
    #[must_use]
    pub fn agent_not_found(agent_id: impl Into<String>) -> Self {
        Self::AgentNotFound {
            agent_id: agent_id.into(),
        }
    }

    /// Create a configuration error.
    #[must_use]
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create an affinity unsatisfied error.
    #[must_use]
    pub fn affinity_unsatisfied(bead_id: impl Into<String>, required: impl Into<String>) -> Self {
        Self::AffinityUnsatisfied {
            bead_id: bead_id.into(),
            required_capability: required.into(),
        }
    }

    /// Create a priority error.
    #[must_use]
    pub fn priority_error(bead_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::PriorityError {
            bead_id: bead_id.into(),
            message: message.into(),
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
        matches!(self, Self::NoBeadsAvailable | Self::NoAgentsAvailable)
    }

    /// Check if error is a configuration issue.
    #[must_use]
    pub const fn is_configuration_error(&self) -> bool {
        matches!(self, Self::ConfigurationError { .. })
    }
}

/// Result type for distribution operations.
pub type DistributionResult<T> = Result<T, DistributionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DistributionError::NoBeadsAvailable;
        assert!(err.to_string().contains("no beads"));

        let err = DistributionError::bead_not_found("bead-1");
        assert!(err.to_string().contains("bead-1"));

        let err = DistributionError::affinity_unsatisfied("bead-1", "rust");
        assert!(err.to_string().contains("bead-1"));
        assert!(err.to_string().contains("rust"));
    }

    #[test]
    fn test_is_retryable() {
        assert!(DistributionError::NoBeadsAvailable.is_retryable());
        assert!(DistributionError::NoAgentsAvailable.is_retryable());
        assert!(!DistributionError::bead_not_found("b").is_retryable());
        assert!(!DistributionError::configuration("bad").is_retryable());
    }

    #[test]
    fn test_is_configuration_error() {
        assert!(DistributionError::configuration("bad").is_configuration_error());
        assert!(!DistributionError::NoBeadsAvailable.is_configuration_error());
    }

    #[test]
    fn test_error_constructors() {
        let err = DistributionError::bead_not_found("b1");
        assert!(matches!(err, DistributionError::BeadNotFound { bead_id } if bead_id == "b1"));

        let err = DistributionError::agent_not_found("a1");
        assert!(matches!(err, DistributionError::AgentNotFound { agent_id } if agent_id == "a1"));

        let err = DistributionError::priority_error("b1", "calc failed");
        assert!(matches!(
            err,
            DistributionError::PriorityError { bead_id, message }
                if bead_id == "b1" && message == "calc failed"
        ));
    }

    #[test]
    fn test_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(DistributionError::NoBeadsAvailable);
        assert!(err.to_string().contains("no beads"));
    }
}
