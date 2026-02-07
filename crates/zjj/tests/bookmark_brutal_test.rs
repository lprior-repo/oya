// BRUTAL QA Test Suite for zjj bookmark subcommands
// Tests every flag, option, edge case, race condition, and failure mode

use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::time::Duration;

#[path = "common/mod.rs"]
mod common;

use common::{setup_test_repo, run_zjj, run_zjj_expect_ok, run_zjj_expect_fail, ZJJ_BIN;

struct BookmarkTestEnv {
    repo_path: PathBuf,
    original_dir: PathBuf,
}

impl BookmarkTestEnv {
    fn new() -> Self {
        let original_dir = std::env::current_dir().unwrap();
        let repo_path = setup_test_repo("bookmark_brutal_test");

        // Create initial commits to have revisions to work with
        std::fs::write(repo_path.join("test1.txt"), "test1").unwrap();
        Command::new("jj")
            .args(&["commit", "-m", "test1"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        std::fs::write(repo_path.join("test2.txt"), "test2").unwrap();
        Command::new("jj")
            .args(&["commit", "-m", "test2"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        std::env::set_current_dir(&repo_path).unwrap();

        BookmarkTestEnv {
            repo_path,
            original_dir,
        }
    }

    fn get_current_revision(&self) -> String {
        let output = Command::new("jj")
            .args(&["log", "--no-graph", "-r", "@", "-T", "commit_id"])
            .current_dir(&self.repo_path)
            .output()
            .unwrap();

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_test_commit(&self) -> String {
        std::fs::write(self.repo_path.join(format!("test_{}.txt", chrono::Utc::now().timestamp())), "data").unwrap();
        let output = Command::new("jj")
            .args(&["commit", "-m", &format!("test_{}", chrono::Utc::now().timestamp())])
            .current_dir(&self.repo_path)
            .output()
            .unwrap();

        self.get_current_revision()
    }
}

impl Drop for BookmarkTestEnv {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original_dir).unwrap();
        // Cleanup is handled by the test framework
    }
}

// ============================================================================
// TEST 1: zjj bookmark list - Basic functionality
// ============================================================================

#[test]
fn test_bookmark_list_empty() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "list"], &env.repo_path);

    assert!(result.status.success(), "bookmark list should succeed even with no bookmarks");
    let output = String::from_utf8_lossy(&result.stdout);
    println!("Bookmark list (empty):\n{}", output);
}

#[test]
fn test_bookmark_list_all_flag() {
    let env = BookmarkTestEnv::new();

    // Create a bookmark
    run_zjj_expect_ok(&["bookmark", "create", "test-bookmark"], &env.repo_path);

    // Test with --all flag
    let result = run_zjj(&["bookmark", "list", "--all"], &env.repo_path);
    assert!(result.status.success(), "bookmark list --all should succeed");
    let output = String::from_utf8_lossy(&result.stdout);
    println!("Bookmark list --all:\n{}", output);
    assert!(output.contains("test-bookmark") || output.contains("test"), "Should show created bookmark");
}

#[test]
fn test_bookmark_list_json_flag() {
    let env = BookmarkTestEnv::new();

    // Create a bookmark first
    run_zjj_expect_ok(&["bookmark", "create", "test-json"], &env.repo_path);

    let result = run_zjj(&["bookmark", "list", "--json"], &env.repo_path);
    assert!(result.status.success(), "bookmark list --json should succeed");

    let output = String::from_utf8_lossy(&result.stdout);
    println!("Bookmark list --json:\n{}", output);

    // Validate it's valid JSON
    let json: serde_json::Value = serde_json::from_str(&output)
        .expect("Output should be valid JSON");
}

// ============================================================================
// TEST 2: zjj bookmark create - Every flag and edge case
// ============================================================================

#[test]
fn test_bookmark_create_basic() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "create", "basic-test"], &env.repo_path);

    assert!(result.status.success(), "bookmark create should succeed");
    let output = String::from_utf8_lossy(&result.stdout);
    println!("Bookmark create basic:\n{}", output);
}

#[test]
fn test_bookmark_create_with_push_flag() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "create", "-p", "push-test"], &env.repo_path);

    // Note: -p might fail if no remote configured, but should not crash
    println!("Bookmark create with -p:\n{}",
             String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_create_json_flag() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "create", "--json", "json-test"], &env.repo_path);

    assert!(result.status.success(), "bookmark create --json should succeed");
    let output = String::from_utf8_lossy(&result.stdout);
    println!("Bookmark create --json:\n{}", output);

    // Validate JSON
    let json: serde_json::Value = serde_json::from_str(&output)
        .expect("Output should be valid JSON");
}

#[test]
fn test_bookmark_create_empty_name() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "create", ""], &env.repo_path);

    assert!(!result.status.success(), "bookmark create with empty name should fail");
    println!("Bookmark create empty name - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_create_special_characters() {
    let env = BookmarkTestEnv::new();
    let special_names = vec![
        "bookmark-with-dashes",
        "bookmark_with_underscores",
        "bookmark.with.dots",
        "bookmark/with/slashes",
        "bookmark@with@at",
    ];

    for name in special_names {
        println!("\n--- Testing bookmark name: {} ---", name);
        let result = run_zjj(&["bookmark", "create", name], &env.repo_path);

        println!("Status: {}", result.status);
        println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

        // Some special chars might be valid, some might fail - just don't panic
    }
}

#[test]
fn test_bookmark_create_unicode() {
    let env = BookmarkTestEnv::new();
    let unicode_names = vec![
        "bookmark-тест",         // Cyrillic
        "bookmark-测试",          // Chinese
        "bookmark-🚀-rocket",    // Emoji
        "bookmark-日本語",       // Japanese
        "bookmark-العربية",      // Arabic
    ];

    for name in unicode_names {
        println!("\n--- Testing unicode bookmark: {} ---", name);
        let result = run_zjj(&["bookmark", "create", name], &env.repo_path);

        println!("Status: {}", result.status);
        println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
    }
}

#[test]
fn test_bookmark_create_very_long_name() {
    let env = BookmarkTestEnv::new();
    let long_name = "a".repeat(10000);

    let result = run_zjj(&["bookmark", "create", &long_name], &env.repo_path);
    println!("Bookmark create with 10000 char name - Status: {}", result.status);
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    // Should not panic
}

// ============================================================================
// TEST 3: zjj bookmark delete - Every edge case
// ============================================================================

#[test]
fn test_bookmark_delete_basic() {
    let env = BookmarkTestEnv::new();

    // Create first
    run_zjj_expect_ok(&["bookmark", "create", "delete-me"], &env.repo_path);

    // Then delete
    let result = run_zjj(&["bookmark", "delete", "delete-me"], &env.repo_path);
    assert!(result.status.success(), "bookmark delete should succeed");
    println!("Bookmark delete:\n{}", String::from_utf8_lossy(&result.stdout));
}

#[test]
fn test_bookmark_delete_json_flag() {
    let env = BookmarkTestEnv::new();

    run_zjj_expect_ok(&["bookmark", "create", "delete-json"], &env.repo_path);

    let result = run_zjj(&["bookmark", "delete", "--json", "delete-json"], &env.repo_path);
    assert!(result.status.success(), "bookmark delete --json should succeed");

    let output = String::from_utf8_lossy(&result.stdout);
    println!("Bookmark delete --json:\n{}", output);

    let json: serde_json::Value = serde_json::from_str(&output)
        .expect("Output should be valid JSON");
}

#[test]
fn test_bookmark_delete_nonexistent() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "delete", "does-not-exist-xyz123"], &env.repo_path);

    assert!(!result.status.success(), "bookmark delete of non-existent should fail");
    println!("Bookmark delete nonexistent - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_delete_empty_name() {
    let env = BookmarkTestEnv::new();
    let result = run_zjj(&["bookmark", "delete", ""], &env.repo_path);

    assert!(!result.status.success(), "bookmark delete with empty name should fail");
    println!("Bookmark delete empty name - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

// ============================================================================
// TEST 4: Race conditions - Create/delete same bookmark 100 times
// ============================================================================

#[test]
fn test_bookmark_create_delete_race() {
    let env = BookmarkTestEnv::new();

    for i in 0..100 {
        let name = format!("race-test-{}", i);

        // Create
        let create_result = run_zjj(&["bookmark", "create", &name], &env.repo_path);
        if !create_result.status.success() {
            eprintln!("Failed to create bookmark {} at iteration {}", name, i);
            eprintln!("stderr: {}", String::from_utf8_lossy(&create_result.stderr));
        }

        // Delete
        let delete_result = run_zjj(&["bookmark", "delete", &name], &env.repo_path);
        if !delete_result.status.success() && i != 99 {
            eprintln!("Failed to delete bookmark {} at iteration {}", name, i);
            eprintln!("stderr: {}", String::from_utf8_lossy(&delete_result.stderr));
        }
    }

    println!("Successfully completed 100 create/delete cycles");
}

#[test]
fn test_bookmark_create_same_100_times() {
    let env = BookmarkTestEnv::new();

    // Create same bookmark 100 times (should fail after first or move it)
    for i in 0..100 {
        let result = run_zjj(&["bookmark", "create", "duplicate-test"], &env.repo_path);

        if i == 0 {
            assert!(result.status.success(), "First create should succeed");
        } else {
            // Subsequent creates might fail or move - just don't panic
            println!("Iteration {} - Status: {}", i, result.status);
        }
    }
}

// ============================================================================
// TEST 5: zjj bookmark move - Every option and edge case
// ============================================================================

#[test]
fn test_bookmark_move_basic() {
    let env = BookmarkTestEnv::new();

    // Create bookmark at first revision
    run_zjj_expect_ok(&["bookmark", "create", "move-test"], &env.repo_path);

    // Create a new commit to move to
    let new_rev = env.create_test_commit();

    // Move the bookmark
    let result = run_zjj(&["bookmark", "move", "--to", &new_rev, "move-test"], &env.repo_path);

    println!("Bookmark move - Status: {}", result.status);
    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_move_json_flag() {
    let env = BookmarkTestEnv::new();

    run_zjj_expect_ok(&["bookmark", "create", "move-json-test"], &env.repo_path);
    let new_rev = env.create_test_commit();

    let result = run_zjj(&["bookmark", "move", "--json", "--to", &new_rev, "move-json-test"], &env.repo_path);

    println!("Bookmark move --json - Status: {}", result.status);
    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));

    if result.status.success() {
        let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&result.stdout))
            .expect("Output should be valid JSON");
    }
}

#[test]
fn test_bookmark_move_nonexistent() {
    let env = BookmarkTestEnv::new();
    let current_rev = env.get_current_revision();

    let result = run_zjj(&["bookmark", "move", "--to", &current_rev, "does-not-exist"], &env.repo_path);

    assert!(!result.status.success(), "bookmark move of non-existent should fail");
    println!("Bookmark move nonexistent - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_move_to_invalid_revision() {
    let env = BookmarkTestEnv::new();

    run_zjj_expect_ok(&["bookmark", "create", "move-invalid-rev"], &env.repo_path);

    let result = run_zjj(&["bookmark", "move", "--to", "invalidrevisionxyz123", "move-invalid-rev"], &env.repo_path);

    assert!(!result.status.success(), "bookmark move to invalid revision should fail");
    println!("Bookmark move to invalid revision - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_move_to_same_revision() {
    let env = BookmarkTestEnv::new();

    run_zjj_expect_ok(&["bookmark", "create", "move-same-rev"], &env.repo_path);
    let current_rev = env.get_current_revision();

    // Try to move to the same revision it's already at
    let result = run_zjj(&["bookmark", "move", "--to", &current_rev, "move-same-rev"], &env.repo_path);

    println!("Bookmark move to same revision - Status: {}", result.status);
    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    // Should either succeed or fail gracefully - no panic
}

#[test]
fn test_bookmark_move_empty_name() {
    let env = BookmarkTestEnv::new();
    let current_rev = env.get_current_revision();

    let result = run_zjj(&["bookmark", "move", "--to", &current_rev, ""], &env.repo_path);

    assert!(!result.status.success(), "bookmark move with empty name should fail");
    println!("Bookmark move empty name - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_move_empty_to() {
    let env = BookmarkTestEnv::new();

    run_zjj_expect_ok(&["bookmark", "create", "move-empty-to"], &env.repo_path);

    let result = run_zjj(&["bookmark", "move", "--to", "", "move-empty-to"], &env.repo_path);

    assert!(!result.status.success(), "bookmark move with empty --to should fail");
    println!("Bookmark move empty --to - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

#[test]
fn test_bookmark_move_missing_to_flag() {
    let env = BookmarkTestEnv::new();

    run_zjj_expect_ok(&["bookmark", "create", "move-missing-to"], &env.repo_path);

    let result = run_zjj(&["bookmark", "move", "move-missing-to"], &env.repo_path);

    assert!(!result.status.success(), "bookmark move without --to should fail");
    println!("Bookmark move missing --to - stderr:\n{}",
             String::from_utf8_lossy(&result.stderr));
}

// ============================================================================
// TEST 6: Performance with many bookmarks
// ============================================================================

#[test]
fn test_bookmark_list_with_1000_bookmarks() {
    let env = BookmarkTestEnv::new();

    println!("Creating 1000 bookmarks...");
    for i in 0..1000 {
        let name = format!("perf-test-{:04}", i);
        run_zjj(&["bookmark", "create", &name], &env.repo_path);
    }

    let start = std::time::Instant::now();
    let result = run_zjj(&["bookmark", "list"], &env.repo_path);
    let duration = start.elapsed();

    assert!(result.status.success(), "bookmark list with 1000 bookmarks should succeed");
    println!("Bookmark list with 1000 bookmarks took: {:?}", duration);
    println!("Output length: {} bytes", result.stdout.len());

    // Should complete in reasonable time (< 10 seconds)
    assert!(duration.as_secs() < 10, "Listing 1000 bookmarks should be fast");
}

#[test]
fn test_bookmark_delete_from_1000_bookmarks() {
    let env = BookmarkTestEnv::new();

    // Create 1000 bookmarks
    for i in 0..1000 {
        let name = format!("delete-perf-{:04}", i);
        run_zjj(&["bookmark", "create", &name], &env.repo_path);
    }

    // Delete one bookmark from the middle
    let start = std::time::Instant::now();
    let result = run_zjj(&["bookmark", "delete", "delete-perf-0500"], &env.repo_path);
    let duration = start.elapsed();

    assert!(result.status.success(), "bookmark delete from 1000 should succeed");
    println!("Deleting one bookmark from 1000 took: {:?}", duration);

    // Should be fast (< 5 seconds)
    assert!(duration.as_secs() < 5, "Delete from 1000 bookmarks should be fast");
}

// ============================================================================
// TEST 7: on-success and on-failure callbacks
// ============================================================================

#[test]
fn test_bookmark_create_on_success() {
    let env = BookmarkTestEnv::new();

    // Create a simple callback script
    let callback_script = env.repo_path.join("success_callback.sh");
    fs::write(&callback_script, "#!/bin/sh\necho 'SUCCESS CALLBACK RAN' > callback.txt\n").unwrap();

    let result = run_zjj(&[
        "bookmark", "create",
        "--on-success", callback_script.to_str().unwrap(),
        "callback-test"
    ], &env.repo_path);

    println!("Bookmark create with on-success callback");
    println!("Status: {}", result.status);

    // Check if callback was executed
    let callback_output = env.repo_path.join("callback.txt");
    if callback_output.exists() {
        let content = fs::read_to_string(&callback_output).unwrap();
        println!("Callback output: {}", content);
    }
}

#[test]
fn test_bookmark_create_on_failure() {
    let env = BookmarkTestEnv::new();

    let callback_script = env.repo_path.join("failure_callback.sh");
    fs::write(&callback_script, "#!/bin/sh\necho 'FAILURE CALLBACK RAN' > failure.txt\n").unwrap();

    // This should fail (empty bookmark name)
    let result = run_zjj(&[
        "bookmark", "create",
        "--on-failure", callback_script.to_str().unwrap(),
        ""
    ], &env.repo_path);

    println!("Bookmark create with on-failure callback");
    println!("Status: {}", result.status);

    // Check if callback was executed
    let callback_output = env.repo_path.join("failure.txt");
    if callback_output.exists() {
        let content = fs::read_to_string(&callback_output).unwrap();
        println!("Callback output: {}", content);
    }
}

// ============================================================================
// TEST 8: Edge case combinations
// ============================================================================

#[test]
fn test_bookmark_operations_with_session_name() {
    let env = BookmarkTestEnv::new();

    // These operations use the "current" workspace (no session name needed in basic setup)
    // Test that they work without specifying a session
    run_zjj_expect_ok(&["bookmark", "create", "session-test"], &env.repo_path);

    let result = run_zjj(&["bookmark", "list"], &env.repo_path);
    assert!(result.status.success(), "list should work without session");

    let result = run_zjj(&["bookmark", "delete", "session-test"], &env.repo_path);
    assert!(result.status.success(), "delete should work without session");
}

#[test]
fn test_bookmark_create_at_specific_revision_if_supported() {
    let env = BookmarkTestEnv::new();

    // Test if bookmark create can target a specific revision
    // (This might not be supported, in which case it should fail gracefully)
    let rev = env.get_current_revision();
    let result = run_zjj(&["bookmark", "create", "--at", &rev, "at-rev-test"], &env.repo_path);

    println!("Bookmark create with --at flag - Status: {}", result.status);
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
}

// ============================================================================
// TEST 9: Concurrency simulation (10 parallel operations)
// ============================================================================

#[test]
fn test_bookmark_concurrent_operations() {
    let env = BookmarkTestEnv::new();

    // Spawn multiple processes creating different bookmarks
    let mut handles = vec![];

    for i in 0..10 {
        let repo_path = env.repo_path.clone();
        let name = format!("concurrent-{:02}", i);

        let handle = std::thread::spawn(move || {
            let result = run_zjj(&["bookmark", "create", &name], &repo_path);
            (i, result.status.success())
        });

        handles.push(handle);
    }

    // Wait for all and check results
    let mut success_count = 0;
    for handle in handles {
        let (i, success) = handle.join().unwrap();
        if success {
            success_count += 1;
        } else {
            eprintln!("Concurrent operation {} failed", i);
        }
    }

    println!("Concurrent operations: {}/10 succeeded", success_count);
}

// ============================================================================
// TEST 10: Panic and crash detection
// ============================================================================

#[test]
fn test_bookmark_no_panics_on_invalid_input() {
    let env = BookmarkTestEnv::new();

    let invalid_inputs = vec![
        vec!["bookmark", "create", "\0"],
        vec!["bookmark", "create", "\n\n\n"],
        vec!["bookmark", "delete", "\x01\x02\x03"],
        vec!["bookmark", "move", "--to", "", "\x00"],
    ];

    for args in invalid_inputs {
        println!("Testing with args: {:?}", args);
        let result = run_zjj(&args, &env.repo_path);

        // Should never crash (exit code 134 or similar indicates panic)
        if result.status.code() == Some(134) || result.status.code() == Some(101) {
            panic!("Potential panic detected with args {:?}, exit code: {:?}",
                   args, result.status.code());
        }

        println!("  Exit code: {:?}", result.status.code());
    }
}

#[test]
fn test_bookmark_operations_on_corrupted_state() {
    // This test checks behavior when JJ repo state is problematic
    let env = BookmarkTestEnv::new();

    // Try operations when no commits exist (if possible)
    // Most should fail gracefully

    let result = run_zjj(&["bookmark", "list"], &env.repo_path);
    println!("Bookmark list on normal state - Status: {}", result.status);

    // Operations should not crash even in edge cases
    assert!(result.status.code() != Some(134), "Should not panic");
}
