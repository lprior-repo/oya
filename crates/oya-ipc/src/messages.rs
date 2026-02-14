//! Message types for Zellij plugin IPC communication.
//!
//! This module defines the message protocol between the Zellij guest plugin
//! (UI) and the host (OYA orchestrator).
//!
//! # Message Flow
//!
//! **Guest → Host (`GuestMessage`)**: Commands from the UI plugin
//! - Query requests (`GetBeadList`, `GetBeadDetail`, etc.)
//! - Command requests (`StartBead`, `CancelBead`, `RetryBead`)
//!
//! **Host → Guest (`HostMessage`)**: Responses and events from the orchestrator
//! - Query responses (`BeadList`, `BeadDetail`, etc.)
//! - Acknowledgments (Ack)
//! - Broadcast events (`BeadStateChanged`, `PhaseProgress`, etc.)

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

    /// Get list of all tasks.
    GetTaskList,

    /// Get details for a specific bead.
    GetBeadDetail {
        /// Bead ID to query
        bead_id: String,
    },

    /// Get details for a specific task.
    GetTaskDetail {
        /// Task slug to query
        slug: String,
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

    /// Run pipeline stages for a task.
    RunStage {
        /// Task slug to update
        slug: String,
        /// Stage name to run
        stage: String,
        /// Optional start stage for range
        from: Option<String>,
        /// Optional end stage for range
        to: Option<String>,
        /// Dry run (no persistence)
        dry_run: bool,
    },

    /// Run the full pipeline for a task.
    RunPipeline {
        /// Task slug to update
        slug: String,
        /// Dry run (no persistence)
        dry_run: bool,
    },

    /// Run the full pipeline for multiple tasks.
    RunPipelineBatch {
        /// Task slugs to update
        slugs: Vec<String>,
        /// Dry run (no persistence)
        dry_run: bool,
    },

    /// Subscribe to plugin events.
    SubscribeEvents {
        /// Event types the guest wants to listen for.
        event_types: Vec<String>,
    },

    /// Unsubscribe from all plugin events.
    UnsubscribeEvents,

    /// Approve a task for integration.
    ApproveTask {
        /// Task slug to approve
        slug: String,
        /// Force approval even if pipeline not passed
        force: bool,
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

    /// List of all tasks.
    TaskList {
        /// List of task summaries
        tasks: Vec<TaskSummary>,
    },

    /// Details for a specific bead.
    BeadDetail {
        /// Bead details
        bead: BeadDetail,
    },

    /// Details for a specific task.
    TaskDetail {
        /// Task details
        task: TaskDetail,
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

    /// Task update result.
    TaskUpdated {
        /// Task slug
        slug: String,
        /// Updated status
        status: String,
        /// Result message
        message: String,
    },

    /// Batch task update result.
    TaskBatchUpdated {
        /// Successful task updates
        updated: Vec<TaskUpdate>,
        /// Failed task updates
        failed: Vec<TaskUpdate>,
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

    // STAGE LIFECYCLE EVENTS
    // ══════════════════════
    /// Stage started for a bead.
    StageStarted {
        /// Bead ID
        bead_id: String,
        /// Stage name (research, plan, implement, review, validate, accept)
        stage: String,
        /// Attempt number (1-indexed)
        attempt: u32,
        /// Timestamp
        timestamp: u64,
    },

    /// Stage completed successfully.
    StageCompleted {
        /// Bead ID
        bead_id: String,
        /// Stage name
        stage: String,
        /// Artifact reference (if any)
        artifact_ref: Option<String>,
        /// Timestamp
        timestamp: u64,
    },

    /// Stage failed with feedback.
    StageFailed {
        /// Bead ID
        bead_id: String,
        /// Stage name
        stage: String,
        /// Feedback message
        feedback: String,
        /// Severity level (minor, major, fundamental)
        severity: String,
        /// Timestamp
        timestamp: u64,
    },

    /// Bead reentered earlier stage.
    StageReentry {
        /// Bead ID
        bead_id: String,
        /// Source stage
        from_stage: String,
        /// Target stage
        to_stage: String,
        /// Reason for reentry
        reason: String,
        /// Attempt number after reentry
        attempt: u32,
        /// Timestamp
        timestamp: u64,
    },

    /// Validation command executed.
    ValidationRan {
        /// Bead ID
        bead_id: String,
        /// Whether validation passed
        passed: bool,
        /// Command output
        output: String,
        /// Command that was run
        command: String,
        /// Exit code
        exit_code: i32,
        /// Timestamp
        timestamp: u64,
    },

    /// Recursion limits exhausted.
    RecursionExhausted {
        /// Bead ID
        bead_id: String,
        /// Total attempts made
        total_attempts: u32,
        /// Last stage that failed
        last_stage: String,
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

/// Summary of a task for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    /// Task slug
    pub slug: String,
    /// Pipeline status (created, `in_progress`, passed, failed, integrated)
    pub status: String,
    /// Current stage (if applicable)
    pub stage: Option<String>,
    /// Priority label (P0-P3)
    pub priority: String,
    /// Language label
    pub language: String,
    /// Task branch name
    pub branch: String,
}

/// Detailed information about a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    /// Task slug
    pub slug: String,
    /// Pipeline status (created, `in_progress`, passed, failed, integrated)
    pub status: String,
    /// Current stage (if applicable)
    pub stage: Option<String>,
    /// Priority label (P0-P3)
    pub priority: String,
    /// Language label
    pub language: String,
    /// Task branch name
    pub branch: String,
}

/// Task update summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    /// Task slug
    pub slug: String,
    /// Updated status (if available)
    pub status: Option<String>,
    /// Result message
    pub message: String,
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
    fn test_guest_message_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let msg = GuestMessage::GetBeadDetail {
            bead_id: "bead-123".to_string(),
        };

        let json = serde_json::to_string(&msg)?;
        assert!(json.contains("get_bead_detail"));
        assert!(json.contains("bead-123"));

        let decoded: GuestMessage = serde_json::from_str(&json)?;
        assert!(matches!(
            decoded,
            GuestMessage::GetBeadDetail { bead_id } if bead_id == "bead-123"
        ));
        Ok(())
    }

    #[test]
    fn test_guest_message_subscribe_events_serialization() -> Result<(), Box<dyn std::error::Error>>
    {
        let msg = GuestMessage::SubscribeEvents {
            event_types: vec!["build".to_string(), "deploy".to_string()],
        };

        let json = serde_json::to_string(&msg)?;
        assert!(json.contains("subscribe_events"));
        assert!(json.contains("build"));
        assert!(json.contains("deploy"));

        let decoded: GuestMessage = serde_json::from_str(&json)?;
        assert!(matches!(
            decoded,
            GuestMessage::SubscribeEvents { event_types } if event_types == vec!["build".to_string(), "deploy".to_string()]
        ));
        Ok(())
    }

    #[test]
    fn test_guest_message_unsubscribe_events_serialization()
    -> Result<(), Box<dyn std::error::Error>> {
        let msg = GuestMessage::UnsubscribeEvents;

        let json = serde_json::to_string(&msg)?;
        assert!(json.contains("unsubscribe_events"));

        let decoded: GuestMessage = serde_json::from_str(&json)?;
        assert!(matches!(decoded, GuestMessage::UnsubscribeEvents));
        Ok(())
    }

    #[test]
    fn test_task_summary_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let summary = TaskSummary {
            slug: "task-123".to_string(),
            status: "in_progress".to_string(),
            stage: Some("lint".to_string()),
            priority: "P1".to_string(),
            language: "Rust".to_string(),
            branch: "task/task-123".to_string(),
        };

        let json = serde_json::to_string(&summary)?;
        let decoded: TaskSummary = serde_json::from_str(&json)?;

        assert_eq!(decoded.slug, "task-123");
        assert_eq!(decoded.stage.as_deref(), Some("lint"));
        Ok(())
    }

    #[test]
    fn test_task_batch_update_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let msg = HostMessage::TaskBatchUpdated {
            updated: vec![TaskUpdate {
                slug: "task-1".to_string(),
                status: Some("passed".to_string()),
                message: "ok".to_string(),
            }],
            failed: vec![TaskUpdate {
                slug: "task-2".to_string(),
                status: None,
                message: "failed".to_string(),
            }],
        };

        let json = serde_json::to_string(&msg)?;
        assert!(json.contains("task_batch_updated"));

        let decoded: HostMessage = serde_json::from_str(&json)?;
        assert!(matches!(decoded, HostMessage::TaskBatchUpdated { .. }));
        Ok(())
    }

    #[test]
    fn test_run_pipeline_batch_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let msg = GuestMessage::RunPipelineBatch {
            slugs: vec!["task-1".to_string(), "task-2".to_string()],
            dry_run: true,
        };

        let json = serde_json::to_string(&msg)?;
        assert!(json.contains("run_pipeline_batch"));

        let decoded: GuestMessage = serde_json::from_str(&json)?;
        assert!(matches!(decoded, GuestMessage::RunPipelineBatch { .. }));
        Ok(())
    }

    #[test]
    fn test_task_update_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let update = TaskUpdate {
            slug: "task-99".to_string(),
            status: Some("failed".to_string()),
            message: "pipeline failed".to_string(),
        };

        let json = serde_json::to_string(&update)?;
        let decoded: TaskUpdate = serde_json::from_str(&json)?;

        assert_eq!(decoded.slug, "task-99");
        assert_eq!(decoded.status.as_deref(), Some("failed"));
        Ok(())
    }

    #[test]
    fn test_host_message_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let msg = HostMessage::BeadStateChanged {
            bead_id: "bead-123".to_string(),
            from_state: "pending".to_string(),
            to_state: "running".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&msg)?;
        assert!(json.contains("bead_state_changed"));

        let decoded: HostMessage = serde_json::from_str(&json)?;
        assert!(matches!(
            decoded,
            HostMessage::BeadStateChanged { bead_id, .. } if bead_id == "bead-123"
        ));
        Ok(())
    }

    #[test]
    fn test_bead_summary_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let summary = BeadSummary {
            id: "bead-123".to_string(),
            title: "Test bead".to_string(),
            state: "pending".to_string(),
            priority: 1,
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&summary)?;
        let decoded: BeadSummary = serde_json::from_str(&json)?;

        assert_eq!(decoded.id, "bead-123");
        assert_eq!(decoded.title, "Test bead");
        Ok(())
    }

    #[test]
    fn test_health_status_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status)?;
        assert!(json.contains("healthy"));

        let decoded: HealthStatus = serde_json::from_str(&json)?;
        assert!(matches!(decoded, HealthStatus::Healthy));
        Ok(())
    }

    #[test]
    fn test_alert_level_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let level = AlertLevel::Critical;
        let json = serde_json::to_string(&level)?;
        assert!(json.contains("critical"));

        let decoded: AlertLevel = serde_json::from_str(&json)?;
        assert!(matches!(decoded, AlertLevel::Critical));
        Ok(())
    }
}
