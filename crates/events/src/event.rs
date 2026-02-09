//! Bead event types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::stage::{Severity, StageKind};
use crate::types::{BeadId, BeadResult, BeadSpec, BeadState, EventId, PhaseId, PhaseOutput};

/// Serialization error type.
pub type SerializationResult<T> = std::result::Result<T, SerializationError>;

/// Error during bincode serialization/deserialization.
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    /// Bincode serialization error.
    #[error("bincode serialization error: {0}")]
    BincodeSerialize(String),

    /// Bincode deserialization error.
    #[error("bincode deserialization error: {0}")]
    BincodeDeserialize(String),

    /// Serialized data exceeds maximum size.
    #[error("serialized size {0} bytes exceeds maximum {1} bytes")]
    SizeExceeded(usize, usize),
}

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
    /// A stage started for this bead.
    StageStarted {
        event_id: EventId,
        bead_id: BeadId,
        stage: StageKind,
        attempt: u32,
        timestamp: DateTime<Utc>,
    },
    /// A stage completed for this bead.
    StageCompleted {
        event_id: EventId,
        bead_id: BeadId,
        stage: StageKind,
        artifact_ref: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// A stage failed for this bead.
    StageFailed {
        event_id: EventId,
        bead_id: BeadId,
        stage: StageKind,
        feedback: String,
        severity: Severity,
        timestamp: DateTime<Utc>,
    },
    /// A bead reentered an earlier stage.
    StageReentry {
        event_id: EventId,
        bead_id: BeadId,
        from_stage: StageKind,
        to_stage: StageKind,
        reason: String,
        attempt: u32,
        timestamp: DateTime<Utc>,
    },
    /// Validation command execution result.
    ValidationRan {
        event_id: EventId,
        bead_id: BeadId,
        passed: bool,
        output: String,
        command: String,
        exit_code: i32,
        timestamp: DateTime<Utc>,
    },
    /// Recursion limits were exhausted.
    RecursionExhausted {
        event_id: EventId,
        bead_id: BeadId,
        total_attempts: u32,
        last_stage: StageKind,
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

    /// Create a new StageStarted event.
    pub fn stage_started(bead_id: BeadId, stage: StageKind, attempt: u32) -> Self {
        Self::StageStarted {
            event_id: EventId::new(),
            bead_id,
            stage,
            attempt,
            timestamp: Utc::now(),
        }
    }

    /// Create a new StageCompleted event.
    pub fn stage_completed(
        bead_id: BeadId,
        stage: StageKind,
        artifact_ref: Option<String>,
    ) -> Self {
        Self::StageCompleted {
            event_id: EventId::new(),
            bead_id,
            stage,
            artifact_ref,
            timestamp: Utc::now(),
        }
    }

    /// Create a new StageFailed event.
    pub fn stage_failed(
        bead_id: BeadId,
        stage: StageKind,
        feedback: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self::StageFailed {
            event_id: EventId::new(),
            bead_id,
            stage,
            feedback: feedback.into(),
            severity,
            timestamp: Utc::now(),
        }
    }

    /// Create a new StageReentry event.
    pub fn stage_reentry(
        bead_id: BeadId,
        from_stage: StageKind,
        to_stage: StageKind,
        reason: impl Into<String>,
        attempt: u32,
    ) -> Self {
        Self::StageReentry {
            event_id: EventId::new(),
            bead_id,
            from_stage,
            to_stage,
            reason: reason.into(),
            attempt,
            timestamp: Utc::now(),
        }
    }

    /// Create a new ValidationRan event.
    pub fn validation_ran(
        bead_id: BeadId,
        passed: bool,
        output: impl Into<String>,
        command: impl Into<String>,
        exit_code: i32,
    ) -> Self {
        Self::ValidationRan {
            event_id: EventId::new(),
            bead_id,
            passed,
            output: output.into(),
            command: command.into(),
            exit_code,
            timestamp: Utc::now(),
        }
    }

    /// Create a new RecursionExhausted event.
    pub fn recursion_exhausted(
        bead_id: BeadId,
        total_attempts: u32,
        last_stage: StageKind,
    ) -> Self {
        Self::RecursionExhausted {
            event_id: EventId::new(),
            bead_id,
            total_attempts,
            last_stage,
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
            | Self::StageStarted { event_id, .. }
            | Self::StageCompleted { event_id, .. }
            | Self::StageFailed { event_id, .. }
            | Self::StageReentry { event_id, .. }
            | Self::ValidationRan { event_id, .. }
            | Self::RecursionExhausted { event_id, .. }
            | Self::WorkerUnhealthy { event_id, .. } => *event_id,
        }
    }

    /// Get the bead ID.
    ///
    /// Returns a default BeadId for events without a bead_id field (e.g., WorkerUnhealthy).
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
            | Self::MetadataUpdated { bead_id, .. }
            | Self::StageStarted { bead_id, .. }
            | Self::StageCompleted { bead_id, .. }
            | Self::StageFailed { bead_id, .. }
            | Self::StageReentry { bead_id, .. }
            | Self::ValidationRan { bead_id, .. }
            | Self::RecursionExhausted { bead_id, .. } => *bead_id,
            Self::WorkerUnhealthy { .. } => BeadId::default(),
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
            | Self::MetadataUpdated { timestamp, .. }
            | Self::StageStarted { timestamp, .. }
            | Self::StageCompleted { timestamp, .. }
            | Self::StageFailed { timestamp, .. }
            | Self::StageReentry { timestamp, .. }
            | Self::ValidationRan { timestamp, .. }
            | Self::RecursionExhausted { timestamp, .. }
            | Self::WorkerUnhealthy { timestamp, .. } => *timestamp,
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
            Self::StageStarted { .. } => "stage_started",
            Self::StageCompleted { .. } => "stage_completed",
            Self::StageFailed { .. } => "stage_failed",
            Self::StageReentry { .. } => "stage_reentry",
            Self::ValidationRan { .. } => "validation_ran",
            Self::RecursionExhausted { .. } => "recursion_exhausted",
            Self::WorkerUnhealthy { .. } => "worker_unhealthy",
        }
    }

    /// Serialize event to bincode binary format.
    ///
    /// Uses compact binary representation for efficient WebSocket transmission.
    /// Ensures serialized size < 1KB as per system constraints.
    ///
    /// # Errors
    ///
    /// Returns `SerializationError` if:
    /// - Bincode serialization fails
    /// - Serialized size exceeds 1KB (1024 bytes)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let event = BeadEvent::completed(bead_id, result);
    /// let bytes = event.to_bincode()?;
    /// assert!(bytes.len() < 1024);
    /// ```
    pub fn to_bincode(&self) -> SerializationResult<Vec<u8>> {
        const MAX_SIZE: usize = 1024;

        bincode::serialize(self)
            .map_err(|e| SerializationError::BincodeSerialize(e.to_string()))
            .and_then(|bytes: Vec<u8>| {
                if bytes.len() > MAX_SIZE {
                    Err(SerializationError::SizeExceeded(bytes.len(), MAX_SIZE))
                } else {
                    Ok(bytes)
                }
            })
    }

    /// Deserialize event from bincode binary format.
    ///
    /// # Errors
    ///
    /// Returns `SerializationError` if deserialization fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let event = BeadEvent::from_bincode(&bytes)?;
    /// ```
    pub fn from_bincode(bytes: &[u8]) -> SerializationResult<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| SerializationError::BincodeDeserialize(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::EventPattern;
    use crate::stage::{Severity, StageKind};
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

    #[test]
    fn test_worker_unhealthy_event() {
        let event = BeadEvent::worker_unhealthy("worker-123", "health check timeout");

        assert_eq!(event.event_type(), "worker_unhealthy");
        match event {
            BeadEvent::WorkerUnhealthy {
                worker_id, reason, ..
            } => {
                assert_eq!(worker_id, "worker-123");
                assert_eq!(reason, "health check timeout");
            }
            _ => unreachable!("worker_unhealthy should produce WorkerUnhealthy event"),
        }
    }

    #[test]
    fn test_stage_started_constructor() {
        let bead_id = BeadId::new();
        let event = BeadEvent::stage_started(bead_id, StageKind::Implement, 2);

        assert_eq!(event.event_type(), "stage_started");
        match event {
            BeadEvent::StageStarted {
                bead_id: id,
                stage,
                attempt,
                ..
            } => {
                assert_eq!(id, bead_id);
                assert_eq!(stage, StageKind::Implement);
                assert_eq!(attempt, 2);
            }
            _ => unreachable!("stage_started should produce StageStarted event"),
        }
    }

    #[test]
    fn test_stage_completed_constructor() {
        let bead_id = BeadId::new();
        let event = BeadEvent::stage_completed(
            bead_id,
            StageKind::Review,
            Some("artifacts/review.txt".to_string()),
        );

        match event {
            BeadEvent::StageCompleted {
                bead_id: id,
                stage,
                artifact_ref,
                ..
            } => {
                assert_eq!(id, bead_id);
                assert_eq!(stage, StageKind::Review);
                assert_eq!(artifact_ref, Some("artifacts/review.txt".to_string()));
            }
            _ => unreachable!("stage_completed should produce StageCompleted event"),
        }
    }

    #[test]
    fn test_stage_failed_constructor() {
        let bead_id = BeadId::new();
        let event = BeadEvent::stage_failed(
            bead_id,
            StageKind::Review,
            "needs redesign",
            Severity::Major,
        );

        match event {
            BeadEvent::StageFailed {
                bead_id: id,
                stage,
                feedback,
                severity,
                ..
            } => {
                assert_eq!(id, bead_id);
                assert_eq!(stage, StageKind::Review);
                assert_eq!(feedback, "needs redesign");
                assert_eq!(severity, Severity::Major);
            }
            _ => unreachable!("stage_failed should produce StageFailed event"),
        }
    }

    #[test]
    fn test_stage_reentry_constructor() {
        let bead_id = BeadId::new();
        let event = BeadEvent::stage_reentry(
            bead_id,
            StageKind::Review,
            StageKind::Plan,
            "major issues",
            3,
        );

        match event {
            BeadEvent::StageReentry {
                bead_id: id,
                from_stage,
                to_stage,
                reason,
                attempt,
                ..
            } => {
                assert_eq!(id, bead_id);
                assert_eq!(from_stage, StageKind::Review);
                assert_eq!(to_stage, StageKind::Plan);
                assert_eq!(reason, "major issues");
                assert_eq!(attempt, 3);
            }
            _ => unreachable!("stage_reentry should produce StageReentry event"),
        }
    }

    #[test]
    fn test_validation_ran_constructor_pass() {
        let bead_id = BeadId::new();
        let event = BeadEvent::validation_ran(bead_id, true, "ok", "moon run :ci", 0);

        match event {
            BeadEvent::ValidationRan {
                bead_id: id,
                passed,
                output,
                command,
                exit_code,
                ..
            } => {
                assert_eq!(id, bead_id);
                assert!(passed);
                assert_eq!(output, "ok");
                assert_eq!(command, "moon run :ci");
                assert_eq!(exit_code, 0);
            }
            _ => unreachable!("validation_ran should produce ValidationRan event"),
        }
    }

    #[test]
    fn test_validation_ran_constructor_fail() {
        let bead_id = BeadId::new();
        let event = BeadEvent::validation_ran(bead_id, false, "lint failure", "moon run :ci", 1);

        match event {
            BeadEvent::ValidationRan { passed, output, .. } => {
                assert!(!passed);
                assert_eq!(output, "lint failure");
            }
            _ => unreachable!("validation_ran should produce ValidationRan event"),
        }
    }

    #[test]
    fn test_recursion_exhausted_constructor() {
        let bead_id = BeadId::new();
        let event = BeadEvent::recursion_exhausted(bead_id, 15, StageKind::Review);

        match event {
            BeadEvent::RecursionExhausted {
                bead_id: id,
                total_attempts,
                last_stage,
                ..
            } => {
                assert_eq!(id, bead_id);
                assert_eq!(total_attempts, 15);
                assert_eq!(last_stage, StageKind::Review);
            }
            _ => unreachable!("recursion_exhausted should produce RecursionExhausted event"),
        }
    }

    #[test]
    fn test_stage_started_bincode_roundtrip() -> SerializationResult<()> {
        let event = BeadEvent::stage_started(BeadId::new(), StageKind::Plan, 1);
        let bytes = event.to_bincode()?;
        let decoded = BeadEvent::from_bincode(&bytes)?;
        assert_eq!(decoded.event_type(), "stage_started");
        Ok(())
    }

    #[test]
    fn test_stage_failed_bincode_roundtrip() -> SerializationResult<()> {
        let event = BeadEvent::stage_failed(
            BeadId::new(),
            StageKind::Review,
            "fundamental issue",
            Severity::Fundamental,
        );
        let bytes = event.to_bincode()?;
        let decoded = BeadEvent::from_bincode(&bytes)?;
        match decoded {
            BeadEvent::StageFailed { severity, .. } => {
                assert_eq!(severity, Severity::Fundamental);
            }
            _ => unreachable!("decoded event should be StageFailed"),
        }
        Ok(())
    }

    #[test]
    fn test_stage_reentry_bincode_roundtrip() -> SerializationResult<()> {
        let event = BeadEvent::stage_reentry(
            BeadId::new(),
            StageKind::Validate,
            StageKind::Implement,
            "ci regression",
            4,
        );
        let bytes = event.to_bincode()?;
        let decoded = BeadEvent::from_bincode(&bytes)?;
        match decoded {
            BeadEvent::StageReentry {
                from_stage,
                to_stage,
                ..
            } => {
                assert_eq!(from_stage, StageKind::Validate);
                assert_eq!(to_stage, StageKind::Implement);
            }
            _ => unreachable!("decoded event should be StageReentry"),
        }
        Ok(())
    }

    #[test]
    fn test_validation_ran_bincode_roundtrip() -> SerializationResult<()> {
        let event =
            BeadEvent::validation_ran(BeadId::new(), false, "tests failed", "moon run :ci", 1);
        let bytes = event.to_bincode()?;
        let decoded = BeadEvent::from_bincode(&bytes)?;
        match decoded {
            BeadEvent::ValidationRan { output, .. } => {
                assert_eq!(output, "tests failed");
            }
            _ => unreachable!("decoded event should be ValidationRan"),
        }
        Ok(())
    }

    #[test]
    fn test_stage_events_match_by_bead_pattern() {
        let bead_id = BeadId::new();
        let pattern = EventPattern::ByBead(bead_id);
        let event = BeadEvent::stage_started(bead_id, StageKind::Research, 1);
        assert!(pattern.matches(&event));
    }
}
