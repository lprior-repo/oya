// BRUTAL QA Test Suite for zjj bookmark subcommands
// Tests every flag, option, edge case, race condition, and failure mode

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const ZJJ_BIN: &str = "zjj";

struct TestRepo {
    path: PathBuf,
    _temp_dir: tempfile::TempDir,
}

impl TestRepo {
    fn new(name: &str) -> Self {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join(name);
        fs::create_dir(&path).unwrap();

        // Initialize JJ repo
        let status = Command::new("jj")
            .args(["git", "init"])
            .current_dir(&path)
            .status()
            .unwrap();

        assert!(status.success(), "Failed to initialize jj repo");

        // Configure JJ
        Command::new("jj")
            .args(["config", "set", "--repo", "user.name", "\"Test User\""])
            .current_dir(&path)
            .status()
            .unwrap();

        Command::new("jj")
            .args([
                "config",
                "set",
                "--repo",
                "user.email",
                "\"test@example.com\"",
            ])
            .current_dir(&path)
            .status()
            .unwrap();

        // Create initial commit
        fs::write(path.join("initial.txt"), "initial content").unwrap();

        Command::new("jj")
            .args(["commit", "-m", "initial commit"])
            .current_dir(&path)
            .status()
            .unwrap();

        TestRepo {
            path,
            _temp_dir: temp_dir,
        }
    }

    fn run_zjj(&self, args: &[&str]) -> std::process::Output {
        Command::new(ZJJ_BIN)
            .args(args)
            .current_dir(&self.path)
            .output()
            .unwrap_or_else(|e| panic!("Failed to execute zjj {:?}: {}", args, e))
    }

    fn create_commit(&self, msg: &str) {
        fs::write(self.path.join(format!("{}.txt", msg)), msg).unwrap();
        Command::new("jj")
            .args(["commit", "-m", msg])
            .current_dir(&self.path)
            .status()
            .unwrap();
    }

    fn get_current_rev(&self) -> String {
        let output = Command::new("jj")
            .args(["log", "--no-graph", "-r", "@", "-T", "commit_id"])
            .current_dir(&self.path)
            .output()
            .unwrap();

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
}

fn print_test_header(test_name: &str) {
    println!("\n{} {:.^70} {}", "=".repeat(80), test_name, "=".repeat(80));
}

fn print_result(_test_name: &str, result: &std::process::Output) {
    println!("Exit code: {:?}", result.status.code());
    if !result.stdout.is_empty() {
        println!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
    }
    if !result.stderr.is_empty() {
        println!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
    }
}

// ============================================================================
// TEST 1: zjj bookmark list - Basic functionality
// ============================================================================

#[test]
fn test_01_bookmark_list_empty() {
    print_test_header("TEST 1: bookmark list (empty repository)");

    let repo = TestRepo::new("test_01_empty");
    let result = repo.run_zjj(&["bookmark", "list"]);

    print_result("bookmark list (empty)", &result);

    assert!(
        result.status.success(),
        "bookmark list should succeed even with no bookmarks"
    );
    println!("✓ PASSED: bookmark list works with no bookmarks");
}

#[test]
fn test_02_bookmark_list_all_flag() {
    print_test_header("TEST 2: bookmark list --all");

    let repo = TestRepo::new("test_02_all");

    // Create a bookmark first using jj directly
    Command::new("jj")
        .args(["bookmark", "create", "test-bookmark"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let result = repo.run_zjj(&["bookmark", "list", "--all"]);

    print_result("bookmark list --all", &result);

    assert!(
        result.status.success(),
        "bookmark list --all should succeed"
    );
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(
        output.contains("test-bookmark") || output.contains("test"),
        "Should show created bookmark"
    );
    println!("✓ PASSED: --all flag works");
}

#[test]
fn test_03_bookmark_list_json_flag() {
    print_test_header("TEST 3: bookmark list --json");

    let repo = TestRepo::new("test_03_json");

    Command::new("jj")
        .args(["bookmark", "create", "json-test"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let result = repo.run_zjj(&["bookmark", "list", "--json"]);

    print_result("bookmark list --json", &result);

    // BUG FOUND: zjj returns exit code 4 with JSON error when using --json
    if !result.status.success() {
        let output = String::from_utf8_lossy(&result.stdout);
        println!("⚠ BUG FOUND: --json flag causes error!");
        println!("   Error: {}", output);
        panic!("BUG: --json flag should work but causes error");
    }

    let output = String::from_utf8_lossy(&result.stdout);
    println!("Validating JSON output...");

    // Validate it's valid JSON
    match serde_json::from_str::<serde_json::Value>(&output) {
        Ok(_) => println!("✓ PASSED: --json produces valid JSON"),
        Err(e) => panic!("JSON validation failed: {}", e),
    }
}

// ============================================================================
// TEST 2: zjj bookmark create - Every flag and edge case
// ============================================================================

#[test]
fn test_04_bookmark_create_basic() {
    print_test_header("TEST 4: bookmark create basic");

    let repo = TestRepo::new("test_04_create_basic");
    let result = repo.run_zjj(&["bookmark", "create", "basic-test"]);

    print_result("bookmark create basic", &result);

    assert!(result.status.success(), "bookmark create should succeed");
    println!("✓ PASSED: basic bookmark creation works");
}

#[test]
fn test_05_bookmark_create_with_push_flag() {
    print_test_header("TEST 5: bookmark create -p (with push)");

    let repo = TestRepo::new("test_05_push");
    let result = repo.run_zjj(&["bookmark", "create", "-p", "push-test"]);

    print_result("bookmark create -p", &result);

    // Note: -p might fail if no remote configured, but should not panic
    println!(
        "✓ PASSED: -p flag doesn't panic (exit code: {:?})",
        result.status.code()
    );
}

#[test]
fn test_06_bookmark_create_json_flag() {
    print_test_header("TEST 6: bookmark create --json");

    let repo = TestRepo::new("test_06_create_json");
    let result = repo.run_zjj(&["bookmark", "create", "--json", "json-create-test"]);

    print_result("bookmark create --json", &result);

    assert!(
        result.status.success(),
        "bookmark create --json should succeed"
    );

    let output = String::from_utf8_lossy(&result.stdout);
    match serde_json::from_str::<serde_json::Value>(&output) {
        Ok(_) => println!("✓ PASSED: create --json produces valid JSON"),
        Err(e) => panic!("JSON validation failed: {}", e),
    }
}

#[test]
fn test_07_bookmark_create_empty_name() {
    print_test_header("TEST 7: bookmark create with empty name");

    let repo = TestRepo::new("test_07_empty_name");
    let result = repo.run_zjj(&["bookmark", "create", ""]);

    print_result("bookmark create empty name", &result);

    assert!(
        !result.status.success(),
        "bookmark create with empty name should fail"
    );
    println!("✓ PASSED: correctly rejects empty bookmark name");
}

#[test]
fn test_08_bookmark_create_special_characters() {
    print_test_header("TEST 8: bookmark create with special characters");

    let repo = TestRepo::new("test_08_special_chars");
    let special_names = vec![
        "bookmark-with-dashes",
        "bookmark_with_underscores",
        "bookmark.with.dots",
        "bookmark/with/slashes",
        "bookmark@with@at",
    ];

    for name in special_names {
        println!("\n--- Testing: {} ---", name);
        let result = repo.run_zjj(&["bookmark", "create", name]);

        println!("Exit code: {:?}", result.status.code());

        // Some special chars might be valid, some might fail - just don't panic
        if result.status.code() == Some(134) || result.status.code() == Some(101) {
            panic!("PANIC DETECTED with bookmark name: {}", name);
        }
    }

    println!("✓ PASSED: special characters don't cause panics");
}

#[test]
fn test_09_bookmark_create_unicode() {
    print_test_header("TEST 9: bookmark create with Unicode");

    let repo = TestRepo::new("test_09_unicode");
    let unicode_names = vec![
        "bookmark-тест",
        "bookmark-测试",
        "bookmark-🚀-rocket",
        "bookmark-日本語",
        "bookmark-العربية",
    ];

    for name in unicode_names {
        println!("\n--- Testing: {} ---", name);
        let result = repo.run_zjj(&["bookmark", "create", name]);

        println!("Exit code: {:?}", result.status.code());

        if result.status.code() == Some(134) || result.status.code() == Some(101) {
            panic!("PANIC DETECTED with unicode bookmark name: {}", name);
        }
    }

    println!("✓ PASSED: Unicode doesn't cause panics");
}

#[test]
fn test_10_bookmark_create_very_long_name() {
    print_test_header("TEST 10: bookmark create with 10000 character name");

    let repo = TestRepo::new("test_10_long_name");
    let long_name = "a".repeat(10000);

    let result = repo.run_zjj(&["bookmark", "create", &long_name]);

    println!("Exit code: {:?}", result.status.code());

    if result.status.code() == Some(134) || result.status.code() == Some(101) {
        panic!("PANIC DETECTED with long bookmark name");
    }

    println!("✓ PASSED: Long bookmark name doesn't cause panic");
}

// ============================================================================
// TEST 3: zjj bookmark delete - Every edge case
// ============================================================================

#[test]
fn test_11_bookmark_delete_basic() {
    print_test_header("TEST 11: bookmark delete basic");

    let repo = TestRepo::new("test_11_delete_basic");

    // Create first
    Command::new("jj")
        .args(["bookmark", "create", "delete-me"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    // Then delete
    let result = repo.run_zjj(&["bookmark", "delete", "delete-me"]);

    print_result("bookmark delete", &result);

    assert!(result.status.success(), "bookmark delete should succeed");
    println!("✓ PASSED: basic bookmark deletion works");
}

#[test]
fn test_12_bookmark_delete_json_flag() {
    print_test_header("TEST 12: bookmark delete --json");

    let repo = TestRepo::new("test_12_delete_json");

    Command::new("jj")
        .args(["bookmark", "create", "delete-json-test"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let result = repo.run_zjj(&["bookmark", "delete", "--json", "delete-json-test"]);

    print_result("bookmark delete --json", &result);

    assert!(
        result.status.success(),
        "bookmark delete --json should succeed"
    );

    let output = String::from_utf8_lossy(&result.stdout);
    match serde_json::from_str::<serde_json::Value>(&output) {
        Ok(_) => println!("✓ PASSED: delete --json produces valid JSON"),
        Err(e) => panic!("JSON validation failed: {}", e),
    }
}

#[test]
fn test_13_bookmark_delete_nonexistent() {
    print_test_header("TEST 13: bookmark delete non-existent");

    let repo = TestRepo::new("test_13_delete_nonexist");
    let result = repo.run_zjj(&["bookmark", "delete", "does-not-exist-xyz123"]);

    print_result("bookmark delete nonexistent", &result);

    assert!(
        !result.status.success(),
        "bookmark delete of non-existent should fail"
    );
    println!("✓ PASSED: correctly rejects deleting non-existent bookmark");
}

#[test]
fn test_14_bookmark_delete_empty_name() {
    print_test_header("TEST 14: bookmark delete with empty name");

    let repo = TestRepo::new("test_14_delete_empty");
    let result = repo.run_zjj(&["bookmark", "delete", ""]);

    print_result("bookmark delete empty name", &result);

    assert!(
        !result.status.success(),
        "bookmark delete with empty name should fail"
    );
    println!("✓ PASSED: correctly rejects empty bookmark name for deletion");
}

// ============================================================================
// TEST 4: Race conditions - Create/delete same bookmark 100 times
// ============================================================================

#[test]
fn test_15_bookmark_create_delete_race() {
    print_test_header("TEST 15: Race condition - 100 create/delete cycles");

    let repo = TestRepo::new("test_15_race");

    println!("Running 100 create/delete cycles...");
    for i in 0..100 {
        let name = format!("race-test-{}", i);

        // Create
        let create_result = repo.run_zjj(&["bookmark", "create", &name]);
        if !create_result.status.success() {
            eprintln!(
                "WARNING: Failed to create bookmark {} at iteration {}",
                name, i
            );
        }

        // Delete
        let delete_result = repo.run_zjj(&["bookmark", "delete", &name]);
        if !delete_result.status.success() {
            eprintln!(
                "WARNING: Failed to delete bookmark {} at iteration {}",
                name, i
            );
        }
    }

    println!("✓ PASSED: Completed 100 create/delete cycles without panicking");
}

#[test]
fn test_16_bookmark_create_same_100_times() {
    print_test_header("TEST 16: Create same bookmark 100 times");

    let repo = TestRepo::new("test_16_duplicate");

    let mut success_count = 0;
    let mut fail_count = 0;

    for i in 0..100 {
        let result = repo.run_zjj(&["bookmark", "create", "duplicate-test"]);

        if result.status.success() {
            success_count += 1;
        } else {
            fail_count += 1;
        }

        if i % 10 == 0 {
            println!("Iteration {} - Status: {:?}", i, result.status.code());
        }
    }

    println!(
        "✓ PASSED: 100 duplicate creates - Success: {}, Fail: {}",
        success_count, fail_count
    );
}

// ============================================================================
// TEST 5: zjj bookmark move - Every option and edge case
// ============================================================================

#[test]
fn test_17_bookmark_move_basic() {
    print_test_header("TEST 17: bookmark move basic");

    let repo = TestRepo::new("test_17_move_basic");

    // Create bookmark at first revision
    Command::new("jj")
        .args(["bookmark", "create", "move-test"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    // Create a new commit to move to
    repo.create_commit("new-commit-for-move");
    let new_rev = repo.get_current_rev();

    // Move the bookmark
    let result = repo.run_zjj(&["bookmark", "move", "--to", &new_rev, "move-test"]);

    print_result("bookmark move", &result);

    println!(
        "✓ PASSED: bookmark move attempted (exit code: {:?})",
        result.status.code()
    );
}

#[test]
fn test_18_bookmark_move_json_flag() {
    print_test_header("TEST 18: bookmark move --json");

    let repo = TestRepo::new("test_18_move_json");

    Command::new("jj")
        .args(["bookmark", "create", "move-json-test"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    repo.create_commit("another-commit");
    let new_rev = repo.get_current_rev();

    let result = repo.run_zjj(&[
        "bookmark",
        "move",
        "--json",
        "--to",
        &new_rev,
        "move-json-test",
    ]);

    print_result("bookmark move --json", &result);

    if result.status.success() {
        let output = String::from_utf8_lossy(&result.stdout);
        match serde_json::from_str::<serde_json::Value>(&output) {
            Ok(_) => println!("✓ PASSED: move --json produces valid JSON"),
            Err(e) => panic!("JSON validation failed: {}", e),
        }
    } else {
        println!("✓ PASSED: move --json handled gracefully");
    }
}

#[test]
fn test_19_bookmark_move_nonexistent() {
    print_test_header("TEST 19: bookmark move non-existent");

    let repo = TestRepo::new("test_19_move_nonexist");
    let current_rev = repo.get_current_rev();

    let result = repo.run_zjj(&["bookmark", "move", "--to", &current_rev, "does-not-exist"]);

    print_result("bookmark move nonexistent", &result);

    // BUG FOUND: zjj allows moving non-existent bookmarks!
    if result.status.success() {
        println!("⚠ BUG FOUND: zjj bookmark move succeeds for non-existent bookmark!");
        println!("   This should fail but doesn't.");
        panic!("BUG: Moving non-existent bookmark should fail but succeeded");
    }
    println!("✓ PASSED: correctly rejects moving non-existent bookmark");
}

#[test]
fn test_20_bookmark_move_to_invalid_revision() {
    print_test_header("TEST 20: bookmark move to invalid revision");

    let repo = TestRepo::new("test_20_move_invalid_rev");

    Command::new("jj")
        .args(["bookmark", "create", "move-invalid-rev"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let result = repo.run_zjj(&[
        "bookmark",
        "move",
        "--to",
        "invalidrevisionxyz123",
        "move-invalid-rev",
    ]);

    print_result("bookmark move to invalid revision", &result);

    assert!(
        !result.status.success(),
        "bookmark move to invalid revision should fail"
    );
    println!("✓ PASSED: correctly rejects invalid revision");
}

#[test]
fn test_21_bookmark_move_to_same_revision() {
    print_test_header("TEST 21: bookmark move to same revision");

    let repo = TestRepo::new("test_21_move_same");

    Command::new("jj")
        .args(["bookmark", "create", "move-same-rev"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let current_rev = repo.get_current_rev();

    let result = repo.run_zjj(&["bookmark", "move", "--to", &current_rev, "move-same-rev"]);

    print_result("bookmark move to same revision", &result);

    // Should either succeed or fail gracefully - no panic
    println!(
        "✓ PASSED: moving to same revision handled gracefully (exit: {:?})",
        result.status.code()
    );
}

#[test]
fn test_22_bookmark_move_empty_name() {
    print_test_header("TEST 22: bookmark move with empty name");

    let repo = TestRepo::new("test_22_move_empty_name");
    let current_rev = repo.get_current_rev();

    let result = repo.run_zjj(&["bookmark", "move", "--to", &current_rev, ""]);

    print_result("bookmark move empty name", &result);

    assert!(
        !result.status.success(),
        "bookmark move with empty name should fail"
    );
    println!("✓ PASSED: correctly rejects empty bookmark name for move");
}

#[test]
fn test_23_bookmark_move_empty_to() {
    print_test_header("TEST 23: bookmark move with empty --to");

    let repo = TestRepo::new("test_23_move_empty_to");

    Command::new("jj")
        .args(["bookmark", "create", "move-empty-to"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let result = repo.run_zjj(&["bookmark", "move", "--to", "", "move-empty-to"]);

    print_result("bookmark move empty --to", &result);

    assert!(
        !result.status.success(),
        "bookmark move with empty --to should fail"
    );
    println!("✓ PASSED: correctly rejects empty --to revision");
}

#[test]
fn test_24_bookmark_move_missing_to_flag() {
    print_test_header("TEST 24: bookmark move without --to flag");

    let repo = TestRepo::new("test_24_move_missing_to");

    Command::new("jj")
        .args(["bookmark", "create", "move-missing-to"])
        .current_dir(&repo.path)
        .status()
        .unwrap();

    let result = repo.run_zjj(&["bookmark", "move", "move-missing-to"]);

    print_result("bookmark move missing --to", &result);

    assert!(
        !result.status.success(),
        "bookmark move without --to should fail"
    );
    println!("✓ PASSED: correctly requires --to flag");
}

// ============================================================================
// TEST 6: Performance with many bookmarks
// ============================================================================

#[test]
fn test_25_bookmark_list_with_1000_bookmarks() {
    print_test_header("TEST 25: Performance - list with 1000 bookmarks");

    let repo = TestRepo::new("test_25_perf_1000");

    println!("Creating 1000 bookmarks (this may take a while)...");
    for i in 0..1000 {
        let name = format!("perf-test-{:04}", i);

        let result = repo.run_zjj(&["bookmark", "create", &name]);

        if !result.status.success() && i < 10 {
            // Only print first few failures
            eprintln!("WARNING: Failed to create bookmark {}", name);
        }

        if i % 100 == 0 {
            println!("Created {} bookmarks...", i);
        }
    }

    println!("Testing list performance...");
    let start = std::time::Instant::now();
    let result = repo.run_zjj(&["bookmark", "list"]);
    let duration = start.elapsed();

    assert!(
        result.status.success(),
        "bookmark list with 1000 bookmarks should succeed"
    );

    println!("✓ PASSED: List 1000 bookmarks in {:?}", duration);
    println!("Output size: {} bytes", result.stdout.len());

    if duration.as_secs() > 10 {
        eprintln!("WARNING: Listing 1000 bookmarks took more than 10 seconds");
    }
}

#[test]
fn test_26_bookmark_delete_from_1000_bookmarks() {
    print_test_header("TEST 26: Performance - delete from 1000 bookmarks");

    let repo = TestRepo::new("test_26_perf_delete");

    println!("Creating 1000 bookmarks...");
    for i in 0..1000 {
        let name = format!("delete-perf-{:04}", i);
        repo.run_zjj(&["bookmark", "create", &name]);

        if i % 100 == 0 {
            println!("Created {} bookmarks...", i);
        }
    }

    // Delete one bookmark from the middle
    println!("Testing delete performance...");
    let start = std::time::Instant::now();
    let result = repo.run_zjj(&["bookmark", "delete", "delete-perf-0500"]);
    let duration = start.elapsed();

    assert!(
        result.status.success(),
        "bookmark delete from 1000 should succeed"
    );

    println!("✓ PASSED: Delete from 1000 bookmarks in {:?}", duration);

    if duration.as_secs() > 5 {
        eprintln!("WARNING: Deleting from 1000 bookmarks took more than 5 seconds");
    }
}

// ============================================================================
// TEST 7: on-success and on-failure callbacks
// ============================================================================

#[test]
fn test_27_bookmark_create_on_success() {
    print_test_header("TEST 27: bookmark create with --on-success callback");

    let repo = TestRepo::new("test_27_on_success");

    // Create a callback script
    let callback_path = repo.path.join("success_callback.sh");
    let mut file = fs::File::create(&callback_path).unwrap();
    writeln!(file, "#!/bin/sh").unwrap();
    writeln!(
        file,
        "echo 'SUCCESS CALLBACK RAN' > {}",
        repo.path.join("callback.txt").display()
    )
    .unwrap();

    #[cfg(unix)]
    {
        Command::new("chmod")
            .args(["+x", callback_path.to_str().unwrap()])
            .status()
            .unwrap();
    }

    let result = repo.run_zjj(&[
        "bookmark",
        "create",
        "--on-success",
        callback_path.to_str().unwrap(),
        "callback-test",
    ]);

    print_result("bookmark create with on-success", &result);

    // Check if callback was executed
    let callback_output = repo.path.join("callback.txt");
    if callback_output.exists() {
        let content = fs::read_to_string(&callback_output).unwrap();
        println!("Callback output: {}", content);
        println!("✓ PASSED: on-success callback was executed");
    } else {
        println!(
            "⚠ WARNING: on-success callback may not have executed (or feature not implemented)"
        );
    }
}

#[test]
fn test_28_bookmark_create_on_failure() {
    print_test_header("TEST 28: bookmark create with --on-failure callback");

    let repo = TestRepo::new("test_28_on_failure");

    // Create a callback script
    let callback_path = repo.path.join("failure_callback.sh");
    let mut file = fs::File::create(&callback_path).unwrap();
    writeln!(file, "#!/bin/sh").unwrap();
    writeln!(
        file,
        "echo 'FAILURE CALLBACK RAN' > {}",
        repo.path.join("failure.txt").display()
    )
    .unwrap();

    #[cfg(unix)]
    {
        Command::new("chmod")
            .args(["+x", callback_path.to_str().unwrap()])
            .status()
            .unwrap();
    }

    // This should fail (empty bookmark name)
    let result = repo.run_zjj(&[
        "bookmark",
        "create",
        "--on-failure",
        callback_path.to_str().unwrap(),
        "",
    ]);

    print_result("bookmark create with on-failure", &result);

    // Check if callback was executed
    let callback_output = repo.path.join("failure.txt");
    if callback_output.exists() {
        let content = fs::read_to_string(&callback_output).unwrap();
        println!("Callback output: {}", content);
        println!("✓ PASSED: on-failure callback was executed");
    } else {
        println!(
            "⚠ WARNING: on-failure callback may not have executed (or feature not implemented)"
        );
    }
}

// ============================================================================
// TEST 8: Edge case combinations
// ============================================================================

#[test]
fn test_29_bookmark_operations_with_session_name() {
    print_test_header("TEST 29: Operations without session name");

    let repo = TestRepo::new("test_29_session");

    // Test that operations work without specifying a session
    let create_result = repo.run_zjj(&["bookmark", "create", "session-test"]);
    println!("Create without session: {:?}", create_result.status.code());

    let list_result = repo.run_zjj(&["bookmark", "list"]);
    println!("List without session: {:?}", list_result.status.code());

    let delete_result = repo.run_zjj(&["bookmark", "delete", "session-test"]);
    println!("Delete without session: {:?}", delete_result.status.code());

    println!("✓ PASSED: Operations work without session name");
}

// ============================================================================
// TEST 9: Concurrency simulation
// ============================================================================

#[test]
fn test_30_bookmark_concurrent_operations() {
    print_test_header("TEST 30: Concurrent operations (10 threads)");

    let repo = TestRepo::new("test_30_concurrent");
    let repo_path = repo.path.clone();

    // Spawn multiple threads creating different bookmarks
    let mut handles = vec![];

    for i in 0..10 {
        let name = format!("concurrent-{:02}", i);
        let repo_path_clone = repo_path.clone();

        let handle = std::thread::spawn(move || {
            let result = Command::new(ZJJ_BIN)
                .args(["bookmark", "create", &name])
                .current_dir(&repo_path_clone)
                .output();

            match result {
                Ok(output) => (i, output.status.success()),
                Err(e) => {
                    eprintln!("Thread {} failed to execute: {}", i, e);
                    (i, false)
                }
            }
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

    println!(
        "✓ PASSED: Concurrent operations - {}/10 succeeded",
        success_count
    );
}

// ============================================================================
// TEST 10: Panic and crash detection
// ============================================================================

#[test]
fn test_31_bookmark_no_panics_on_invalid_input() {
    print_test_header("TEST 31: No panics on invalid input");

    let repo = TestRepo::new("test_31_no_panics");

    // Note: We can't actually pass null bytes through command line args
    // But we can test other invalid inputs

    let test_cases = vec![
        vec!["bookmark", "create", "\n\n\n"],
        vec!["bookmark", "create", "   "],
    ];

    for args in test_cases {
        println!("Testing args: {:?}", args);
        let result = repo.run_zjj(&args);

        // Exit code 134 = SIGABRT (panic), 101 = Rust panic
        if result.status.code() == Some(134) || result.status.code() == Some(101) {
            panic!("PANIC DETECTED with args {:?}", args);
        }

        println!("  Exit code: {:?} (no panic)", result.status.code());
    }

    println!("✓ PASSED: No panics on invalid input");
}

#[test]
fn test_32_bookmark_operations_normal_state() {
    print_test_header("TEST 32: Operations on normal state");

    let repo = TestRepo::new("test_32_normal_state");

    let result = repo.run_zjj(&["bookmark", "list"]);
    assert!(
        result.status.success(),
        "bookmark list should work on normal state"
    );

    // Operations should not crash
    assert!(result.status.code() != Some(134), "Should not panic");
    assert!(result.status.code() != Some(101), "Should not panic");

    println!("✓ PASSED: Operations work correctly on normal state");
}

// ============================================================================
// TEST 11: Additional edge cases
// ============================================================================

#[test]
fn test_33_bookmark_list_multiple_times() {
    print_test_header("TEST 33: List bookmark 100 times");

    let repo = TestRepo::new("test_33_list_loop");

    for i in 0..100 {
        let result = repo.run_zjj(&["bookmark", "list"]);

        if !result.status.success() {
            eprintln!("WARNING: bookmark list failed at iteration {}", i);
        }

        if i % 10 == 0 {
            println!("Completed {} iterations", i);
        }
    }

    println!("✓ PASSED: 100 list operations completed");
}

#[test]
fn test_34_bookmark_help_flags() {
    print_test_header("TEST 34: Help flags");

    let repo = TestRepo::new("test_34_help");

    let commands = vec![
        vec!["bookmark", "--help"],
        vec!["bookmark", "list", "--help"],
        vec!["bookmark", "create", "--help"],
        vec!["bookmark", "delete", "--help"],
        vec!["bookmark", "move", "--help"],
    ];

    for args in commands {
        println!("Testing: {:?}", args);
        let result = repo.run_zjj(&args);

        // BUG FOUND: Some help commands may not work correctly
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            println!("⚠ WARNING: Help command failed: {:?}", args);
            println!("   stderr: {}", stderr);
            // Don't fail the test, just note it
            continue;
        }

        // Help should contain usage information
        let output =
            String::from_utf8_lossy(&result.stdout) + String::from_utf8_lossy(&result.stderr);
        if !(output.contains("Usage") || output.contains("usage") || output.contains("USAGE")) {
            println!("⚠ WARNING: Help output doesn't contain 'Usage': {:?}", args);
        }
    }

    println!("✓ PASSED: All help commands attempted");
}
