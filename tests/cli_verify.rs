//! Acceptance tests for `oya verify` CLI command
//!
//! Tests the following exit codes:
//! - 0: Config valid
//! - 1: Config file not found
//! - 2: YAML parse error
//! - 3: Field validation error (e.g., empty model)
//!
//! These tests MUST FAIL (red) until the `oya verify` subcommand is implemented.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn oya_binary() -> PathBuf {
    std::env::current_exe()
        .map(|p| p.parent().map(|d| d.parent().map(|d| d.join("oya")).unwrap_or_default()))
        .unwrap_or_default()
        .unwrap_or_else(|| PathBuf::from("target/debug/oya"))
}

fn create_temp_config(content: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    fs::write(dir.path().join("oya.yaml"), content).expect("failed to write config");
    dir
}

/// Test: Valid config returns exit code 0
#[test]
fn test_verify_valid_config_returns_zero() {
    let temp_dir = create_temp_config(
        r#"model: zai-coding-plan/glm-5
model_tiers:
  b:
    - zai-coding-plan/glm-5
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 0, "valid config should return exit code 0");
}

/// Test: Missing config returns exit code 1
#[test]
fn test_verify_missing_config_returns_one() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(oya_binary())
        .args(["verify", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 1, "missing config should return exit code 1");
}

/// Test: Invalid YAML returns exit code 2
#[test]
fn test_verify_invalid_yaml_returns_two() {
    let temp_dir = create_temp_config(
        r#"model: zai-coding-plan/glm-5
model_tiers:
  - this is invalid yaml
    indentation error
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 2, "invalid YAML should return exit code 2");
}

/// Test: Empty model field returns exit code 3
#[test]
fn test_verify_empty_model_returns_three() {
    let temp_dir = create_temp_config(
        r#"model: ""
model_tiers:
  b:
    - zai-coding-plan/glm-5
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 3, "empty model field should return exit code 3");
}

/// Test: --json flag produces valid JSON output on success
#[test]
fn test_verify_json_flag_returns_valid_json() {
    let temp_dir = create_temp_config(
        r#"model: zai-coding-plan/glm-5
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify", "--json", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 0, "valid config with --json should return exit code 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output should be valid JSON");
}

/// Test: --json flag includes error details on failure
#[test]
fn test_verify_json_flag_includes_error_on_failure() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(oya_binary())
        .args(["verify", "--json", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 1, "missing config with --json should return exit code 1");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output should be valid JSON");

    assert!(json.get("error").is_some(), "JSON output should include error field");
}

/// Test: Verify current directory when no --path specified
#[test]
fn test_verify_current_directory_implicit() {
    let temp_dir = create_temp_config(
        r#"model: zai-coding-plan/glm-5
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify"])
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 0, "valid config in cwd should return exit code 0");
}

/// Test: Valid config with model_tiers validates structure
#[test]
fn test_verify_model_tiers_structure_valid() {
    let temp_dir = create_temp_config(
        r#"model_tiers:
  d:
    - zai-coding-plan/glm-4.6
  c:
    - opencode/glm-5-free
  b:
    - zai-coding-plan/glm-5
  a:
    - openai/gpt-5.3-codex
  s:
    - anthropic/claude-opus-4-6
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 0, "valid model_tiers structure should return exit code 0");
}

/// Test: Invalid model_tiers returns exit code 3
#[test]
fn test_verify_invalid_model_tiers_returns_three() {
    let temp_dir = create_temp_config(
        r#"model_tiers:
  x:
    - ""
"#,
    );

    let output = Command::new(oya_binary())
        .args(["verify", "--path"])
        .arg(temp_dir.path())
        .output()
        .expect("failed to execute oya verify");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(exit_code, 3, "model_tiers with empty model string should return exit code 3");
}
