//! Pipeline composition for stage execution.
//!
//! Provides a functional, composable way to build and execute CI/CD pipelines.
//! Uses the builder pattern with method chaining for ergonomic API.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use crate::domain::{Language, Stage, Task};
use crate::error::{Error, Result};
use crate::retry::{retry_on_retryable, RetryConfig};
use crate::stages::execute_stage;

/// Result of executing a single stage.
#[derive(Debug, Clone)]
pub struct StageExecution {
    /// Name of the stage.
    pub stage_name: String,
    /// Whether the stage passed.
    pub passed: bool,
    /// Duration of the stage execution.
    pub duration: Duration,
    /// Number of attempts made (including retries).
    pub attempts: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

impl StageExecution {
    /// Create a successful stage execution.
    #[must_use]
    pub fn success(stage_name: impl Into<String>, duration: Duration, attempts: u32) -> Self {
        Self {
            stage_name: stage_name.into(),
            passed: true,
            duration,
            attempts,
            error: None,
        }
    }

    /// Create a failed stage execution.
    #[must_use]
    pub fn failure(
        stage_name: impl Into<String>,
        duration: Duration,
        attempts: u32,
        error: impl Into<String>,
    ) -> Self {
        Self {
            stage_name: stage_name.into(),
            passed: false,
            duration,
            attempts,
            error: Some(error.into()),
        }
    }
}

/// Result of executing a complete pipeline.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// All stage executions in order.
    pub stages: Vec<StageExecution>,
    /// Total duration of the pipeline.
    pub total_duration: Duration,
    /// Whether all stages passed.
    pub all_passed: bool,
    /// Index of first failed stage (if any).
    pub first_failure: Option<usize>,
}

impl PipelineResult {
    /// Get all passed stages.
    #[must_use]
    pub fn passed_stages(&self) -> Vec<&StageExecution> {
        self.stages.iter().filter(|s| s.passed).collect()
    }

    /// Get all failed stages.
    #[must_use]
    pub fn failed_stages(&self) -> Vec<&StageExecution> {
        self.stages.iter().filter(|s| !s.passed).collect()
    }

    /// Get success rate as a percentage.
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.stages.is_empty() {
            return 100.0;
        }
        let passed = self.stages.iter().filter(|s| s.passed).count();
        (passed as f64 / self.stages.len() as f64) * 100.0
    }

    /// Convert to Result, returning Ok(()) if all passed, Err otherwise.
    pub fn into_result(self) -> Result<()> {
        if self.all_passed {
            Ok(())
        } else {
            let failed = self
                .stages
                .iter()
                .find(|s| !s.passed)
                .map(|s| format!("{}: {}", s.stage_name, s.error.as_deref().unwrap_or("unknown")))
                .unwrap_or_else(|| "unknown failure".to_string());
            Err(Error::InvalidRecord { reason: failed })
        }
    }
}

/// Strategy for handling stage failures.
#[derive(Debug, Clone, Copy, Default)]
pub enum FailureStrategy {
    /// Stop pipeline on first failure.
    #[default]
    StopOnFirst,
    /// Continue executing remaining stages even after failure.
    ContinueOnFailure,
    /// Skip dependent stages but continue with independent ones.
    SkipDependents,
}

/// Hook that can be called before/after stage execution.
pub type StageHook = Box<dyn Fn(&str, &Language, &Path) + Send + Sync>;

/// Builder for constructing and executing pipelines.
pub struct Pipeline {
    /// Stages to execute.
    stages: Vec<Stage>,
    /// Language for the pipeline.
    language: Language,
    /// Working directory.
    worktree_path: PathBuf,
    /// Retry configuration.
    retry_config: RetryConfig,
    /// Failure handling strategy.
    failure_strategy: FailureStrategy,
    /// Hook called before each stage.
    before_stage: Option<StageHook>,
    /// Hook called after each stage.
    after_stage: Option<StageHook>,
    /// Whether to run in dry-run mode.
    dry_run: bool,
}

impl Pipeline {
    /// Create a new pipeline builder.
    #[must_use]
    pub fn new(language: Language, worktree_path: PathBuf) -> Self {
        Self {
            stages: Vec::new(),
            language,
            worktree_path,
            retry_config: RetryConfig::default(),
            failure_strategy: FailureStrategy::default(),
            before_stage: None,
            after_stage: None,
            dry_run: false,
        }
    }

    /// Create a pipeline from a task.
    #[must_use]
    pub fn from_task(task: &Task) -> Self {
        Self::new(task.language, task.worktree_path.clone())
    }

    /// Add a single stage to the pipeline.
    #[must_use]
    pub fn with_stage(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Add multiple stages to the pipeline.
    #[must_use]
    pub fn with_stages(mut self, stages: impl IntoIterator<Item = Stage>) -> Self {
        self.stages.extend(stages);
        self
    }

    /// Set the retry configuration.
    #[must_use]
    pub fn with_retry(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Set the failure strategy.
    #[must_use]
    pub fn with_failure_strategy(mut self, strategy: FailureStrategy) -> Self {
        self.failure_strategy = strategy;
        self
    }

    /// Set a hook to run before each stage.
    #[must_use]
    pub fn before_each<F>(mut self, hook: F) -> Self
    where
        F: Fn(&str, &Language, &Path) + Send + Sync + 'static,
    {
        self.before_stage = Some(Box::new(hook));
        self
    }

    /// Set a hook to run after each stage.
    #[must_use]
    pub fn after_each<F>(mut self, hook: F) -> Self
    where
        F: Fn(&str, &Language, &Path) + Send + Sync + 'static,
    {
        self.after_stage = Some(Box::new(hook));
        self
    }

    /// Enable dry-run mode.
    #[must_use]
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Execute the pipeline.
    pub fn execute(self) -> PipelineResult {
        let start = Instant::now();
        let mut executions = Vec::new();
        let mut should_stop = false;

        for stage in &self.stages {
            if should_stop {
                debug!(stage = %stage.name, "Skipping stage due to previous failure");
                continue;
            }

            let execution = self.execute_single_stage(stage);
            let passed = execution.passed;
            executions.push(execution);

            if !passed {
                match self.failure_strategy {
                    FailureStrategy::StopOnFirst => {
                        should_stop = true;
                    }
                    FailureStrategy::ContinueOnFailure => {
                        // Continue with next stage
                    }
                    FailureStrategy::SkipDependents => {
                        // For now, treat as stop (dependency tracking not implemented)
                        should_stop = true;
                    }
                }
            }
        }

        let first_failure = executions.iter().position(|e| !e.passed);
        let all_passed = first_failure.is_none();

        PipelineResult {
            stages: executions,
            total_duration: start.elapsed(),
            all_passed,
            first_failure,
        }
    }

    /// Execute and return Result directly.
    pub fn run(self) -> Result<PipelineResult> {
        let language = self.language;
        let result = self.execute();
        if result.all_passed {
            Ok(result)
        } else {
            // Still return the result for inspection, wrapped in Err
            Err(Error::StageFailed {
                language: language.to_string(),
                stage: result
                    .first_failure
                    .and_then(|i| result.stages.get(i))
                    .map(|s| s.stage_name.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                reason: result
                    .first_failure
                    .and_then(|i| result.stages.get(i))
                    .and_then(|s| s.error.clone())
                    .unwrap_or_else(|| "unknown error".to_string()),
            })
        }
    }

    fn execute_single_stage(&self, stage: &Stage) -> StageExecution {
        let stage_start = Instant::now();

        // Call before hook
        if let Some(ref hook) = self.before_stage {
            hook(&stage.name, &self.language, &self.worktree_path);
        }

        if self.dry_run {
            info!(stage = %stage.name, "DRY RUN: would execute stage");
            return StageExecution::success(&stage.name, stage_start.elapsed(), 1);
        }

        // Execute with retry
        let retry_config = RetryConfig::default().with_max_attempts(stage.retries);
        let language = self.language;
        let worktree_path = self.worktree_path.clone();
        let stage_name = stage.name.clone();

        let mut attempts = 0u32;
        let result = retry_on_retryable(&retry_config, || {
            attempts += 1;
            execute_stage(&stage_name, language, &worktree_path)
        });

        // Call after hook
        if let Some(ref hook) = self.after_stage {
            hook(&stage.name, &self.language, &self.worktree_path);
        }

        let duration = stage_start.elapsed();

        match result {
            Ok(()) => {
                info!(
                    stage = %stage.name,
                    duration_ms = duration.as_millis(),
                    attempts,
                    "Stage passed"
                );
                StageExecution::success(&stage.name, duration, attempts)
            }
            Err(e) => {
                warn!(
                    stage = %stage.name,
                    duration_ms = duration.as_millis(),
                    attempts,
                    error = %e,
                    "Stage failed"
                );
                StageExecution::failure(&stage.name, duration, attempts, e.to_string())
            }
        }
    }
}

/// Functional helper to execute stages in sequence.
pub fn execute_stages_sequentially(
    stages: &[Stage],
    language: Language,
    worktree_path: &Path,
) -> PipelineResult {
    Pipeline::new(language, worktree_path.to_path_buf())
        .with_stages(stages.iter().cloned())
        .execute()
}

/// Functional helper to execute a single stage with retries.
pub fn execute_stage_with_retry(
    stage: &Stage,
    language: Language,
    worktree_path: &Path,
    retry_config: &RetryConfig,
) -> StageExecution {
    let start = Instant::now();
    let mut attempts = 0u32;

    let result = retry_on_retryable(retry_config, || {
        attempts += 1;
        execute_stage(&stage.name, language, worktree_path)
    });

    match result {
        Ok(()) => StageExecution::success(&stage.name, start.elapsed(), attempts),
        Err(e) => StageExecution::failure(&stage.name, start.elapsed(), attempts, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::standard_pipeline;

    #[test]
    fn test_pipeline_builder() {
        let pipeline = Pipeline::new(Language::Rust, PathBuf::from("/tmp"))
            .with_retry(RetryConfig::quick())
            .with_failure_strategy(FailureStrategy::StopOnFirst)
            .dry_run();

        assert!(pipeline.dry_run);
    }

    #[test]
    fn test_stage_execution_success() {
        let exec = StageExecution::success("implement", Duration::from_millis(100), 1);
        assert!(exec.passed);
        assert!(exec.error.is_none());
    }

    #[test]
    fn test_stage_execution_failure() {
        let exec = StageExecution::failure("lint", Duration::from_millis(50), 2, "formatting error");
        assert!(!exec.passed);
        assert_eq!(exec.error.as_deref(), Some("formatting error"));
    }

    #[test]
    fn test_pipeline_result_success_rate() {
        let result = PipelineResult {
            stages: vec![
                StageExecution::success("a", Duration::ZERO, 1),
                StageExecution::success("b", Duration::ZERO, 1),
                StageExecution::failure("c", Duration::ZERO, 1, "err"),
                StageExecution::success("d", Duration::ZERO, 1),
            ],
            total_duration: Duration::ZERO,
            all_passed: false,
            first_failure: Some(2),
        };

        assert!((result.success_rate() - 75.0).abs() < 0.01);
        assert_eq!(result.passed_stages().len(), 3);
        assert_eq!(result.failed_stages().len(), 1);
    }

    #[test]
    fn test_dry_run_pipeline() {
        let stages = standard_pipeline();
        let result = Pipeline::new(Language::Rust, PathBuf::from("/tmp"))
            .with_stages(stages)
            .dry_run()
            .execute();

        assert!(result.all_passed);
        assert_eq!(result.stages.len(), 9);
    }
}
