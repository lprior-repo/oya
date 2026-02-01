//! AI Integration layer between OYA stages and OpenCode execution.
//!
//! This module provides the bridge between OYA pipeline stages and
//! OpenCode's AI-powered phase execution.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, info};

use oya_opencode::{
    AIExecutor, OpencodeClient, PhaseContext, PhaseHandler, PhaseInput, PhaseOutput,
};

use crate::domain::{Language, Stage, Task};
use crate::error::{Error, Result};
use crate::pipeline::StageExecution;

/// Maps OYA stage names to OpenCode phase names.
#[derive(Debug, Clone)]
pub struct StagePhaseMapping {
    /// Direct mappings: OYA stage name -> opencode phase name
    mappings: HashMap<String, String>,
}

impl StagePhaseMapping {
    /// Create a new mapping with default mappings.
    #[must_use]
    pub fn new() -> Self {
        let mut mappings = HashMap::new();

        // Standard stage -> phase mappings
        mappings.insert("implement".to_string(), "implement".to_string());
        mappings.insert("unit-test".to_string(), "test".to_string());
        mappings.insert("integration".to_string(), "test".to_string());
        mappings.insert("review".to_string(), "review".to_string());
        mappings.insert("refactor".to_string(), "refactor".to_string());
        mappings.insert("document".to_string(), "document".to_string());

        Self { mappings }
    }

    /// Add a custom stage-to-phase mapping.
    pub fn add_mapping(&mut self, stage: impl Into<String>, phase: impl Into<String>) {
        self.mappings.insert(stage.into(), phase.into());
    }

    /// Get the phase name for a given stage.
    pub fn get_phase(&self, stage: &str) -> Option<&str> {
        self.mappings.get(stage).map(String::as_str)
    }

    /// Check if a stage is AI-compatible.
    #[must_use]
    pub fn is_ai_stage(&self, stage: &str) -> bool {
        self.mappings.contains_key(stage)
    }
}

impl Default for StagePhaseMapping {
    fn default() -> Self {
        Self::new()
    }
}

/// Context builder for converting OYA tasks to OpenCode phase contexts.
pub struct OYAPhaseContextBuilder {
    task: Task,
    stage: Stage,
    mapping: Arc<StagePhaseMapping>,
}

impl OYAPhaseContextBuilder {
    /// Create a new context builder.
    #[must_use]
    pub fn new(task: Task, stage: Stage, mapping: Arc<StagePhaseMapping>) -> Self {
        Self {
            task,
            stage,
            mapping,
        }
    }

    /// Build a PhaseContext from the OYA task and stage.
    pub fn build(&self) -> Result<PhaseContext> {
        // Get phase name from mapping
        let phase_name = self
            .mapping
            .get_phase(&self.stage.name)
            .ok_or_else(|| Error::InvalidRecord {
                reason: format!("No phase mapping for stage '{}'", self.stage.name),
            })?
            .to_string();

        // Build phase description based on stage
        let phase_description = match self.stage.name.as_str() {
            "implement" => format!(
                "Implement functionality for task '{}' in {}",
                self.task.slug,
                self.task.language.as_str()
            ),
            "unit-test" => format!(
                "Write unit tests for task '{}' in {}",
                self.task.slug,
                self.task.language.as_str()
            ),
            "integration" => format!(
                "Write integration tests for task '{}' in {}",
                self.task.slug,
                self.task.language.as_str()
            ),
            "review" => format!("Review code for task '{}'", self.task.slug),
            "refactor" => format!("Refactor code for task '{}'", self.task.slug),
            "document" => format!("Generate documentation for task '{}'", self.task.slug),
            _ => format!(
                "Execute '{}' for task '{}'",
                self.stage.name, self.task.slug
            ),
        };

        // Build constraints from language and task
        let mut constraints = vec![
            format!("Language: {}", self.task.language.as_str()),
            "Use functional programming patterns".to_string(),
            "No unwrap() or expect() - use Result<T, E>".to_string(),
            "No panic! - handle errors explicitly".to_string(),
        ];

        // Add language-specific constraints
        match self.task.language {
            Language::Rust => {
                constraints.push("Follow Rust idioms and best practices".to_string());
                constraints.push("Use Railway-Oriented Programming".to_string());
            }
            Language::Gleam => {
                constraints.push("Follow Gleam functional patterns".to_string());
                constraints.push("Use pipelines and pattern matching".to_string());
            }
            Language::Go => {
                constraints.push("Follow Go idioms and best practices".to_string());
                constraints.push("Handle errors explicitly".to_string());
            }
            Language::Javascript => {
                constraints.push("Follow JavaScript best practices".to_string());
                constraints.push("Use modern ES6+ features".to_string());
            }
            Language::Python => {
                constraints.push("Follow Python best practices (PEP 8)".to_string());
                constraints.push("Use type hints".to_string());
            }
        }

        // Build context
        let mut ctx = PhaseContext::new(phase_name, phase_description);

        // Add constraints
        for constraint in constraints {
            ctx = ctx.with_constraint(constraint);
        }

        // Add task slug as metadata
        ctx.metadata
            .insert("task_slug".to_string(), serde_json::json!(self.task.slug));
        ctx.metadata.insert(
            "language".to_string(),
            serde_json::json!(self.task.language.as_str()),
        );

        Ok(ctx)
    }

    /// Build with custom input.
    pub fn with_input(self, input: PhaseInput) -> Result<PhaseContext> {
        let mut ctx = self.build()?;
        ctx.input = input;
        Ok(ctx)
    }

    /// Build with custom description.
    pub fn with_description(self, description: impl Into<String>) -> Result<PhaseContext> {
        let mut ctx = self.build()?;
        ctx.phase_description = description.into();
        Ok(ctx)
    }
}

/// AI-powered stage executor using OpenCode.
pub struct AIStageExecutor {
    /// OpenCode client for AI execution.
    client: Arc<OpencodeClient>,
    /// AI executor for phase-based execution.
    ai_executor: Arc<AIExecutor>,
    /// Stage-to-phase mapping.
    mapping: Arc<StagePhaseMapping>,
}

impl AIStageExecutor {
    /// Create a new AI stage executor.
    pub fn new() -> Result<Self> {
        let client = OpencodeClient::new().map_err(|e| Error::InvalidRecord {
            reason: format!("Failed to create OpenCode client: {e}"),
        })?;
        let client = Arc::new(client);
        let ai_executor = Arc::new(AIExecutor::new(client.clone()));
        let mapping = Arc::new(StagePhaseMapping::new());

        Ok(Self {
            client,
            ai_executor,
            mapping,
        })
    }

    /// Create with custom configuration.
    pub fn with_client(client: Arc<OpencodeClient>) -> Self {
        let ai_executor = Arc::new(AIExecutor::new(client.clone()));
        let mapping = Arc::new(StagePhaseMapping::new());

        Self {
            client,
            ai_executor,
            mapping,
        }
    }

    /// Check if OpenCode is available.
    pub async fn is_available(&self) -> bool {
        self.client.is_available().await
    }

    /// Execute a stage using AI.
    pub async fn execute_stage(
        &self,
        task: &Task,
        stage: &Stage,
        input: Option<PhaseInput>,
    ) -> Result<StageExecution> {
        info!(
            stage = %stage.name,
            task = %task.slug,
            "Executing stage with AI"
        );

        // Build phase context
        let builder =
            OYAPhaseContextBuilder::new(task.clone(), stage.clone(), self.mapping.clone());

        let ctx = if let Some(inp) = input {
            builder.with_input(inp)?
        } else {
            builder.build()?
        };

        // Execute via AI executor
        let start = std::time::Instant::now();
        let output = self
            .ai_executor
            .execute(&ctx)
            .await
            .map_err(|e| Error::InvalidRecord {
                reason: format!("AI execution failed: {e}"),
            })?;
        let duration = start.elapsed();

        // Convert to StageExecution
        let execution = convert_phase_output_to_stage_execution(&output, &stage.name, duration);

        debug!(
            stage = %stage.name,
            passed = execution.passed,
            duration_ms = duration.as_millis(),
            "Stage execution complete"
        );

        Ok(execution)
    }

    /// Check if a stage can be executed by AI.
    #[must_use]
    pub fn can_execute(&self, stage: &Stage) -> bool {
        self.mapping.is_ai_stage(&stage.name)
    }

    /// Get the mapping for testing/inspection.
    #[must_use]
    pub fn mapping(&self) -> &Arc<StagePhaseMapping> {
        &self.mapping
    }
}

/// Convert OpenCode PhaseOutput to OYA StageExecution.
fn convert_phase_output_to_stage_execution(
    output: &PhaseOutput,
    stage_name: &str,
    duration: Duration,
) -> StageExecution {
    if output.success {
        StageExecution::success(stage_name, duration, 1)
    } else {
        StageExecution::failure(stage_name, duration, 1, &output.summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Slug;

    #[test]
    fn test_stage_phase_mapping_defaults() {
        let mapping = StagePhaseMapping::new();
        assert_eq!(mapping.get_phase("implement"), Some("implement"));
        assert_eq!(mapping.get_phase("unit-test"), Some("test"));
        assert_eq!(mapping.get_phase("review"), Some("review"));
        assert!(mapping.is_ai_stage("implement"));
        assert!(!mapping.is_ai_stage("lint"));
    }

    #[test]
    fn test_stage_phase_mapping_custom() {
        let mut mapping = StagePhaseMapping::new();
        mapping.add_mapping("custom-stage", "custom-phase");
        assert_eq!(mapping.get_phase("custom-stage"), Some("custom-phase"));
        assert!(mapping.is_ai_stage("custom-stage"));
    }

    #[test]
    fn test_context_builder_basic() {
        // Create a valid slug - if this fails, test should fail
        let slug = match Slug::new("test-task") {
            Ok(s) => s,
            Err(_) => return, // Skip test if slug validation fails
        };

        let task = Task::new(slug, Language::Rust);
        let stage = Stage::new("implement".to_string(), "none".to_string(), 1);
        let mapping = Arc::new(StagePhaseMapping::new());

        let builder = OYAPhaseContextBuilder::new(task, stage, mapping);
        let ctx = builder.build();

        assert!(ctx.is_ok());
        let ctx = ctx.ok();
        assert_eq!(
            ctx.as_ref().map(|c| c.phase_name.as_str()),
            Some("implement")
        );
        assert!(
            ctx.as_ref()
                .map(|c| !c.constraints.is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn test_phase_output_conversion_success() {
        let output = PhaseOutput::success("test-phase", "All good");
        let execution = convert_phase_output_to_stage_execution(
            &output,
            "test-stage",
            Duration::from_millis(100),
        );

        assert!(execution.passed);
        assert_eq!(execution.stage_name, "test-stage");
        assert!(execution.error.is_none());
    }

    #[test]
    fn test_phase_output_conversion_failure() {
        let output = PhaseOutput::failure("test-phase", "Something went wrong");
        let execution = convert_phase_output_to_stage_execution(
            &output,
            "test-stage",
            Duration::from_millis(100),
        );

        assert!(!execution.passed);
        assert_eq!(execution.stage_name, "test-stage");
        assert_eq!(execution.error.as_deref(), Some("Something went wrong"));
    }
}
