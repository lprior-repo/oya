//! Bead state machine with validated transitions.
//!
//! Implements the 8-state lifecycle from the nuoc design.

use serde::{Deserialize, Serialize};

/// 8-state lifecycle for beads.
///
/// State transitions are validated to ensure correctness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadState {
    /// Waiting for dependencies to resolve.
    Pending,
    /// Dependencies resolved, ready to be claimed by a worker.
    Scheduled,
    /// Claimed by a worker, about to start execution.
    Ready,
    /// Actively executing.
    Running,
    /// Paused by user request.
    Suspended,
    /// Waiting after a failure before retry.
    BackingOff,
    /// System pause (resource constraint or throttling).
    Paused,
    /// Terminal state: success or failure.
    Completed,
}

impl BeadState {
    /// Check if transition to target state is valid.
    #[must_use]
    pub fn can_transition_to(&self, target: &BeadState) -> bool {
        self.valid_transitions().contains(target)
    }

    /// Get all valid transitions from current state.
    #[must_use]
    pub fn valid_transitions(&self) -> &'static [BeadState] {
        match self {
            // Pending can become Scheduled (deps resolved) or Completed (cancelled)
            BeadState::Pending => &[BeadState::Scheduled, BeadState::Completed],

            // Scheduled can become Ready (claimed) or Pending (deps changed)
            BeadState::Scheduled => &[BeadState::Ready, BeadState::Pending, BeadState::Completed],

            // Ready can become Running (started) or Scheduled (unclaimed)
            BeadState::Ready => &[BeadState::Running, BeadState::Scheduled],

            // Running can become many states
            BeadState::Running => &[
                BeadState::Completed,  // Success or failure
                BeadState::Suspended,  // User pause
                BeadState::Paused,     // System pause
                BeadState::BackingOff, // Retry pending
            ],

            // Suspended can resume or be cancelled
            BeadState::Suspended => &[BeadState::Ready, BeadState::Completed],

            // BackingOff can retry or give up
            BeadState::BackingOff => &[BeadState::Scheduled, BeadState::Completed],

            // Paused can resume
            BeadState::Paused => &[BeadState::Ready, BeadState::Completed],

            // Completed is terminal
            BeadState::Completed => &[],
        }
    }

    /// Check if this is a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, BeadState::Completed)
    }

    /// Check if this state can be claimed by a worker.
    #[must_use]
    pub const fn is_claimable(&self) -> bool {
        matches!(self, BeadState::Scheduled)
    }

    /// Check if this state represents active work.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, BeadState::Ready | BeadState::Running)
    }

    /// Check if this state is waiting.
    #[must_use]
    pub const fn is_waiting(&self) -> bool {
        matches!(
            self,
            BeadState::Pending | BeadState::Scheduled | BeadState::BackingOff | BeadState::Paused
        )
    }
}

impl std::fmt::Display for BeadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeadState::Pending => write!(f, "pending"),
            BeadState::Scheduled => write!(f, "scheduled"),
            BeadState::Ready => write!(f, "ready"),
            BeadState::Running => write!(f, "running"),
            BeadState::Suspended => write!(f, "suspended"),
            BeadState::BackingOff => write!(f, "backing_off"),
            BeadState::Paused => write!(f, "paused"),
            BeadState::Completed => write!(f, "completed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_transitions() {
        let state = BeadState::Pending;
        assert!(state.can_transition_to(&BeadState::Scheduled));
        assert!(state.can_transition_to(&BeadState::Completed));
        assert!(!state.can_transition_to(&BeadState::Running));
    }

    #[test]
    fn test_scheduled_transitions() {
        let state = BeadState::Scheduled;
        assert!(state.can_transition_to(&BeadState::Ready));
        assert!(state.can_transition_to(&BeadState::Pending));
        assert!(!state.can_transition_to(&BeadState::Running));
    }

    #[test]
    fn test_running_transitions() {
        let state = BeadState::Running;
        assert!(state.can_transition_to(&BeadState::Completed));
        assert!(state.can_transition_to(&BeadState::Suspended));
        assert!(state.can_transition_to(&BeadState::Paused));
        assert!(state.can_transition_to(&BeadState::BackingOff));
        assert!(!state.can_transition_to(&BeadState::Pending));
    }

    #[test]
    fn test_completed_is_terminal() {
        let state = BeadState::Completed;
        assert!(state.is_terminal());
        assert!(state.valid_transitions().is_empty());
    }

    #[test]
    fn test_claimable_states() {
        assert!(BeadState::Scheduled.is_claimable());
        assert!(!BeadState::Pending.is_claimable());
        assert!(!BeadState::Running.is_claimable());
    }

    #[test]
    fn test_active_states() {
        assert!(BeadState::Ready.is_active());
        assert!(BeadState::Running.is_active());
        assert!(!BeadState::Pending.is_active());
        assert!(!BeadState::Completed.is_active());
    }

    #[test]
    fn test_waiting_states() {
        assert!(BeadState::Pending.is_waiting());
        assert!(BeadState::Scheduled.is_waiting());
        assert!(BeadState::BackingOff.is_waiting());
        assert!(!BeadState::Running.is_waiting());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", BeadState::Pending), "pending");
        assert_eq!(format!("{}", BeadState::Running), "running");
        assert_eq!(format!("{}", BeadState::BackingOff), "backing_off");
    }

    #[test]
    fn test_serialization() {
        let state = BeadState::Running;
        let json = serde_json::to_string(&state);
        assert!(json.is_ok());
        if let Ok(json_str) = json {
            assert_eq!(json_str, "\"Running\"");
        }
    }

    #[test]
    fn test_full_lifecycle() {
        // Happy path: Pending → Scheduled → Ready → Running → Completed
        let mut state = BeadState::Pending;

        assert!(state.can_transition_to(&BeadState::Scheduled));
        state = BeadState::Scheduled;

        assert!(state.can_transition_to(&BeadState::Ready));
        state = BeadState::Ready;

        assert!(state.can_transition_to(&BeadState::Running));
        state = BeadState::Running;

        assert!(state.can_transition_to(&BeadState::Completed));
        state = BeadState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_retry_lifecycle() {
        // Retry path: Running → BackingOff → Scheduled → Ready → Running
        let mut state = BeadState::Running;

        assert!(state.can_transition_to(&BeadState::BackingOff));
        state = BeadState::BackingOff;

        assert!(state.can_transition_to(&BeadState::Scheduled));
        state = BeadState::Scheduled;

        assert!(state.can_transition_to(&BeadState::Ready));
        state = BeadState::Ready;

        assert!(state.can_transition_to(&BeadState::Running));
    }
}
