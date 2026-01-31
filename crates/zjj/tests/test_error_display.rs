//! Integration tests for user-friendly error display
//!
//! Verifies that errors are shown without stack traces in production

mod common;

use common::TestHarness;

#[test]
fn test_error_no_stack_trace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Try to run a command that will fail (list without init)
    let result = harness.jjz(&["list"]);

    // Should fail
    assert!(!result.success, "Command should fail without init");

    // Error output should NOT contain stack trace indicators
    let stderr = result.stderr;

    // Should not contain stack trace markers
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Error should not contain 'Stack backtrace:'\nActual stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("stack backtrace:"),
        "Error should not contain 'stack backtrace:'\nActual stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("   at "),
        "Error should not contain stack frames (   at)\nActual stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("backtrace::"),
        "Error should not contain backtrace module\nActual stderr:\n{stderr}"
    );

    // Should contain user-friendly error message
    assert!(
        stderr.contains("Error:"),
        "Error should start with 'Error:'\nActual stderr:\n{stderr}"
    );
}

#[test]
fn test_error_format_for_missing_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to focus a nonexistent session
    let result = harness.jjz(&["focus", "nonexistent"]);

    assert!(!result.success, "Should fail for nonexistent session");

    let stderr = result.stderr;

    // Should have clean error message, no stack trace
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Should not show stack trace\nActual stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:"),
        "Should start with Error:\nActual stderr:\n{stderr}"
    );
}

#[test]
fn test_error_format_for_invalid_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to add session with invalid name
    let result = harness.jjz(&["add", "-invalid", "--no-open"]);

    assert!(!result.success, "Should fail for invalid name");

    let stderr = result.stderr;

    // Should have clean error message
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Should not show stack trace\nActual stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:") || stderr.contains("error:"),
        "Should contain error indicator\nActual stderr:\n{stderr}"
    );

    // Should mention the validation issue
    assert!(
        stderr.contains("Invalid") || stderr.contains("invalid") || stderr.contains("name"),
        "Should mention validation issue\nActual stderr:\n{stderr}"
    );
}

#[test]
fn test_error_exit_code() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Try to run command that will fail
    let result = harness.jjz(&["list"]);

    // Should exit with non-zero code
    assert!(
        !result.success,
        "Command should fail (exit code should be non-zero)"
    );
}

#[test]
fn test_database_error_display() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Corrupt the database
    let db_path = harness.state_db_path();
    if std::fs::write(&db_path, "corrupted data").is_err() {
        std::process::abort()
    }

    // Try to list sessions
    let result = harness.jjz(&["list"]);

    assert!(!result.success, "Should fail with corrupted database");

    let stderr = result.stderr;

    // Should show clean error without stack trace
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Should not show stack trace\nActual stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:"),
        "Should start with Error:\nActual stderr:\n{stderr}"
    );
}
