//! Functional checkpoint manager with state machine pattern.
//!
//! This module provides a state machine-based checkpoint manager that uses pure functions
//! for state transitions, following functional programming principles.

use std::time::Instant;

use crate::types::PhaseOutput;

/// Checkpoint strategy determining when to create checkpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointStrategy {
    /// Checkpoint after every phase.
    Always,
    /// Checkpoint only after successful phases.
    OnSuccess,
    /// Checkpoint after every N phases.
    Interval(usize),
}

/// Checkpoint decision result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointDecision {
    /// Create a checkpoint.
    Checkpoint,
    /// Skip checkpointing.
    Skip,
}

impl CheckpointDecision {
    /// Create from boolean.
    fn from(should_checkpoint: bool) -> Self {
        if should_checkpoint {
            Self::Checkpoint
        } else {
            Self::Skip
        }
    }

    /// Returns true if should checkpoint.
    pub fn should_checkpoint(&self) -> bool {
        matches!(self, Self::Checkpoint)
    }
}

/// Internal checkpoint state.
#[derive(Debug, Clone)]
struct CheckpointState {
    last_checkpoint: Option<Instant>,
    phases_since_last: usize,
    strategy: CheckpointStrategy,
}

/// Functional checkpoint manager using state machine pattern.
///
/// This manager uses pure functions for state transitions, making it easy to test
/// and reason about. The `update` method provides the mutable interface for
/// compatibility with existing code.
#[derive(Debug, Clone)]
pub struct CheckpointManager {
    state: CheckpointState,
}

impl CheckpointManager {
    /// Create a new checkpoint manager with the given strategy.
    #[must_use]
    pub fn new(strategy: CheckpointStrategy) -> Self {
        Self {
            state: CheckpointState {
                last_checkpoint: None,
                phases_since_last: 0,
                strategy,
            },
        }
    }

    /// Pure function: computes next state and decision from current state and input.
    ///
    /// This is the core state transition function. Given the current state and a phase
    /// output, it returns the new state and whether to checkpoint.
    fn should_checkpoint(
        &self,
        phase_output: &PhaseOutput,
    ) -> (CheckpointState, CheckpointDecision) {
        let should = match self.state.strategy {
            CheckpointStrategy::Always => true,
            CheckpointStrategy::OnSuccess => phase_output.success,
            CheckpointStrategy::Interval(n) => self.state.phases_since_last >= n,
        };

        let new_state = if should {
            CheckpointState {
                last_checkpoint: Some(Instant::now()),
                phases_since_last: 0,
                strategy: self.state.strategy,
            }
        } else {
            CheckpointState {
                phases_since_last: self.state.phases_since_last + 1,
                ..self.state.clone()
            }
        };

        (new_state, CheckpointDecision::from(should))
    }

    /// Update the checkpoint manager state and return the decision.
    ///
    /// This is the exterior mutation method for compatibility with existing code.
    /// Internally, it uses the pure `should_checkpoint` function.
    pub fn update(&mut self, phase_output: &PhaseOutput) -> CheckpointDecision {
        let (new_state, decision) = self.should_checkpoint(phase_output);
        self.state = new_state;
        decision
    }

    /// Get the current checkpoint strategy.
    #[must_use]
    pub const fn strategy(&self) -> CheckpointStrategy {
        self.state.strategy
    }

    /// Get the number of phases since the last checkpoint.
    #[must_use]
    pub const fn phases_since_last(&self) -> usize {
        self.state.phases_since_last
    }

    /// Get the time of the last checkpoint, if any.
    #[must_use]
    pub fn last_checkpoint(&self) -> Option<Instant> {
        self.state.last_checkpoint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a successful phase output.
    fn success_output() -> PhaseOutput {
        PhaseOutput::success(vec![1, 2, 3])
    }

    /// Helper to create a failed phase output.
    fn failure_output() -> PhaseOutput {
        PhaseOutput {
            success: false,
            data: std::sync::Arc::new(vec![]),
            message: Some("Failed".to_string()),
            artifacts: vec![],
            duration_ms: 100,
        }
    }

    #[test]
    fn test_checkpoint_strategy_always() {
        let mut manager = CheckpointManager::new(CheckpointStrategy::Always);

        // Should always checkpoint
        let decision1 = manager.update(&success_output());
        assert!(decision1.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 0);

        let decision2 = manager.update(&failure_output());
        assert!(decision2.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 0);
    }

    #[test]
    fn test_checkpoint_strategy_on_success() {
        let mut manager = CheckpointManager::new(CheckpointStrategy::OnSuccess);

        // Successful phase -> checkpoint
        let decision1 = manager.update(&success_output());
        assert!(decision1.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 0);

        // Failed phase -> skip
        let decision2 = manager.update(&failure_output());
        assert!(!decision2.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 1);

        // Another failed phase -> skip
        let decision3 = manager.update(&failure_output());
        assert!(!decision3.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 2);

        // Successful phase -> checkpoint
        let decision4 = manager.update(&success_output());
        assert!(decision4.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 0);
    }

    #[test]
    fn test_checkpoint_strategy_interval() {
        let mut manager = CheckpointManager::new(CheckpointStrategy::Interval(3));

        // First phase -> skip (0 < 3)
        let decision1 = manager.update(&success_output());
        assert!(!decision1.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 1);

        // Second phase -> skip (1 < 3)
        let decision2 = manager.update(&success_output());
        assert!(!decision2.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 2);

        // Third phase -> checkpoint (2 >= 3)
        let decision3 = manager.update(&success_output());
        assert!(decision3.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 0);

        // Fourth phase -> skip (counter reset)
        let decision4 = manager.update(&success_output());
        assert!(!decision4.should_checkpoint());
        assert_eq!(manager.phases_since_last(), 1);
    }

    #[test]
    fn test_checkpoint_state_transitions() {
        let manager = CheckpointManager::new(CheckpointStrategy::Interval(2));

        // Test pure state transitions
        let (state1, decision1) = manager.should_checkpoint(&success_output());
        assert!(!decision1.should_checkpoint());
        assert_eq!(state1.phases_since_last, 1);

        let manager2 = CheckpointManager { state: state1 };
        let (state2, decision2) = manager2.should_checkpoint(&success_output());
        assert!(decision2.should_checkpoint());
        assert_eq!(state2.phases_since_last, 0);
        assert!(state2.last_checkpoint.is_some());
    }

    #[test]
    fn test_checkpoint_decision_from_bool() {
        assert_eq!(
            CheckpointDecision::from(true),
            CheckpointDecision::Checkpoint
        );
        assert_eq!(CheckpointDecision::from(false), CheckpointDecision::Skip);
    }

    #[test]
    fn test_checkpoint_manager_accessors() {
        let manager = CheckpointManager::new(CheckpointStrategy::OnSuccess);
        assert_eq!(manager.strategy(), CheckpointStrategy::OnSuccess);
        assert_eq!(manager.phases_since_last(), 0);
        assert!(manager.last_checkpoint().is_none());

        // After update, last_checkpoint should still be None (no checkpoint yet)
        let mut manager = manager;
        manager.update(&failure_output());
        assert!(manager.last_checkpoint().is_none());

        // After successful phase, should have checkpoint time
        manager.update(&success_output());
        assert!(manager.last_checkpoint().is_some());
    }

    #[test]
    fn test_checkpoint_with_zero_interval() {
        let mut manager = CheckpointManager::new(CheckpointStrategy::Interval(0));

        // Zero interval means checkpoint every phase (like Always)
        let decision1 = manager.update(&success_output());
        assert!(decision1.should_checkpoint());

        let decision2 = manager.update(&failure_output());
        assert!(decision2.should_checkpoint());
    }

    #[test]
    fn test_checkpoint_state_immutability() {
        let manager = CheckpointManager::new(CheckpointStrategy::Always);

        // Pure function should not mutate original state
        let _ = manager.should_checkpoint(&success_output());
        assert_eq!(manager.phases_since_last(), 0);
        assert!(manager.last_checkpoint().is_none());

        // update should mutate
        let mut manager = manager;
        let _ = manager.update(&success_output());
        assert_eq!(manager.phases_since_last(), 0);
        assert!(manager.last_checkpoint().is_some());
    }
}
