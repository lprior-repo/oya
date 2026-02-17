//! Restate Workflow Orchestration for Canonical Stage Transitions
//!
//! ## Contract
//! - **Preconditions**: Run exists, stage completed, next stage is canonical
//! - **Postconditions**: State transitioned, persisted, workflow progressed
//! - **Invariants**: Monotonic progress, bounded attempts, terminal absorbing
//!
//! ## Type Safety
//! - Pure functions for validation (no I/O, no side effects)
//! - Result<T, WorkflowError> for all fallible operations
//! - Zero unwrap/panic/expect throughout

use crate::domain::{FailureCategory, RunState, StageName as Stage};
use std::time::Duration;
use thiserror::Error;

// =============================================================================
// Error Taxonomy (Exhaustive, Semantic)
// =============================================================================

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Run not found: {0}")]
    RunNotFound(String),

    #[error("Invalid state transition: from '{from}' to '{to}'")]
    InvalidTransition { from: String, to: String },

    #[error("Attempt limit exceeded for stage '{stage}': attempt {attempt} exceeds max {max}")]
    AttemptLimitExceeded { stage: String, attempt: u32, max: u32 },

    #[error("Non-canonical stage transition: from '{from}' to '{to}'")]
    NonCanonicalTransition { from: String, to: String },

    #[error("Non-retryable failure: category '{category}', reason: {reason}")]
    NonRetryableFailure { category: String, reason: String },

    #[error(
        "State corruption detected: workflow='{workflow_state}', persistence='{persistent_state}'"
    )]
    StateCorruption { workflow_state: String, persistent_state: String },

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("Restate SDK error: {0}")]
    Restate(String),

    #[error("Context overflow: size {size_bytes} bytes exceeds max {max_bytes} bytes")]
    ContextOverflow { size_bytes: usize, max_bytes: usize },

    #[error("Concurrent modification: expected version {expected_version}, got {actual_version}")]
    ConcurrentModification { expected_version: u64, actual_version: u64 },
}

// =============================================================================
// Retry Action (Result Type)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryAction {
    Scheduled { backoff_duration: Duration, next_stage: Stage },
    TerminalFailure { reason: String },
}

// =============================================================================
// Terminal State (Explicit)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalState {
    Shipped,
    Failed,
    Aborted,
}

// =============================================================================
// Pure Core Functions (No I/O, No Side Effects, Total)
// =============================================================================

/// Get next canonical stage in DAG
///
/// Pure function: deterministic mapping from stage to next stage
/// Contract: Returns Some(next) if not terminal, None for ShipGate
#[must_use]
pub fn get_next_canonical_stage(stage: Stage) -> Option<Stage> {
    stage.next()
}

/// Validate stage transition is canonical
///
/// Pure function: checks if to_stage is the canonical next step
/// Contract: Returns true only for valid sequential transitions
#[must_use]
pub fn is_canonical_transition(from_stage: Stage, to_stage: Stage) -> bool {
    get_next_canonical_stage(from_stage) == Some(to_stage)
}

/// Calculate retry backoff duration (exponential)
///
/// Pure function: deterministic backoff calculation
/// Contract: 2^attempt seconds, bounded at 300 seconds max
#[must_use]
pub fn calculate_backoff(attempt_number: u32) -> Duration {
    const MAX_BACKOFF_SECS: u64 = 300; // 5 minutes

    // Exponential backoff: 2^attempt seconds
    let backoff_secs = u64::from(2u32)
        .checked_pow(attempt_number)
        .map(|secs| secs.min(MAX_BACKOFF_SECS))
        .unwrap_or(MAX_BACKOFF_SECS);

    Duration::from_secs(backoff_secs)
}

/// Check if failure category is retryable
///
/// Pure function: categorizes failure types
/// Contract: Returns true for retryable, false for terminal failures
#[must_use]
pub fn is_retryable_failure(category: &FailureCategory) -> bool {
    match category {
        // Retryable failures (transient)
        FailureCategory::TestFailed
        | FailureCategory::TestInfraFailed
        | FailureCategory::CompileFailed
        | FailureCategory::LintFailed
        | FailureCategory::MergeConflict
        | FailureCategory::RateLimited => true,

        // Terminal failures (cannot retry)
        FailureCategory::AuthFailed
        | FailureCategory::ContextOverflow
        | FailureCategory::ProviderUnavailable
        | FailureCategory::OutputParseFailure
        | FailureCategory::MaxAttemptsExceeded => false,
    }
}

/// Validate run state can transition to next stage
///
/// Pure function: state machine validation
/// Contract: Returns Ok(()) if transition is valid, Err otherwise
pub fn validate_transition(
    current_state: &RunState,
    next_stage: Stage,
) -> Result<(), WorkflowError> {
    match current_state {
        RunState::Pending => {
            // Pending can only transition to Running(Research)
            if next_stage == Stage::Research {
                Ok(())
            } else {
                Err(WorkflowError::InvalidTransition {
                    from: "Pending".to_string(),
                    to: format!("Running({:?})", next_stage),
                })
            }
        }
        RunState::Running { current_stage } => {
            // Running can transition to next canonical stage
            let next_stage_for_error = next_stage.clone();
            if is_canonical_transition(current_stage.clone(), next_stage) {
                Ok(())
            } else {
                Err(WorkflowError::NonCanonicalTransition {
                    from: format!("{:?}", current_stage),
                    to: format!("{:?}", next_stage_for_error),
                })
            }
        }
        RunState::Waiting { .. } => {
            // Waiting can transition to Running (retry lane)
            if next_stage == Stage::Tdd15 {
                Ok(())
            } else {
                Err(WorkflowError::InvalidTransition {
                    from: "Waiting".to_string(),
                    to: format!("Running({:?})", next_stage),
                })
            }
        }
        RunState::Shipped { .. } | RunState::Failed { .. } | RunState::Aborted { .. } => {
            // Terminal states have no outgoing transitions
            Err(WorkflowError::InvalidTransition {
                from: format!("{:?}", current_state),
                to: format!("Running({:?})", next_stage),
            })
        }
    }
}

/// Determine retry action based on failure and attempt count
///
/// Pure function: retry policy evaluation
/// Contract: Returns RetryAction with backoff or terminal failure
pub fn determine_retry_action(
    failed_stage: Stage,
    attempt: u32,
    failure: &FailureCategory,
    reason: &str,
) -> Result<RetryAction, WorkflowError> {
    let max_attempts = failed_stage.max_attempts();

    // Check if failure is retryable
    if !is_retryable_failure(failure) {
        return Ok(RetryAction::TerminalFailure {
            reason: format!(
                "Non-retryable failure: {} - {}",
                failure_category_to_string(failure),
                reason
            ),
        });
    }

    // Check if attempt limit exceeded
    if attempt >= max_attempts {
        return Err(WorkflowError::AttemptLimitExceeded {
            stage: stage_to_string(failed_stage),
            attempt,
            max: max_attempts,
        });
    }

    // Schedule retry with backoff
    let backoff_duration = calculate_backoff(attempt);

    Ok(RetryAction::Scheduled {
        backoff_duration,
        next_stage: Stage::Tdd15, // Retry lane entry point
    })
}

/// Check if state is terminal (absorbing)
///
/// Pure function: state classification
/// Contract: Returns true for Shipped/Failed/Aborted
#[must_use]
pub fn is_terminal_state(state: &RunState) -> bool {
    matches!(state, RunState::Shipped { .. } | RunState::Failed { .. } | RunState::Aborted { .. })
}

// =============================================================================
// Helper Functions (Pure)
// =============================================================================

fn stage_to_string(stage: Stage) -> String {
    stage.as_str().to_string()
}

fn failure_category_to_string(category: &FailureCategory) -> &'static str {
    match category {
        FailureCategory::TestFailed => "test_failed",
        FailureCategory::TestInfraFailed => "test_infra_failed",
        FailureCategory::CompileFailed => "compile_failed",
        FailureCategory::LintFailed => "lint_failed",
        FailureCategory::MergeConflict => "merge_conflict",
        FailureCategory::RateLimited => "rate_limited",
        FailureCategory::AuthFailed => "auth_failed",
        FailureCategory::ContextOverflow => "context_overflow",
        FailureCategory::ProviderUnavailable => "provider_unavailable",
        FailureCategory::OutputParseFailure => "output_parse_failure",
        FailureCategory::MaxAttemptsExceeded => "max_attempts_exceeded",
    }
}

// =============================================================================
// Async Shell: Restate Workflow Handlers
// =============================================================================

/*
#[cfg(feature = "restate")]
pub mod restate_handlers {
...
}
*/

// =============================================================================
// Tests (Pure Function Verification)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_next_canonical_stage_returns_correct_sequence() {
        assert_eq!(get_next_canonical_stage(Stage::Research), Some(Stage::Plan));
        assert_eq!(get_next_canonical_stage(Stage::Plan), Some(Stage::Contract));
        assert_eq!(get_next_canonical_stage(Stage::Contract), Some(Stage::Tdd15));
        assert_eq!(get_next_canonical_stage(Stage::Tdd15), Some(Stage::Qa));
        assert_eq!(get_next_canonical_stage(Stage::Qa), Some(Stage::RedQueen));
        assert_eq!(get_next_canonical_stage(Stage::RedQueen), Some(Stage::GptReview));
        assert_eq!(get_next_canonical_stage(Stage::GptReview), Some(Stage::ShipGate));
        assert_eq!(get_next_canonical_stage(Stage::ShipGate), None);
    }

    #[test]
    fn test_is_canonical_transition_validates_stage_sequence() {
        assert!(is_canonical_transition(Stage::Research, Stage::Plan));
        assert!(is_canonical_transition(Stage::Plan, Stage::Contract));
        assert!(is_canonical_transition(Stage::Contract, Stage::Tdd15));
        assert!(is_canonical_transition(Stage::Tdd15, Stage::Qa));
        assert!(!is_canonical_transition(Stage::Plan, Stage::Tdd15)); // Skips Contract
        assert!(!is_canonical_transition(Stage::ShipGate, Stage::Contract)); // Backwards
    }

    #[test]
    fn test_calculate_backoff_returns_exponential_backoff() {
        assert_eq!(calculate_backoff(1), Duration::from_secs(2));
        assert_eq!(calculate_backoff(2), Duration::from_secs(4));
        assert_eq!(calculate_backoff(3), Duration::from_secs(8));
        assert_eq!(calculate_backoff(4), Duration::from_secs(16));
    }

    #[test]
    fn test_calculate_backoff_is_bounded_at_max() {
        assert_eq!(calculate_backoff(10), Duration::from_secs(300)); // Max 5 minutes
        assert_eq!(calculate_backoff(100), Duration::from_secs(300));
    }

    #[test]
    fn test_is_retryable_failure_categorizes_correctly() {
        // Retryable
        assert!(is_retryable_failure(&FailureCategory::TestFailed));
        assert!(is_retryable_failure(&FailureCategory::CompileFailed));
        assert!(is_retryable_failure(&FailureCategory::LintFailed));
        assert!(is_retryable_failure(&FailureCategory::RateLimited));

        // Non-retryable
        assert!(!is_retryable_failure(&FailureCategory::AuthFailed));
        assert!(!is_retryable_failure(&FailureCategory::ContextOverflow));
        assert!(!is_retryable_failure(&FailureCategory::ProviderUnavailable));
    }

    #[test]
    fn test_validate_transition_allows_valid_transitions() {
        assert!(validate_transition(&RunState::Pending, Stage::Research).is_ok());

        let running_contract = RunState::Running { current_stage: Stage::Contract };
        assert!(validate_transition(&running_contract, Stage::Tdd15).is_ok());
    }

    #[test]
    fn test_validate_transition_blocks_invalid_transitions() {
        // Pending can't skip to Plan
        assert!(validate_transition(&RunState::Pending, Stage::Plan).is_err());

        // Pending can't skip to Tdd15
        assert!(validate_transition(&RunState::Pending, Stage::Tdd15).is_err());

        // Can't skip stages
        let running_contract = RunState::Running { current_stage: Stage::Contract };
        assert!(validate_transition(&running_contract, Stage::Qa).is_err());

        // Terminal states can't transition
        let shipped = RunState::Shipped { completed_at: chrono::Utc::now() };
        assert!(validate_transition(&shipped, Stage::Contract).is_err());
    }

    #[test]
    fn test_determine_retry_action_schedules_retry_for_retryable_failure() {
        let result =
            determine_retry_action(Stage::Qa, 1, &FailureCategory::TestFailed, "Test X failed");

        assert!(matches!(&result, Ok(RetryAction::Scheduled { .. })));
        if let Ok(RetryAction::Scheduled { backoff_duration, next_stage }) = result {
            assert_eq!(backoff_duration, Duration::from_secs(2));
            assert_eq!(next_stage, Stage::Tdd15);
        }
    }

    #[test]
    fn test_determine_retry_action_returns_terminal_for_non_retryable() {
        let result = determine_retry_action(
            Stage::Contract,
            1,
            &FailureCategory::AuthFailed,
            "Invalid credentials",
        );

        assert!(matches!(&result, Ok(RetryAction::TerminalFailure { .. })));
        if let Ok(RetryAction::TerminalFailure { reason }) = result {
            assert!(reason.contains("Non-retryable failure"));
        }
    }

    #[test]
    fn test_determine_retry_action_errors_on_max_attempts() {
        let result = determine_retry_action(
            Stage::Contract,
            3, // Max attempts
            &FailureCategory::TestFailed,
            "Still failing",
        );

        assert!(matches!(
            result,
            Err(WorkflowError::AttemptLimitExceeded { stage: _, attempt: 3, max: 3 })
        ));
    }

    #[test]
    fn test_is_terminal_state_identifies_terminal_states() {
        assert!(is_terminal_state(&RunState::Shipped { completed_at: chrono::Utc::now() }));
        assert!(is_terminal_state(&RunState::Failed {
            reason: String::new(),
            failed_at: chrono::Utc::now()
        }));
        assert!(is_terminal_state(&RunState::Aborted {
            reason: String::new(),
            aborted_at: chrono::Utc::now()
        }));

        assert!(!is_terminal_state(&RunState::Pending));
        assert!(!is_terminal_state(&RunState::Running { current_stage: Stage::Contract }));
    }
}
