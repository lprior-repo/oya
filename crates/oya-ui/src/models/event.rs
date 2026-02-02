//! Event types for WebSocket communication
//!
//! These types mirror the backend oya-events::BeadEvent for WASM compatibility.
//! The oya-events crate depends on tokio which doesn't support WASM targets.

use serde::{Deserialize, Serialize};

/// Simplified BeadEvent for UI communication
///
/// This is a subset of the backend BeadEvent optimized for UI display.
/// The backend serializes events using bincode over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BeadEvent {
    /// A new bead was created
    Created {
        bead_id: String,
        title: String,
    },
    /// Bead started execution
    Started {
        bead_id: String,
    },
    /// Bead completed successfully
    Completed {
        bead_id: String,
        success: bool,
    },
    /// Bead failed
    Failed {
        bead_id: String,
        error: String,
    },
    /// Bead was cancelled
    Cancelled {
        bead_id: String,
    },
    /// Phase started within a bead
    PhaseStarted {
        bead_id: String,
        phase: String,
    },
    /// Phase completed within a bead
    PhaseCompleted {
        bead_id: String,
        phase: String,
        success: bool,
    },
    /// State change notification
    StateChanged {
        bead_id: String,
        old_state: String,
        new_state: String,
    },
    /// Progress update
    Progress {
        bead_id: String,
        percent: u8,
        message: String,
    },
    /// Log message from execution
    Log {
        bead_id: String,
        level: String,
        message: String,
    },
    /// Heartbeat to keep connection alive
    Heartbeat {
        timestamp: u64,
    },
    /// Unknown event type (for forward compatibility)
    Unknown {
        raw: String,
    },
}

impl BeadEvent {
    /// Returns a string describing the event type
    pub fn event_type(&self) -> &str {
        match self {
            BeadEvent::Created { .. } => "Created",
            BeadEvent::Started { .. } => "Started",
            BeadEvent::Completed { .. } => "Completed",
            BeadEvent::Failed { .. } => "Failed",
            BeadEvent::Cancelled { .. } => "Cancelled",
            BeadEvent::PhaseStarted { .. } => "PhaseStarted",
            BeadEvent::PhaseCompleted { .. } => "PhaseCompleted",
            BeadEvent::StateChanged { .. } => "StateChanged",
            BeadEvent::Progress { .. } => "Progress",
            BeadEvent::Log { .. } => "Log",
            BeadEvent::Heartbeat { .. } => "Heartbeat",
            BeadEvent::Unknown { .. } => "Unknown",
        }
    }

    /// Returns the bead ID if this event is associated with a bead
    pub fn bead_id(&self) -> Option<&str> {
        match self {
            BeadEvent::Created { bead_id, .. }
            | BeadEvent::Started { bead_id }
            | BeadEvent::Completed { bead_id, .. }
            | BeadEvent::Failed { bead_id, .. }
            | BeadEvent::Cancelled { bead_id }
            | BeadEvent::PhaseStarted { bead_id, .. }
            | BeadEvent::PhaseCompleted { bead_id, .. }
            | BeadEvent::StateChanged { bead_id, .. }
            | BeadEvent::Progress { bead_id, .. }
            | BeadEvent::Log { bead_id, .. } => Some(bead_id),
            BeadEvent::Heartbeat { .. } | BeadEvent::Unknown { .. } => None,
        }
    }

    /// Returns true if this is a terminal event (completed, failed, cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BeadEvent::Completed { .. } | BeadEvent::Failed { .. } | BeadEvent::Cancelled { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type() {
        let created = BeadEvent::Created {
            bead_id: "bead-1".into(),
            title: "Test".into(),
        };
        assert_eq!(created.event_type(), "Created");

        let heartbeat = BeadEvent::Heartbeat { timestamp: 12345 };
        assert_eq!(heartbeat.event_type(), "Heartbeat");
    }

    #[test]
    fn test_bead_id() {
        let created = BeadEvent::Created {
            bead_id: "bead-1".into(),
            title: "Test".into(),
        };
        assert_eq!(created.bead_id(), Some("bead-1"));

        let heartbeat = BeadEvent::Heartbeat { timestamp: 12345 };
        assert_eq!(heartbeat.bead_id(), None);
    }

    #[test]
    fn test_is_terminal() {
        let completed = BeadEvent::Completed {
            bead_id: "bead-1".into(),
            success: true,
        };
        assert!(completed.is_terminal());

        let started = BeadEvent::Started {
            bead_id: "bead-1".into(),
        };
        assert!(!started.is_terminal());
    }

    #[test]
    fn test_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let event = BeadEvent::Progress {
            bead_id: "bead-1".into(),
            percent: 50,
            message: "Halfway done".into(),
        };

        let json = serde_json::to_string(&event)?;
        assert!(json.contains("Progress"));
        assert!(json.contains("50"));

        let restored: BeadEvent = serde_json::from_str(&json)?;
        assert_eq!(restored.event_type(), "Progress");
        Ok(())
    }
}
