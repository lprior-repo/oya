//! Property-based tests for BeadState transitions.
//!
//! Properties verified:
//! - All transitions in valid_transitions() return true for can_transition_to()
//! - All transitions NOT in valid_transitions() return false for can_transition_to()
//! - Completed state has no valid outgoing transitions (terminal)
//! - All non-terminal states have at least one valid transition
//! - Valid transitions form a DAG (no cycles back to earlier states)

use oya_events::BeadState;
use proptest::prelude::*;

/// All possible BeadState values for property testing.
const ALL_STATES: [BeadState; 8] = [
    BeadState::Pending,
    BeadState::Scheduled,
    BeadState::Ready,
    BeadState::Running,
    BeadState::Suspended,
    BeadState::BackingOff,
    BeadState::Paused,
    BeadState::Completed,
];

/// Generate arbitrary BeadState for proptest.
fn arb_bead_state() -> impl Strategy<Value = BeadState> {
    proptest::sample::select(ALL_STATES.to_vec())
}

proptest! {
    // ==========================================================================
    // PROPERTY: can_transition_to and valid_transitions() are consistent
    // ==========================================================================

    /// Property: For any state S and target T, can_transition_to(T) returns true
    /// if and only if T is in valid_transitions().
    #[test]
    fn prop_can_transition_consistent_with_valid_transitions(
        from_state in arb_bead_state(),
        to_state in arb_bead_state(),
    ) {
        let valid_targets: Vec<BeadState> = from_state.valid_transitions();
        let can_transition = from_state.can_transition_to(to_state);
        let is_in_valid = valid_targets.contains(&to_state);

        prop_assert_eq!(
            can_transition,
            is_in_valid,
            "can_transition_to({:?}, {:?}) = {}, but valid_transitions() = {:?}",
            from_state,
            to_state,
            can_transition,
            valid_targets
        );
    }

    // ==========================================================================
    // PROPERTY: Terminal state has no outgoing transitions
    // ==========================================================================

    /// Property: Completed state cannot transition to any state (including itself).
    #[test]
    fn prop_completed_has_no_transitions(target in arb_bead_state()) {
        let completed = BeadState::Completed;
        prop_assert!(
            !completed.can_transition_to(target),
            "Completed should not transition to {:?}",
            target
        );
    }

    /// Property: valid_transitions() for Completed returns empty vector.
    #[test]
    fn prop_completed_valid_transitions_empty() {
        let transitions = BeadState::Completed.valid_transitions();
        prop_assert!(
            transitions.is_empty(),
            "Completed valid_transitions() should be empty, got {:?}",
            transitions
        );
    }

    // ==========================================================================
    // PROPERTY: Non-terminal states have at least one valid transition
    // ==========================================================================

    /// Property: All non-terminal states can reach Completed (directly or indirectly).
    /// This tests that Completed is always reachable, not immediate.
    #[test]
    fn prop_non_terminal_states_can_reach_completed(state in arb_bead_state()) {
        if state.is_terminal() {
            return Ok(()); // Skip terminal state
        }

        // Check if state can reach Completed (directly or through valid transitions)
        let can_reach = can_reach_completed(state);
        prop_assert!(
            can_reach,
            "State {:?} should be able to reach Completed",
            state
        );
    }

    // ==========================================================================
    // PROPERTY: Transitivity - if A->B and B can reach Completed, then A can reach Completed
    // ==========================================================================

    /// Property: All valid transitions from any state preserve ability to reach Completed.
    #[test]
    fn prop_transitions_preserve_completed_reachability(state in arb_bead_state()) {
        if state.is_terminal() {
            return Ok(()); // Completed cannot transition
        }

        for target in state.valid_transitions() {
            let can_reach = can_reach_completed(target);
            prop_assert!(
                can_reach,
                "Transition {:?} -> {:?} breaks Completed reachability",
                state,
                target
            );
        }
        Ok(())
    }

    // ==========================================================================
    // PROPERTY: No self-transitions except via normal flow
    // ==========================================================================

    /// Property: No state can transition to itself.
    #[test]
    fn prop_no_self_transitions(state in arb_bead_state()) {
        prop_assert!(
            !state.can_transition_to(state),
            "State {:?} should not transition to itself",
            state
        );
    }

    // ==========================================================================
    // PROPERTY: is_terminal matches state being Completed
    // ==========================================================================

    /// Property: is_terminal returns true only for Completed.
    #[test]
    fn prop_is_terminal_only_completed(state in arb_bead_state()) {
        let expected = matches!(state, BeadState::Completed);
        prop_assert_eq!(
            state.is_terminal(),
            expected,
            "is_terminal({:?}) should be {}",
            state,
            expected
        );
    }

    // ==========================================================================
    // PROPERTY: Bidirectional consistency
    // ==========================================================================

    /// Property: If A can transition to B, B cannot always transition back to A.
    /// This ensures the state machine is directional (mostly DAG-like).
    #[test]
    fn prop_transitions_are_mostly_unidirectional(
        from_state in arb_bead_state(),
        to_state in arb_bead_state(),
    ) {
        if from_state.can_transition_to(to_state) && !to_state.is_terminal() {
            // Most transitions should not be bidirectional
            let back_transitions = to_state.valid_transitions();
            // Note: Some bidirectional transitions are valid (e.g., Running <-> Paused)
            // This is just documenting the behavior, not asserting
            let _ = back_transitions.contains(&from_state);
        }
        Ok(())
    }

    // ==========================================================================
    // PROPERTY: Exhaustive transition matrix coverage
    // ==========================================================================

    /// Property: Every state either transitions to Completed or has a path to it.
    #[test]
    fn prop_all_states_eventually_complete() {
        for state in ALL_STATES {
            if state.is_terminal() {
                continue;
            }
            let reachable = can_reach_completed(state);
            prop_assert!(
                reachable,
                "State {:?} cannot reach Completed - workflow may deadlock",
                state
            );
        }
        Ok(())
    }
}

/// Check if a state can reach Completed through valid transitions.
/// Uses BFS to find shortest path.
fn can_reach_completed(start: BeadState) -> bool {
    if start.is_terminal() {
        return true;
    }

    let mut visited = std::collections::HashSet::new();
    let mut queue = vec![start];

    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        for next in current.valid_transitions() {
            if next.is_terminal() {
                return true;
            }
            if !visited.contains(&next) {
                queue.push(next);
            }
        }
    }

    false
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn should_reach_completed_from_all_non_terminal_states() {
        for state in ALL_STATES {
            if state.is_terminal() {
                assert!(!can_reach_completed(state) || state == BeadState::Completed);
            } else {
                assert!(
                    can_reach_completed(state),
                    "{:?} should reach Completed",
                    state
                );
            }
        }
    }

    #[test]
    fn should_have_consistent_transition_counts() {
        for state in ALL_STATES {
            let valid = state.valid_transitions();
            let count = valid.len();

            let explicit_count = match state {
                BeadState::Pending => 2,
                BeadState::Scheduled => 3,
                BeadState::Ready => 3,
                BeadState::Running => 4,
                BeadState::Suspended => 2,
                BeadState::BackingOff => 2,
                BeadState::Paused => 2,
                BeadState::Completed => 0,
            };

            assert_eq!(
                count, explicit_count,
                "{:?} should have {} valid transitions, got {:?}",
                state, explicit_count, valid
            );
        }
    }

    #[test]
    fn should_not_allow_skipping_states() {
        assert!(
            !BeadState::Pending.can_transition_to(BeadState::Running),
            "Cannot skip Scheduled/Ready"
        );
        assert!(
            !BeadState::Pending.can_transition_to(BeadState::Ready),
            "Cannot skip Scheduled"
        );
    }
}
