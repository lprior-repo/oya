//! Messages for the SchedulerActor.
//!
//! Design principles:
//! - Commands are fire-and-forget (use `cast!`)
//! - Queries return responses (use `call!`)
//! - Business errors are returned in RPC replies, NOT as actor crashes

use ractor::RpcReplyPort;

use crate::dag::BeadId;
use crate::scheduler::{SchedulerStats, WorkflowId};

use super::errors::ActorError;

/// Bead state for state change tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadState {
    /// Bead is pending (waiting for dependencies).
    Pending,
    /// Bead is ready to execute.
    Ready,
    /// Bead is currently executing.
    Running,
    /// Bead has completed successfully.
    Completed,
    /// Bead has failed.
    Failed,
}

impl std::fmt::Display for BeadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Ready => write!(f, "ready"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Workflow status information.
#[derive(Debug, Clone)]
pub struct WorkflowStatus {
    /// The workflow ID.
    pub workflow_id: WorkflowId,
    /// Total number of beads in the workflow.
    pub total_beads: usize,
    /// Number of completed beads.
    pub completed_beads: usize,
    /// Number of ready beads (can be executed now).
    pub ready_beads: usize,
    /// Whether the workflow is complete.
    pub is_complete: bool,
}

/// Messages for the SchedulerActor.
///
/// This enum defines all messages the scheduler can receive.
/// Commands are fire-and-forget, queries expect responses via RpcReplyPort.
#[derive(Debug)]
pub enum SchedulerMessage {
    // ═══════════════════════════════════════════════════════════════════════
    // COMMANDS (fire-and-forget via cast!)
    // ═══════════════════════════════════════════════════════════════════════

    /// Register a new workflow (idempotent - no error if exists).
    RegisterWorkflow {
        /// The workflow ID to register.
        workflow_id: WorkflowId,
    },

    /// Unregister a workflow.
    UnregisterWorkflow {
        /// The workflow ID to unregister.
        workflow_id: WorkflowId,
    },

    /// Schedule a bead in a workflow.
    ScheduleBead {
        /// The workflow this bead belongs to.
        workflow_id: WorkflowId,
        /// The bead ID to schedule.
        bead_id: BeadId,
    },

    /// Add a dependency between beads.
    /// The `from_bead` must complete before `to_bead` can start.
    AddDependency {
        /// The workflow containing the beads.
        workflow_id: WorkflowId,
        /// The bead that must complete first.
        from_bead: BeadId,
        /// The bead that depends on from_bead.
        to_bead: BeadId,
    },

    /// Handle external bead completion (from EventBus).
    OnBeadCompleted {
        /// The workflow containing the bead.
        workflow_id: WorkflowId,
        /// The bead that completed.
        bead_id: BeadId,
    },

    /// Handle state change (from EventBus).
    OnStateChanged {
        /// The bead whose state changed.
        bead_id: BeadId,
        /// The previous state.
        from: BeadState,
        /// The new state.
        to: BeadState,
    },

    /// Claim a bead for a worker.
    ClaimBead {
        /// The bead to claim.
        bead_id: BeadId,
        /// The worker claiming the bead.
        worker_id: String,
    },

    /// Release a bead claim.
    ReleaseBead {
        /// The bead to release.
        bead_id: BeadId,
    },

    /// Initiate graceful shutdown.
    Shutdown,

    // ═══════════════════════════════════════════════════════════════════════
    // QUERIES (request-response via call! / call_t!)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get ready beads for a workflow.
    GetWorkflowReadyBeads {
        /// The workflow to query.
        workflow_id: WorkflowId,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Vec<BeadId>, ActorError>>,
    },

    /// Get scheduler statistics.
    GetStats {
        /// Reply port for the response.
        reply: RpcReplyPort<SchedulerStats>,
    },

    /// Check if a specific bead is ready to execute.
    IsBeadReady {
        /// The bead to check.
        bead_id: BeadId,
        /// The workflow containing the bead.
        workflow_id: WorkflowId,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<bool, ActorError>>,
    },

    /// Get workflow status.
    GetWorkflowStatus {
        /// The workflow to query.
        workflow_id: WorkflowId,
        /// Reply port for the response.
        reply: RpcReplyPort<Option<WorkflowStatus>>,
    },

    /// Get all ready beads across all workflows.
    GetAllReadyBeads {
        /// Reply port for the response.
        reply: RpcReplyPort<Vec<(WorkflowId, BeadId)>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_bead_state() {
        assert_eq!(BeadState::Pending.to_string(), "pending");
        assert_eq!(BeadState::Ready.to_string(), "ready");
        assert_eq!(BeadState::Running.to_string(), "running");
        assert_eq!(BeadState::Completed.to_string(), "completed");
        assert_eq!(BeadState::Failed.to_string(), "failed");
    }

    #[test]
    fn should_create_workflow_status() {
        let status = WorkflowStatus {
            workflow_id: "wf-123".to_string(),
            total_beads: 10,
            completed_beads: 5,
            ready_beads: 2,
            is_complete: false,
        };

        assert_eq!(status.workflow_id, "wf-123");
        assert_eq!(status.total_beads, 10);
        assert_eq!(status.completed_beads, 5);
        assert!(!status.is_complete);
    }
}
