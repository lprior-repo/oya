//! IntegrationTestWorker - Background worker for running integration tests.
//!
//! This worker discovers and runs integration tests for the zellij-frontend crate,
//! supporting test execution, result tracking, and event emission on completion.
//! Uses pure functional patterns with zero panics and zero unwrap.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Configuration for integration test execution.
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    /// Path to the crate root.
    pub crate_path: PathBuf,
    /// Test execution timeout.
    pub timeout: Duration,
    /// Whether to emit events on test completion.
    pub emit_events: bool,
    /// Maximum number of test retries on failure.
    pub max_retries: u32,
    /// Test execution mode (dev or release).
    pub mode: TestMode,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            crate_path: PathBuf::from("."),
            timeout: Duration::from_secs(300),
            emit_events: true,
            max_retries: 2,
            mode: TestMode::Dev,
        }
    }
}

/// Test execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestMode {
    /// Run tests in dev mode (faster compilation).
    Dev,
    /// Run tests in release mode (optimized).
    Release,
}

impl TestMode {
    /// Get the cargo profile flag for this mode.
    #[must_use]
    pub const fn cargo_flag(&self) -> Option<&'static str> {
        match self {
            Self::Dev => None,
            Self::Release => Some("--release"),
        }
    }
}

impl IntegrationTestConfig {
    /// Create a new integration test config.
    #[must_use]
    pub fn new(crate_path: PathBuf) -> Self {
        Self {
            crate_path,
            ..Default::default()
        }
    }

    /// Set the timeout for test execution.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set whether to emit events.
    #[must_use]
    pub const fn with_emit_events(mut self, emit: bool) -> Self {
        self.emit_events = emit;
        self
    }

    /// Set the maximum number of retries.
    #[must_use]
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the test execution mode.
    #[must_use]
    pub const fn with_mode(mut self, mode: TestMode) -> Self {
        self.mode = mode;
        self
    }

    /// Create a config for testing with shorter timeouts.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            crate_path: PathBuf::from("."),
            timeout: Duration::from_secs(30),
            emit_events: false,
            max_retries: 1,
            mode: TestMode::Dev,
        }
    }
}

/// Errors that can occur during integration test execution.
#[derive(Debug, Error)]
pub enum IntegrationTestError {
    #[error("Crate path not found: {0}")]
    CrateNotFound(PathBuf),

    #[error("Test file not found: {0}")]
    TestFileNotFound(PathBuf),

    #[error("Test execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Test timeout after {0:?}")]
    Timeout(Duration),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No tests found in crate")]
    NoTestsFound,
}

/// Result of a single integration test execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    /// Test name or identifier.
    pub test_name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Execution time in milliseconds.
    pub duration_ms: u64,
    /// Output from the test (stdout/stderr combined).
    pub output: String,
    /// Error message if the test failed.
    pub error: Option<String>,
    /// Timestamp when the test completed.
    pub timestamp: DateTime<Utc>,
}

impl TestResult {
    /// Create a successful test result.
    #[must_use]
    pub fn success(test_name: String, duration_ms: u64, output: String) -> Self {
        Self {
            test_name,
            passed: true,
            duration_ms,
            output,
            error: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a failed test result.
    #[must_use]
    pub fn failure(test_name: String, duration_ms: u64, output: String, error: String) -> Self {
        Self {
            test_name,
            passed: false,
            duration_ms,
            output,
            error: Some(error),
            timestamp: Utc::now(),
        }
    }
}

/// Summary of integration test execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSummary {
    /// Total number of tests run.
    pub total: u32,
    /// Number of tests that passed.
    pub passed: u32,
    /// Number of tests that failed.
    pub failed: u32,
    /// Number of tests that were ignored (by test attributes).
    pub ignored: u32,
    /// Total execution time in milliseconds.
    pub total_duration_ms: u64,
    /// Individual test results.
    pub test_results: Vec<TestResult>,
    /// Timestamp when the test run completed.
    pub timestamp: DateTime<Utc>,
}

impl TestSummary {
    /// Create a new test summary.
    #[must_use]
    pub fn new(
        total: u32,
        passed: u32,
        failed: u32,
        ignored: u32,
        total_duration_ms: u64,
        test_results: Vec<TestResult>,
    ) -> Self {
        Self {
            total,
            passed,
            failed,
            ignored,
            total_duration_ms,
            test_results,
            timestamp: Utc::now(),
        }
    }

    /// Check if all tests passed.
    #[must_use]
    pub const fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Calculate the pass rate as a percentage.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }

        let pass_rate = (self.passed as f64 / self.total as f64) * 100.0;
        pass_rate.min(100.0)
    }
}

/// Integration test worker.
#[derive(Clone)]
pub struct IntegrationTestWorker {
    /// Worker configuration.
    config: IntegrationTestConfig,
    /// Worker ID for identification.
    worker_id: String,
}

impl IntegrationTestWorker {
    /// Create a new integration test worker.
    #[must_use]
    pub fn new(config: IntegrationTestConfig) -> Self {
        let worker_id = format!("integration-test-worker-{}", uuid::Uuid::new_v4());
        Self { config, worker_id }
    }

    /// Get the worker ID.
    #[must_use]
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    /// Discover integration test files in the crate.
    ///
    /// Returns a list of test file paths found in the `tests/` directory.
    pub fn discover_tests(&self) -> Result<Vec<PathBuf>, IntegrationTestError> {
        let tests_dir = self.config.crate_path.join("tests");

        if !tests_dir.exists() {
            return Err(IntegrationTestError::TestFileNotFound(tests_dir));
        }

        let mut test_files = Vec::new();

        let entries = std::fs::read_dir(&tests_dir).map_err(IntegrationTestError::Io)?;

        for entry in entries {
            let entry = entry.map_err(IntegrationTestError::Io)?;
            let path = entry.path();

            // Only include .rs files
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                test_files.push(path);
            }
        }

        if test_files.is_empty() {
            return Err(IntegrationTestError::NoTestsFound);
        }

        debug!(
            worker_id = %self.worker_id,
            test_count = test_files.len(),
            "Discovered integration tests"
        );

        Ok(test_files)
    }

    /// Run all integration tests for the crate.
    ///
    /// Executes `cargo test` for integration tests and returns a summary.
    pub fn run_all_tests(&self) -> Result<TestSummary, IntegrationTestError> {
        info!(
            worker_id = %self.worker_id,
            crate_path = %self.config.crate_path.display(),
            "Starting integration test run"
        );

        let start_time = std::time::Instant::now();

        // Build the cargo test command
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.config.crate_path)
            .arg("test")
            .arg("--test")
            .arg("*");

        // Add release flag if in release mode
        if let Some(flag) = self.config.mode.cargo_flag() {
            cmd.arg(flag);
        }

        // Execute the test command
        let output = cmd.output().map_err(|e| {
            IntegrationTestError::ExecutionFailed(format!("Failed to execute cargo test: {e}"))
        })?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Parse the test output to extract results
        let test_results = self.parse_cargo_test_output(&combined_output);

        // Calculate summary statistics
        let total = test_results.len() as u32;
        let passed = test_results.iter().filter(|r| r.passed).count() as u32;
        let failed = total.saturating_sub(passed);
        let ignored = 0; // TODO: Parse ignored count from output

        let summary = TestSummary::new(total, passed, failed, ignored, duration_ms, test_results);

        info!(
            worker_id = %self.worker_id,
            total = summary.total,
            passed = summary.passed,
            failed = summary.failed,
            duration_ms = summary.total_duration_ms,
            pass_rate = summary.pass_rate(),
            "Integration test run completed"
        );

        Ok(summary)
    }

    /// Run a specific integration test file.
    ///
    /// Executes a single test file and returns the result.
    pub fn run_test_file(&self, test_file: &Path) -> Result<TestResult, IntegrationTestError> {
        let test_name = test_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!(
            worker_id = %self.worker_id,
            test_name = %test_name,
            "Running integration test file"
        );

        let start_time = std::time::Instant::now();

        // Build the cargo test command for this specific test
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.config.crate_path)
            .arg("test")
            .arg("--test")
            .arg(&test_name);

        // Add release flag if in release mode
        if let Some(flag) = self.config.mode.cargo_flag() {
            cmd.arg(flag);
        }

        // Execute the test command
        let output = cmd.output().map_err(|e| {
            IntegrationTestError::ExecutionFailed(format!("Failed to execute test: {e}"))
        })?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined_output = format!("{}\n{}", stdout, stderr);

        let passed = output.status.success();

        let result = match passed {
            true => TestResult::success(test_name, duration_ms, combined_output),
            false => TestResult::failure(
                test_name,
                duration_ms,
                combined_output,
                format!("Exit code: {:?}", output.status.code()),
            ),
        };

        Ok(result)
    }

    /// Parse cargo test output to extract individual test results.
    ///
    /// This is a simplified parser that extracts test names and pass/fail status.
    fn parse_cargo_test_output(&self, output: &str) -> Vec<TestResult> {
        let mut results = Vec::new();

        // Look for test result lines like "test test_name ... ok" or "test test_name ... FAILED"
        for line in output.lines() {
            let line = line.trim();

            // Parse test result lines
            if line.starts_with("test ")
                && (line.ends_with(" ... ok") || line.ends_with(" ... FAILED"))
            {
                let parts = line.split_whitespace().collect::<Vec<_>>();

                if parts.len() >= 4 {
                    // Use get for functional safety - we know parts.len() >= 4
                    let test_name = parts.get(1).map_or_else(String::new, |s| s.to_string());
                    let status = parts.get(3).map_or("", |s| *s);

                    let passed = status == "ok" || status == "ignored";

                    results.push(TestResult {
                        test_name,
                        passed,
                        duration_ms: 0, // Cargo doesn't always provide timing per test
                        output: line.to_string(),
                        error: if passed {
                            None
                        } else {
                            Some(format!("Test {status}"))
                        },
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        results
    }

    /// Run tests with retry logic on failure.
    ///
    /// Retries failed tests up to `max_retries` times.
    pub fn run_tests_with_retry(&self) -> Result<TestSummary, IntegrationTestError> {
        let mut attempt: u32 = 0;

        loop {
            attempt = attempt.saturating_add(1_u32);

            match self.run_all_tests() {
                Ok(summary) => {
                    if summary.all_passed() || attempt > self.config.max_retries {
                        return Ok(summary);
                    }

                    warn!(
                        worker_id = %self.worker_id,
                        attempt = attempt,
                        max_retries = self.config.max_retries,
                        failed = summary.failed,
                        "Tests failed, retrying"
                    );
                }
                Err(e) => {
                    if attempt >= self.config.max_retries {
                        return Err(e);
                    }

                    warn!(
                        worker_id = %self.worker_id,
                        attempt = attempt,
                        max_retries = self.config.max_retries,
                        error = %e,
                        "Test execution failed, retrying"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #![allow(clippy::arithmetic_side_effects)]

    use super::*;

    #[test]
    fn test_integration_test_config_default() {
        let config = IntegrationTestConfig::default();
        assert_eq!(config.crate_path, PathBuf::from("."));
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert!(config.emit_events);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.mode, TestMode::Dev);
    }

    #[test]
    fn test_integration_test_config_builder() {
        let config = IntegrationTestConfig::new(PathBuf::from("/path/to/crate"))
            .with_timeout(Duration::from_secs(600))
            .with_emit_events(false)
            .with_max_retries(5)
            .with_mode(TestMode::Release);

        assert_eq!(config.crate_path, PathBuf::from("/path/to/crate"));
        assert_eq!(config.timeout, Duration::from_secs(600));
        assert!(!config.emit_events);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.mode, TestMode::Release);
    }

    #[test]
    fn test_test_mode_cargo_flag() {
        assert_eq!(TestMode::Dev.cargo_flag(), None);
        assert_eq!(TestMode::Release.cargo_flag(), Some("--release"));
    }

    #[test]
    fn test_test_result_success() {
        let result =
            TestResult::success("test_example".to_string(), 100, "Test output".to_string());

        assert!(result.passed);
        assert_eq!(result.test_name, "test_example");
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.output, "Test output");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_test_result_failure() {
        let result = TestResult::failure(
            "test_example".to_string(),
            100,
            "Test output".to_string(),
            "Assertion failed".to_string(),
        );

        assert!(!result.passed);
        assert_eq!(result.test_name, "test_example");
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.error, Some("Assertion failed".to_string()));
    }

    #[test]
    fn test_test_summary_all_passed() {
        let summary = TestSummary::new(10, 10, 0, 0, 1000, vec![]);
        assert!(summary.all_passed());
        assert_eq!(summary.pass_rate(), 100.0);
    }

    #[test]
    fn test_test_summary_with_failures() {
        let summary = TestSummary::new(10, 7, 3, 0, 1000, vec![]);
        assert!(!summary.all_passed());
        assert_eq!(summary.pass_rate(), 70.0);
    }

    #[test]
    fn test_test_summary_empty() {
        let summary = TestSummary::new(0, 0, 0, 0, 0, vec![]);
        assert!(summary.all_passed());
        assert_eq!(summary.pass_rate(), 100.0);
    }

    #[test]
    fn test_integration_test_worker_new() {
        let config = IntegrationTestConfig::default();
        let worker = IntegrationTestWorker::new(config);

        assert!(worker.worker_id().starts_with("integration-test-worker-"));
    }

    #[test]
    fn test_parse_cargo_test_output() {
        let config = IntegrationTestConfig::default();
        let worker = IntegrationTestWorker::new(config);

        let output = r#"
running 3 tests
test test_foo ... ok
test test_bar ... ok
test test_baz ... FAILED

test result: ok. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
"#;

        let results = worker.parse_cargo_test_output(output);

        assert_eq!(results.len(), 3);
        if let Some(r0) = results.get(0) {
            if let Some(r1) = results.get(1) {
                if let Some(r2) = results.get(2) {
                    assert_eq!(r0.test_name, "test_foo");
                    assert!(r0.passed);
                    assert_eq!(r1.test_name, "test_bar");
                    assert!(r1.passed);
                    assert_eq!(r2.test_name, "test_baz");
                    assert!(!r2.passed);
                }
            }
        }
        assert_eq!(r1.test_name, "test_bar");
        assert!(r1.passed);
        assert_eq!(r2.test_name, "test_baz");
        assert!(!r2.passed);
    }

    #[test]
    fn test_integration_test_config_for_testing() {
        let config = IntegrationTestConfig::for_testing();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.emit_events);
        assert_eq!(config.max_retries, 1);
        assert_eq!(config.mode, TestMode::Dev);
    }
}
