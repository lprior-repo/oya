//! Configuration for swarm operations.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};

/// Configuration for the swarm system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Target number of beads to complete.
    #[serde(default = "default_target_beads")]
    pub target_beads: usize,

    /// Number of Test Writer agents.
    #[serde(default = "default_test_writers")]
    pub test_writers: usize,

    /// Number of Implementer agents.
    #[serde(default = "default_implementers")]
    pub implementers: usize,

    /// Number of Reviewer agents.
    #[serde(default = "default_reviewers")]
    pub reviewers: usize,

    /// Enable Planner agent (contract-first development).
    #[serde(default = "default_planner")]
    pub planner: bool,

    /// Enable continuous-deployment principles (ALWAYS TRUE - cannot be disabled).
    #[serde(default = "default_continuous_deployment")]
    pub continuous_deployment: bool,

    /// Maximum execution time in seconds.
    #[serde(default = "default_max_timeout")]
    pub max_timeout_secs: u64,

    /// Maximum consecutive failures before abort.
    #[serde(default = "default_max_failures")]
    pub max_consecutive_failures: usize,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            target_beads: default_target_beads(),
            test_writers: default_test_writers(),
            implementers: default_implementers(),
            reviewers: default_reviewers(),
            planner: default_planner(),
            continuous_deployment: default_continuous_deployment(),
            max_timeout_secs: default_max_timeout(),
            max_consecutive_failures: default_max_failures(),
        }
    }
}

impl SwarmConfig {
    /// Create a new swarm config with defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            target_beads: 25,
            test_writers: 4,
            implementers: 4,
            reviewers: 4,
            planner: true,
            continuous_deployment: true,
            max_timeout_secs: 3600,
            max_consecutive_failures: 10,
        }
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.target_beads == 0 {
            return Err("target_beads must be greater than 0".to_string());
        }

        if self.test_writers == 0 {
            return Err("test_writers must be greater than 0".to_string());
        }

        if self.implementers == 0 {
            return Err("implementers must be greater than 0".to_string());
        }

        if self.reviewers == 0 {
            return Err("reviewers must be greater than 0".to_string());
        }

        // continuous_deployment CANNOT be disabled
        if !self.continuous_deployment {
            return Err(
                "continuous_deployment CANNOT be disabled - it is the absolute law of this system"
                    .to_string(),
            );
        }

        if self.max_timeout_secs == 0 {
            return Err("max_timeout_secs must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Get total number of agents.
    #[must_use]
    pub const fn total_agents(&self) -> usize {
        self.test_writers + self.implementers + self.reviewers + if self.planner { 1 } else { 0 }
    }
}

fn default_target_beads() -> usize {
    25
}

fn default_test_writers() -> usize {
    4
}

fn default_implementers() -> usize {
    4
}

fn default_reviewers() -> usize {
    4
}

fn default_planner() -> bool {
    true
}

fn default_continuous_deployment() -> bool {
    // ABSOLUTE LAW: Continuous deployment is ALWAYS ON and CANNOT be disabled
    true
}

fn default_max_timeout() -> u64 {
    3600 // 1 hour
}

fn default_max_failures() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SwarmConfig::default();
        assert_eq!(config.target_beads, 25);
        assert_eq!(config.test_writers, 4);
        assert_eq!(config.implementers, 4);
        assert_eq!(config.reviewers, 4);
        assert_eq!(config.planner, true);
        assert_eq!(config.continuous_deployment, true);
        assert_eq!(config.total_agents(), 13);
    }

    #[test]
    fn test_config_validate() {
        let config = SwarmConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_continuous_deployment_required() {
        let mut config = SwarmConfig::default();
        config.continuous_deployment = false;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CANNOT be disabled"));
    }
}
