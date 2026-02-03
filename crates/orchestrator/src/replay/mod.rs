//! Replay and recovery engine for orchestrator state.
//!
//! This module provides event sourcing and replay capabilities for
//! recovering orchestrator state after crashes or restarts.
//!
//! # Architecture
//!
//! The replay system uses event sourcing:
//! 1. All state changes are captured as `OrchestratorEvent`s
//! 2. Events are persisted to the database
//! 3. Periodic checkpoints capture full state snapshots
//! 4. Recovery replays events since the last checkpoint
//!
//! # Key Types
//!
//! - [`OrchestratorEvent`]: Enum of all state-changing events
//! - [`ReplayEngine`]: Recovers state by replaying events
//! - [`CheckpointManager`]: Creates and manages checkpoints

mod checkpoint;
mod engine;
mod events;
mod projection;

pub use checkpoint::CheckpointManager;
pub use engine::ReplayEngine;
pub use events::OrchestratorEvent;
pub use projection::OrchestratorProjection;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = OrchestratorEvent::WorkflowRegistered {
            workflow_id: "wf-test".to_string(),
            name: "Test Workflow".to_string(),
            dag_json: r#"{"nodes":[]}"#.to_string(),
        };

        let json = serde_json::to_string(&event);
        assert!(json.is_ok(), "serialization should succeed");

        if let Ok(serialized) = json {
            let deserialized: Result<OrchestratorEvent, _> = serde_json::from_str(&serialized);
            assert!(deserialized.is_ok(), "deserialization should succeed");
        }
    }
}
