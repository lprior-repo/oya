//! Message types for Zellij plugin IPC communication.
//!
//! This module defines the message protocol between the Zellij guest plugin
//! (UI) and the host (OYA orchestrator).
//!
//! # Message Flow
//!
//! **Guest → Host (GuestMessage)**: Commands from the UI plugin
//! - Query requests (GetBeadList, GetBeadDetail, etc.)
//! - Command requests (StartBead, CancelBead, RetryBead)
//!
//! **Host → Guest (HostMessage)**: Responses and events from the orchestrator
//! - Query responses (BeadList, BeadDetail, etc.)
//! - Acknowledgments (Ack)
//! - Broadcast events (BeadStateChanged, PhaseProgress, etc.)

use serde::{Deserialize, Serialize};

/// Messages from Zellij guest plugin to host.
///
/// These are requests from the UI that the host processes and responds to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuestMessage {
    // QUERIES
    // ═══════
    /// Get list of all beads.
    GetBeadList,

    /// Get details for a specific bead.
    GetBeadDetail {
        /// Bead ID to query
        bead_id: String,
    },

    /// Get workflow graph for visualization.
    GetWorkflowGraph {
        /// Workflow ID to query
        workflow_id: String,
    },

    /// Get agent pool statistics.
    GetAgentPool,

    /// Get system health status.
    GetSystemHealth,

    // COMMANDS
    // ════════
    /// Start executing a bead.
    StartBead {
        /// Bead ID to start
        bead_id: String,
    },

    /// Cancel a running bead.
    CancelBead {
        /// Bead ID to cancel
        bead_id: String,
    },

    /// Retry a failed bead.
    RetryBead {
        /// Bead ID to retry
        bead_id: String,
    },
}

/// Messages from host to Zellij guest plugin.
///
/// These are responses to queries, acknowledgments of commands,
/// and broadcast events from the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostMessage {
    // QUERY RESPONSES
    // ═══════════════
    /// List of all beads.
    BeadList {
        /// List of bead summaries
        beads: Vec<BeadSummary>,
    },

    /// Details for a specific bead.
    BeadDetail {
        /// Bead details
        bead: BeadDetail,
    },

    /// Workflow graph for visualization.
    WorkflowGraph {
        /// Workflow ID
        workflow_id: String,
        /// Graph nodes (beads)
        nodes: Vec<GraphNode>,
        /// Graph edges (dependencies)
        edges: Vec<GraphEdge>,
    },

    /// Agent pool statistics.
    AgentPoolStats {
        /// Total agents
        total_agents: usize,
        /// Active agents
        active_agents: usize,
        /// Idle agents
        idle_agents: usize,
        /// Beads assigned
        beads_assigned: usize,
        /// Beads completed
        beads_completed: usize,
    },

    /// System health status.
    SystemHealth {
        /// Overall health status
        status: HealthStatus,
        /// Component health
        components: Vec<ComponentHealth>,
    },

    // COMMAND ACKNOWLEDGMENTS
    // ════════════════════════
    /// Acknowledgment of successful command.
    Ack {
        /// Command that was acknowledged
        command: String,
        /// Result message
        message: String,
    },

    /// Error response.
    Error {
        /// Error message
        message: String,
    },

    // BROADCAST EVENTS
    // ═════════════════
    /// Bead state changed.
    BeadStateChanged {
        /// Bead ID
        bead_id: String,
        /// Previous state
        from_state: String,
        /// New state
        to_state: String,
        /// Timestamp
        timestamp: u64,
    },

    /// Phase progress update.
    PhaseProgress {
        /// Bead ID
        bead_id: String,
        /// Phase ID
        phase_id: String,
        /// Progress percentage (0-100)
        progress: u8,
        /// Current step description
        current_step: String,
    },

    /// Agent heartbeat.
    AgentHeartbeat {
        /// Agent ID
        agent_id: String,
        /// Current state
        state: String,
        /// Current bead (if any)
        current_bead: Option<String>,
        /// Timestamp
        timestamp: u64,
    },

    /// System alert.
    SystemAlert {
        /// Alert level
        level: AlertLevel,
        /// Alert message
        message: String,
        /// Related component (if any)
        component: Option<String>,
        /// Timestamp
        timestamp: u64,
    },
}

/// Summary of a bead for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadSummary {
    /// Bead ID
    pub id: String,
    /// Bead title
    pub title: String,
    /// Current state
    pub state: String,
    /// Priority
    pub priority: u8,
    /// Creation timestamp
    pub created_at: u64,
}

/// Detailed information about a bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadDetail {
    /// Bead ID
    pub id: String,
    /// Bead title
    pub title: String,
    /// Full description
    pub description: String,
    /// Current state
    pub state: String,
    /// Priority
    pub priority: u8,
    /// Type (feature, bugfix, etc.)
    pub issue_type: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
    /// Labels
    pub labels: Vec<String>,
    /// Dependencies (bead IDs that must complete first)
    pub dependencies: Vec<String>,
}

/// Graph node for workflow visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Bead ID
    pub id: String,
    /// Bead title
    pub label: String,
    /// Current state
    pub state: String,
    /// Position (x, y) for layout
    pub position: Option<(f32, f32)>,
}

/// Graph edge for workflow visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// From bead ID
    pub from: String,
    /// To bead ID
    pub to: String,
    /// Edge label (optional)
    pub label: Option<String>,
}

/// Health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is unhealthy
    Unhealthy,
}

/// Component health information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Health status
    pub status: HealthStatus,
    /// Status message
    pub message: String,
    /// Last check timestamp
    pub last_check: u64,
}

/// Alert level for system alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_message_serialization() {
        let msg = GuestMessage::GetBeadDetail {
            bead_id: "bead-123".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("serialization should succeed");
        assert!(json.contains("get_bead_detail"));
        assert!(json.contains("bead-123"));

        let decoded: GuestMessage =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert!(matches!(
            decoded,
            GuestMessage::GetBeadDetail { bead_id } if bead_id == "bead-123"
        ));
    }

    #[test]
    fn test_host_message_serialization() {
        let msg = HostMessage::BeadStateChanged {
            bead_id: "bead-123".to_string(),
            from_state: "pending".to_string(),
            to_state: "running".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&msg).expect("serialization should succeed");
        assert!(json.contains("bead_state_changed"));

        let decoded: HostMessage =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert!(matches!(
            decoded,
            HostMessage::BeadStateChanged { bead_id, .. } if bead_id == "bead-123"
        ));
    }

    #[test]
    fn test_bead_summary_serialization() {
        let summary = BeadSummary {
            id: "bead-123".to_string(),
            title: "Test bead".to_string(),
            state: "pending".to_string(),
            priority: 1,
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&summary).expect("serialization should succeed");
        let decoded: BeadSummary =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(decoded.id, "bead-123");
        assert_eq!(decoded.title, "Test bead");
    }

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).expect("serialization should succeed");
        assert!(json.contains("healthy"));

        let decoded: HealthStatus =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert!(matches!(decoded, HealthStatus::Healthy));
    }

    #[test]
    fn test_alert_level_serialization() {
        let level = AlertLevel::Critical;
        let json = serde_json::to_string(&level).expect("serialization should succeed");
        assert!(json.contains("critical"));

        let decoded: AlertLevel =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert!(matches!(decoded, AlertLevel::Critical));
    }
}
