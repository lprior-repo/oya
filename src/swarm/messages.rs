//! Message types for swarm communication.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};

/// Messages for orchestrator communication.
#[derive(Debug)]
pub enum SwarmMessage {
    /// Start the swarm with target configuration.
    Start {
        /// Target number of beads to complete.
        target_beads: usize,

        /// Number of Test Writer agents.
        test_writers: usize,

        /// Number of Implementer agents.
        implementers: usize,

        /// Number of Reviewer agents.
        reviewers: usize,

        /// Whether to enable Planner agent.
        planner: bool,
    },

    /// Contract is ready for a bead.
    ContractReady {
        /// Bead identifier.
        bead_id: String,

        /// Path to contract file.
        contract_path: String,
    },

    /// Implementation is complete.
    ImplementationComplete {
        /// Bead identifier.
        bead_id: String,

        /// Workspace where implementation happened.
        workspace: String,

        /// Test results from moon ci.
        test_results: serde_json::Value,
    },

    /// Bead has landed successfully.
    BeadLanded {
        /// Bead identifier.
        bead_id: String,

        /// Git commit hash.
        commit_hash: String,
    },

    /// Agent failed during processing.
    AgentFailed {
        /// Type of agent (test_writer, implementer, reviewer, planner).
        agent_type: String,

        /// Agent identifier.
        agent_id: String,

        /// Bead being processed (if any).
        bead_id: Option<String>,

        /// Error message.
        error: String,
    },

    /// Request swarm status.
    GetStatus {
        /// Reply channel for status response.
        reply: Option<tokio::sync::oneshot::Sender<SwarmStatus>>,
    },

    /// Shutdown the swarm gracefully.
    Shutdown,
}

impl Clone for SwarmMessage {
    fn clone(&self) -> Self {
        match self {
            Self::Start {
                target_beads,
                test_writers,
                implementers,
                reviewers,
                planner,
            } => Self::Start {
                target_beads: *target_beads,
                test_writers: *test_writers,
                implementers: *implementers,
                reviewers: *reviewers,
                planner: *planner,
            },
            Self::ContractReady {
                bead_id,
                contract_path,
            } => Self::ContractReady {
                bead_id: bead_id.clone(),
                contract_path: contract_path.clone(),
            },
            Self::ImplementationComplete {
                bead_id,
                workspace,
                test_results,
            } => Self::ImplementationComplete {
                bead_id: bead_id.clone(),
                workspace: workspace.clone(),
                test_results: test_results.clone(),
            },
            Self::BeadLanded {
                bead_id,
                commit_hash,
            } => Self::BeadLanded {
                bead_id: bead_id.clone(),
                commit_hash: commit_hash.clone(),
            },
            Self::AgentFailed {
                agent_type,
                agent_id,
                bead_id,
                error,
            } => Self::AgentFailed {
                agent_type: agent_type.clone(),
                agent_id: agent_id.clone(),
                bead_id: bead_id.clone(),
                error: error.clone(),
            },
            Self::GetStatus { .. } => {
                // Can't clone oneshot::Sender, so create a None variant
                Self::GetStatus { reply: None }
            }
            Self::Shutdown => Self::Shutdown,
        }
    }
}

/// Swarm status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmStatus {
    /// Current state of the swarm.
    pub state: SwarmState,

    /// Total beads completed.
    pub landed_beads: usize,

    /// Target number of beads.
    pub target_beads: usize,

    /// Beads currently in progress.
    pub in_progress: usize,

    /// Beads pending assignment.
    pub pending: usize,

    /// Beads that failed.
    pub failed: usize,

    /// Active agents by type.
    pub active_agents: AgentCounts,

    /// Start time (Unix timestamp).
    pub start_time: Option<u64>,

    /// Estimated completion time (Unix timestamp).
    pub estimated_completion: Option<u64>,
}

/// Swarm state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmState {
    /// Swarm is starting up.
    Starting,

    /// Swarm is running normally.
    Running,

    /// Swarm is shutting down.
    ShuttingDown,

    /// Swarm has stopped.
    Stopped,

    /// Swarm completed successfully.
    Completed,
}

/// Active agent counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCounts {
    /// Number of active Test Writers.
    pub test_writers: usize,

    /// Number of active Implementers.
    pub implementers: usize,

    /// Number of active Reviewers.
    pub reviewers: usize,

    /// Number of active Planners.
    pub planners: usize,
}

impl AgentCounts {
    /// Create new agent counts.
    #[must_use]
    pub const fn new(
        test_writers: usize,
        implementers: usize,
        reviewers: usize,
        planners: usize,
    ) -> Self {
        Self {
            test_writers,
            implementers,
            reviewers,
            planners,
        }
    }

    /// Get total number of agents.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.test_writers + self.implementers + self.reviewers + self.planners
    }
}

impl Default for SwarmStatus {
    fn default() -> Self {
        Self {
            state: SwarmState::Starting,
            landed_beads: 0,
            target_beads: 25,
            in_progress: 0,
            pending: 0,
            failed: 0,
            active_agents: AgentCounts::new(0, 0, 0, 0),
            start_time: None,
            estimated_completion: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_counts_total() {
        let counts = AgentCounts::new(4, 4, 4, 1);
        assert_eq!(counts.total(), 13);
    }

    #[test]
    fn test_swarm_status_default() {
        let status = SwarmStatus::default();
        assert_eq!(status.state, SwarmState::Starting);
        assert_eq!(status.landed_beads, 0);
        assert_eq!(status.target_beads, 25);
    }
}
