//! Orchestrator event definitions for event sourcing.
//!
//! All state-changing operations are captured as events that can be
//! replayed to recover orchestrator state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Events that represent state changes in the orchestrator.
///
/// Each event captures the minimal information needed to reconstruct
/// the state change during replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrchestratorEvent {
    /// A new workflow was registered with the orchestrator.
    WorkflowRegistered {
        /// Unique workflow identifier
        workflow_id: String,
        /// Workflow name
        name: String,
        /// Serialized DAG structure
        dag_json: String,
    },

    /// A workflow was unregistered/removed.
    WorkflowUnregistered {
        /// Workflow identifier
        workflow_id: String,
    },

    /// A workflow's status changed.
    WorkflowStatusChanged {
        /// Workflow identifier
        workflow_id: String,
        /// New status
        status: String,
    },

    /// A bead was scheduled for execution.
    BeadScheduled {
        /// Workflow this bead belongs to
        workflow_id: String,
        /// Bead identifier
        bead_id: String,
    },

    /// A bead was claimed by a worker.
    BeadClaimed {
        /// Bead identifier
        bead_id: String,
        /// Worker that claimed the bead
        worker_id: String,
    },

    /// A bead started execution.
    BeadStarted {
        /// Bead identifier
        bead_id: String,
        /// When execution started
        started_at: DateTime<Utc>,
    },

    /// A bead completed successfully.
    BeadCompleted {
        /// Bead identifier
        bead_id: String,
        /// When execution completed
        completed_at: DateTime<Utc>,
    },

    /// A bead failed.
    BeadFailed {
        /// Bead identifier
        bead_id: String,
        /// Error message
        error: String,
        /// When the failure occurred
        failed_at: DateTime<Utc>,
    },

    /// A bead was cancelled.
    BeadCancelled {
        /// Bead identifier
        bead_id: String,
        /// Why it was cancelled
        reason: String,
    },

    /// A checkpoint was created.
    CheckpointCreated {
        /// Checkpoint identifier
        checkpoint_id: String,
        /// Event sequence number at checkpoint time
        event_sequence: u64,
    },

    /// An agent registered with the orchestrator.
    AgentRegistered {
        /// Agent identifier
        agent_id: String,
        /// Agent capabilities
        capabilities: Vec<String>,
    },

    /// An agent was unregistered.
    AgentUnregistered {
        /// Agent identifier
        agent_id: String,
    },

    /// An agent heartbeat was received.
    AgentHeartbeat {
        /// Agent identifier
        agent_id: String,
        /// When the heartbeat was received
        timestamp: DateTime<Utc>,
    },
}

impl OrchestratorEvent {
    /// Get the event type name.
    #[must_use]
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::WorkflowRegistered { .. } => "workflow_registered",
            Self::WorkflowUnregistered { .. } => "workflow_unregistered",
            Self::WorkflowStatusChanged { .. } => "workflow_status_changed",
            Self::BeadScheduled { .. } => "bead_scheduled",
            Self::BeadClaimed { .. } => "bead_claimed",
            Self::BeadStarted { .. } => "bead_started",
            Self::BeadCompleted { .. } => "bead_completed",
            Self::BeadFailed { .. } => "bead_failed",
            Self::BeadCancelled { .. } => "bead_cancelled",
            Self::CheckpointCreated { .. } => "checkpoint_created",
            Self::AgentRegistered { .. } => "agent_registered",
            Self::AgentUnregistered { .. } => "agent_unregistered",
            Self::AgentHeartbeat { .. } => "agent_heartbeat",
        }
    }

    /// Get the workflow ID if this event is workflow-related.
    #[must_use]
    pub fn workflow_id(&self) -> Option<&str> {
        match self {
            Self::WorkflowRegistered { workflow_id, .. }
            | Self::WorkflowUnregistered { workflow_id }
            | Self::WorkflowStatusChanged { workflow_id, .. }
            | Self::BeadScheduled { workflow_id, .. } => Some(workflow_id),
            _ => None,
        }
    }

    /// Get the bead ID if this event is bead-related.
    #[must_use]
    pub fn bead_id(&self) -> Option<&str> {
        match self {
            Self::BeadScheduled { bead_id, .. }
            | Self::BeadClaimed { bead_id, .. }
            | Self::BeadStarted { bead_id, .. }
            | Self::BeadCompleted { bead_id, .. }
            | Self::BeadFailed { bead_id, .. }
            | Self::BeadCancelled { bead_id, .. } => Some(bead_id),
            _ => None,
        }
    }
}

/// Stored event record with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Unique event identifier
    pub id: String,
    /// Event sequence number
    pub sequence: u64,
    /// The event itself
    pub event: OrchestratorEvent,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
}

impl EventRecord {
    /// Create a new event record.
    #[must_use]
    pub fn new(id: impl Into<String>, sequence: u64, event: OrchestratorEvent) -> Self {
        Self {
            id: id.into(),
            sequence,
            event,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_names() {
        assert_eq!(
            OrchestratorEvent::WorkflowRegistered {
                workflow_id: "wf".to_string(),
                name: "n".to_string(),
                dag_json: "{}".to_string(),
            }
            .event_type(),
            "workflow_registered"
        );

        assert_eq!(
            OrchestratorEvent::BeadCompleted {
                bead_id: "b".to_string(),
                completed_at: Utc::now(),
            }
            .event_type(),
            "bead_completed"
        );
    }

    #[test]
    fn test_workflow_id_extraction() {
        let event = OrchestratorEvent::WorkflowRegistered {
            workflow_id: "wf-123".to_string(),
            name: "Test".to_string(),
            dag_json: "{}".to_string(),
        };
        assert_eq!(event.workflow_id(), Some("wf-123"));

        let bead_event = OrchestratorEvent::BeadCompleted {
            bead_id: "b-1".to_string(),
            completed_at: Utc::now(),
        };
        assert_eq!(bead_event.workflow_id(), None);
    }

    #[test]
    fn test_bead_id_extraction() {
        let event = OrchestratorEvent::BeadClaimed {
            bead_id: "bead-456".to_string(),
            worker_id: "w-1".to_string(),
        };
        assert_eq!(event.bead_id(), Some("bead-456"));

        let wf_event = OrchestratorEvent::WorkflowUnregistered {
            workflow_id: "wf".to_string(),
        };
        assert_eq!(wf_event.bead_id(), None);
    }

    #[test]
    fn test_event_serialization() {
        let event = OrchestratorEvent::BeadFailed {
            bead_id: "b-1".to_string(),
            error: "Something went wrong".to_string(),
            failed_at: Utc::now(),
        };

        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        if let Ok(serialized) = json {
            assert!(serialized.contains("bead_failed"));
            assert!(serialized.contains("Something went wrong"));
        }
    }
}
