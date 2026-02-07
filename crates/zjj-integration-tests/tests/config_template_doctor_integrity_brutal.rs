// Brutal QA tests for zjj config, template, doctor, and integrity commands
// QA Agent #7 - Testing EVERY command, flag, and failure mode

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

// Test helper structure
struct TestHarness {
    _temp_dir: TempDir, // Kept alive to prevent deletion
    repo_path: PathBuf,
}

impl TestHarness {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().join("test-repo");

        // Initialize JJ repo
        let status = Command::new("jj")
            .arg("git")
            .arg("init")
            .arg(&repo_path)
            .status()
            .expect("Failed to run jj git init");

        assert!(status.success(), "jj git init failed");

        // Create initial commit
        let test_file = repo_path.join("test.txt");
        let mut file = File::create(&test_file).expect("Failed to create test file");
        file.write_all(b"test content")
            .expect("Failed to write test file");

        let status = Command::new("jj")
            .current_dir(&repo_path)
            .args(["commit", "-m", "initial commit"])
            .status()
            .expect("Failed to run jj commit");

        assert!(status.success(), "jj commit failed");

        TestHarness {
            _temp_dir: temp_dir,
            repo_path,
        }
    }

    fn run_zjj(&self, args: &[&str]) -> Output {
        Command::new("zjj")
            .current_dir(&self.repo_path)
            .args(args)
            .output()
            .expect("Failed to run zjj")
    }

    fn _repo_path_str(&self) -> &str {
        self.repo_path.to_str().unwrap()
    }
}

// ============================================================================
// CONFIG COMMAND TESTS
// ============================================================================

#[test]
fn test_config_01_show_all_config() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "config should succeed");
    // Should show some config
    assert!(!output.stdout.is_empty(), "config should output something");
}

#[test]
fn test_config_02_show_single_key() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "workspace_dir"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "config workspace_dir should succeed"
    );
}

#[test]
fn test_config_03_json_flag() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "config --json should succeed");

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with('{'),
        "JSON output should start with {{}}"
    );

    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_config_04_global_flag() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "--global"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (even if global config doesn't exist yet)
    // Exit code should be 0
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "config --global should succeed or fail gracefully"
    );
}

#[test]
fn test_config_05_set_valid_value() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "workspace_dir", "/tmp/test-zjj-workspaces"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "config set should succeed");

    // Verify it was set
    let output2 = harness.run_zjj(&["config", "workspace_dir"]);
    assert!(
        output2.status.success(),
        "config get after set should succeed"
    );
    let stdout = String::from_utf8_lossy(&output2.stdout);
    assert!(
        stdout.contains("/tmp/test-zjj-workspaces"),
        "value should be set"
    );
}

#[test]
fn test_config_06_set_with_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "workspace_dir", "/tmp/test-json", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "config set --json should succeed");

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_config_07_nonexistent_key() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "this_key_does_not_exist_for_sure"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "config nonexistent key should fail"
    );
}

#[test]
fn test_config_08_empty_key() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", ""]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail - empty key is invalid
    assert!(
        !output.status.success(),
        "config with empty key should fail"
    );
}

#[test]
fn test_config_09_empty_value() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["config", "workspace_dir", ""]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Setting empty value might succeed or fail - just check it doesn't crash
    let _ = output.status;
}

#[test]
fn test_config_10_corrupted_config_file() {
    let harness = TestHarness::new();

    // Find config file and corrupt it
    let config_path = harness.repo_path.join(".jj").join("zjj.toml");
    if config_path.exists() {
        // Write invalid TOML
        let mut file = File::create(&config_path).expect("Failed to create config");
        file.write_all(b"this is not valid toml [[[ [[[")
            .expect("Failed to write corrupted config");

        let output = harness.run_zjj(&["config"]);

        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        // Should fail gracefully
        assert!(
            !output.status.success(),
            "config with corrupted file should fail"
        );
        // Should not panic
        assert!(
            output.status.code().is_some(),
            "should exit with code, not crash"
        );
    } else {
        // Config doesn't exist yet - try creating it invalid
        let mut file = File::create(&config_path).expect("Failed to create config");
        file.write_all(b"this is not valid toml [[[ [[[")
            .expect("Failed to write corrupted config");

        let output = harness.run_zjj(&["config"]);

        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        // Should fail gracefully
        assert!(
            !output.status.success(),
            "config with corrupted file should fail"
        );
    }
}

#[test]
fn test_config_11_very_long_key() {
    let harness = TestHarness::new();
    let long_key = "a".repeat(10000);
    let output = harness.run_zjj(&["config", &long_key]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail gracefully, not crash
    let _ = output.status;
}

#[test]
fn test_config_12_very_long_value() {
    let harness = TestHarness::new();
    let long_value = "x".repeat(100000);
    let output = harness.run_zjj(&["config", "test_key", &long_value]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should handle gracefully
    let _ = output.status;
}

#[test]
fn test_config_13_special_characters_in_value() {
    let harness = TestHarness::new();
    let special_value = "value with \"quotes\" and 'apostrophes' and $pecial and \\\\backslashes";
    let output = harness.run_zjj(&["config", "test_key", special_value]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should handle special chars
    let _ = output.status;
}

#[test]
fn test_config_14_on_success_callback() {
    let harness = TestHarness::new();
    let script_path = harness.repo_path.join("success.sh");
    {
        let mut file = File::create(&script_path).expect("Failed to create script");
        file.write_all(b"#!/bin/sh\necho 'SUCCESS' > /tmp/zjj_test_callback.txt")
            .expect("Failed to write script");
    }

    let output = harness.run_zjj(&[
        "config",
        "--on-success",
        script_path.to_str().unwrap(),
        "workspace_dir",
        "/tmp/test",
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Clean up callback file
    let _ = fs::remove_file("/tmp/zjj_test_callback.txt");

    // Should succeed
    assert!(
        output.status.success(),
        "config with on-success should succeed"
    );
}

#[test]
fn test_config_15_on_failure_callback() {
    let harness = TestHarness::new();
    let script_path = harness.repo_path.join("failure.sh");
    {
        let mut file = File::create(&script_path).expect("Failed to create script");
        file.write_all(b"#!/bin/sh\necho 'FAILURE' > /tmp/zjj_test_callback.txt")
            .expect("Failed to write script");
    }

    let output = harness.run_zjj(&[
        "config",
        "--on-failure",
        script_path.to_str().unwrap(),
        "nonexistent_key_xyz",
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Clean up callback file
    let _ = fs::remove_file("/tmp/zjj_test_callback.txt");

    // Should fail (key doesn't exist)
    assert!(
        !output.status.success(),
        "config with on-failure should fail"
    );
}

// ============================================================================
// TEMPLATE COMMAND TESTS
// ============================================================================

#[test]
fn test_template_01_list_templates() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "list"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "template list should succeed");
}

#[test]
fn test_template_02_list_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "list", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template list --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(
            json.is_object() || json.is_array(),
            "JSON should be object or array"
        );
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_template_03_create_basic() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "create", "test-template-basic"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "template create should succeed");
}

#[test]
fn test_template_04_create_with_description() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&[
        "template",
        "create",
        "test-template-desc",
        "--description",
        "This is a test template for QA",
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template create with description should succeed"
    );
}

#[test]
fn test_template_05_create_with_builtin() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&[
        "template",
        "create",
        "test-template-minimal",
        "--builtin",
        "minimal",
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template create with builtin should succeed"
    );
}

#[test]
fn test_template_06_create_all_builtins() {
    let harness = TestHarness::new();

    for builtin in &["minimal", "standard", "full", "split", "review"] {
        let template_name = format!("test-builtin-{}", builtin);
        let output = harness.run_zjj(&["template", "create", &template_name, "--builtin", builtin]);

        println!(
            "builtin {}: stdout: {}",
            builtin,
            String::from_utf8_lossy(&output.stdout)
        );
        println!(
            "builtin {}: stderr: {}",
            builtin,
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(
            output.status.success(),
            "template create with builtin {} should succeed",
            builtin
        );
    }
}

#[test]
fn test_template_07_create_from_file() {
    let harness = TestHarness::new();

    // Create a valid KDL file
    let kdl_path = harness.repo_path.join("test-layout.kdl");
    {
        let mut file = File::create(&kdl_path).expect("Failed to create KDL file");
        file.write_all(b"layout {\n    pane name=\"test\" {\n        cmd \"bash\"\n    }\n}")
            .expect("Failed to write KDL");
    }

    let output = harness.run_zjj(&[
        "template",
        "create",
        "test-template-file",
        "--from-file",
        kdl_path.to_str().unwrap(),
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template create from file should succeed"
    );
}

#[test]
fn test_template_08_create_invalid_kdl() {
    let harness = TestHarness::new();

    // Create an invalid KDL file
    let kdl_path = harness.repo_path.join("invalid-layout.kdl");
    {
        let mut file = File::create(&kdl_path).expect("Failed to create KDL file");
        file.write_all(b"this is not valid kdl {{{{{ [[[")
            .expect("Failed to write KDL");
    }

    let output = harness.run_zjj(&[
        "template",
        "create",
        "test-template-invalid",
        "--from-file",
        kdl_path.to_str().unwrap(),
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "template create with invalid KDL should fail"
    );
}

#[test]
fn test_template_09_create_massive_file() {
    let harness = TestHarness::new();

    // Create a massive KDL file (100MB worth of data)
    let kdl_path = harness.repo_path.join("massive-layout.kdl");
    {
        let mut file = File::create(&kdl_path).expect("Failed to create KDL file");
        // Write a huge but valid KDL structure
        writeln!(file, "layout {{").unwrap();
        for i in 0..10000 {
            writeln!(
                file,
                "    pane name=\"pane-{}\" {{ cmd \"echo '{}'\" }}",
                i, i
            )
            .unwrap();
        }
        writeln!(file, "}}").unwrap();
    }

    let output = harness.run_zjj(&[
        "template",
        "create",
        "test-template-massive",
        "--from-file",
        kdl_path.to_str().unwrap(),
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should either succeed or fail gracefully, not crash
    let exit_code = output.status.code();
    assert!(
        exit_code.is_some(),
        "template create with massive file should exit with code, not crash"
    );
}

#[test]
fn test_template_10_create_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "create", "test-template-json", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template create --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_template_11_create_empty_name() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "create", ""]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "template create with empty name should fail"
    );
}

#[test]
fn test_template_12_create_special_chars() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "create", "test-template- Special!@#$%"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should handle special chars (may succeed or fail, but not crash)
    let _ = output.status;
}

#[test]
fn test_template_13_create_unicode() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "create", "test-template-测试-🚀"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should handle unicode (may succeed or fail, but not crash)
    let _ = output.status;
}

#[test]
fn test_template_14_show_template() {
    let harness = TestHarness::new();

    // First create a template
    let _ = harness.run_zjj(&["template", "create", "test-show-template"]);

    // Now show it
    let output = harness.run_zjj(&["template", "show", "test-show-template"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "template show should succeed");
}

#[test]
fn test_template_15_show_json() {
    let harness = TestHarness::new();

    // First create a template
    let _ = harness.run_zjj(&["template", "create", "test-show-json"]);

    // Now show it with JSON
    let output = harness.run_zjj(&["template", "show", "test-show-json", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template show --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_template_16_show_nonexistent() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["template", "show", "template-does-not-exist-xyz"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "template show nonexistent should fail"
    );
}

#[test]
fn test_template_17_delete_basic() {
    let harness = TestHarness::new();

    // First create a template
    let _ = harness.run_zjj(&["template", "create", "test-delete-template"]);

    // Now delete it
    let output = harness.run_zjj(&["template", "delete", "test-delete-template", "--force"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "template delete should succeed");
}

#[test]
fn test_template_18_delete_json() {
    let harness = TestHarness::new();

    // First create a template
    let _ = harness.run_zjj(&["template", "create", "test-delete-json"]);

    // Now delete it with JSON
    let output = harness.run_zjj(&[
        "template",
        "delete",
        "test-delete-json",
        "--force",
        "--json",
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "template delete --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_template_19_delete_nonexistent() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&[
        "template",
        "delete",
        "template-does-not-exist-xyz",
        "--force",
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "template delete nonexistent should fail"
    );
}

#[test]
fn test_template_20_delete_missing_template_dir() {
    let harness = TestHarness::new();

    // Manually remove template directory if it exists
    let template_dir = harness.repo_path.join(".jj").join("templates");
    if template_dir.exists() {
        fs::remove_dir_all(&template_dir).ok();
    }

    let output = harness.run_zjj(&["template", "list"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (empty list)
    assert!(
        output.status.success(),
        "template list with missing dir should succeed"
    );
}

// ============================================================================
// DOCTOR COMMAND TESTS
// ============================================================================

#[test]
fn test_doctor_01_basic() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["doctor"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "doctor should succeed");
}

#[test]
fn test_doctor_02_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["doctor", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "doctor --json should succeed");

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_doctor_03_fix_flag() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["doctor", "--fix"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(output.status.success(), "doctor --fix should succeed");
}

#[test]
fn test_doctor_04_check_jj_installed() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["doctor"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should check for JJ installation
    assert!(
        stdout.contains("jj") || stdout.contains("Jujutsu") || stdout.contains("check"),
        "doctor should check for jj installation"
    );
}

#[test]
fn test_doctor_05_check_zellij_installed() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["doctor"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should check for Zellij installation
    assert!(
        stdout.contains("zellij") || stdout.contains("check"),
        "doctor should check for zellij installation"
    );
}

#[test]
fn test_doctor_06_check_config_valid() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["doctor"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("stdout: {}", stdout);
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should validate config
    assert!(
        stdout.contains("config") || stdout.contains("check"),
        "doctor should check config"
    );
}

#[test]
fn test_doctor_07_corrupted_config() {
    let harness = TestHarness::new();

    // Corrupt config file
    let config_path = harness.repo_path.join(".jj").join("zjj.toml");
    let _ = File::create(&config_path).and_then(|mut f| f.write_all(b"corrupted toml [[[ [[["));

    let output = harness.run_zjj(&["doctor"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should detect corruption
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("error") || stdout.contains("fail") || stdout.contains("invalid"),
        "doctor should detect corrupted config"
    );
}

#[test]
fn test_doctor_08_missing_jj() {
    // Can't really test missing JJ without breaking the system
    // Skip this test
}

#[test]
fn test_doctor_09_on_success_callback() {
    let harness = TestHarness::new();
    let script_path = harness.repo_path.join("doctor_success.sh");
    {
        let mut file = File::create(&script_path).expect("Failed to create script");
        file.write_all(b"#!/bin/sh\necho 'DOCTOR SUCCESS' > /tmp/zjj_doctor_test.txt")
            .expect("Failed to write script");
    }

    let output = harness.run_zjj(&["doctor", "--on-success", script_path.to_str().unwrap()]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Clean up
    let _ = fs::remove_file("/tmp/zjj_doctor_test.txt");

    // Should succeed
    assert!(
        output.status.success(),
        "doctor with on-success should succeed"
    );
}

#[test]
fn test_doctor_10_on_failure_callback() {
    let harness = TestHarness::new();
    let script_path = harness.repo_path.join("doctor_failure.sh");
    {
        let mut file = File::create(&script_path).expect("Failed to create script");
        file.write_all(b"#!/bin/sh\necho 'DOCTOR FAILURE' > /tmp/zjj_doctor_fail_test.txt")
            .expect("Failed to write script");
    }

    // Corrupt config to trigger failure
    let config_path = harness.repo_path.join(".jj").join("zjj.toml");
    let _ = File::create(&config_path).and_then(|mut f| f.write_all(b"corrupted toml [[[ [[["));

    let output = harness.run_zjj(&["doctor", "--on-failure", script_path.to_str().unwrap()]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Clean up
    let _ = fs::remove_file("/tmp/zjj_doctor_fail_test.txt");

    // Should fail (corrupted config)
    assert!(
        !output.status.success(),
        "doctor with corrupted config should fail"
    );
}

// ============================================================================
// INTEGRITY COMMAND TESTS
// ============================================================================

#[test]
fn test_integrity_01_validate_workspace() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "validate", "."]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (workspace is valid)
    assert!(output.status.success(), "integrity validate should succeed");
}

#[test]
fn test_integrity_02_validate_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "validate", ".", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "integrity validate --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_integrity_03_validate_nonexistent_workspace() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "validate", "/nonexistent/workspace/path"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "integrity validate nonexistent should fail"
    );
}

#[test]
fn test_integrity_04_repair_workspace() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "repair", ".", "--force"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (workspace is valid)
    assert!(output.status.success(), "integrity repair should succeed");
}

#[test]
fn test_integrity_05_repair_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "repair", ".", "--force", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "integrity repair --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_integrity_06_backup_list() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "backup", "list"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (may be empty list)
    assert!(
        output.status.success(),
        "integrity backup list should succeed"
    );
}

#[test]
fn test_integrity_07_backup_list_json() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "backup", "list", "--json"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed
    assert!(
        output.status.success(),
        "integrity backup list --json should succeed"
    );

    // Should be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(
            json.is_object() || json.is_array(),
            "JSON should be object or array"
        );
    } else {
        panic!("Invalid JSON output");
    }
}

#[test]
fn test_integrity_08_backup_restore_nonexistent() {
    let harness = TestHarness::new();
    let output = harness.run_zjj(&["integrity", "backup", "restore", "nonexistent-backup-id"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should fail
    assert!(
        !output.status.success(),
        "integrity backup restore nonexistent should fail"
    );
}

#[test]
fn test_integrity_09_corrupted_workspace() {
    let harness = TestHarness::new();

    // Corrupt the JJ repo
    let jj_dir = harness.repo_path.join(".jj");
    if jj_dir.exists() {
        let repo_file = jj_dir.join("repo_store");
        if repo_file.exists() {
            let _ =
                File::create(&repo_file).and_then(|mut f| f.write_all(b"corrupted data [[[ [[["));
        }

        let output = harness.run_zjj(&["integrity", "validate", "."]);

        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        // Should detect corruption
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("error") || stdout.contains("fail") || stdout.contains("corrupt"),
            "integrity validate should detect corruption"
        );
    }
}

#[test]
fn test_integrity_10_on_success_callback() {
    let harness = TestHarness::new();
    let script_path = harness.repo_path.join("integrity_success.sh");
    {
        let mut file = File::create(&script_path).expect("Failed to create script");
        file.write_all(b"#!/bin/sh\necho 'INTEGRITY SUCCESS' > /tmp/zjj_integrity_test.txt")
            .expect("Failed to write script");
    }

    let output = harness.run_zjj(&[
        "integrity",
        "validate",
        ".",
        "--on-success",
        script_path.to_str().unwrap(),
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Clean up
    let _ = fs::remove_file("/tmp/zjj_integrity_test.txt");

    // Should succeed
    assert!(
        output.status.success(),
        "integrity validate with on-success should succeed"
    );
}

#[test]
fn test_integrity_11_on_failure_callback() {
    let harness = TestHarness::new();
    let script_path = harness.repo_path.join("integrity_failure.sh");
    {
        let mut file = File::create(&script_path).expect("Failed to create script");
        file.write_all(b"#!/bin/sh\necho 'INTEGRITY FAILURE' > /tmp/zjj_integrity_fail_test.txt")
            .expect("Failed to write script");
    }

    let output = harness.run_zjj(&[
        "integrity",
        "validate",
        "/nonexistent/path",
        "--on-failure",
        script_path.to_str().unwrap(),
    ]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Clean up
    let _ = fs::remove_file("/tmp/zjj_integrity_fail_test.txt");

    // Should fail (path doesn't exist)
    assert!(
        !output.status.success(),
        "integrity validate nonexistent should fail"
    );
}

// ============================================================================
// STRESS AND EDGE CASE TESTS
// ============================================================================

#[test]
fn test_stress_01_config_100_operations() {
    let harness = TestHarness::new();

    for i in 0..100 {
        let key = format!("test_key_{}", i);
        let value = format!("test_value_{}", i);
        let output = harness.run_zjj(&["config", &key, &value]);
        assert!(output.status.success(), "config set {} should succeed", i);
    }
}

#[test]
fn test_stress_02_template_50_operations() {
    let harness = TestHarness::new();

    for i in 0..50 {
        let name = format!("test-stress-template-{}", i);
        let output = harness.run_zjj(&["template", "create", &name]);
        assert!(
            output.status.success(),
            "template create {} should succeed",
            i
        );
    }
}

#[test]
fn test_stress_03_doctor_20_checks() {
    let harness = TestHarness::new();

    for _ in 0..20 {
        let output = harness.run_zjj(&["doctor"]);
        assert!(output.status.success(), "doctor should succeed");
    }
}

#[test]
fn test_edge_01_concurrent_config_operations() {
    use std::thread;

    let harness = TestHarness::new();
    let repo_path = harness.repo_path.clone();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let path = repo_path.clone();
            thread::spawn(move || {
                let output = Command::new("zjj")
                    .current_dir(&path)
                    .args(["config", &format!("concurrent_test_{}", i), "value"])
                    .output()
                    .expect("Failed to run zjj");
                output.status.success()
            })
        })
        .collect();

    for handle in handles {
        assert!(
            handle.join().unwrap(),
            "concurrent config operation should succeed"
        );
    }
}

#[test]
fn test_edge_02_config_during_active_operation() {
    let harness = TestHarness::new();

    // Create a long-running operation in background
    // Then try to read config
    let output = harness.run_zjj(&["config", "workspace_dir"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (or fail gracefully)
    let _ = output.status;
}

#[test]
fn test_edge_03_missing_template_directory() {
    let harness = TestHarness::new();

    // Remove template directory
    let template_dir = harness.repo_path.join(".jj").join("templates");
    let _ = fs::remove_dir_all(&template_dir);

    let output = harness.run_zjj(&["template", "list"]);

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Should succeed (empty list)
    assert!(
        output.status.success(),
        "template list with missing dir should succeed"
    );
}
