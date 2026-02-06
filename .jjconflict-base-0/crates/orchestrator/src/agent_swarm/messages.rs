//! Agent message types for communication.

use serde::{Deserialize, Serialize};

/// Messages that can be sent to agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// Assign a bead to the agent for execution.
    AssignBead {
        /// The bead ID to execute
        bead_id: String,
        /// The workflow this bead belongs to
        workflow_id: String,
        /// Optional payload data
        payload: Option<serde_json::Value>,
    },

    /// Request the agent to send a heartbeat.
    Heartbeat,

    /// Cancel the currently assigned bead.
    CancelBead {
        /// The bead ID to cancel
        bead_id: String,
        /// Reason for cancellation
        reason: String,
    },

    /// Request agent to gracefully shutdown.
    Shutdown {
        /// Reason for shutdown
        reason: String,
        /// Whether to wait for current work to complete
        graceful: bool,
    },

    /// Request agent status.
    GetStatus,

    /// Update agent configuration.
    UpdateConfig {
        /// New configuration values
        config: serde_json::Value,
    },
}

/// Responses from agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentResponse {
    /// Bead assignment accepted.
    BeadAccepted {
        /// The bead ID that was accepted
        bead_id: String,
    },

    /// Bead assignment rejected.
    BeadRejected {
        /// The bead ID that was rejected
        bead_id: String,
        /// Reason for rejection
        reason: String,
    },

    /// Bead execution started.
    BeadStarted {
        /// The bead ID that started
        bead_id: String,
    },

    /// Bead execution completed successfully.
    BeadCompleted {
        /// The bead ID that completed
        bead_id: String,
        /// Optional result data
        result: Option<serde_json::Value>,
    },

    /// Bead execution failed.
    BeadFailed {
        /// The bead ID that failed
        bead_id: String,
        /// Error message
        error: String,
    },

    /// Heartbeat response.
    HeartbeatAck {
        /// Agent's current state
        state: String,
        /// Current bead being worked on (if any)
        current_bead: Option<String>,
    },

    /// Status response.
    Status {
        /// Agent state
        state: String,
        /// Current bead (if any)
        current_bead: Option<String>,
        /// Agent capabilities
        capabilities: Vec<String>,
        /// Uptime in seconds
        uptime_secs: u64,
    },

    /// Shutdown acknowledged.
    ShutdownAck,

    /// Generic error response.
    Error {
        /// Error message
        message: String,
    },
}

impl AgentMessage {
    /// Create an assign bead message.
    #[must_use]
    pub fn assign_bead(
        bead_id: impl Into<String>,
        workflow_id: impl Into<String>,
        payload: Option<serde_json::Value>,
    ) -> Self {
        Self::AssignBead {
            bead_id: bead_id.into(),
            workflow_id: workflow_id.into(),
            payload,
        }
    }

    /// Create a cancel bead message.
    #[must_use]
    pub fn cancel_bead(bead_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::CancelBead {
            bead_id: bead_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a shutdown message.
    #[must_use]
    pub fn shutdown(reason: impl Into<String>, graceful: bool) -> Self {
        Self::Shutdown {
            reason: reason.into(),
            graceful,
        }
    }
}

impl AgentResponse {
    /// Create a bead completed response.
    #[must_use]
    pub fn bead_completed(bead_id: impl Into<String>, result: Option<serde_json::Value>) -> Self {
        Self::BeadCompleted {
            bead_id: bead_id.into(),
            result,
        }
    }

    /// Create a bead failed response.
    #[must_use]
    pub fn bead_failed(bead_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::BeadFailed {
            bead_id: bead_id.into(),
            error: error.into(),
        }
    }

    /// Check if response indicates success.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(
            self,
            Self::BeadAccepted { .. }
                | Self::BeadStarted { .. }
                | Self::BeadCompleted { .. }
                | Self::HeartbeatAck { .. }
                | Self::Status { .. }
                | Self::ShutdownAck
        )
    }

    /// Check if response indicates an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(
            self,
            Self::BeadRejected { .. } | Self::BeadFailed { .. } | Self::Error { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = AgentMessage::assign_bead("bead-1", "wf-1", None);
        let json = serde_json::to_string(&msg);
        assert!(json.is_ok());

        if let Ok(s) = json {
            assert!(s.contains("assign_bead"));
            assert!(s.contains("bead-1"));
        }
    }

    #[test]
    fn test_response_is_success() {
        assert!(
            AgentResponse::BeadCompleted {
                bead_id: "b".to_string(),
                result: None
            }
            .is_success()
        );

        assert!(
            !AgentResponse::BeadFailed {
                bead_id: "b".to_string(),
                error: "err".to_string()
            }
            .is_success()
        );
    }

    #[test]
    fn test_response_is_error() {
        assert!(
            AgentResponse::BeadFailed {
                bead_id: "b".to_string(),
                error: "err".to_string()
            }
            .is_error()
        );

        assert!(
            !AgentResponse::BeadCompleted {
                bead_id: "b".to_string(),
                result: None
            }
            .is_error()
        );
    }
}
