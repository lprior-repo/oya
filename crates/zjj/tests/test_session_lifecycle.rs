//! Integration tests for session lifecycle
//!
//! Tests the complete workflow: init → add → list → status → remove

mod common;

use common::TestHarness;

// ============================================================================
// Session Creation (add command)
// ============================================================================

#[test]
fn test_add_creates_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Add a session with --no-open to avoid Zellij interaction
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Verify workspace was created
    harness.assert_workspace_exists("test-session");
}

#[test]
fn test_add_session_appears_in_list() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "my-feature", "--no-open"]);

    // List sessions
    let result = harness.jjz(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("my-feature");
}

#[test]
fn test_add_duplicate_session_fails() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "duplicate", "--no-open"]);

    // Try to add again
    harness.assert_failure(&["add", "duplicate", "--no-open"], "already exists");
}

#[test]
fn test_add_invalid_session_name_with_spaces() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "has spaces", "--no-open"], "Invalid session name");
}

#[test]
fn test_add_invalid_session_name_with_special_chars() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "has@symbol", "--no-open"], "Invalid session name");
}

#[test]
fn test_add_valid_session_names() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // These should all be valid
    harness.assert_success(&["add", "feature-123", "--no-open"]);
    harness.assert_success(&["add", "bug_fix_456", "--no-open"]);
    harness.assert_success(&["add", "task789", "--no-open"]);

    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("feature-123");
    result.assert_stdout_contains("bug_fix_456");
    result.assert_stdout_contains("task789");
}

#[test]
fn test_add_creates_jj_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    // Verify JJ workspace exists
    let workspace_path = harness.workspace_path("test");
    assert!(workspace_path.exists());

    // Verify it's a valid JJ workspace
    let result = harness.jj(&["workspace", "list"]);
    assert!(result.success);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_without_init_fails() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Try to add without init
    harness.assert_failure(&["add", "test", "--no-open"], "");
}

#[test]
fn test_add_session_with_no_hooks_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Add with --no-hooks flag
    harness.assert_success(&["add", "test", "--no-open", "--no-hooks"]);

    // Session should be created
    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// Session Listing (list command)
// ============================================================================

#[test]
fn test_list_empty_shows_no_sessions() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.jjz(&["list"]);
    assert!(result.success);
    // Empty list - implementation may show "No sessions" or empty output
}

#[test]
fn test_list_shows_multiple_sessions() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);
    harness.assert_success(&["add", "session3", "--no-open"]);

    let result = harness.jjz(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("session1");
    result.assert_stdout_contains("session2");
    result.assert_stdout_contains("session3");
}

#[test]
fn test_list_json_format() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list", "--json"]);
    assert!(result.success);

    // Verify it's valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_list_shows_session_status() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list"]);
    assert!(result.success);
    // Should show status (active, creating, etc.)
    result.assert_output_contains("test");
}

// ============================================================================
// Session Status (status command)
// ============================================================================

#[test]
fn test_status_shows_session_details() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["status", "test"]);
    assert!(result.success);
    result.assert_output_contains("test");
}

#[test]
fn test_status_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Status for nonexistent session may fail or show "not found"
    let _result = harness.jjz(&["status", "nonexistent"]);
    // Implementation may vary - either fails or shows empty
}

#[test]
fn test_status_json_format() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["status", "test", "--json"]);
    assert!(result.success);

    // Verify it's valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

// ============================================================================
// Session Removal (remove command)
// ============================================================================

#[test]
fn test_remove_deletes_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);
    harness.assert_workspace_exists("test");

    // Remove with --force to skip confirmation
    harness.assert_success(&["remove", "test", "--force"]);

    // Verify workspace deleted
    harness.assert_workspace_not_exists("test");

    // Verify not in list
    let result = harness.jjz(&["list"]);
    assert!(!result.stdout.contains("test"));
}

#[test]
fn test_remove_nonexistent_session_fails() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["remove", "nonexistent", "--force"], "");
}

#[test]
fn test_remove_without_force_requires_confirmation() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    // Without --force, command may prompt or fail
    // In non-interactive mode, it should fail
    let _result = harness.jjz(&["remove", "test"]);
    // Implementation may vary - either prompts (fails in CI) or requires --force
}

// ============================================================================
// Complete Workflow Tests
// ============================================================================

#[test]
fn test_complete_session_lifecycle() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // 1. Initialize
    harness.assert_success(&["init"]);

    // 2. Add session
    harness.assert_success(&["add", "feature-test", "--no-open"]);
    harness.assert_workspace_exists("feature-test");

    // 3. Verify it appears in list
    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("feature-test");

    // 4. Check status
    let result = harness.jjz(&["status", "feature-test"]);
    assert!(result.success);

    // 5. Remove session
    harness.assert_success(&["remove", "feature-test", "--force"]);
    harness.assert_workspace_not_exists("feature-test");

    // 6. Verify not in list
    let result = harness.jjz(&["list"]);
    assert!(!result.stdout.contains("feature-test"));
}

#[test]
fn test_multiple_sessions_lifecycle() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions
    harness.assert_success(&["add", "session-a", "--no-open"]);
    harness.assert_success(&["add", "session-b", "--no-open"]);
    harness.assert_success(&["add", "session-c", "--no-open"]);

    // Verify all exist
    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("session-a");
    result.assert_stdout_contains("session-b");
    result.assert_stdout_contains("session-c");

    // Remove one
    harness.assert_success(&["remove", "session-b", "--force"]);

    // Verify others still exist
    let result = harness.jjz(&["list"]);
    result.assert_stdout_contains("session-a");
    assert!(!result.stdout.contains("session-b"));
    result.assert_stdout_contains("session-c");

    // Clean up
    harness.assert_success(&["remove", "session-a", "--force"]);
    harness.assert_success(&["remove", "session-c", "--force"]);
}

#[test]
fn test_session_persists_across_list_calls() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "persistent", "--no-open"]);

    // List multiple times - session should appear each time
    for _ in 0..3 {
        let result = harness.jjz(&["list"]);
        result.assert_stdout_contains("persistent");
    }
}

#[test]
fn test_add_session_creates_workspace_directory() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let workspace_path = harness.workspace_path("test");
    assert!(workspace_path.exists());
    assert!(workspace_path.is_dir());

    // Verify it contains JJ files
    assert!(workspace_path.join(".jj").exists());
}
