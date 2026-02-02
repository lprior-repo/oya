//! Event types for communication between backend and frontend
//!
//! These types define the events that flow from the Tauri backend
//! to the frontend for real-time updates.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// Bead event for UI communication
///
/// This is the canonical event type for all bead-related state changes.
/// Events flow from backend to frontend via Tauri's event system.
#[derive(
    Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(tag = "type")]
pub enum BeadEvent {
    /// A new bead was created
    Created { bead_id: String, title: String },
    /// Bead started execution
    Started { bead_id: String },
    /// Bead completed successfully
    Completed { bead_id: String, success: bool },
    /// Bead failed
    Failed { bead_id: String, error: String },
    /// Bead was cancelled
    Cancelled { bead_id: String },
    /// Phase started within a bead
    PhaseStarted { bead_id: String, phase: String },
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
    Heartbeat { timestamp: u64 },
    /// Unknown event type (for forward compatibility)
    Unknown { raw: String },
}

impl BeadEvent {
    /// Returns a string describing the event type
    #[must_use]
    pub const fn event_type(&self) -> &str {
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
    #[must_use]
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
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            BeadEvent::Completed { .. } | BeadEvent::Failed { .. } | BeadEvent::Cancelled { .. }
        )
    }
}

/// Stream chunk for high-throughput text streaming
#[derive(
    Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct StreamChunk {
    /// Stream identifier
    pub stream_id: String,
    /// Raw bytes (UTF-8 text)
    pub data: Vec<u8>,
    /// Absolute offset in the stream
    pub offset: u64,
}

impl StreamChunk {
    /// Create a new stream chunk
    #[must_use]
    pub fn new(stream_id: impl Into<String>, data: Vec<u8>, offset: u64) -> Self {
        Self {
            stream_id: stream_id.into(),
            data,
            offset,
        }
    }

    /// Get the data as a string (lossy conversion)
    #[must_use]
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.data)
    }
}

/// Stream ended event
#[derive(
    Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct StreamEnded {
    /// Stream identifier
    pub stream_id: String,
    /// Exit code (if process)
    pub exit_code: Option<i32>,
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

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk::new("stream-1", b"Hello, world!".to_vec(), 0);
        assert_eq!(chunk.stream_id, "stream-1");
        assert_eq!(chunk.as_str_lossy(), "Hello, world!");
        assert_eq!(chunk.offset, 0);
    }

    #[test]
    fn test_rkyv_event() {
        let event = BeadEvent::Progress {
            bead_id: "bead-1".into(),
            percent: 75,
            message: "Almost done".into(),
        };

        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&event);
        assert!(bytes.is_ok());
    }
}
