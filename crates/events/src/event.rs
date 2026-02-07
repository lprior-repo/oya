//! Bead event types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{BeadId, BeadResult, BeadSpec, BeadState, EventId, PhaseId, PhaseOutput};

/// Bead events for inter-bead coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BeadEvent {
    /// A new bead was created.
    Created {
        event_id: EventId,
        bead_id: BeadId,
        spec: BeadSpec,
        timestamp: DateTime<Utc>,
    },
    /// Bead state changed.
    StateChanged {
        event_id: EventId,
        bead_id: BeadId,
        from: BeadState,
        to: BeadState,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// A phase completed within a bead.
    PhaseCompleted {
        event_id: EventId,
        bead_id: BeadId,
        phase_id: PhaseId,
        phase_name: String,
        output: PhaseOutput,
        timestamp: DateTime<Utc>,
    },
    /// A dependency was resolved.
    DependencyResolved {
        event_id: EventId,
        bead_id: BeadId,
        dependency_id: BeadId,
        timestamp: DateTime<Utc>,
    },
    /// Bead execution failed.
    Failed {
        event_id: EventId,
        bead_id: BeadId,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Bead completed (terminal state).
    Completed {
        event_id: EventId,
        bead_id: BeadId,
        result: BeadResult,
        timestamp: DateTime<Utc>,
    },
    /// Bead was claimed by an agent.
    Claimed {
        event_id: EventId,
        bead_id: BeadId,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Bead was unclaimed (released).
    Unclaimed {
        event_id: EventId,
        bead_id: BeadId,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Bead priority changed.
    PriorityChanged {
        event_id: EventId,
        bead_id: BeadId,
        old_priority: u32,
        new_priority: u32,
        timestamp: DateTime<Utc>,
    },
    /// Metadata updated.
    MetadataUpdated {
        event_id: EventId,
        bead_id: BeadId,
        metadata: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// Worker health check failed.
    WorkerUnhealthy {
        event_id: EventId,
        worker_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

impl BeadEvent {
    /// Create a new Created event.
    pub fn created(bead_id: BeadId, spec: BeadSpec) -> Self {
        Self::Created {
            event_id: EventId::new(),
            bead_id,
            spec,
            timestamp: Utc::now(),
        }
    }

    /// Create a new StateChanged event.
    pub fn state_changed(bead_id: BeadId, from: BeadState, to: BeadState) -> Self {
        Self::StateChanged {
            event_id: EventId::new(),
            bead_id,
            from,
            to,
            reason: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a StateChanged event with a reason.
    pub fn state_changed_with_reason(
        bead_id: BeadId,
        from: BeadState,
        to: BeadState,
        reason: impl Into<String>,
    ) -> Self {
        Self::StateChanged {
            event_id: EventId::new(),
            bead_id,
            from,
            to,
            reason: Some(reason.into()),
            timestamp: Utc::now(),
        }
    }

    /// Create a new PhaseCompleted event.
    pub fn phase_completed(
        bead_id: BeadId,
        phase_id: PhaseId,
        phase_name: impl Into<String>,
        output: PhaseOutput,
    ) -> Self {
        Self::PhaseCompleted {
            event_id: EventId::new(),
            bead_id,
            phase_id,
            phase_name: phase_name.into(),
            output,
            timestamp: Utc::now(),
        }
    }

    /// Create a new DependencyResolved event.
    pub fn dependency_resolved(bead_id: BeadId, dependency_id: BeadId) -> Self {
        Self::DependencyResolved {
            event_id: EventId::new(),
            bead_id,
            dependency_id,
            timestamp: Utc::now(),
        }
    }

    /// Create a new Failed event.
    pub fn failed(bead_id: BeadId, error: impl Into<String>) -> Self {
        Self::Failed {
            event_id: EventId::new(),
            bead_id,
            error: error.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a new Completed event.
    pub fn completed(bead_id: BeadId, result: BeadResult) -> Self {
        Self::Completed {
            event_id: EventId::new(),
            bead_id,
            result,
            timestamp: Utc::now(),
        }
    }

    /// Create a new Claimed event.
    pub fn claimed(bead_id: BeadId, agent_id: impl Into<String>) -> Self {
        Self::Claimed {
            event_id: EventId::new(),
            bead_id,
            agent_id: agent_id.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a new Unclaimed event.
    pub fn unclaimed(bead_id: BeadId, reason: Option<String>) -> Self {
        Self::Unclaimed {
            event_id: EventId::new(),
            bead_id,
            reason,
            timestamp: Utc::now(),
        }
    }

    /// Create a new PriorityChanged event.
    pub fn priority_changed(bead_id: BeadId, old_priority: u32, new_priority: u32) -> Self {
        Self::PriorityChanged {
            event_id: EventId::new(),
            bead_id,
            old_priority,
            new_priority,
            timestamp: Utc::now(),
        }
    }

    /// Create a new MetadataUpdated event.
    pub fn metadata_updated(bead_id: BeadId, metadata: serde_json::Value) -> Self {
        Self::MetadataUpdated {
            event_id: EventId::new(),
            bead_id,
            metadata,
            timestamp: Utc::now(),
        }
    }

    /// Create a new WorkerUnhealthy event.
    pub fn worker_unhealthy(worker_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::WorkerUnhealthy {
            event_id: EventId::new(),
            worker_id: worker_id.into(),
            reason: reason.into(),
            timestamp: Utc::now(),
        }
    }

    /// Get the event ID.
    pub fn event_id(&self) -> EventId {
        match self {
            Self::Created { event_id, .. }
            | Self::StateChanged { event_id, .. }
            | Self::PhaseCompleted { event_id, .. }
            | Self::DependencyResolved { event_id, .. }
            | Self::Failed { event_id, .. }
            | Self::Completed { event_id, .. }
            | Self::Claimed { event_id, .. }
            | Self::Unclaimed { event_id, .. }
            | Self::PriorityChanged { event_id, .. }
            | Self::MetadataUpdated { event_id, .. }
            | Self::WorkerUnhealthy { event_id, .. } => *event_id,
        }
    }

    /// Get the bead ID.
    pub fn bead_id(&self) -> BeadId {
        match self {
            Self::Created { bead_id, .. }
            | Self::StateChanged { bead_id, .. }
            | Self::PhaseCompleted { bead_id, .. }
            | Self::DependencyResolved { bead_id, .. }
            | Self::Failed { bead_id, .. }
            | Self::Completed { bead_id, .. }
            | Self::Claimed { bead_id, .. }
            | Self::Unclaimed { bead_id, .. }
            | Self::PriorityChanged { bead_id, .. }
            | Self::MetadataUpdated { bead_id, .. } => *bead_id,
        }
    }

    /// Get the timestamp.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Created { timestamp, .. }
            | Self::StateChanged { timestamp, .. }
            | Self::PhaseCompleted { timestamp, .. }
            | Self::DependencyResolved { timestamp, .. }
            | Self::Failed { timestamp, .. }
            | Self::Completed { timestamp, .. }
            | Self::Claimed { timestamp, .. }
            | Self::Unclaimed { timestamp, .. }
            | Self::PriorityChanged { timestamp, .. }
            | Self::MetadataUpdated { timestamp, .. } => *timestamp,
        }
    }

    /// Get the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created { .. } => "created",
            Self::StateChanged { .. } => "state_changed",
            Self::PhaseCompleted { .. } => "phase_completed",
            Self::DependencyResolved { .. } => "dependency_resolved",
            Self::Failed { .. } => "failed",
            Self::Completed { .. } => "completed",
            Self::Claimed { .. } => "claimed",
            Self::Unclaimed { .. } => "unclaimed",
            Self::PriorityChanged { .. } => "priority_changed",
            Self::MetadataUpdated { .. } => "metadata_updated",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Complexity;

    #[test]
    fn test_created_event() {
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        let event = BeadEvent::created(bead_id, spec);

        assert_eq!(event.bead_id(), bead_id);
        assert_eq!(event.event_type(), "created");
    }

    #[test]
    fn test_state_changed_event() {
        let bead_id = BeadId::new();
        let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);

        assert_eq!(event.bead_id(), bead_id);
        assert_eq!(event.event_type(), "state_changed");
    }

    #[test]
    fn test_completed_event() {
        let bead_id = BeadId::new();
        let result = BeadResult::success(vec![1, 2, 3], 1000);
        let event = BeadEvent::completed(bead_id, result);

        assert_eq!(event.bead_id(), bead_id);
        assert_eq!(event.event_type(), "completed");
    }
}
