#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{BeadId, EvidenceEnvelope, EvidenceKind, RunId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunState {
    run_id: RunId,
    bead_id: BeadId,
    phase: RunPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    Planned,
    Started,
    PromptRecorded,
    AgentRequested,
    AgentRan,
    GateRunning,
    Repairing,
    Completed,
    Blocked,
    RepairBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEvent {
    RunStarted,
    PromptRecorded,
    AgentRequested,
    AgentSucceeded,
    AgentFailed,
    GateStarted,
    GatePassed,
    GateFailed,
    FindingRecorded,
    RepairRequested,
    RepairAttempted,
    RepairBlocked,
    VcsSyncFailed,
    DiffValidationFailed,
    PullRequestCreated,
    PullRequestFailed,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RunStateTransitionError {
    #[error("run state evidence run id mismatch: expected {expected}, got {actual}")]
    RunIdMismatch { expected: String, actual: String },
    #[error("run state evidence bead id mismatch: expected {expected}, got {actual}")]
    BeadIdMismatch { expected: String, actual: String },
    #[error("run state evidence '{record_id}' is missing metadata '{key}'")]
    MissingMetadata { record_id: String, key: &'static str },
    #[error("run state evidence '{record_id}' has unsupported status '{status}'")]
    InvalidStatus { record_id: String, status: String },
    #[error("invalid run state transition from {from} with {event}")]
    InvalidTransition { from: RunPhase, event: RunEvent },
}

impl RunState {
    #[must_use]
    pub fn planned(run_id: RunId, bead_id: BeadId) -> Self {
        Self { run_id, bead_id, phase: RunPhase::Planned }
    }

    #[must_use]
    pub fn run_id(&self) -> &RunId {
        &self.run_id
    }

    #[must_use]
    pub fn bead_id(&self) -> &BeadId {
        &self.bead_id
    }

    #[must_use]
    pub fn phase(&self) -> RunPhase {
        self.phase
    }

    /// Applies a typed run event to this state.
    ///
    /// # Errors
    /// Returns `RunStateTransitionError::InvalidTransition` when the event is
    /// not legal for the current phase.
    pub fn apply_event(&self, event: RunEvent) -> Result<Self, RunStateTransitionError> {
        transition_phase(self.phase, event).map(|phase| Self {
            run_id: self.run_id.clone(),
            bead_id: self.bead_id.clone(),
            phase,
        })
    }

    /// Applies one evidence record after validating it belongs to this run.
    ///
    /// # Errors
    /// Returns a typed transition error when the evidence belongs to another
    /// run/bead, lacks required status metadata, or is invalid for the phase.
    pub fn apply_evidence(
        &self,
        envelope: &EvidenceEnvelope,
    ) -> Result<Self, RunStateTransitionError> {
        self.ensure_evidence_matches(envelope)?;
        self.apply_event(event_from_evidence(envelope)?)
    }

    /// Applies evidence records in order to this state.
    ///
    /// # Errors
    /// Returns the first typed transition error encountered while replaying the
    /// chain.
    pub fn apply_evidence_chain(
        &self,
        evidence: &[EvidenceEnvelope],
    ) -> Result<Self, RunStateTransitionError> {
        evidence.iter().try_fold(self.clone(), |state, envelope| state.apply_evidence(envelope))
    }

    fn ensure_evidence_matches(
        &self,
        envelope: &EvidenceEnvelope,
    ) -> Result<(), RunStateTransitionError> {
        if envelope.run_id != self.run_id {
            return Err(RunStateTransitionError::RunIdMismatch {
                expected: self.run_id.as_str().to_owned(),
                actual: envelope.run_id.as_str().to_owned(),
            });
        }
        if envelope.bead_id != self.bead_id {
            return Err(RunStateTransitionError::BeadIdMismatch {
                expected: self.bead_id.as_str().to_owned(),
                actual: envelope.bead_id.as_str().to_owned(),
            });
        }
        Ok(())
    }
}

impl RunPhase {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Started => "started",
            Self::PromptRecorded => "prompt_recorded",
            Self::AgentRequested => "agent_requested",
            Self::AgentRan => "agent_ran",
            Self::GateRunning => "gate_running",
            Self::Repairing => "repairing",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::RepairBlocked => "repair_blocked",
        }
    }

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::RepairBlocked)
    }
}

impl Display for RunPhase {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl RunEvent {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RunStarted => "run_started",
            Self::PromptRecorded => "prompt_recorded",
            Self::AgentRequested => "agent_requested",
            Self::AgentSucceeded => "agent_succeeded",
            Self::AgentFailed => "agent_failed",
            Self::GateStarted => "gate_started",
            Self::GatePassed => "gate_passed",
            Self::GateFailed => "gate_failed",
            Self::FindingRecorded => "finding_recorded",
            Self::RepairRequested => "repair_requested",
            Self::RepairAttempted => "repair_attempted",
            Self::RepairBlocked => "repair_blocked",
            Self::VcsSyncFailed => "vcs_sync_failed",
            Self::DiffValidationFailed => "diff_validation_failed",
            Self::PullRequestCreated => "pull_request_created",
            Self::PullRequestFailed => "pull_request_failed",
        }
    }
}

impl Display for RunEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn transition_phase(phase: RunPhase, event: RunEvent) -> Result<RunPhase, RunStateTransitionError> {
    transition_target(phase, event)
        .ok_or(RunStateTransitionError::InvalidTransition { from: phase, event })
}

fn transition_target(phase: RunPhase, event: RunEvent) -> Option<RunPhase> {
    match event {
        RunEvent::RunStarted => next_if(phase == RunPhase::Planned, RunPhase::Started),
        RunEvent::PromptRecorded => next_if(phase == RunPhase::Started, RunPhase::PromptRecorded),
        RunEvent::AgentRequested => {
            next_if(phase == RunPhase::PromptRecorded, RunPhase::AgentRequested)
        }
        RunEvent::AgentSucceeded => next_if(phase == RunPhase::AgentRequested, RunPhase::AgentRan),
        RunEvent::AgentFailed => next_if(phase == RunPhase::AgentRequested, RunPhase::Blocked),
        RunEvent::GateStarted => next_if(gate_can_start(phase), RunPhase::GateRunning),
        RunEvent::GatePassed => next_if(phase == RunPhase::GateRunning, RunPhase::Completed),
        RunEvent::GateFailed => next_if(phase == RunPhase::GateRunning, RunPhase::Blocked),
        RunEvent::FindingRecorded => next_if(phase == RunPhase::Blocked, RunPhase::Blocked),
        RunEvent::RepairRequested => next_if(phase == RunPhase::Blocked, RunPhase::Repairing),
        RunEvent::RepairAttempted => next_if(phase == RunPhase::Repairing, RunPhase::Repairing),
        RunEvent::RepairBlocked => next_if(repair_can_block(phase), RunPhase::RepairBlocked),
        RunEvent::VcsSyncFailed => next_if(vcs_sync_can_block(phase), RunPhase::Blocked),
        RunEvent::DiffValidationFailed => {
            next_if(diff_validation_can_block(phase), RunPhase::Blocked)
        }
        RunEvent::PullRequestCreated => next_if(phase == RunPhase::Completed, RunPhase::Completed),
        RunEvent::PullRequestFailed => next_if(phase == RunPhase::Completed, RunPhase::Blocked),
    }
}

fn gate_can_start(phase: RunPhase) -> bool {
    matches!(phase, RunPhase::AgentRan | RunPhase::Repairing)
}

fn repair_can_block(phase: RunPhase) -> bool {
    matches!(phase, RunPhase::Blocked | RunPhase::Repairing)
}

fn vcs_sync_can_block(phase: RunPhase) -> bool {
    matches!(phase, RunPhase::Planned | RunPhase::Started)
}

fn diff_validation_can_block(phase: RunPhase) -> bool {
    matches!(
        phase,
        RunPhase::Planned
            | RunPhase::Started
            | RunPhase::PromptRecorded
            | RunPhase::AgentRequested
            | RunPhase::AgentRan
            | RunPhase::GateRunning
            | RunPhase::Completed
            | RunPhase::Blocked
    )
}

fn next_if(allowed: bool, phase: RunPhase) -> Option<RunPhase> {
    allowed.then_some(phase)
}

fn event_from_evidence(envelope: &EvidenceEnvelope) -> Result<RunEvent, RunStateTransitionError> {
    match envelope.kind {
        EvidenceKind::RunStarted => Ok(RunEvent::RunStarted),
        EvidenceKind::PromptRecord => Ok(RunEvent::PromptRecorded),
        EvidenceKind::AgentRequest => Ok(RunEvent::AgentRequested),
        EvidenceKind::AgentRun => agent_run_event(envelope),
        EvidenceKind::GateRunStarted => Ok(RunEvent::GateStarted),
        EvidenceKind::GateRunFinished => gate_finished_event(envelope),
        EvidenceKind::Finding => Ok(RunEvent::FindingRecorded),
        EvidenceKind::RepairRequest => Ok(RunEvent::RepairRequested),
        EvidenceKind::RepairAttempt => Ok(RunEvent::RepairAttempted),
        EvidenceKind::RepairBlocked => Ok(RunEvent::RepairBlocked),
        EvidenceKind::VcsSyncFailed => Ok(RunEvent::VcsSyncFailed),
        EvidenceKind::DiffValidationFailed => Ok(RunEvent::DiffValidationFailed),
        EvidenceKind::PullRequestCreated => Ok(RunEvent::PullRequestCreated),
        EvidenceKind::PullRequestFailed => Ok(RunEvent::PullRequestFailed),
    }
}

fn agent_run_event(envelope: &EvidenceEnvelope) -> Result<RunEvent, RunStateTransitionError> {
    match required_status(envelope)?.as_str() {
        "succeeded" => Ok(RunEvent::AgentSucceeded),
        "failed" => Ok(RunEvent::AgentFailed),
        status => invalid_status(envelope, status),
    }
}

fn gate_finished_event(envelope: &EvidenceEnvelope) -> Result<RunEvent, RunStateTransitionError> {
    match required_status(envelope)?.as_str() {
        "passed" => Ok(RunEvent::GatePassed),
        "failed" => Ok(RunEvent::GateFailed),
        status => invalid_status(envelope, status),
    }
}

fn required_status(envelope: &EvidenceEnvelope) -> Result<String, RunStateTransitionError> {
    envelope.metadata.get("status").cloned().ok_or_else(|| {
        RunStateTransitionError::MissingMetadata {
            record_id: envelope.record_id.as_str().to_owned(),
            key: "status",
        }
    })
}

fn invalid_status<T>(
    envelope: &EvidenceEnvelope,
    status: &str,
) -> Result<T, RunStateTransitionError> {
    Err(RunStateTransitionError::InvalidStatus {
        record_id: envelope.record_id.as_str().to_owned(),
        status: status.to_owned(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::lifecycle::types::{EvidenceEnvelopeParts, EvidenceMetadata, EvidenceRecordId};

    #[test]
    fn run_state_machine_allows_completed_one_bead_flow() {
        let state = planned_state().apply_evidence_chain(&completed_evidence()).unwrap();

        assert_eq!(state.phase(), RunPhase::Completed);
        assert_eq!(state.run_id().as_str(), "run-demo");
        assert_eq!(state.bead_id().as_str(), "demo");
        assert!(state.phase().is_terminal());
    }

    #[test]
    fn run_state_machine_allows_blocked_one_bead_flow() {
        let state = planned_state().apply_evidence_chain(&blocked_evidence()).unwrap();

        assert_eq!(state.phase(), RunPhase::Blocked);
        assert!(!state.phase().is_terminal());
    }

    #[test]
    fn run_state_machine_allows_repair_blocked_terminal_flow() {
        let state = planned_state().apply_evidence_chain(&repair_blocked_evidence()).unwrap();

        assert_eq!(state.phase(), RunPhase::RepairBlocked);
        assert!(state.phase().is_terminal());
    }

    #[test]
    fn run_state_machine_rejects_invalid_transition_with_typed_error() {
        let result = planned_state().apply_evidence(&gate_finished("passed", 1, None));

        assert!(matches!(
            result,
            Err(RunStateTransitionError::InvalidTransition {
                from: RunPhase::Planned,
                event: RunEvent::GatePassed
            })
        ));
    }

    #[test]
    fn run_state_machine_rejects_event_after_terminal_completion() {
        let completed = planned_state().apply_evidence_chain(&completed_evidence()).unwrap();

        let result = completed.apply_event(RunEvent::RepairRequested);

        assert!(matches!(
            result,
            Err(RunStateTransitionError::InvalidTransition {
                from: RunPhase::Completed,
                event: RunEvent::RepairRequested
            })
        ));
    }

    #[test]
    fn run_state_machine_rejects_event_after_terminal_repair_blocked() {
        let blocked = planned_state().apply_evidence_chain(&repair_blocked_evidence()).unwrap();

        let result = blocked.apply_event(RunEvent::GateStarted);

        assert!(matches!(
            result,
            Err(RunStateTransitionError::InvalidTransition {
                from: RunPhase::RepairBlocked,
                event: RunEvent::GateStarted
            })
        ));
    }

    #[test]
    fn run_state_machine_rejects_foreign_evidence_with_typed_error() {
        let result = planned_state().apply_evidence(&foreign_run_started());

        assert!(matches!(
            result,
            Err(RunStateTransitionError::RunIdMismatch { expected, actual })
                if expected == "run-demo" && actual == "run-other"
        ));
    }

    #[test]
    fn run_state_machine_rejects_foreign_bead_with_typed_error() {
        let result = planned_state().apply_evidence(&foreign_bead_run_started());

        assert!(matches!(
            result,
            Err(RunStateTransitionError::BeadIdMismatch { expected, actual })
                if expected == "demo" && actual == "other"
        ));
    }

    #[test]
    fn run_state_machine_rejects_missing_status_metadata() {
        let state = planned_state().apply_evidence_chain(&evidence_before_agent_run()).unwrap();

        let result = state.apply_evidence(&simple_evidence(
            EvidenceKind::AgentRun,
            4,
            Some(evidence_before_agent_run().last().unwrap().checksum.clone()),
        ));

        assert!(matches!(
            result,
            Err(RunStateTransitionError::MissingMetadata { key: "status", .. })
        ));
    }

    #[test]
    fn run_state_machine_rejects_invalid_status_metadata() {
        let state = planned_state().apply_evidence_chain(&evidence_before_agent_run()).unwrap();
        let previous_checksum = evidence_before_agent_run().last().unwrap().checksum.clone();

        let result = state.apply_evidence(&status_evidence(
            EvidenceKind::AgentRun,
            "maybe",
            4,
            Some(previous_checksum),
        ));

        assert!(matches!(
            result,
            Err(RunStateTransitionError::InvalidStatus { status, .. }) if status == "maybe"
        ));
    }

    fn completed_evidence() -> Vec<EvidenceEnvelope> {
        let run_started = simple_evidence(EvidenceKind::RunStarted, 1, None);
        let prompt =
            simple_evidence(EvidenceKind::PromptRecord, 2, Some(run_started.checksum.clone()));
        let request = simple_evidence(EvidenceKind::AgentRequest, 3, Some(prompt.checksum.clone()));
        let agent =
            status_evidence(EvidenceKind::AgentRun, "succeeded", 4, Some(request.checksum.clone()));
        let gate_started =
            simple_evidence(EvidenceKind::GateRunStarted, 5, Some(agent.checksum.clone()));
        let gate_finished = status_evidence(
            EvidenceKind::GateRunFinished,
            "passed",
            6,
            Some(gate_started.checksum.clone()),
        );

        vec![run_started, prompt, request, agent, gate_started, gate_finished]
    }

    fn blocked_evidence() -> Vec<EvidenceEnvelope> {
        let run_started = simple_evidence(EvidenceKind::RunStarted, 1, None);
        let prompt =
            simple_evidence(EvidenceKind::PromptRecord, 2, Some(run_started.checksum.clone()));
        let request = simple_evidence(EvidenceKind::AgentRequest, 3, Some(prompt.checksum.clone()));
        let agent =
            status_evidence(EvidenceKind::AgentRun, "failed", 4, Some(request.checksum.clone()));

        vec![run_started, prompt, request, agent]
    }

    fn repair_blocked_evidence() -> Vec<EvidenceEnvelope> {
        let mut evidence = blocked_evidence();
        let finding = simple_evidence(
            EvidenceKind::Finding,
            5,
            evidence.last().map(|envelope| envelope.checksum.clone()),
        );
        let request =
            simple_evidence(EvidenceKind::RepairRequest, 6, Some(finding.checksum.clone()));
        let repair_blocked =
            simple_evidence(EvidenceKind::RepairBlocked, 7, Some(request.checksum.clone()));
        evidence.push(finding);
        evidence.push(request);
        evidence.push(repair_blocked);
        evidence
    }

    fn evidence_before_agent_run() -> Vec<EvidenceEnvelope> {
        let run_started = simple_evidence(EvidenceKind::RunStarted, 1, None);
        let prompt =
            simple_evidence(EvidenceKind::PromptRecord, 2, Some(run_started.checksum.clone()));
        let request = simple_evidence(EvidenceKind::AgentRequest, 3, Some(prompt.checksum.clone()));

        vec![run_started, prompt, request]
    }

    fn planned_state() -> RunState {
        RunState::planned(run_id(), bead_id())
    }

    fn simple_evidence(
        kind: EvidenceKind,
        offset_seconds: i64,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        evidence(kind, EvidenceMetadata::new(), offset_seconds, previous_checksum, run_id())
    }

    fn status_evidence(
        kind: EvidenceKind,
        status: &str,
        offset_seconds: i64,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        let mut metadata = EvidenceMetadata::new();
        metadata.insert("status".to_owned(), status.to_owned());
        evidence(kind, metadata, offset_seconds, previous_checksum, run_id())
    }

    fn gate_finished(
        status: &str,
        offset_seconds: i64,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
    ) -> EvidenceEnvelope {
        status_evidence(EvidenceKind::GateRunFinished, status, offset_seconds, previous_checksum)
    }

    fn foreign_run_started() -> EvidenceEnvelope {
        evidence(EvidenceKind::RunStarted, EvidenceMetadata::new(), 1, None, foreign_run_id())
    }

    fn foreign_bead_run_started() -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse("ev-other-run-state-machine-1").unwrap(),
            run_id: run_id(),
            bead_id: BeadId::parse("other").unwrap(),
            timestamp: Utc.timestamp_opt(1_779_999_601, 0).unwrap(),
            kind: EvidenceKind::RunStarted,
            metadata: EvidenceMetadata::new(),
            previous_checksum: None,
        })
        .unwrap()
    }

    fn evidence(
        kind: EvidenceKind,
        metadata: EvidenceMetadata,
        offset_seconds: i64,
        previous_checksum: Option<crate::lifecycle::types::EvidenceChecksum>,
        run_id: RunId,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope::new(EvidenceEnvelopeParts {
            record_id: EvidenceRecordId::parse(&format!(
                "ev-demo-run-state-machine-{offset_seconds}"
            ))
            .unwrap(),
            run_id,
            bead_id: bead_id(),
            timestamp: Utc.timestamp_opt(1_779_999_600 + offset_seconds, 0).unwrap(),
            kind,
            metadata,
            previous_checksum,
        })
        .unwrap()
    }

    fn bead_id() -> BeadId {
        BeadId::parse("demo").unwrap()
    }

    fn run_id() -> RunId {
        RunId::parse("run-demo").unwrap()
    }

    fn foreign_run_id() -> RunId {
        RunId::parse("run-other").unwrap()
    }
}
