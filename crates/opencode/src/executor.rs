//! AI Executor for phase-based workflow execution.
//!
//! This module provides the `AIExecutor` that integrates opencode with
//! the OYA phase-based workflow system.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::client::OpencodeClient;
use crate::error::{Error, Result};
use crate::types::ExecutionResult;

/// Context for phase execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseContext {
    /// Name of the phase being executed.
    pub phase_name: String,
    /// Description of what the phase should accomplish.
    pub phase_description: String,
    /// Input data for the phase.
    pub input: PhaseInput,
    /// Constraints and requirements.
    pub constraints: Vec<String>,
    /// Previous phase outputs (for context).
    #[serde(default)]
    pub previous_outputs: Vec<PhaseOutput>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PhaseContext {
    /// Create a new phase context.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            phase_name: name.into(),
            phase_description: description.into(),
            input: PhaseInput::default(),
            constraints: Vec::new(),
            previous_outputs: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a constraint.
    #[must_use]
    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    /// Set the input.
    #[must_use]
    pub fn with_input(mut self, input: PhaseInput) -> Self {
        self.input = input;
        self
    }

    /// Add a previous output for context.
    #[must_use]
    pub fn with_previous_output(mut self, output: PhaseOutput) -> Self {
        self.previous_outputs.push(output);
        self
    }

    /// Generate a prompt for the AI based on this context.
    pub fn generate_prompt(&self) -> Result<String> {
        let mut prompt = String::new();

        // Phase header
        prompt.push_str(&format!("# Phase: {}\n\n", self.phase_name));
        prompt.push_str(&format!("{}\n\n", self.phase_description));

        // Input section
        if !self.input.files.is_empty() || self.input.code.is_some() || self.input.text.is_some() {
            prompt.push_str("## Input\n\n");

            if let Some(ref text) = self.input.text {
                prompt.push_str(&format!("{}\n\n", text));
            }

            if let Some(ref code) = self.input.code {
                prompt.push_str(&format!("```\n{}\n```\n\n", code));
            }

            if !self.input.files.is_empty() {
                prompt.push_str("Files to consider:\n");
                for file in &self.input.files {
                    prompt.push_str(&format!("- {}\n", file));
                }
                prompt.push('\n');
            }
        }

        // Constraints section
        if !self.constraints.is_empty() {
            prompt.push_str("## Constraints\n\n");
            for constraint in &self.constraints {
                prompt.push_str(&format!("- {}\n", constraint));
            }
            prompt.push('\n');
        }

        // Previous context
        if !self.previous_outputs.is_empty() {
            prompt.push_str("## Previous Phase Outputs\n\n");
            for output in &self.previous_outputs {
                prompt.push_str(&format!("### {}\n", output.phase_name));
                if output.success {
                    prompt.push_str(&format!("{}\n\n", output.summary));
                } else {
                    prompt.push_str(&format!("Failed: {}\n\n", output.summary));
                }
            }
        }

        Ok(prompt)
    }
}

/// Input data for a phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseInput {
    /// Text input/instructions.
    #[serde(default)]
    pub text: Option<String>,
    /// Code to analyze or modify.
    #[serde(default)]
    pub code: Option<String>,
    /// Files to work with.
    #[serde(default)]
    pub files: Vec<String>,
    /// Structured data.
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

impl PhaseInput {
    /// Create a text input.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// Create a code input.
    pub fn code(code: impl Into<String>) -> Self {
        Self {
            code: Some(code.into()),
            ..Default::default()
        }
    }

    /// Create a files input.
    pub fn files(files: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    /// Add a file to the input.
    #[must_use]
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.files.push(file.into());
        self
    }
}

/// Output from a phase execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseOutput {
    /// Name of the phase that produced this output.
    pub phase_name: String,
    /// Whether the phase succeeded.
    pub success: bool,
    /// Summary of what was done.
    pub summary: String,
    /// Detailed output/artifacts.
    pub output: String,
    /// Files that were modified.
    #[serde(default)]
    pub modified_files: Vec<String>,
    /// Any warnings or notes.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Additional data.
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

impl PhaseOutput {
    /// Create a successful phase output.
    pub fn success(phase_name: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            phase_name: phase_name.into(),
            success: true,
            summary: summary.into(),
            output: String::new(),
            modified_files: Vec::new(),
            warnings: Vec::new(),
            data: HashMap::new(),
        }
    }

    /// Create a failed phase output.
    pub fn failure(phase_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            phase_name: phase_name.into(),
            success: false,
            summary: reason.into(),
            output: String::new(),
            modified_files: Vec::new(),
            warnings: Vec::new(),
            data: HashMap::new(),
        }
    }

    /// Set the detailed output.
    #[must_use]
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = output.into();
        self
    }

    /// Add a modified file.
    #[must_use]
    pub fn with_modified_file(mut self, file: impl Into<String>) -> Self {
        self.modified_files.push(file.into());
        self
    }

    /// Add a warning.
    #[must_use]
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

impl From<ExecutionResult> for PhaseOutput {
    fn from(result: ExecutionResult) -> Self {
        Self {
            phase_name: String::new(),
            success: result.success,
            summary: if result.success {
                "Execution completed successfully".to_string()
            } else {
                "Execution failed".to_string()
            },
            output: result.output,
            modified_files: result.modified_files.into_iter().map(|f| f.path).collect(),
            warnings: Vec::new(),
            data: result.metadata,
        }
    }
}

/// Trait for handling phase execution.
#[async_trait]
pub trait PhaseHandler: Send + Sync {
    /// Execute a phase and return the output.
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput>;

    /// Get the name of this handler.
    fn name(&self) -> &str;

    /// Check if this handler can execute the given phase.
    fn can_handle(&self, phase_name: &str) -> bool;
}

/// AI-powered executor that uses opencode for phase execution.
pub struct AIExecutor {
    /// The opencode client.
    opencode: Arc<OpencodeClient>,
    /// Phases this executor handles.
    handled_phases: Vec<String>,
}

impl AIExecutor {
    /// Create a new AI executor with the given opencode client.
    pub fn new(opencode: Arc<OpencodeClient>) -> Self {
        Self {
            opencode,
            handled_phases: vec![
                "implement".to_string(),
                "refactor".to_string(),
                "fix".to_string(),
                "test".to_string(),
                "document".to_string(),
                "review".to_string(),
            ],
        }
    }

    /// Create a new AI executor with default client.
    pub fn default_client() -> Result<Self> {
        let client = OpencodeClient::new()?;
        Ok(Self::new(Arc::new(client)))
    }

    /// Add a phase that this executor handles.
    #[must_use]
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.handled_phases.push(phase.into());
        self
    }

    /// Set the phases this executor handles.
    #[must_use]
    pub fn with_phases(mut self, phases: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.handled_phases = phases.into_iter().map(Into::into).collect();
        self
    }
}

#[async_trait]
impl PhaseHandler for AIExecutor {
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        info!(phase = %ctx.phase_name, "Executing phase with AI");

        // Generate prompt from context
        let prompt = ctx.generate_prompt()?;
        debug!(prompt_len = prompt.len(), "Generated prompt");

        // Execute via opencode
        let result = self.opencode.execute(&prompt).await?;

        // Convert to PhaseOutput
        let mut output = PhaseOutput::from(result);
        output.phase_name = ctx.phase_name.clone();

        if !output.success {
            warn!(phase = %ctx.phase_name, "Phase execution failed");
        } else {
            info!(phase = %ctx.phase_name, "Phase execution succeeded");
        }

        Ok(output)
    }

    fn name(&self) -> &str {
        "AIExecutor"
    }

    fn can_handle(&self, phase_name: &str) -> bool {
        self.handled_phases
            .iter()
            .any(|p| p.eq_ignore_ascii_case(phase_name))
    }
}

/// Registry of phase handlers.
pub struct PhaseRegistry {
    handlers: Vec<Arc<dyn PhaseHandler>>,
}

impl PhaseRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register a handler.
    pub fn register(&mut self, handler: Arc<dyn PhaseHandler>) {
        self.handlers.push(handler);
    }

    /// Find a handler for the given phase.
    pub fn find_handler(&self, phase_name: &str) -> Option<&Arc<dyn PhaseHandler>> {
        self.handlers.iter().find(|h| h.can_handle(phase_name))
    }

    /// Execute a phase using the appropriate handler.
    pub async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        let handler = self
            .find_handler(&ctx.phase_name)
            .ok_or_else(|| Error::phase_failed(&ctx.phase_name, "No handler found"))?;

        handler.execute(ctx).await
    }
}

impl Default for PhaseRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_context_prompt_generation() {
        let ctx = PhaseContext::new("implement", "Implement a new feature")
            .with_input(PhaseInput::text("Create a hello world function"))
            .with_constraint("Use Rust")
            .with_constraint("No unsafe code");

        let prompt = ctx.generate_prompt();
        assert!(prompt.is_ok());
        let prompt = prompt.ok();
        assert!(prompt.as_ref().map(|p| p.contains("implement")).unwrap_or(false));
        assert!(prompt.as_ref().map(|p| p.contains("No unsafe code")).unwrap_or(false));
    }

    #[test]
    fn test_phase_output_from_execution_result() {
        let result = ExecutionResult::success("Done!");
        let output = PhaseOutput::from(result);
        assert!(output.success);
        assert_eq!(output.output, "Done!");
    }

    #[test]
    fn test_phase_input_builder() {
        let input = PhaseInput::text("Hello")
            .with_file("src/main.rs")
            .with_file("Cargo.toml");

        assert_eq!(input.text, Some("Hello".to_string()));
        assert_eq!(input.files.len(), 2);
    }

    #[test]
    fn test_phase_output_builder() {
        let output = PhaseOutput::success("test", "All tests passed")
            .with_output("Ran 10 tests")
            .with_modified_file("src/lib.rs")
            .with_warning("Deprecated API used");

        assert!(output.success);
        assert_eq!(output.modified_files.len(), 1);
        assert_eq!(output.warnings.len(), 1);
    }

    #[test]
    fn test_phase_registry() {
        let registry = PhaseRegistry::new();
        assert!(registry.find_handler("unknown").is_none());
    }
}
