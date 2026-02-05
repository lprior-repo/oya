#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Bead lifecycle state machine for worker actors.
//!
//! This module models the eight-state lifecycle described in the
//! architecture notes (nuoc design):
//! Pending → Scheduled → Ready → Running → (Suspended|BackingOff|Paused)
//! → Completed.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The canonical bead lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadState {
    /// Waiting for dependencies.
    Pending,
    /// Ready to be claimed by a worker.
    Scheduled,
    /// Claimed and about to start.
    Ready,
    /// Actively executing.
    Running,
    /// Paused by user request.
    Suspended,
    /// Waiting after failure before retry.
    BackingOff,
    /// System-level pause (e.g., resource limits).
    Paused,
    /// Terminal: success or failure.
    Completed,
}

impl BeadState {
    /// Whether this state is terminal.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Whether this state represents active execution or ownership.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Scheduled
                | Self::Ready
                | Self::Running
                | Self::Suspended
                | Self::BackingOff
                | Self::Paused
        )
    }

    /// Check if the transition to `target` is allowed.
    #[must_use]
    pub fn can_transition_to(&self, target: BeadState) -> bool {
        matches!(
            (self, target),
            // From Pending
            (Self::Pending, Self::Scheduled) | (Self::Pending, Self::Completed)
            // From Scheduled
            | (Self::Scheduled, Self::Ready) | (Self::Scheduled, Self::Pending) | (Self::Scheduled, Self::Completed)
            // From Ready
            | (Self::Ready, Self::Running) | (Self::Ready, Self::Scheduled) | (Self::Ready, Self::Completed)
            // From Running
            | (Self::Running, Self::Suspended) | (Self::Running, Self::BackingOff)
            | (Self::Running, Self::Paused) | (Self::Running, Self::Completed)
            // From Suspended
            | (Self::Suspended, Self::Running) | (Self::Suspended, Self::Completed)
            // From BackingOff
            | (Self::BackingOff, Self::Running) | (Self::BackingOff, Self::Completed)
            // From Paused
            | (Self::Paused, Self::Running) | (Self::Paused, Self::Completed)
        )
    }

    /// All valid transitions from this state.
    #[must_use]
    pub const fn valid_transitions(&self) -> &'static [BeadState] {
        match self {
            Self::Pending => &[Self::Scheduled, Self::Completed],
            Self::Scheduled => &[Self::Ready, Self::Pending, Self::Completed],
            Self::Ready => &[Self::Running, Self::Scheduled, Self::Completed],
            Self::Running => &[
                Self::Suspended,
                Self::BackingOff,
                Self::Paused,
                Self::Completed,
            ],
            Self::Suspended => &[Self::Running, Self::Completed],
            Self::BackingOff => &[Self::Running, Self::Completed],
            Self::Paused => &[Self::Running, Self::Completed],
            Self::Completed => &[],
        }
    }
}

/// Describes a successful state transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    bead_id: String,
    from: BeadState,
    to: BeadState,
}

impl StateTransition {
    /// Create a new state transition record.
    #[must_use]
    pub fn new(bead_id: impl Into<String>, from: BeadState, to: BeadState) -> Self {
        Self {
            bead_id: bead_id.into(),
            from,
            to,
        }
    }

    /// The bead ID this transition applies to.
    #[must_use]
    pub fn bead_id(&self) -> &str {
        &self.bead_id
    }

    /// Previous state.
    #[must_use]
    pub const fn from(&self) -> BeadState {
        self.from
    }

    /// New state.
    #[must_use]
    pub const fn to(&self) -> BeadState {
        self.to
    }
}

/// Stateful bead lifecycle with validated transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadLifecycle {
    bead_id: String,
    state: BeadState,
}

impl BeadLifecycle {
    /// Create a new lifecycle starting in `Pending`.
    #[must_use]
    pub fn new(bead_id: impl Into<String>) -> Self {
        Self {
            bead_id: bead_id.into(),
            state: BeadState::Pending,
        }
    }

    /// Rehydrate an existing lifecycle from a persisted state.
    #[must_use]
    pub fn from_state(bead_id: impl Into<String>, state: BeadState) -> Self {
        Self {
            bead_id: bead_id.into(),
            state,
        }
    }

    /// Current state.
    #[must_use]
    pub const fn state(&self) -> BeadState {
        self.state
    }

    /// Allowed transitions from the current state.
    #[must_use]
    pub const fn valid_transitions(&self) -> &'static [BeadState] {
        self.state.valid_transitions()
    }

    /// Whether the lifecycle can move to `target`.
    #[must_use]
    pub fn can_transition_to(&self, target: BeadState) -> bool {
        self.state.can_transition_to(target)
    }

    /// Transition to a new state, enforcing lifecycle rules.
    ///
    /// # Errors
    /// Returns `Error::InvalidRecord` if the transition is not allowed
    /// or is a no-op.
    pub fn transition_to(&mut self, target: BeadState) -> Result<StateTransition> {
        let from = self.state;

        if from == target {
            return Err(Error::InvalidRecord {
                reason: format!(
                    "bead '{}' is already in state {}",
                    self.bead_id,
                    display_state(target)
                ),
            });
        }

        if !from.can_transition_to(target) {
            return Err(Error::InvalidRecord {
                reason: format!(
                    "invalid transition for bead '{}': {} -> {}",
                    self.bead_id,
                    display_state(from),
                    display_state(target)
                ),
            });
        }

        self.state = target;
        Ok(StateTransition::new(self.bead_id.clone(), from, target))
    }

    /// Convenience: Pending -> Scheduled.
    pub fn schedule(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Scheduled)
    }

    /// Convenience: Scheduled -> Ready.
    pub fn mark_ready(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Ready)
    }

    /// Convenience: Ready -> Running.
    pub fn start(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Running)
    }

    /// Convenience: Running -> Suspended.
    pub fn suspend(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Suspended)
    }

    /// Convenience: Suspended|Paused|BackingOff -> Running.
    pub fn resume(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Running)
    }

    /// Convenience: Running -> BackingOff.
    pub fn backoff(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::BackingOff)
    }

    /// Convenience: Running -> Paused.
    pub fn pause(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Paused)
    }

    /// Convenience: Any -> Completed (when valid).
    pub fn complete(&mut self) -> Result<StateTransition> {
        self.transition_to(BeadState::Completed)
    }
}

fn display_state(state: BeadState) -> &'static str {
    match state {
        BeadState::Pending => "pending",
        BeadState::Scheduled => "scheduled",
        BeadState::Ready => "ready",
        BeadState::Running => "running",
        BeadState::Suspended => "suspended",
        BeadState::BackingOff => "backing_off",
        BeadState::Paused => "paused",
        BeadState::Completed => "completed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_pending() {
        let lifecycle = BeadLifecycle::new("bead-1");
        assert_eq!(lifecycle.state(), BeadState::Pending);
        assert!(!lifecycle.state().is_active());
        assert!(!lifecycle.state().is_terminal());
    }

    #[test]
    fn allowed_transition_succeeds() {
        let mut lifecycle = BeadLifecycle::new("bead-2");

        let transition = lifecycle.schedule();
        assert!(transition.is_ok());

        if let Ok(transition) = transition {
            assert_eq!(transition.from(), BeadState::Pending);
            assert_eq!(transition.to(), BeadState::Scheduled);
            assert_eq!(lifecycle.state(), BeadState::Scheduled);
        }
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut lifecycle = BeadLifecycle::new("bead-3");
        let result = lifecycle.start();

        assert!(result.is_err());
        let reason_matches = result
            .map(|_| false)
            .unwrap_or_else(|err| matches!(err, Error::InvalidRecord { .. }));
        assert!(reason_matches);
    }

    #[test]
    fn completes_from_running() {
        let mut lifecycle = BeadLifecycle::from_state("bead-4", BeadState::Running);
        let result = lifecycle.complete();

        assert!(result.is_ok());
        assert_eq!(lifecycle.state(), BeadState::Completed);
        assert!(lifecycle.state().is_terminal());
        assert!(lifecycle.valid_transitions().is_empty());
    }

    #[test]
    fn valid_transitions_match_expectations() {
        let lifecycle = BeadLifecycle::from_state("bead-5", BeadState::Running);
        let next_states = lifecycle.valid_transitions();

        assert_eq!(
            next_states,
            &[
                BeadState::Suspended,
                BeadState::BackingOff,
                BeadState::Paused,
                BeadState::Completed
            ]
        );
    }
}
