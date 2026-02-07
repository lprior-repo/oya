//! Type-state builders for constructing domain objects.
//!
//! Uses the type-state pattern to ensure required fields are set at compile time.
//! This eliminates runtime errors from missing required fields.
//!
//! # Example
//!
//! ```ignore
//! let task = TaskBuilder::new()
//!     .slug("my-task")?           // Required
//!     .language(Language::Rust)    // Required
//!     .worktree("/path/to/worktree")? // Required
//!     .priority(Priority::P1)      // Optional
//!     .build()?;
//! ```

use std::marker::PhantomData;

use crate::domain::{Language, Priority, Slug, Task, TaskStatus};
use crate::error::{Error, Result};

// =============================================================================
// Type-State Markers
// =============================================================================

/// Marker for a field that has not been set.
pub struct Missing;

/// Marker for a field that has been set.
pub struct Present;

/// Marker for an optional field.
pub struct Optional;

// =============================================================================
// TaskBuilder - Type-State Builder for Task
// =============================================================================

/// Builder for creating Task instances with type-state validation.
///
/// Required fields must be set before `build()` can be called.
/// The type system enforces this at compile time.
#[derive(Debug)]
pub struct TaskBuilder<SlugState, LangState> {
    slug: Option<Slug>,
    language: Option<Language>,
    priority: Priority,
    status: TaskStatus,
    branch: Option<String>,
    _slug_state: PhantomData<SlugState>,
    _lang_state: PhantomData<LangState>,
}

impl TaskBuilder<Missing, Missing> {
    /// Create a new TaskBuilder with no fields set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slug: None,
            language: None,

            priority: Priority::default(),
            status: TaskStatus::default(),
            branch: None,
            _slug_state: PhantomData,
            _lang_state: PhantomData,
        }
    }
}

impl Default for TaskBuilder<Missing, Missing> {
    fn default() -> Self {
        Self::new()
    }
}

impl<SlugState, LangState> TaskBuilder<SlugState, LangState> {
    /// Set the priority (optional, defaults to P2).
    #[must_use]
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the initial status (optional, defaults to Created).
    #[must_use]
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Set a custom branch name (optional, defaults to `feat/<slug>`).
    #[must_use]
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }
}

// Slug setter - transitions Missing -> Present
impl<LangState> TaskBuilder<Missing, LangState> {
    /// Set the task slug (required).
    pub fn slug(self, slug: impl Into<String>) -> Result<TaskBuilder<Present, LangState>> {
        let validated_slug = Slug::new(slug)?;
        Ok(TaskBuilder {
            slug: Some(validated_slug),
            language: self.language,

            priority: self.priority,
            status: self.status,
            branch: self.branch,
            _slug_state: PhantomData,
            _lang_state: PhantomData,
        })
    }
}

// Language setter - transitions Missing -> Present
impl<SlugState> TaskBuilder<SlugState, Missing> {
    /// Set the language (required).
    #[must_use]
    pub fn language(self, language: Language) -> TaskBuilder<SlugState, Present> {
        TaskBuilder {
            slug: self.slug,
            language: Some(language),

            priority: self.priority,
            status: self.status,
            branch: self.branch,
            _slug_state: PhantomData,
            _lang_state: PhantomData,
        }
    }
}

// Build is only available when all required fields are Present
impl TaskBuilder<Present, Present> {
    /// Build the Task.
    ///
    /// Only available when slug and language have been set.
    /// Type-state guarantees these fields are present, so this always succeeds.
    pub fn build(self) -> Result<Task> {
        let slug = self
            .slug
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: "slug not set (type-state violation)".into(),
            })?;
        let language = self
            .language
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: "language not set (type-state violation)".into(),
            })?;

        let branch = self
            .branch
            .unwrap_or_else(|| format!("{}{}", Task::BRANCH_PREFIX, slug));

        Ok(Task {
            slug,
            language,
            status: self.status,
            priority: self.priority,
            branch,
        })
    }
}

// =============================================================================
// Fluent Extensions for Task
// =============================================================================

impl Task {
    /// Start building a new Task.
    #[must_use]
    pub fn builder() -> TaskBuilder<Missing, Missing> {
        TaskBuilder::new()
    }

    /// Start a stage (functional status transition).
    #[must_use]
    pub fn start_stage(self, stage_name: impl Into<String>) -> Self {
        Task::with_status(
            self,
            TaskStatus::InProgress {
                stage: stage_name.into(),
            },
        )
    }

    /// Mark task as passed.
    #[must_use]
    pub fn mark_passed(self) -> Self {
        Task::with_status(self, TaskStatus::PassedPipeline)
    }

    /// Mark task as failed.
    #[must_use]
    pub fn mark_failed(self, stage: impl Into<String>, reason: impl Into<String>) -> Self {
        Task::with_status(
            self,
            TaskStatus::FailedPipeline {
                stage: stage.into(),
                reason: reason.into(),
            },
        )
    }

    /// Mark task as integrated.
    #[must_use]
    pub fn mark_integrated(self) -> Self {
        Task::with_status(self, TaskStatus::Integrated)
    }
}

// =============================================================================
// StageBuilder - Builder for custom stages
// =============================================================================

/// Builder for creating custom Stage definitions.
#[derive(Debug, Default)]
pub struct StageBuilder {
    name: Option<String>,
    gate: Option<String>,
    retries: u32,
}

impl StageBuilder {
    /// Create a new StageBuilder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the stage name (required).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the gate description (required).
    #[must_use]
    pub fn gate(mut self, gate: impl Into<String>) -> Self {
        self.gate = Some(gate.into());
        self
    }

    /// Set the retry count (optional, defaults to 0).
    #[must_use]
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Build the Stage.
    ///
    /// Returns None if required fields are missing.
    #[must_use]
    pub fn build(self) -> Option<Stage> {
        Some(Stage::new(self.name?, self.gate?, self.retries))
    }

    /// Build the Stage, using defaults for missing optional fields.
    ///
    /// Returns Err if required fields are missing.
    pub fn try_build(self) -> Result<Stage> {
        let name = self
            .name
            .ok_or_else(|| crate::error::Error::InvalidRecord {
                reason: "stage name is required".into(),
            })?;
        let gate = self.gate.unwrap_or_else(|| format!("{name} passes"));

        Ok(Stage::new(name, gate, self.retries))
    }
}

// =============================================================================
// PipelineBuilder - Railway-Oriented Builder for Pipeline
// =============================================================================

use crate::domain::Stage;
use crate::pipeline::{FailureStrategy, Pipeline};
use crate::retry::RetryConfig;
use itertools::Itertools;

/// Builder for constructing Pipeline instances with validation.
///
/// Uses Railway-Oriented Programming: validate() returns Result<Self>,
/// then build() chains with and_then for pure composition.
#[derive(Debug)]
pub struct PipelineBuilder {
    stages: Vec<Stage>,
    language: Option<Language>,
    retry_config: RetryConfig,
    failure_strategy: FailureStrategy,
    dry_run: bool,
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineBuilder {
    /// Create a new PipelineBuilder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            language: None,
            retry_config: RetryConfig::default(),
            failure_strategy: FailureStrategy::default(),
            dry_run: false,
        }
    }

    /// Set the language for the pipeline.
    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = Some(language);
        self
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

    /// Enable dry-run mode.
    #[must_use]
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Validate the pipeline configuration.
    ///
    /// Uses Railway-Oriented Programming: returns Result<Self> that can
    /// be chained with and_then for pure composition.
    ///
    /// # Checks
    /// - Language is set
    /// - No duplicate stage names
    /// - No circular dependencies (when implemented)
    pub fn validate(self) -> Result<Self> {
        // Railway-Oriented Programming: validate each check, composing results
        // Perform validation directly to avoid reference issues with cached Result

        // Check language is set
        let _language = self.language.ok_or_else(|| Error::InvalidRecord {
            reason: "language must be set".to_string(),
        })?;

        // Check for duplicate stage names using itertools
        let stage_names: Vec<&String> = self.stages.iter().map(|s| &s.name).collect();
        let duplicates: Vec<&String> = stage_names.iter().duplicates().cloned().collect();

        if !duplicates.is_empty() {
            let duplicate_names: Vec<&str> = duplicates.iter().map(|s| s.as_str()).collect();
            return Err(Error::DuplicateStages {
                stages: duplicate_names.join(", "),
            });
        }

        // Check for circular dependencies (placeholder for future implementation)
        self.check_circular_deps()?;

        Ok(self)
    }

    /// Check for circular dependencies in stage execution.
    ///
    /// This is a placeholder for future dependency graph implementation.
    /// When stages have dependencies, this will validate they form a DAG.
    fn check_circular_deps(&self) -> Result<()> {
        // TODO: Implement circular dependency detection when stage dependencies are added
        // For now, all stages are independent, so no cycles possible
        Ok(())
    }

    /// Build the Pipeline using Railway-Oriented Programming.
    ///
    /// Chains validate() with and_then for pure composition:
    /// - First validate the configuration
    /// - Then create the pipeline instance
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pipeline = PipelineBuilder::new()
    ///     .language(Language::Rust)
    ///     .with_stages(stages)
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<Pipeline> {
        // Railway track: validate first, then build
        self.validate().and_then(|builder| {
            let language = builder.language.ok_or_else(|| Error::InvalidRecord {
                reason: "language must be set".to_string(),
            })?;

            // Build pipeline using the builder pattern methods
            Ok(Pipeline::new(language)
                .with_stages(builder.stages)
                .with_retry(builder.retry_config)
                .with_failure_strategy(builder.failure_strategy)
                .dry_run())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_builder_all_required() {
        let task = TaskBuilder::new()
            .slug("my-task")
            .map(|b| b.language(Language::Rust))
            .and_then(|b| b.build());

        assert!(task.is_ok());
        if let Ok(t) = task {
            assert_eq!(t.slug.as_str(), "my-task");
            assert_eq!(t.language, Language::Rust);
            assert_eq!(t.branch, "feat/my-task");
        }
    }

    #[test]
    fn test_task_builder_with_optional() {
        let result = TaskBuilder::new()
            .slug("high-priority")
            .map(|b| b.language(Language::Go).priority(Priority::P1))
            .and_then(|b| b.build());

        assert!(result.is_ok());
        if let Ok(task) = result {
            assert_eq!(task.priority, Priority::P1);
        }
    }

    #[test]
    fn test_task_builder_invalid_slug() {
        let result = TaskBuilder::new().slug("INVALID SLUG");
        assert!(result.is_err());
    }

    #[test]
    fn test_task_fluent_status() {
        let slug = Slug::new("test");
        assert!(slug.is_ok());
        if let Ok(s) = slug {
            let task = Task::new(s, Language::Rust)
                .start_stage("implement")
                .mark_passed()
                .mark_integrated();

            assert!(matches!(task.status, TaskStatus::Integrated));
        }
    }

    #[test]
    fn test_stage_builder() {
        let stage = Stage::builder()
            .name("custom")
            .gate("Custom gate passes")
            .retries(5)
            .build();

        assert!(stage.is_some());
        if let Some(s) = stage {
            assert_eq!(s.name, "custom");
            assert_eq!(s.retries, 5);
        }
    }

    #[test]
    fn test_stage_builder_try_build() {
        let result = Stage::builder().name("minimal").try_build();

        assert!(result.is_ok());
        if let Ok(s) = result {
            assert_eq!(s.name, "minimal");
            assert_eq!(s.gate, "minimal passes");
        }
    }

    #[test]
    fn test_stage_builder_missing_name() {
        let result = Stage::builder().gate("some gate").try_build();
        assert!(result.is_err());
    }

    // =============================================================================
    // PipelineBuilder Tests
    // =============================================================================

    use crate::domain::Stage;

    #[test]
    fn test_pipeline_builder_validate_success() {
        use crate::domain::standard_pipeline;

        let result = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stages(standard_pipeline())
            .validate();

        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_builder_validate_missing_language() -> Result<()> {
        let result = PipelineBuilder::new()
            .with_stage(Stage::new("test", "test passes", 0))
            .validate();

        assert!(result.is_err());
        if let Err(Error::InvalidRecord { reason }) = result {
            assert!(reason.contains("language"));
            Ok(())
        } else {
            Err(Error::unknown("Expected InvalidRecord error"))
        }
    }

    #[test]
    fn test_pipeline_builder_validate_duplicate_stages() -> Result<()> {
        let result = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("duplicate", "first", 0))
            .with_stage(Stage::new("other", "other passes", 0))
            .with_stage(Stage::new("duplicate", "second", 0))
            .validate();

        assert!(result.is_err());
        if let Err(Error::DuplicateStages { stages }) = result {
            assert!(stages.contains("duplicate"));
            Ok(())
        } else {
            Err(Error::unknown("Expected DuplicateStages error"))
        }
    }

    #[test]
    fn test_pipeline_builder_build_success() {
        use crate::domain::standard_pipeline;

        let result = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stages(standard_pipeline())
            .build();

        assert!(result.is_ok());
        // Pipeline builder succeeded, we can't inspect private fields
        // but the success of build() indicates the structure is correct
    }

    #[test]
    fn test_pipeline_builder_railway_pattern() {
        use crate::domain::standard_pipeline;

        // Railway-Oriented Programming: chain validate and build
        let result = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stages(standard_pipeline())
            .validate()
            .and_then(|builder| builder.build());

        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_builder_with_retry() {
        use crate::retry::RetryConfig;

        let result = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("test", "test passes", 0))
            .with_retry(RetryConfig::quick())
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_builder_dry_run() {
        let result = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("test", "test passes", 0))
            .dry_run()
            .build();

        assert!(result.is_ok());
        // Pipeline builder succeeded with dry_run enabled
    }
}
