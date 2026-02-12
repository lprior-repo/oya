use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use zellij_frontend::{IntegrationTestConfig, IntegrationTestError, IntegrationTestWorker};

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, content)?;
    Ok(())
}

fn create_minimal_test_crate() -> Result<(TempDir, PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let crate_root = temp_dir.path().to_path_buf();
    let tests_dir = crate_root.join("tests");
    fs::create_dir_all(&tests_dir)?;

    write_file(
        &crate_root.join("Cargo.toml"),
        "[package]\nname = \"integration-test-worker-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;

    fs::create_dir_all(crate_root.join("src"))?;
    write_file(
        &crate_root.join("src").join("lib.rs"),
        "pub fn fixture() {}\n",
    )?;

    write_file(
        &tests_dir.join("worker_pass.rs"),
        "#[test]\nfn worker_pass() { assert_eq!(2 + 2, 4); }\n",
    )?;

    write_file(
        &tests_dir.join("worker_other.rs"),
        "#[test]\nfn worker_other() { assert!(true); }\n",
    )?;

    Ok((temp_dir, crate_root))
}

#[test]
fn discover_tests_returns_only_rust_test_files() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp_dir, crate_root) = create_minimal_test_crate()?;
    write_file(
        &crate_root.join("tests").join("README.md"),
        "not a rust test\n",
    )?;

    let worker = IntegrationTestWorker::new(IntegrationTestConfig::new(crate_root));
    let tests = worker.discover_tests()?;

    assert_eq!(tests.len(), 2);
    assert!(tests
        .iter()
        .all(|path| path.extension().and_then(|s| s.to_str()) == Some("rs")));
    Ok(())
}

#[test]
fn discover_tests_errors_when_tests_dir_missing() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let worker =
        IntegrationTestWorker::new(IntegrationTestConfig::new(temp_dir.path().to_path_buf()));

    let result = worker.discover_tests();
    assert!(matches!(
        result,
        Err(IntegrationTestError::TestFileNotFound(_))
    ));
    Ok(())
}

#[test]
fn run_test_file_executes_named_integration_test() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp_dir, crate_root) = create_minimal_test_crate()?;
    let test_file = crate_root.join("tests").join("worker_pass.rs");
    let worker = IntegrationTestWorker::new(IntegrationTestConfig::new(crate_root));

    let result = worker.run_test_file(&test_file)?;

    assert!(result.passed);
    assert_eq!(result.test_name, "worker_pass");
    Ok(())
}

#[test]
fn run_all_tests_discovers_and_executes_suite() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp_dir, crate_root) = create_minimal_test_crate()?;
    let worker = IntegrationTestWorker::new(IntegrationTestConfig::new(crate_root));

    let summary = worker.run_all_tests()?;

    assert_eq!(summary.total, 2);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.failed, 0);
    assert!(summary.all_passed());
    Ok(())
}

#[test]
fn run_test_file_errors_for_missing_file() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp_dir, crate_root) = create_minimal_test_crate()?;
    let worker = IntegrationTestWorker::new(IntegrationTestConfig::new(crate_root.clone()));
    let missing = crate_root.join("tests").join("does_not_exist.rs");

    let result = worker.run_test_file(&missing);

    assert!(matches!(
        result,
        Err(IntegrationTestError::TestFileNotFound(path)) if path == missing
    ));
    Ok(())
}

#[test]
fn run_all_tests_reports_mixed_results() -> Result<(), Box<dyn std::error::Error>> {
    let (temp_dir, crate_root) = create_minimal_test_crate()?;
    write_file(
        &crate_root.join("tests").join("worker_fail.rs"),
        "#[test]\nfn worker_fail() { assert_eq!(1, 2); }\n",
    )?;

    let _hold = temp_dir;
    let worker = IntegrationTestWorker::new(IntegrationTestConfig::new(crate_root));
    let summary = worker.run_all_tests()?;

    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.failed, 1);
    assert!(!summary.all_passed());
    Ok(())
}

#[test]
fn run_all_tests_errors_when_tests_dir_missing() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let worker =
        IntegrationTestWorker::new(IntegrationTestConfig::new(temp_dir.path().to_path_buf()));

    let result = worker.run_all_tests();
    assert!(matches!(
        result,
        Err(IntegrationTestError::TestFileNotFound(_))
    ));
    Ok(())
}
