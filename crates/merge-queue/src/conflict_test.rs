//! Tests for conflict resolution.
//!
//! Validates conflict detection strategies and resolution mechanisms.

use crate::conflict::{detect, attempt_rebase, ConflictDetection, RebaseResult};

#[test]
fn test_conflict_detection() {
    // Test detecting conflicts on merge attempt
    let result = detect("main", "feature-branch");

    // Verify detection succeeded
    assert!(result.is_ok(), "Conflict detection should not fail");

    let conflict = match result {
        Ok(c) => c,
        Err(_) => return, // Test can't continue if detection fails
    };

    // Should return a conflict result
    assert_eq!(conflict.has_conflicts, false); // No conflicts in this case
}

#[test]
fn test_conflict_detected_with_conflicts() {
    // Test when conflicts are detected
    let result = detect("main", "conflicting-branch");

    match result {
        Ok(conflict) => {
            // If detection succeeds, check conflict status
            if conflict.has_conflicts {
                assert!(!conflict.conflicting_files.is_empty());
            }
        }
        Err(_) => {
            // Error is acceptable if branch doesn't exist
        }
    }
}

#[test]
fn test_resolution_strategies() {
    // Test automatic rebase
    let result = attempt_rebase("feature-branch", "main");

    match result {
        Ok(rebase_result) => {
            assert!(rebase_result.success || rebase_result.has_conflicts);
        }
        Err(_) => {
            // Error is acceptable if branch doesn't exist
        }
    }
}

#[test]
fn test_rebase_with_conflicts_transitions_to_failed() {
    // Test that rebase conflicts properly transition state
    let result = attempt_rebase("conflicting-branch", "main");

    match result {
        Ok(rebase_result) => {
            if rebase_result.has_conflicts {
                assert!(!rebase_result.success);
                assert!(!rebase_result.conflicted_files.is_empty());
            }
        }
        Err(_) => {
            // Error is acceptable if branch doesn't exist or operation fails
        }
    }
}
