//! State machine for event replay lifecycle.
//!
//! This module defines a type-safe state machine for tracking the lifecycle
//! of an event replay operation, from initialization through completion or failure.

use crate::error::Result;

/// States in the event replay lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReplayState {
    /// Initial state before any replay operation begins.
    #[default]
    Uninitialized,

    /// Loading events from the event store.
    Loading { events_loaded: u64 },

    /// Actively replaying events to build projections.
    Replaying {
        events_processed: u64,
        events_total: u64,
    },

    /// Replay completed successfully.
    Complete { events_processed: u64 },

    /// Replay failed with an error.
    Failed { error: String },
}

impl ReplayState {
    /// Transition from current state to Loading state.
    ///
    /// # Errors
    /// Returns an error if the current state is not Uninitialized.
    pub fn start_loading(&self) -> Result<Self> {
        match self {
            ReplayState::Uninitialized => Ok(ReplayState::Loading { events_loaded: 0 }),
            current => Err(crate::error::Error::InvalidState {
                current: format!("{:?}", current),
                attempted: "Loading".into(),
            }),
        }
    }

    /// Transition from Loading to Replaying state.
    ///
    /// # Errors
    /// Returns an error if the current state is not Loading.
    pub fn start_replaying(&self, events_total: u64) -> Result<Self> {
        match self {
            ReplayState::Loading { .. } => Ok(ReplayState::Replaying {
                events_processed: 0,
                events_total,
            }),
            current => Err(crate::error::Error::InvalidState {
                current: format!("{:?}", current),
                attempted: "Replaying".into(),
            }),
        }
    }

    /// Update replay progress while in Replaying state.
    ///
    /// # Errors
    /// Returns an error if the current state is not Replaying.
    pub fn update_progress(&self, events_processed: u64) -> Result<Self> {
        match self {
            ReplayState::Replaying { events_total, .. } => Ok(ReplayState::Replaying {
                events_processed,
                events_total: *events_total,
            }),
            current => Err(crate::error::Error::InvalidState {
                current: format!("{:?}", current),
                attempted: "update_progress".into(),
            }),
        }
    }

    /// Transition from Replaying to Complete state.
    ///
    /// # Errors
    /// Returns an error if the current state is not Replaying.
    pub fn complete(&self) -> Result<Self> {
        match self {
            ReplayState::Replaying {
                events_processed, ..
            } => Ok(ReplayState::Complete {
                events_processed: *events_processed,
            }),
            current => Err(crate::error::Error::InvalidState {
                current: format!("{:?}", current),
                attempted: "Complete".into(),
            }),
        }
    }

    /// Transition from any state to Failed state.
    pub fn fail(&self, error: String) -> ReplayState {
        ReplayState::Failed { error }
    }

    /// Check if the replay is in a terminal state (Complete or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ReplayState::Complete { .. } | ReplayState::Failed { .. }
        )
    }

    /// Check if the replay is in an active state (Loading or Replaying).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ReplayState::Loading { .. } | ReplayState::Replaying { .. }
        )
    }

    /// Get a human-readable description of the current state.
    pub fn description(&self) -> &str {
        match self {
            ReplayState::Uninitialized => "Not started",
            ReplayState::Loading { .. } => "Loading events",
            ReplayState::Replaying { .. } => "Replaying events",
            ReplayState::Complete { .. } => "Complete",
            ReplayState::Failed { .. } => "Failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // STATE CREATION AND DEFAULT
    // ==========================================================================

    #[test]
    fn test_default_state() {
        let state = ReplayState::default();
        assert_eq!(state, ReplayState::Uninitialized);
    }

    #[test]
    fn test_uninitialized_description() {
        let state = ReplayState::Uninitialized;
        assert_eq!(state.description(), "Not started");
    }

    // ==========================================================================
    // VALID TRANSITIONS
    // ==========================================================================

    #[test]
    fn test_start_loading_from_uninitialized() {
        let state = ReplayState::Uninitialized;
        let next = state.start_loading();

        assert!(next.is_ok());
        if let Ok(loaded) = next {
            assert_eq!(loaded, ReplayState::Loading { events_loaded: 0 });
        }
    }

    #[test]
    fn test_start_replaying_from_loading() {
        let state = ReplayState::Loading { events_loaded: 100 };
        let next = state.start_replaying(100);

        assert!(next.is_ok());
        if let Ok(replaying) = next {
            assert_eq!(
                replaying,
                ReplayState::Replaying {
                    events_processed: 0,
                    events_total: 100
                }
            );
        }
    }

    #[test]
    fn test_update_progress_while_replaying() {
        let state = ReplayState::Replaying {
            events_processed: 50,
            events_total: 100,
        };
        let next = state.update_progress(75);

        assert!(next.is_ok());
        if let Ok(updated) = next {
            assert_eq!(
                updated,
                ReplayState::Replaying {
                    events_processed: 75,
                    events_total: 100
                }
            );
        }
    }

    #[test]
    fn test_complete_from_replaying() {
        let state = ReplayState::Replaying {
            events_processed: 100,
            events_total: 100,
        };
        let next = state.complete();

        assert!(next.is_ok());
        if let Ok(complete) = next {
            assert_eq!(
                complete,
                ReplayState::Complete {
                    events_processed: 100
                }
            );
        }
    }

    #[test]
    fn test_fail_from_any_state() {
        // Can fail from Uninitialized
        let state = ReplayState::Uninitialized;
        let failed = state.fail("test error".into());
        assert_eq!(
            failed,
            ReplayState::Failed {
                error: "test error".into()
            }
        );

        // Can fail from Loading
        let state = ReplayState::Loading { events_loaded: 50 };
        let failed = state.fail("load failed".into());
        assert_eq!(
            failed,
            ReplayState::Failed {
                error: "load failed".into()
            }
        );

        // Can fail from Replaying
        let state = ReplayState::Replaying {
            events_processed: 25,
            events_total: 100,
        };
        let failed = state.fail("replay failed".into());
        assert_eq!(
            failed,
            ReplayState::Failed {
                error: "replay failed".into()
            }
        );
    }

    // ==========================================================================
    // INVALID TRANSITIONS
    // ==========================================================================

    #[test]
    fn test_cannot_start_loading_from_loading() {
        let state = ReplayState::Loading { events_loaded: 10 };
        let result = state.start_loading();

        assert!(result.is_err());
        if let Err(crate::error::Error::InvalidState { current, attempted }) = result {
            assert!(current.contains("Loading"));
            assert_eq!(attempted, "Loading");
        } else {
            panic!("Expected InvalidState error");
        }
    }

    #[test]
    fn test_cannot_start_loading_from_replaying() {
        let state = ReplayState::Replaying {
            events_processed: 50,
            events_total: 100,
        };
        let result = state.start_loading();

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_start_loading_from_complete() {
        let state = ReplayState::Complete {
            events_processed: 100,
        };
        let result = state.start_loading();

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_start_loading_from_failed() {
        let state = ReplayState::Failed {
            error: "test".into(),
        };
        let result = state.start_loading();

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_start_replaying_from_uninitialized() {
        let state = ReplayState::Uninitialized;
        let result = state.start_replaying(100);

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_start_replaying_from_replaying() {
        let state = ReplayState::Replaying {
            events_processed: 50,
            events_total: 100,
        };
        let result = state.start_replaying(200);

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_start_replaying_from_complete() {
        let state = ReplayState::Complete {
            events_processed: 100,
        };
        let result = state.start_replaying(100);

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_update_progress_from_loading() {
        let state = ReplayState::Loading { events_loaded: 50 };
        let result = state.update_progress(75);

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_update_progress_from_complete() {
        let state = ReplayState::Complete {
            events_processed: 100,
        };
        let result = state.update_progress(50);

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_complete_from_uninitialized() {
        let state = ReplayState::Uninitialized;
        let result = state.complete();

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_complete_from_loading() {
        let state = ReplayState::Loading { events_loaded: 100 };
        let result = state.complete();

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_complete_from_complete() {
        let state = ReplayState::Complete {
            events_processed: 100,
        };
        let result = state.complete();

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_complete_from_failed() {
        let state = ReplayState::Failed {
            error: "test".into(),
        };
        let result = state.complete();

        assert!(result.is_err());
    }

    // ==========================================================================
    // STATE QUERIES
    // ==========================================================================

    #[test]
    fn test_is_terminal() {
        assert!(!ReplayState::Uninitialized.is_terminal());
        assert!(!ReplayState::Loading { events_loaded: 0 }.is_terminal());
        assert!(!ReplayState::Replaying {
            events_processed: 0,
            events_total: 100
        }
        .is_terminal());
        assert!(ReplayState::Complete {
            events_processed: 100
        }
        .is_terminal());
        assert!(ReplayState::Failed {
            error: "test".into()
        }
        .is_terminal());
    }

    #[test]
    fn test_is_active() {
        assert!(!ReplayState::Uninitialized.is_active());
        assert!(ReplayState::Loading { events_loaded: 0 }.is_active());
        assert!(ReplayState::Replaying {
            events_processed: 0,
            events_total: 100
        }
        .is_active());
        assert!(!ReplayState::Complete {
            events_processed: 100
        }
        .is_active());
        assert!(!ReplayState::Failed {
            error: "test".into()
        }
        .is_active());
    }

    #[test]
    fn test_description() {
        assert_eq!(ReplayState::Uninitialized.description(), "Not started");
        assert_eq!(
            ReplayState::Loading { events_loaded: 50 }.description(),
            "Loading events"
        );
        assert_eq!(
            ReplayState::Replaying {
                events_processed: 25,
                events_total: 100
            }
            .description(),
            "Replaying events"
        );
        assert_eq!(
            ReplayState::Complete {
                events_processed: 100
            }
            .description(),
            "Complete"
        );
        assert_eq!(
            ReplayState::Failed {
                error: "test".into()
            }
            .description(),
            "Failed"
        );
    }

    // ==========================================================================
    // FULL LIFECYCLE TESTS
    // ==========================================================================

    #[test]
    fn test_successful_replay_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        // Start
        let mut state = ReplayState::Uninitialized;

        // Start loading
        state = state.start_loading()?;
        assert!(matches!(state, ReplayState::Loading { .. }));

        // Finish loading and start replaying
        state = state.start_replaying(100)?;
        assert!(matches!(state, ReplayState::Replaying { .. }));

        // Update progress
        state = state.update_progress(50)?;
        assert!(matches!(state, ReplayState::Replaying { .. }));

        // Complete
        state = state.complete()?;
        assert!(matches!(state, ReplayState::Complete { .. }));
        assert!(state.is_terminal());
        Ok(())
    }

    #[test]
    fn test_failed_replay_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        // Start
        let mut state = ReplayState::Uninitialized;

        // Start loading
        state = state.start_loading()?;

        // Fail during loading
        state = state.fail("load error".into());
        assert!(matches!(state, ReplayState::Failed { .. }));
        assert!(state.is_terminal());
        assert_eq!(
            state,
            ReplayState::Failed {
                error: "load error".into()
            }
        );
        Ok(())
    }

    #[test]
    fn test_failed_during_replaying() -> Result<(), Box<dyn std::error::Error>> {
        let mut state = ReplayState::Uninitialized;

        state = state.start_loading()?;
        state = state.start_replaying(100)?;
        state = state.update_progress(50)?;

        // Fail during replaying
        state = state.fail("projection error".into());
        assert!(matches!(state, ReplayState::Failed { .. }));
        assert!(state.is_terminal());
        Ok(())
    }
}
