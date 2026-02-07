//! Stage execution tracking with progress reporting and artifact collection.
//!
//! Provides real-time progress feedback for long-running stages,
//! and captures artifacts (coverage reports, test results, etc.).

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use crate::error::{Error, Result};
use crate::process::run_command;

/// Progress level for stage execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressLevel {
    Silent,
    Minimal,
    Normal,
    Verbose,
}

/// Progress update for stage execution.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub timestamp: Instant,
    pub message: String,
    pub percent_complete: f64,
    pub elapsed: Duration,
    pub estimated_remaining: Option<Duration>,
}

/// Artifact type produced by a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    TestResults,
    CoverageReport,
    LintReport,
    SecurityReport,
    BuildArtifacts,
    Logs,
    Other(&'static str),
}

/// Artifact captured during stage execution.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub artifact_type: ArtifactType,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub content: Option<String>,
}

impl ArtifactType {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::TestResults => "test_results",
            Self::CoverageReport => "coverage_report",
            Self::LintReport => "lint_report",
            Self::SecurityReport => "security_report",
            Self::BuildArtifacts => "build_artifacts",
            Self::Logs => "logs",
            Self::Other(name) => name,
        }
    }
}

/// Stage execution context with progress tracking.
#[derive(Debug)]
pub struct ExecutionContext {
    pub stage_name: String,
    pub worktree_path: PathBuf,
    pub start_time: Instant,
    pub progress_level: ProgressLevel,
    pub artifacts: Vec<Artifact>,
    pub metadata: HashMap<String, String>,
}

impl ExecutionContext {
    /// Create a new execution context.
    #[must_use]
    pub fn new(stage_name: impl Into<String>, worktree_path: PathBuf) -> Self {
        Self {
            stage_name: stage_name.into(),
            worktree_path,
            start_time: Instant::now(),
            progress_level: ProgressLevel::Normal,
            artifacts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set progress level.
    #[must_use]
    pub fn with_progress_level(mut self, level: ProgressLevel) -> Self {
        self.progress_level = level;
        self
    }

    /// Add metadata key-value.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Report progress.
    pub fn report_progress(&self, message: impl Into<String>, percent: f64) {
        let elapsed = self.start_time.elapsed();
        let update = ProgressUpdate {
            timestamp: Instant::now(),
            message: message.into(),
            percent_complete: percent.clamp(0.0, 100.0),
            elapsed,
            estimated_remaining: None,
        };

        match self.progress_level {
            ProgressLevel::Silent => {}
            ProgressLevel::Minimal if (percent as i32) % 25 == 0 => {
                info!(
                    stage = %self.stage_name,
                    percent,
                    elapsed_ms = elapsed.as_millis(),
                    "{}",
                    update.message
                );
            }
            ProgressLevel::Minimal | ProgressLevel::Normal => {
                info!(
                    stage = %self.stage_name,
                    percent,
                    elapsed_ms = elapsed.as_millis(),
                    "{}",
                    update.message
                );
            }
            ProgressLevel::Verbose => {
                debug!(
                    stage = %self.stage_name,
                    percent,
                    elapsed_ms = elapsed.as_millis(),
                    "{}",
                    update.message
                );
            }
        }
    }

    /// Capture an artifact.
    pub fn capture_artifact(&mut self, artifact_type: ArtifactType, path: PathBuf) -> Result<()> {
        if !path.exists() {
            warn!(?path, "Artifact path does not exist");
            return Ok(());
        }

        let metadata = std::fs::metadata(&path).map_err(|e| Error::file_read_failed(&path, e))?;

        let size_bytes = metadata.len();
        let content = if metadata.len() < 1_000_000 {
            // Only read if under 1MB
            std::fs::read_to_string(&path).ok()
        } else {
            None
        };

        let artifact = Artifact {
            artifact_type,
            path: path.clone(),
            size_bytes,
            content,
        };

        info!(
            stage = %self.stage_name,
            artifact_type = artifact_type.name(),
            path = ?path,
            size_bytes,
            "Artifact captured"
        );

        self.artifacts.push(artifact);
        Ok(())
    }

    /// Get elapsed time.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Find artifact by type.
    #[must_use]
    pub fn find_artifact(&self, artifact_type: &ArtifactType) -> Option<&Artifact> {
        self.artifacts
            .iter()
            .find(|a| &a.artifact_type == artifact_type)
    }

    /// Find all artifacts of a type.
    #[must_use]
    pub fn find_artifacts(&self, artifact_type: &ArtifactType) -> Vec<&Artifact> {
        self.artifacts
            .iter()
            .filter(|a| &a.artifact_type == artifact_type)
            .collect()
    }
}

/// Collect common artifacts after stage execution.
pub fn collect_artifacts(
    ctx: &mut ExecutionContext,
    stage_name: &str,
    language: &str,
) -> Result<()> {
    // Collect coverage reports
    if stage_name == "coverage" {
        let coverage_files = coverage_patterns(language)
            .iter()
            .flat_map(|pattern| {
                glob::glob(pattern)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|entry| entry.is_file())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for file in coverage_files {
            ctx.capture_artifact(ArtifactType::CoverageReport, file)?;
        }
    }

    // Collect test results
    if stage_name == "unit-test" || stage_name == "integration" {
        let test_files = test_result_patterns(language)
            .iter()
            .flat_map(|pattern| {
                glob::glob(pattern)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|entry| entry.is_file())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for file in test_files {
            ctx.capture_artifact(ArtifactType::TestResults, file)?;
        }
    }

    // Collect lint reports
    if stage_name == "lint" {
        let lint_files = lint_report_patterns(language)
            .iter()
            .flat_map(|pattern| {
                glob::glob(pattern)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|entry| entry.is_file())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for file in lint_files {
            ctx.capture_artifact(ArtifactType::LintReport, file)?;
        }
    }

    // Collect security reports
    if stage_name == "security" {
        let security_files = security_report_patterns(language)
            .iter()
            .flat_map(|pattern| {
                glob::glob(pattern)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|entry| entry.is_file())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for file in security_files {
            ctx.capture_artifact(ArtifactType::SecurityReport, file)?;
        }
    }

    Ok(())
}

/// Get coverage report file patterns for a language.
fn coverage_patterns(language: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "rust" => vec![
            "**/*.xml".to_string(),
            "**/cobertura.xml".to_string(),
            "**/lcov.info".to_string(),
        ],
        "go" => vec!["**/coverage.out".to_string(), "**/*.out".to_string()],
        "python" => vec![
            "**/.coverage".to_string(),
            "**/coverage.xml".to_string(),
            "**/htmlcov/**/*".to_string(),
        ],
        "gleam" => vec!["**/test/*.gleam".to_string()],
        "javascript" | "js" => vec![
            "**/coverage/**/*".to_string(),
            "**/coverage.json".to_string(),
            "**/lcov.info".to_string(),
        ],
        _ => Vec::new(),
    }
}

/// Get test result file patterns for a language.
fn test_result_patterns(language: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "rust" => vec!["**/test-results/**/*".to_string()],
        "go" => vec!["**/*.test".to_string()],
        "python" => vec![
            "**/.pytest_cache/**/*".to_string(),
            "**/test-results/**/*".to_string(),
        ],
        "gleam" => vec!["**/build/dev/erlang/*/test/**/*.beam".to_string()],
        "javascript" | "js" => vec![
            "**/test-results/**/*".to_string(),
            "**/*.test.js".to_string(),
        ],
        _ => Vec::new(),
    }
}

/// Get lint report patterns for a language.
fn lint_report_patterns(language: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "rust" => vec!["**/clippy-report/**/*.html".to_string()],
        "go" => vec!["**/*_lint.log".to_string()],
        "python" => vec![
            "**/.ruff_cache/**/*".to_string(),
            "**/ruff-report.json".to_string(),
        ],
        "gleam" => vec!["**/.gleam/**/*".to_string()],
        "javascript" | "js" => vec![
            "**/eslint-report.json".to_string(),
            "**/eslint-report.html".to_string(),
        ],
        _ => Vec::new(),
    }
}

/// Get security report patterns for a language.
fn security_report_patterns(language: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "rust" => vec!["**/audit*.json".to_string(), "**/audit*.txt".to_string()],
        "go" => vec!["**/gosec*.json".to_string(), "**/gosec*.txt".to_string()],
        "python" => vec!["**/bandit*.json".to_string(), "**/bandit*.txt".to_string()],
        "gleam" => vec!["**/gleam.lock".to_string()],
        "javascript" | "js" => vec![
            "**/npm-audit*.json".to_string(),
            "**/audit*.json".to_string(),
        ],
        _ => Vec::new(),
    }
}

/// Execute command with progress tracking.
pub fn execute_with_progress(
    ctx: &ExecutionContext,
    cmd: &str,
    args: &[&str],
) -> Result<crate::process::CommandResult> {
    let cmd_display = format!("{} {}", cmd, args.join(" "));

    ctx.report_progress(format!("Executing: {}", cmd_display), 0.0);

    let result = run_command(cmd, args, &ctx.worktree_path)?;

    ctx.report_progress("Command completed", 100.0);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_update() {
        let ctx = ExecutionContext::new("test-stage", PathBuf::from("/tmp"));

        ctx.report_progress("Starting", 0.0);
        ctx.report_progress("Halfway", 50.0);
        ctx.report_progress("Complete", 100.0);

        // Should not panic
    }

    #[test]
    fn test_execution_context_metadata() {
        let ctx = ExecutionContext::new("test-stage", PathBuf::from("/tmp"))
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        assert_eq!(ctx.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_coverage_patterns() {
        let rust_patterns = coverage_patterns("rust");
        assert!(rust_patterns.iter().any(|p| p.contains("xml")));
        assert!(rust_patterns.iter().any(|p| p.contains("lcov")));

        let python_patterns = coverage_patterns("python");
        assert!(python_patterns.iter().any(|p| p.contains(".coverage")));
    }

    #[test]
    fn test_artifact_type_display() {
        let artifact_type = ArtifactType::CoverageReport;
        // Artifact types are used for matching and filtering
        let is_coverage = matches!(artifact_type, ArtifactType::CoverageReport);
        assert!(is_coverage);
    }
}
