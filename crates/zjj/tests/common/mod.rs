// Common test infrastructure for zjj tests

use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;

pub static ZJJ_BIN: &str = "zjj";

/// Set up a test JJ repository with basic configuration
pub fn setup_test_repo(name: &str) -> PathBuf {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.into_path();

    // Initialize JJ repo
    let status = Command::new("jj")
        .args(&["init", "--git"])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    assert!(status.success(), "Failed to initialize jj repo");

    // Configure JJ
    let status = Command::new("jj")
        .args(&["config", "set", "--repo", "user.name", "\"Test User\""])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    let status = Command::new("jj")
        .args(&["config", "set", "--repo", "user.email", "\"test@example.com\""])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    // Create initial commit
    fs::write(repo_path.join("initial.txt"), "initial content").unwrap();

    let status = Command::new("jj")
        .args(&["commit", "-m", "initial commit"])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    repo_path
}

/// Run a zjj command and return the result
pub fn run_zjj(args: &[&str], dir: &Path) -> std::process::Output {
    Command::new(ZJJ_BIN)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute zjj {:?}: {}", args, e))
}

/// Run a zjj command and expect success
pub fn run_zjj_expect_ok(args: &[&str], dir: &Path) -> std::process::Output {
    let result = run_zjj(args, dir);
    if !result.status.success() {
        panic!(
            "zjj {:?} failed:\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    result
}

/// Run a zjj command and expect failure
pub fn run_zjj_expect_fail(args: &[&str], dir: &Path) -> std::process::Output {
    let result = run_zjj(args, dir);
    if result.status.success() {
        panic!(
            "zjj {:?} should have failed but succeeded:\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    result
}

/// Clean up test repository
pub fn cleanup_test_repo(repo_path: PathBuf) {
    // TempDir will handle cleanup when dropped
    let _ = repo_path;
}
