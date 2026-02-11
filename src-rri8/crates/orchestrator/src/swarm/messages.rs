//! Message types for swarm communication.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};

/// Commands from CLI to SwarmOrchestratorActor.
#[derive(Debug, Clone)]
pub enum SwarmCommand {
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

        /// Reply channel for status.
        reply: tokio::sync::oneshot::Sender<SwarmStatus>,
    },

    /// Stop the swarm.
    Stop {
        /// Whether to shutdown gracefully.
        graceful: bool,

        /// Reply channel for result.
        reply: tokio::sync::oneshot::Sender<Result<(), SwarmError>>,
    },

    /// Request swarm status.
    GetStatus {
        /// Reply channel for status response.
        reply: tokio::sync::oneshot::Sender<SwarmStatus>,
    },
}

/// Messages from SwarmOrchestratorActor to SwarmSupervisor.
#[derive(Debug, Clone)]
pub enum SwarmSupervisorMessage {
    /// Spawn swarm agents.
    SpawnAgents {
        /// Agent configuration.
        config: SwarmAgentConfig,
    },

    /// Agent failed during processing.
    AgentFailed {
        /// Agent identifier.
        agent_id: String,

        /// Type of agent.
        agent_type: SwarmAgentType,

        /// Bead being processed (if any).
        bead_id: Option<String>,

        /// Error message.
        error: String,
    },

    /// Bead phase completed.
    BeadPhaseComplete {
        /// Bead identifier.
        bead_id: String,

        /// Phase that completed.
        phase: BeadPhase,

        /// Result of the phase.
        result: BeadResult,
    },

    /// Shutdown the swarm.
    Shutdown,

    /// Get supervisor status.
    GetStatus {
        /// Reply channel for status.
        reply: tokio::sync::oneshot::Sender<SwarmStatus>,
    },
}

/// Swarm agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmAgentConfig {
    /// Number of Test Writer agents.
    pub test_writers: usize,

    /// Number of Implementer agents.
    pub implementers: usize,

    /// Number of Reviewer agents.
    pub reviewers: usize,

    /// Enable Planner agent.
    pub planner: bool,

    /// Continuous deployment enabled (always true).
    pub continuous_deployment: bool,
}

/// Type of swarm agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmAgentType {
    /// Test Writer agent.
    TestWriter,

    /// Implementer agent.
    Implementer,

    /// Reviewer agent.
    Reviewer,

    /// Planner agent.
    Planner,
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

/// Error type for swarm operations.
#[derive(Debug, Clone)]
pub enum SwarmError {
    /// Bead not found.
    BeadNotFound { bead_id: String },

    /// Invalid state transition.
    InvalidStateTransition {
        bead_id: String,
        from: String,
        to: String,
    },

    /// Agent failure.
    AgentFailed {
        agent_type: SwarmAgentType,
        agent_id: String,
        error: String,
    },

    /// Quality gate failure.
    QualityGateFailed {
        gate: String,
        bead_id: String,
        reason: String,
    },

    /// Workspace operation failed.
    WorkspaceFailed {
        workspace: String,
        operation: String,
        reason: String,
    },

    /// Command execution failed.
    CommandFailed {
        command: String,
        exit_code: i32,
    },

    /// Configuration error.
    ConfigError {
        parameter: String,
        reason: String,
    },

    /// Database error.
    DatabaseError {
        operation: String,
        reason: String,
    },
}

impl std::fmt::Display for SwarmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeadNotFound { bead_id } => {
                write!(f, "Bead not found: {}", bead_id)
            }
            Self::InvalidStateTransition { bead_id, from, to } => {
                write!(f, "Invalid state transition for bead '{}': {} -> {}", bead_id, from, to)
            }
            Self::AgentFailed {
                agent_type,
                agent_id,
                error,
            } => {
                write!(f, "Agent {:?} ({}) failed: {}", agent_type, agent_id, error)
            }
            Self::QualityGateFailed {
                gate,
                bead_id,
                reason,
            } => {
                write!(f, "Quality gate '{}' failed for bead '{}': {}", gate, bead_id, reason)
            }
            Self::WorkspaceFailed {
                workspace,
                operation,
                reason,
            } => {
                write!(
                    f,
                    "Workspace '{}' operation '{}' failed: {}",
                    workspace, operation, reason
                )
            }
            Self::CommandFailed { command, exit_code } => {
                write!(f, "Command '{}' failed with exit code {}", command, exit_code)
            }
            Self::ConfigError { parameter, reason } => {
                write!(f, "Configuration error for '{}': {}", parameter, reason)
            }
            Self::DatabaseError { operation, reason } => {
                write!(f, "Database error during '{}': {}", operation, reason)
            }
        }
    }
}

impl std::error::Error for SwarmError {}

/// Work item for a bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadWork {
    /// Bead identifier.
    pub bead_id: String,

    /// Bead title/description.
    pub title: String,

    /// Current phase.
    pub phase: BeadPhase,

    /// Priority (0 = highest).
    pub priority: u32,

    /// Dependencies (other bead IDs).
    pub dependencies: Vec<String>,
}

/// Phase of bead processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadPhase {
    /// Contract writing (Test Writer).
    Contract,

    /// Implementation (Implementer).
    Implementation,

    /// Review (Reviewer).
    Review,

    /// Complete.
    Complete,
}

/// Result of bead phase processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadResult {
    /// Whether the phase succeeded.
    pub success: bool,

    /// Test results (JSON).
    pub test_results: Option<serde_json::Value>,

    /// Workspace where work happened.
    pub workspace: Option<String>,

    /// Commit hash (if landed).
    pub commit_hash: Option<String>,

    /// Error message (if failed).
    pub error: Option<String>,
}

/// Messages for TestWriterActor.
#[derive(Debug, Clone)]
pub enum TestWriterMessage {
    /// Get next bead to write contract for.
    GetNextBead {
        /// Reply channel.
        reply: tokio::sync::oneshot::Sender<Option<BeadWork>>,
    },

    /// Write contract for a bead.
    WriteContract {
        /// Bead identifier.
        bead_id: String,

        /// Contract specification.
        contract: TestContract,
    },

    /// Mark bead as ready for implementation.
    ReadyForImplementation {
        /// Bead identifier.
        bead_id: String,
    },
}

/// Test contract from Test Writer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestContract {
    /// Bead identifier.
    pub bead_id: String,

    /// Error variants (exhaustive).
    pub error_variants: Vec<String>,

    /// Preconditions (valid inputs).
    pub preconditions: Vec<String>,

    /// Postconditions (promises made).
    pub postconditions: Vec<String>,

    /// Invariants (always true).
    pub invariants: Vec<String>,

    /// Test code with Given-When-Then structure.
    pub test_plan: String,

    /// Edge cases from break analysis.
    pub edge_cases: Vec<String>,
}

/// Messages for ImplementerActor.
#[derive(Debug, Clone)]
pub enum ImplementerMessage {
    /// Get next bead to implement.
    GetNextBead {
        /// Reply channel.
        reply: tokio::sync::oneshot::Sender<Option<BeadWork>>,
    },

    /// Implement a bead (follows contract).
    ImplementBead {
        /// Bead identifier.
        bead_id: String,

        /// Workspace name.
        workspace: String,
    },

    /// Submit implementation for review.
    SubmitForReview {
        /// Bead identifier.
        bead_id: String,

        /// Test results.
        test_results: serde_json::Value,
    },
}

/// Messages for ReviewerActor.
#[derive(Debug, Clone)]
pub enum ReviewerMessage {
    /// Get next bead to review.
    GetNextBead {
        /// Reply channel.
        reply: tokio::sync::oneshot::Sender<Option<BeadWork>>,
    },

    /// Review a bead.
    ReviewBead {
        /// Bead identifier.
        bead_id: String,
    },

    /// Land a bead (commit + push).
    LandBead {
        /// Bead identifier.
        bead_id: String,

        /// Commit hash.
        commit_hash: String,
    },
}

/// Messages for PlannerActor.
#[derive(Debug, Clone)]
pub enum PlannerMessage {
    /// Review bead requirements.
    ReviewRequirements {
        /// Bead identifier.
        bead_id: String,
    },

    /// Coordinate contract between Test Writer and Implementer.
    CoordinateContract {
        /// Bead identifier.
        bead_id: String,
    },
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
