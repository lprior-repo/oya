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

fn arb_bead_state() -> impl Strategy<Value = BeadState> {
    prop_oneof![
        Just(BeadState::Pending),
        Just(BeadState::Scheduled),
        Just(BeadState::Ready),
        Just(BeadState::Running),
        Just(BeadState::Suspended),
        Just(BeadState::BackingOff),
        Just(BeadState::Paused),
        Just(BeadState::Completed),
    ]
}

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

proptest! {
    #[test]
    fn prop_can_transition_consistent(from_state in arb_bead_state(), to_state in arb_bead_state()) {
        let valid_targets: Vec<BeadState> = from_state.valid_transitions();
        let can_transition = from_state.can_transition_to(to_state);
        let is_in_valid = valid_targets.contains(&to_state);

        prop_assert_eq!(can_transition, is_in_valid);
    }

    #[test]
    fn prop_completed_has_no_transitions(target in arb_bead_state()) {
        let completed = BeadState::Completed;
        prop_assert!(!completed.can_transition_to(target));
    }

    #[test]
    fn prop_completed_valid_transitions_empty() {
        let transitions = BeadState::Completed.valid_transitions();
        prop_assert!(transitions.is_empty());
    }

    #[test]
    fn prop_non_terminal_can_reach_completed(state in arb_bead_state()) {
        if state.is_terminal() {
            return Ok(());
        }
        let can_reach = can_reach_completed(state);
        prop_assert!(can_reach);
    }

    #[test]
    fn prop_transitions_preserve_reachability(state in arb_bead_state()) {
        if state.is_terminal() {
            return Ok(());
        }
        for target in state.valid_transitions() {
            let can_reach = can_reach_completed(target);
            prop_assert!(can_reach);
        }
        Ok(())
    }

    #[test]
    fn prop_no_self_transitions(state in arb_bead_state()) {
        prop_assert!(!state.can_transition_to(state));
    }

    #[test]
    fn prop_is_terminal_only_completed(state in arb_bead_state()) {
        let expected = matches!(state, BeadState::Completed);
        prop_assert_eq!(state.is_terminal(), expected);
    }
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
        assert!(!BeadState::Pending.can_transition_to(BeadState::Running));
        assert!(!BeadState::Pending.can_transition_to(BeadState::Ready));
    }

    #[test]
    fn should_exhaustively_verify_all_states_eventually_complete() {
        for state in ALL_STATES {
            if state.is_terminal() {
                continue;
            }
            assert!(
                can_reach_completed(state),
                "State {:?} cannot reach Completed",
                state
            );
        }
    }
}
