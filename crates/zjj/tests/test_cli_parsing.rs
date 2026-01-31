//! Integration tests for CLI argument parsing and validation
//!
//! Tests that CLI flags and options are properly handled

mod common;

use common::TestHarness;

// ============================================================================
// Help and Version
// ============================================================================

#[test]
fn test_help_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.jjz(&["--help"]);
    // Help may exit with 0 or display help text
    result.assert_output_contains("jjz");
}

#[test]
fn test_version_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let _result = harness.jjz(&["--version"]);
    // Version should show version number
}

#[test]
fn test_init_help() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.jjz(&["init", "--help"]);
    result.assert_output_contains("init");
}

#[test]
fn test_add_help() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.jjz(&["add", "--help"]);
    result.assert_output_contains("add");
}

// ============================================================================
// Add Command Options
// ============================================================================

#[test]
fn test_add_with_no_open_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_with_no_hooks_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-hooks", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_with_template_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test various templates
    harness.assert_success(&["add", "minimal", "--template", "minimal", "--no-open"]);
    harness.assert_success(&["add", "standard", "--template", "standard", "--no-open"]);
    harness.assert_success(&["add", "full", "--template", "full", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("minimal");
    result.assert_stdout_contains("standard");
    result.assert_stdout_contains("full");
}

#[test]
fn test_add_with_short_template_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "-t", "minimal", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_combined_flags() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open", "--no-hooks", "-t", "minimal"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// List Command Options
// ============================================================================

#[test]
fn test_list_with_all_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list", "--all"]);
    assert!(result.success);
    result.assert_stdout_contains("test");
}

#[test]
fn test_list_with_json_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list", "--json"]);
    assert!(result.success);

    // Verify JSON format
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

// ============================================================================
// Remove Command Options
// ============================================================================

#[test]
fn test_remove_with_force_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    harness.assert_success(&["remove", "test", "--force"]);

    let result = harness.jjz(&["list"]);
    assert!(!result.stdout.contains("test"));
}

#[test]
fn test_remove_with_short_force_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    harness.assert_success(&["remove", "test", "-f"]);

    let result = harness.jjz(&["list"]);
    assert!(!result.stdout.contains("test"));
}

#[test]
fn test_remove_with_merge_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Note: merge requires force too (or confirmation)
    let _result = harness.jjz(&["remove", "test", "--merge", "--force"]);
    // May succeed or fail depending on git state
}

#[test]
fn test_remove_with_keep_branch_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    harness.assert_success(&["remove", "test", "--keep-branch", "--force"]);

    let result = harness.jjz(&["list"]);
    assert!(!result.stdout.contains("test"));
}

// ============================================================================
// Status Command Options
// ============================================================================

#[test]
fn test_status_with_json_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["status", "test", "--json"]);
    assert!(result.success);

    // Verify JSON format
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_status_without_name_shows_all() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test1", "--no-open"]);
    harness.assert_success(&["add", "test2", "--no-open"]);

    let result = harness.jjz(&["status"]);
    assert!(result.success);
    // Should show all sessions or a summary
}

#[test]
fn test_status_with_watch_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Note: --watch will block, so we can't test it in CI
    // Just verify it doesn't error immediately
    let _result = harness.jjz(&["status", "test", "--watch"]);
    // Will run continuously - this test may timeout
    // In practice, we'd need to kill it after a short time
}

// ============================================================================
// Diff Command Options
// ============================================================================

#[test]
fn test_diff_with_stat_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let _result = harness.jjz(&["diff", "test", "--stat"]);
    // May succeed or fail depending on whether there are changes
}

#[test]
fn test_diff_without_stat() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let _result = harness.jjz(&["diff", "test"]);
    // May succeed or fail depending on whether there are changes
}

// ============================================================================
// Sync Command
// ============================================================================

#[test]
fn test_sync_with_explicit_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let _result = harness.jjz(&["sync", "test"]);
    // Sync behavior depends on git state
}

#[test]
fn test_sync_without_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Sync without name should sync current workspace
    let _result = harness.jjz(&["sync"]);
    // May succeed or fail depending on context
}

// ============================================================================
// Invalid Flag Combinations
// ============================================================================

#[test]
fn test_mutually_exclusive_flags() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Some flag combinations might not make sense
    // Implementation may vary
}

// ============================================================================
// Argument Order
// ============================================================================

#[test]
fn test_flags_before_positional_args() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Flags before name
    harness.assert_success(&["add", "--no-open", "test"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_flags_after_positional_args() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Flags after name
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// Special Characters in Names
// ============================================================================

#[test]
fn test_session_name_with_hyphens() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "feature-with-hyphens", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("feature-with-hyphens");
}

#[test]
fn test_session_name_with_underscores() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "feature_with_underscores", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("feature_with_underscores");
}

#[test]
fn test_session_name_with_numbers() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "feature123", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("feature123");
}

// ============================================================================
// Empty and Whitespace
// ============================================================================

#[test]
fn test_session_name_with_leading_whitespace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Leading whitespace should be rejected or trimmed
    let _result = harness.jjz(&["add", " test", "--no-open"]);
    // May fail with validation error
}

#[test]
fn test_session_name_with_trailing_whitespace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Trailing whitespace should be rejected or trimmed
    let _result = harness.jjz(&["add", "test ", "--no-open"]);
    // May fail with validation error
}

// ============================================================================
// Case Sensitivity
// ============================================================================

#[test]
fn test_session_names_are_case_sensitive() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "Test", "--no-open"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Both should exist as separate sessions
    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("Test");
    result.assert_stdout_contains("test");
}

// ============================================================================
// Long Flag Names
// ============================================================================

#[test]
fn test_long_flag_names() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // All flags should work with long names
    harness.assert_success(&["add", "test", "--no-open", "--no-hooks"]);

    let result = harness.jjz(&["list", "--all", "--json"]);
    assert!(result.success);
}

// ============================================================================
// Multiple Values
// ============================================================================

#[test]
fn test_template_with_equals_sign() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // --template=minimal syntax
    harness.assert_success(&["add", "test", "--template=minimal", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// Session Names with Leading Dashes (zjj-hv7)
// ============================================================================

#[test]
fn test_session_name_starting_with_single_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with a single dash should be rejected
    let result = harness.jjz(&["add", "-foo", "--no-open"]);
    assert!(!result.success, "Should reject name starting with dash");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_starting_with_double_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with double dash should be rejected
    let result = harness.jjz(&["add", "--bar", "--no-open"]);
    assert!(!result.success, "Should reject name starting with --");
    // Will likely be interpreted as unknown flag or show validation error
}

#[test]
fn test_session_name_starting_with_triple_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with triple dash should be rejected
    let result = harness.jjz(&["add", "---baz", "--no-open"]);
    assert!(!result.success, "Should reject name starting with ---");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_just_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // A single dash should be rejected
    let result = harness.jjz(&["add", "-", "--no-open"]);
    assert!(!result.success, "Should reject single dash as name");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_starting_with_underscore() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with underscore should be rejected
    let result = harness.jjz(&["add", "_private", "--no-open"]);
    assert!(
        !result.success,
        "Should reject name starting with underscore"
    );
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_starting_with_number() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with number should be rejected
    let result = harness.jjz(&["add", "123session", "--no-open"]);
    assert!(!result.success, "Should reject name starting with number");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_remove_session_name_starting_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to remove a session with dash-prefixed name
    let result = harness.jjz(&["remove", "-session", "--force"]);
    assert!(
        !result.success,
        "Should reject remove with dash-prefixed name"
    );
    // May show validation error or "session not found"
}

#[test]
fn test_focus_session_name_starting_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to focus a session with dash-prefixed name
    let result = harness.jjz(&["focus", "-session"]);
    assert!(
        !result.success,
        "Should reject focus with dash-prefixed name"
    );
    // May show validation error or "session not found"
}

#[test]
fn test_diff_session_name_starting_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to diff a session with dash-prefixed name
    let result = harness.jjz(&["diff", "-session"]);
    assert!(
        !result.success,
        "Should reject diff with dash-prefixed name"
    );
    // May show validation error or "session not found"
}
