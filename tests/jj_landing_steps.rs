//! JJ Landing Step Functions Acceptance Tests
//!
//! ATDD tests for bead src-2cq: Replace zjj landing steps with jj commands
//!
//! These tests specify the contract for new step functions that replace
//! zjj_sync_step() and zjj_done_step() with direct jj commands.

use oya::landing::{
    jj_bookmark_set_step, jj_fetch_step, jj_git_push_step, jj_rebase_step, jj_workspace_forget_step,
};
use oya::types::FailureCategory;
use oya::types::StageName;

// =============================================================================
// CONTRACT: jj_fetch_step
// =============================================================================

/// Contract: jj_fetch_step returns CommandStep for `jj git fetch`
///
/// Replaces: Part of zjj_sync functionality
/// Semantics: Fetch latest from remote before rebase
#[test]
fn test_jj_fetch_step_returns_correct_command_step() {
    let step = jj_fetch_step();

    assert_eq!(step.id, "jj_fetch");
    assert_eq!(step.label, "jj git fetch");
    assert_eq!(step.program, "jj");
    assert_eq!(step.args, vec!["git", "fetch"]);
    assert_eq!(step.failure_category, FailureCategory::MergeConflict);
    assert_eq!(step.next_stage, StageName::Implementation);
    assert!(step.timeout_seconds > 0, "timeout must be positive");
}

/// Contract: jj_fetch_step has reasonable timeout for network operation
#[test]
fn test_jj_fetch_step_has_reasonable_timeout() {
    let step = jj_fetch_step();

    assert!(step.timeout_seconds >= 30, "Network fetch should have at least 30s timeout");
    assert!(step.timeout_seconds <= 300, "Network fetch timeout should not exceed 5 minutes");
}

// =============================================================================
// CONTRACT: jj_rebase_step
// =============================================================================

/// Contract: jj_rebase_step returns CommandStep for `jj rebase -s <workspace> -d main`
///
/// Replaces: Part of zjj_sync functionality (syncing workspace with main)
/// Semantics: Rebase workspace changes onto current main
#[test]
fn test_jj_rebase_step_returns_correct_command_step() {
    let bead_id = "src-2cq";
    let step = jj_rebase_step(bead_id);

    assert_eq!(step.id, "jj_rebase");
    assert_eq!(step.label, "jj rebase onto main");
    assert_eq!(step.program, "jj");
    assert!(step.args.contains(&"rebase".to_string()), "must contain 'rebase' arg");
    assert!(step.args.contains(&"-s".to_string()), "must contain '-s' source flag");
    assert!(step.args.contains(&"-d".to_string()), "must contain '-d' destination flag");
    assert!(step.args.contains(&"main".to_string()), "must rebase onto 'main'");
    assert_eq!(step.failure_category, FailureCategory::MergeConflict);
    assert_eq!(step.next_stage, StageName::Implementation);
}

/// Contract: jj_rebase_step includes workspace name derived from bead_id
#[test]
fn test_jj_rebase_step_includes_workspace_name() {
    let bead_id = "src-abc";
    let step = jj_rebase_step(bead_id);

    let expected_workspace = format!("oya-{}", bead_id);
    assert!(
        step.args.contains(&expected_workspace),
        "rebase source should be workspace 'oya-{}', got args: {:?}",
        bead_id,
        step.args
    );
}

/// Contract: jj_rebase_step has timeout suitable for rebase operation
#[test]
fn test_jj_rebase_step_has_reasonable_timeout() {
    let step = jj_rebase_step("test-bead");

    assert!(step.timeout_seconds >= 30, "Rebase should have at least 30s timeout");
    assert!(step.timeout_seconds <= 180, "Rebase timeout should not exceed 3 minutes");
}

// =============================================================================
// CONTRACT: jj_bookmark_set_step
// =============================================================================

/// Contract: jj_bookmark_set_step returns CommandStep for creating bookmark
///
/// Replaces: Part of zjj_done functionality (bookmark creation)
/// Semantics: Create bookmark for workspace changes
#[test]
fn test_jj_bookmark_set_step_returns_correct_command_step() {
    let bead_id = "src-2cq";
    let step = jj_bookmark_set_step(bead_id);

    assert_eq!(step.id, "jj_bookmark_set");
    assert_eq!(step.label, "jj bookmark set");
    assert_eq!(step.program, "jj");
    assert!(step.args.contains(&"bookmark".to_string()), "must contain 'bookmark' arg");
    assert_eq!(step.failure_category, FailureCategory::MergeConflict);
    assert_eq!(step.next_stage, StageName::Implementation);
}

/// Contract: jj_bookmark_set_step uses bead_id in bookmark name
#[test]
fn test_jj_bookmark_set_step_uses_bead_id_in_bookmark_name() {
    let bead_id = "src-xyz";
    let step = jj_bookmark_set_step(bead_id);

    let expected_bookmark = format!("oya-{}", bead_id);
    assert!(
        step.args.contains(&expected_bookmark),
        "bookmark name should be 'oya-{}', got args: {:?}",
        bead_id,
        step.args
    );
}

/// Contract: jj_bookmark_set_step has reasonable timeout
#[test]
fn test_jj_bookmark_set_step_has_reasonable_timeout() {
    let step = jj_bookmark_set_step("test-bead");

    assert!(step.timeout_seconds >= 10, "Bookmark operation should have at least 10s timeout");
    assert!(step.timeout_seconds <= 60, "Bookmark operation should not exceed 60s timeout");
}

// =============================================================================
// CONTRACT: jj_git_push_step
// =============================================================================

/// Contract: jj_git_push_step returns CommandStep for `jj git push`
///
/// Replaces: Part of zjj_done functionality (pushing to remote)
/// Semantics: Push bookmark to remote
#[test]
fn test_jj_git_push_step_returns_correct_command_step() {
    let bead_id = "src-2cq";
    let step = jj_git_push_step(bead_id);

    assert_eq!(step.id, "jj_git_push");
    assert_eq!(step.label, "jj git push");
    assert_eq!(step.program, "jj");
    assert!(step.args.contains(&"git".to_string()), "must contain 'git' arg");
    assert!(step.args.contains(&"push".to_string()), "must contain 'push' arg");
    assert_eq!(step.failure_category, FailureCategory::MergeConflict);
    assert_eq!(step.next_stage, StageName::Implementation);
}

/// Contract: jj_git_push_step pushes specific bookmark
#[test]
fn test_jj_git_push_step_pushes_specific_bookmark() {
    let bead_id = "src-123";
    let step = jj_git_push_step(bead_id);

    let expected_bookmark = format!("oya-{}", bead_id);
    assert!(
        step.args.contains(&expected_bookmark),
        "push should target bookmark 'oya-{}', got args: {:?}",
        bead_id,
        step.args
    );
}

/// Contract: jj_git_push_step has timeout suitable for network push
#[test]
fn test_jj_git_push_step_has_reasonable_timeout() {
    let step = jj_git_push_step("test-bead");

    assert!(step.timeout_seconds >= 30, "Git push should have at least 30s timeout");
    assert!(step.timeout_seconds <= 180, "Git push should not exceed 3 minutes timeout");
}

// =============================================================================
// CONTRACT: jj_workspace_forget_step
// =============================================================================

/// Contract: jj_workspace_forget_step returns CommandStep for `jj workspace forget`
///
/// Replaces: Part of zjj_done functionality (cleanup)
/// Semantics: Remove workspace after successful merge
#[test]
fn test_jj_workspace_forget_step_returns_correct_command_step() {
    let bead_id = "src-2cq";
    let step = jj_workspace_forget_step(bead_id);

    assert_eq!(step.id, "jj_workspace_forget");
    assert_eq!(step.label, "jj workspace forget");
    assert_eq!(step.program, "jj");
    assert!(step.args.contains(&"workspace".to_string()), "must contain 'workspace' arg");
    assert!(step.args.contains(&"forget".to_string()), "must contain 'forget' arg");
    assert_eq!(step.failure_category, FailureCategory::MergeConflict);
    assert_eq!(step.next_stage, StageName::Implementation);
}

/// Contract: jj_workspace_forget_step forgets specific workspace
#[test]
fn test_jj_workspace_forget_step_forgets_specific_workspace() {
    let bead_id = "src-forget";
    let step = jj_workspace_forget_step(bead_id);

    let expected_workspace = format!("oya-{}", bead_id);
    assert!(
        step.args.contains(&expected_workspace),
        "forget should target workspace 'oya-{}', got args: {:?}",
        bead_id,
        step.args
    );
}

/// Contract: jj_workspace_forget_step has reasonable timeout
#[test]
fn test_jj_workspace_forget_step_has_reasonable_timeout() {
    let step = jj_workspace_forget_step("test-bead");

    assert!(step.timeout_seconds >= 10, "Workspace forget should have at least 10s timeout");
    assert!(step.timeout_seconds <= 60, "Workspace forget should not exceed 60s timeout");
}

// =============================================================================
// MERGE SEMANTICS PRESERVATION
// =============================================================================

/// Contract: All jj landing steps use MergeConflict failure category
///
/// This preserves the semantics from zjj_sync_step and zjj_done_step
/// which both used FailureCategory::MergeConflict
#[test]
fn test_all_jj_landing_steps_use_merge_conflict_category() {
    let bead_id = "test-bead";

    let steps = [
        jj_fetch_step(),
        jj_rebase_step(bead_id),
        jj_bookmark_set_step(bead_id),
        jj_git_push_step(bead_id),
        jj_workspace_forget_step(bead_id),
    ];

    for step in &steps {
        assert_eq!(
            step.failure_category,
            FailureCategory::MergeConflict,
            "Step '{}' must use MergeConflict failure category to preserve zjj semantics",
            step.id
        );
    }
}

/// Contract: All jj landing steps return to Implementation stage on failure
///
/// This preserves the semantics from zjj_sync_step and zjj_done_step
/// which both used Stage::Implementation as next_stage
#[test]
fn test_all_jj_landing_steps_return_to_implementation_stage() {
    let bead_id = "test-bead";

    let steps = [
        jj_fetch_step(),
        jj_rebase_step(bead_id),
        jj_bookmark_set_step(bead_id),
        jj_git_push_step(bead_id),
        jj_workspace_forget_step(bead_id),
    ];

    for step in &steps {
        assert_eq!(
            step.next_stage,
            StageName::Implementation,
            "Step '{}' must return to Implementation stage on failure",
            step.id
        );
    }
}

/// Contract: Landing step sequence replaces zjj_sync + zjj_done
///
/// The sequence [fetch, rebase, bookmark, push, forget] must provide
/// equivalent semantics to [zjj sync, zjj done]
#[test]
fn test_landing_step_sequence_provides_equivalent_semantics() {
    let bead_id = "test-equivalence";

    let jj_steps: Vec<(&str, u64)> = vec![
        ("jj_fetch", jj_fetch_step().timeout_seconds),
        ("jj_rebase", jj_rebase_step(bead_id).timeout_seconds),
        ("jj_bookmark_set", jj_bookmark_set_step(bead_id).timeout_seconds),
        ("jj_git_push", jj_git_push_step(bead_id).timeout_seconds),
        ("jj_workspace_forget", jj_workspace_forget_step(bead_id).timeout_seconds),
    ];

    assert_eq!(jj_steps.len(), 5, "Landing phase must have exactly 5 jj steps");

    let total_timeout: u64 = jj_steps.iter().map(|(_, t)| *t).sum();
    assert!(
        total_timeout <= 600,
        "Total landing timeout should be reasonable (< 10 min), got {}s",
        total_timeout
    );
}

// =============================================================================
// BEAD ID VALIDATION
// =============================================================================

/// Contract: Steps with bead_id must handle various valid bead_id formats
#[test]
fn test_jj_steps_handle_valid_bead_ids() {
    let valid_bead_ids = ["src-2cq", "src-abc", "feature-123", "bugfix-xyz-1"];

    for bead_id in &valid_bead_ids {
        let rebase = jj_rebase_step(bead_id);
        assert!(rebase.args.iter().any(|a| a.contains(bead_id)));

        let bookmark = jj_bookmark_set_step(bead_id);
        assert!(bookmark.args.iter().any(|a| a.contains(bead_id)));

        let push = jj_git_push_step(bead_id);
        assert!(push.args.iter().any(|a| a.contains(bead_id)));

        let forget = jj_workspace_forget_step(bead_id);
        assert!(forget.args.iter().any(|a| a.contains(bead_id)));
    }
}
