// Integration tests for the IntegrationTestWorker
// These tests verify the worker's functionality with real test execution

use std::fs;
use std::path::Path;
use tempfile::tempdir;

use zellij_frontend::{IntegrationTestConfig, IntegrationTestError, IntegrationTestWorker};

#[test]
fn test_worker_discovers_and_runs_integration_tests() {
    // Create a temporary directory for testing
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let crate_path = temp_dir.path();

    // Create a simple test file
    let tests_dir = crate_path.join("tests");
    fs::create_dir_all(&tests_dir).expect("Failed to create tests dir");

    let test_file = tests_dir.join("test_example.rs");
    fs::write(
        &test_file,
        r#"
        #[test]
        fn test_example() {
            assert_eq!(1 + 1, 2);
        }
    "#,
    )
    .expect("Failed to write test file");

    // Create the worker
    let config = IntegrationTestConfig::new(crate_path.to_path_buf());
    let worker = IntegrationTestWorker::new(config);

    // This test should fail initially
    let result = worker.run_all_tests();

    assert!(result.is_ok(), "Worker should be able to run tests");
    let summary = result.unwrap();
    assert!(summary.all_passed(), "All tests should pass");
    assert_eq!(summary.total, 1, "Should find 1 test");
}

#[test]
fn test_worker_handles_missing_tests_directory() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let crate_path = temp_dir.path();

    // Don't create tests directory - this should cause an error
    let config = IntegrationTestConfig::new(crate_path.to_path_buf());
    let worker = IntegrationTestWorker::new(config);

    // This test should fail initially
    let result = worker.run_all_tests();

    assert!(
        result.is_err(),
        "Should fail when tests directory is missing"
    );
    if let Err(e) = result {
        assert!(matches!(e, IntegrationTestError::TestFileNotFound(_)));
    }
}

#[test]
fn test_worker_runs_specific_test_file() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let crate_path = temp_dir.path();

    let tests_dir = crate_path.join("tests");
    fs::create_dir_all(&tests_dir).expect("Failed to create tests dir");

    let test_file = tests_dir.join("specific_test.rs");
    fs::write(
        &test_file,
        r#"
        #[test]
        fn test_specific() {
            assert_eq!(2 + 2, 4);
        }
    "#,
    )
    .expect("Failed to write test file");

    let config = IntegrationTestConfig::new(crate_path.to_path_buf());
    let worker = IntegrationTestWorker::new(config);

    // This test should fail initially
    let result = worker.run_test_file(&test_file);

    assert!(result.is_ok(), "Should be able to run specific test file");
    let test_result = result.unwrap();
    assert!(test_result.passed, "Specific test should pass");
}
