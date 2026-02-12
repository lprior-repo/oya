use oya_events::BeadState;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_completed_is_terminal() {
        prop_assert!(BeadState::Completed.is_terminal());
    }
}
