//! Intra-bead stage machine for recursive orchestration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::BeadId;

/// Stage in the recursive intra-bead lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    /// Analyze requirements and constraints.
    Research,
    /// Produce a concrete implementation plan.
    Plan,
    /// Implement the plan.
    Implement,
    /// Review implementation quality and correctness.
    Review,
    /// Run validation checks.
    Validate,
    /// Terminal acceptance stage.
    Accept,
}

impl StageKind {
    /// Return the next forward stage, if any.
    pub fn next(self) -> Option<StageKind> {
        match self {
            Self::Research => Some(Self::Plan),
            Self::Plan => Some(Self::Implement),
            Self::Implement => Some(Self::Review),
            Self::Review => Some(Self::Validate),
            Self::Validate => Some(Self::Accept),
            Self::Accept => None,
        }
    }

    /// Return true when this stage requires an agent process.
    pub fn requires_agent(self) -> bool {
        matches!(
            self,
            Self::Research | Self::Plan | Self::Implement | Self::Review
        )
    }

    /// Return true if this stage is terminal.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Accept)
    }

    fn as_index(self) -> usize {
        match self {
            Self::Research => 0,
            Self::Plan => 1,
            Self::Implement => 2,
            Self::Review => 3,
            Self::Validate => 4,
            Self::Accept => 5,
        }
    }
}

/// Severity level of stage feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Small issues requiring implementation tweaks.
    Minor,
    /// Significant issues requiring planning changes.
    Major,
    /// Fundamental misunderstanding requiring research reset.
    Fundamental,
}

/// Why a transition occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionReason {
    /// Normal forward completion.
    Completed,
    /// Gate failure causing reentry.
    GateFailed {
        /// Feedback text from gate.
        feedback: String,
        /// Severity derived by gate.
        severity: Severity,
    },
    /// Stage timed out.
    Timeout,
    /// Human override transition.
    ManualOverride {
        /// Explanation for override.
        reason: String,
    },
}

/// Recorded transition between stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageTransition {
    /// Previous stage.
    pub from: StageKind,
    /// Destination stage.
    pub to: StageKind,
    /// Reason for transition.
    pub reason: TransitionReason,
    /// Transition timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Exhaustion behavior once retry limits are reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExhaustionPolicy {
    /// Fail the bead when limits are exceeded.
    Fail,
    /// Park bead for a human decision.
    ParkForHuman,
}

/// Policy controlling recursion and retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursionPolicy {
    /// Maximum total stage entries across the bead lifecycle.
    pub max_total_attempts: u32,
    /// Maximum retries for non-research stages.
    pub max_stage_retries: u32,
    /// Maximum retries for research stage.
    pub max_research_retries: u32,
    /// What to do when limits are exhausted.
    pub on_exhaustion: ExhaustionPolicy,
}

impl Default for RecursionPolicy {
    fn default() -> Self {
        Self {
            max_total_attempts: 15,
            max_stage_retries: 3,
            max_research_retries: 1,
            on_exhaustion: ExhaustionPolicy::ParkForHuman,
        }
    }
}

/// Errors from state machine operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StateMachineError {
    /// Total attempt budget exhausted.
    #[error("total attempts exhausted")]
    TotalAttemptsExhausted,
    /// Current stage retry budget exhausted.
    #[error("stage retries exhausted")]
    StageRetriesExhausted,
    /// Machine already at terminal stage.
    #[error("state machine is already terminal")]
    AlreadyTerminal,
    /// Invalid reentry target for current stage.
    #[error("invalid reentry target")]
    InvalidReentry,
}

/// Intra-bead state machine tracking recursive stage execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadStateMachine {
    /// Owning bead ID.
    pub bead_id: BeadId,
    current_stage: StageKind,
    stage_attempts: [u32; 6],
    total_attempts: u32,
    history: Vec<StageTransition>,
    policy: RecursionPolicy,
}

impl BeadStateMachine {
    /// Create a new machine at Research with default policy.
    pub fn new(bead_id: BeadId) -> Self {
        Self {
            bead_id,
            current_stage: StageKind::Research,
            stage_attempts: [0; 6],
            total_attempts: 0,
            history: Vec::new(),
            policy: RecursionPolicy::default(),
        }
    }

    /// Create a new machine at Research with a custom policy.
    pub fn with_policy(bead_id: BeadId, policy: RecursionPolicy) -> Self {
        Self {
            bead_id,
            current_stage: StageKind::Research,
            stage_attempts: [0; 6],
            total_attempts: 0,
            history: Vec::new(),
            policy,
        }
    }

    /// Return current stage.
    pub fn current_stage(&self) -> StageKind {
        self.current_stage
    }

    /// Return configured recursion policy.
    pub fn policy(&self) -> RecursionPolicy {
        self.policy
    }

    /// Enter current stage, incrementing counters after bound checks.
    pub fn enter_stage(&mut self) -> Result<(), StateMachineError> {
        if self.total_attempts >= self.policy.max_total_attempts {
            return Err(StateMachineError::TotalAttemptsExhausted);
        }

        let stage_limit = if self.current_stage == StageKind::Research {
            self.policy.max_research_retries
        } else {
            self.policy.max_stage_retries
        };

        let idx = self.current_stage.as_index();
        if self.stage_attempts[idx] >= stage_limit {
            return Err(StateMachineError::StageRetriesExhausted);
        }

        self.stage_attempts[idx] += 1;
        self.total_attempts += 1;
        Ok(())
    }

    /// Advance forward by one stage.
    pub fn advance(&mut self) -> Result<StageTransition, StateMachineError> {
        let from = self.current_stage;
        let to = from.next().ok_or(StateMachineError::AlreadyTerminal)?;

        self.current_stage = to;
        let transition = StageTransition {
            from,
            to,
            reason: TransitionReason::Completed,
            timestamp: Utc::now(),
        };
        self.history.push(transition.clone());
        Ok(transition)
    }

    /// Reenter a previous stage with gate feedback and severity.
    pub fn reenter(
        &mut self,
        target: StageKind,
        feedback: impl Into<String>,
        severity: Severity,
    ) -> Result<StageTransition, StateMachineError> {
        if self.current_stage.is_terminal() {
            return Err(StateMachineError::AlreadyTerminal);
        }

        if target == self.current_stage || target.as_index() >= self.current_stage.as_index() {
            return Err(StateMachineError::InvalidReentry);
        }

        let from = self.current_stage;
        let to = target;
        self.current_stage = target;

        let transition = StageTransition {
            from,
            to,
            reason: TransitionReason::GateFailed {
                feedback: feedback.into(),
                severity,
            },
            timestamp: Utc::now(),
        };
        self.history.push(transition.clone());
        Ok(transition)
    }

    /// Resolve standard reentry target for a given severity.
    pub fn reentry_target_for_severity(severity: Severity) -> StageKind {
        match severity {
            Severity::Minor => StageKind::Implement,
            Severity::Major => StageKind::Plan,
            Severity::Fundamental => StageKind::Research,
        }
    }

    /// Return true if machine is complete.
    pub fn is_complete(&self) -> bool {
        self.current_stage.is_terminal()
    }

    /// Return transition history.
    pub fn history(&self) -> &[StageTransition] {
        &self.history
    }

    /// Return attempt count for a stage.
    pub fn stage_attempts(&self, stage: StageKind) -> u32 {
        self.stage_attempts[stage.as_index()]
    }

    /// Return total attempts across all stages.
    pub fn total_attempts(&self) -> u32 {
        self.total_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_at_research() {
        let machine = BeadStateMachine::new(BeadId::new());
        assert_eq!(machine.current_stage(), StageKind::Research);
        assert_eq!(machine.total_attempts(), 0);
    }

    #[test]
    fn test_new_is_not_complete() {
        let machine = BeadStateMachine::new(BeadId::new());
        assert!(!machine.is_complete());
    }

    #[test]
    fn test_with_policy_uses_custom_policy() {
        let policy = RecursionPolicy {
            max_total_attempts: 7,
            max_stage_retries: 2,
            max_research_retries: 1,
            on_exhaustion: ExhaustionPolicy::Fail,
        };
        let machine = BeadStateMachine::with_policy(BeadId::new(), policy);
        assert_eq!(machine.policy(), policy);
    }

    #[test]
    fn test_default_policy_values() {
        let policy = RecursionPolicy::default();
        assert_eq!(policy.max_total_attempts, 15);
        assert_eq!(policy.max_stage_retries, 3);
        assert_eq!(policy.max_research_retries, 1);
        assert_eq!(policy.on_exhaustion, ExhaustionPolicy::ParkForHuman);
    }

    #[test]
    fn test_advance_research_to_plan() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let transition = machine.advance()?;
        assert_eq!(transition.to, StageKind::Plan);
        Ok(())
    }

    #[test]
    fn test_advance_plan_to_implement() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let transition = machine.advance()?;
        assert_eq!(transition.to, StageKind::Implement);
        Ok(())
    }

    #[test]
    fn test_advance_implement_to_review() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition = machine.advance()?;
        assert_eq!(transition.to, StageKind::Review);
        Ok(())
    }

    #[test]
    fn test_advance_review_to_validate() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition = machine.advance()?;
        assert_eq!(transition.to, StageKind::Validate);
        Ok(())
    }

    #[test]
    fn test_advance_validate_to_accept() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition = machine.advance()?;
        assert_eq!(transition.to, StageKind::Accept);
        Ok(())
    }

    #[test]
    fn test_full_forward_progression() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        assert!(machine.is_complete());
        Ok(())
    }

    #[test]
    fn test_advance_past_accept_errors() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let result = machine.advance();
        assert_eq!(result, Err(StateMachineError::AlreadyTerminal));
        Ok(())
    }

    #[test]
    fn test_reenter_review_to_implement_minor() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition = machine.reenter(StageKind::Implement, "fix details", Severity::Minor)?;
        assert_eq!(transition.to, StageKind::Implement);
        Ok(())
    }

    #[test]
    fn test_reenter_review_to_plan_major() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition = machine.reenter(StageKind::Plan, "redesign", Severity::Major)?;
        assert_eq!(transition.to, StageKind::Plan);
        Ok(())
    }

    #[test]
    fn test_reenter_review_to_research_fundamental() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition =
            machine.reenter(StageKind::Research, "wrong approach", Severity::Fundamental)?;
        assert_eq!(transition.to, StageKind::Research);
        Ok(())
    }

    #[test]
    fn test_reenter_validate_to_implement() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let transition = machine.reenter(StageKind::Implement, "ci failed", Severity::Minor)?;
        assert_eq!(transition.to, StageKind::Implement);
        Ok(())
    }

    #[test]
    fn test_reenter_cannot_go_forward() {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let result = machine.reenter(StageKind::Plan, "", Severity::Minor);
        assert_eq!(result, Err(StateMachineError::InvalidReentry));
    }

    #[test]
    fn test_reenter_cannot_go_to_same_stage() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let result = machine.reenter(StageKind::Review, "", Severity::Minor);
        assert_eq!(result, Err(StateMachineError::InvalidReentry));
        Ok(())
    }

    #[test]
    fn test_reentry_target_for_severity() {
        assert_eq!(
            BeadStateMachine::reentry_target_for_severity(Severity::Minor),
            StageKind::Implement
        );
        assert_eq!(
            BeadStateMachine::reentry_target_for_severity(Severity::Major),
            StageKind::Plan
        );
        assert_eq!(
            BeadStateMachine::reentry_target_for_severity(Severity::Fundamental),
            StageKind::Research
        );
    }

    #[test]
    fn test_total_attempts_exhaustion() {
        let policy = RecursionPolicy {
            max_total_attempts: 1,
            ..RecursionPolicy::default()
        };
        let mut machine = BeadStateMachine::with_policy(BeadId::new(), policy);
        assert!(machine.enter_stage().is_ok());
        let result = machine.enter_stage();
        assert_eq!(result, Err(StateMachineError::TotalAttemptsExhausted));
    }

    #[test]
    fn test_stage_retries_exhaustion() {
        let policy = RecursionPolicy {
            max_total_attempts: 10,
            max_stage_retries: 1,
            max_research_retries: 2,
            on_exhaustion: ExhaustionPolicy::ParkForHuman,
        };
        let mut machine = BeadStateMachine::with_policy(BeadId::new(), policy);
        assert!(machine.advance().is_ok());
        assert!(machine.enter_stage().is_ok());
        let result = machine.enter_stage();
        assert_eq!(result, Err(StateMachineError::StageRetriesExhausted));
    }

    #[test]
    fn test_research_retries_uses_separate_limit() {
        let policy = RecursionPolicy {
            max_total_attempts: 10,
            max_stage_retries: 3,
            max_research_retries: 1,
            on_exhaustion: ExhaustionPolicy::ParkForHuman,
        };
        let mut machine = BeadStateMachine::with_policy(BeadId::new(), policy);
        assert!(machine.enter_stage().is_ok());
        let result = machine.enter_stage();
        assert_eq!(result, Err(StateMachineError::StageRetriesExhausted));
    }

    #[test]
    fn test_enter_stage_increments_counters() {
        let mut machine = BeadStateMachine::new(BeadId::new());
        assert!(machine.enter_stage().is_ok());
        assert_eq!(machine.stage_attempts(StageKind::Research), 1);
        assert_eq!(machine.total_attempts(), 1);
    }

    #[test]
    fn test_history_records_forward_transitions() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        assert_eq!(machine.history().len(), 1);
        assert_eq!(machine.history()[0].reason, TransitionReason::Completed);
        Ok(())
    }

    #[test]
    fn test_history_records_reentry_transitions() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.reenter(StageKind::Plan, "major issue", Severity::Major)?;
        assert_eq!(machine.history().len(), 4);
        assert!(matches!(
            machine.history()[3].reason,
            TransitionReason::GateFailed {
                feedback: _,
                severity: Severity::Major
            }
        ));
        Ok(())
    }

    #[test]
    fn test_history_preserves_order() -> Result<(), StateMachineError> {
        let mut machine = BeadStateMachine::new(BeadId::new());
        let _ = machine.advance()?;
        let _ = machine.advance()?;
        let _ = machine.reenter(StageKind::Research, "redo", Severity::Fundamental)?;

        assert_eq!(machine.history().len(), 3);
        assert_eq!(machine.history()[0].from, StageKind::Research);
        assert_eq!(machine.history()[0].to, StageKind::Plan);
        assert_eq!(machine.history()[2].from, StageKind::Implement);
        assert_eq!(machine.history()[2].to, StageKind::Research);
        Ok(())
    }
}
