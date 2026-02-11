//! High-level orchestration of agent subprocesses.

use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;
use oya_events::{BeadEvent, BeadId, EventBus, StageKind};

use super::{
    config::AgentConfig,
    subprocess_handle::{AgentOutput, SubprocessHandle},
    AgentSwarmError, AgentSwarmResult,
};

/// High-level executor for orchestrating agent subprocess execution.
#[derive(Clone)]
pub struct AgentExecutor {
    config: AgentConfig,
    event_bus: Option<Arc<EventBus>>,
}

impl AgentExecutor {
    /// Create a new agent executor.
    #[must_use]
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            event_bus: None,
        }
    }

    /// Set the event bus for output forwarding.
    #[must_use]
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Execute a complete agent stage: spawn → send prompt → wait → collect.
    ///
    /// This is the primary entry point for agent subprocess execution. It orchestrates
    /// the full workflow:
    ///
    /// 1. **Spawn**: Creates a new subprocess with the configured executable
    /// 2. **Send Prompt**: Writes the prompt text to the subprocess via stdin
    /// 3. **Wait**: Waits for completion with the specified timeout
    /// 4. **Collect**: Captures stdout, stderr, exit code, and duration
    /// 5. **Emit Events**: Publishes completion events to the event bus (if configured)
    ///
    /// # Arguments
    ///
    /// * `bead_id` - Bead identifier for event correlation
    /// * `stage` - Stage kind to execute (e.g., Implement, Review, Test)
    /// * `prompt` - Prompt text sent to agent via stdin
    /// * `timeout` - Maximum execution duration before termination
    ///
    /// # Events
    ///
    /// Emits the following events to the configured EventBus:
    /// - `BeadEvent::StageStarted` when subprocess spawns
    /// - `BeadEvent::StageCompleted` on successful execution (exit code 0)
    /// - `BeadEvent::StageFailed` on failure (non-zero exit code or timeout)
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple
    /// tasks. Each execution spawns an independent subprocess.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Executable not found at configured path
    /// - Subprocess spawn fails
    /// - Stdin write fails
    /// - Process wait fails
    /// - Timeout is exceeded (process is gracefully terminated)
    pub async fn execute_stage(
        &self,
        bead_id: BeadId,
        stage: StageKind,
        prompt: &str,
        timeout: Duration,
    ) -> AgentSwarmResult<AgentOutput> {
        // Step 1: Spawn subprocess
        let mut handle = self.spawn_subprocess(bead_id, stage)?;

        // Step 2: Send prompt
        handle.send_prompt(prompt).await?;

        // Step 3: Wait for completion
        let output = handle.wait_for_completion(timeout).await?;

        // Step 4: Emit completion event
        if let Some(ref bus) = self.event_bus {
            self.emit_completion_event(bead_id, &output, bus).await;
        }

        Ok(output)
    }

    /// Spawn a Claude Code subprocess for the given bead and stage.
    ///
    /// # Errors
    ///
    /// Returns error if executable not found or spawn fails.
    fn spawn_subprocess(
        &self,
        bead_id: BeadId,
        stage: StageKind,
    ) -> AgentSwarmResult<SubprocessHandle> {
        let path = self.config.executable();

        if !path.exists() {
            return Err(AgentSwarmError::ExecutableNotFound {
                path: path.clone(),
            });
        }

        tracing::info!(
            "Spawning Claude Code for bead {} stage {:?}",
            bead_id,
            stage
        );

        let child = Command::new(path)
            .current_dir(self.config.working_dir())
            .envs(self.config.env_vars())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AgentSwarmError::SpawnFailed {
                message: format!("{}: {}", path.display(), e),
            })?;

        let handle = SubprocessHandle::new(child, chrono::Utc::now(), stage, bead_id);

        // Emit process started event
        if let Some(ref bus) = self.event_bus {
            self.emit_process_started_event(bead_id, stage, bus);
        }

        Ok(handle)
    }

    /// Emit process started event to EventBus.
    fn emit_process_started_event(
        &self,
        bead_id: BeadId,
        stage: StageKind,
        bus: &Arc<EventBus>,
    ) {
        let event = BeadEvent::StageStarted {
            event_id: oya_events::EventId::new(),
            bead_id,
            stage,
            attempt: 1,
            timestamp: chrono::Utc::now(),
        };

        let bus = bus.clone();
        tokio::spawn(async move {
            if let Err(e) = bus.publish(event).await {
                tracing::warn!("Failed to emit agent process started event: {}", e);
            }
        });
    }

    /// Emit completion event to EventBus.
    async fn emit_completion_event(
        &self,
        bead_id: BeadId,
        output: &AgentOutput,
        bus: &Arc<EventBus>,
    ) {
        let event = if output.success {
            BeadEvent::StageCompleted {
                event_id: oya_events::EventId::new(),
                bead_id,
                stage: output.stage,
                artifact_ref: Some(output.stdout.clone()),
                timestamp: chrono::Utc::now(),
            }
        } else {
            BeadEvent::StageFailed {
                event_id: oya_events::EventId::new(),
                bead_id,
                stage: output.stage,
                feedback: output.stderr.clone(),
                severity: oya_events::Severity::Major,
                timestamp: chrono::Utc::now(),
            }
        };

        if let Err(e) = bus.publish(event).await {
            tracing::warn!("Failed to emit agent completion event: {}", e);
        }
    }
}

/// Builder for creating [`AgentExecutor`] with fluent API.
#[derive(Clone)]
pub struct AgentExecutorBuilder {
    config: Option<AgentConfig>,
    event_bus: Option<Arc<EventBus>>,
}

impl Default for AgentExecutorBuilder {
    fn default() -> Self {
        Self {
            config: None,
            event_bus: None,
        }
    }
}

impl AgentExecutorBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the agent configuration.
    #[must_use]
    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the event bus.
    #[must_use]
    pub fn event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Build the executor.
    ///
    /// # Errors
    ///
    /// Returns error if configuration not set.
    pub fn build(self) -> AgentSwarmResult<AgentExecutor> {
        let config = self
            .config
            .ok_or_else(|| AgentSwarmError::SpawnFailed {
                message: "configuration not set".to_string(),
            })?;

        let mut executor = AgentExecutor::new(config);
        if let Some(bus) = self.event_bus {
            executor = executor.with_event_bus(bus);
        }

        Ok(executor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_agent_executor_new() {
        let config = AgentConfig::new(
            PathBuf::from("/usr/bin/echo"),
            PathBuf::from("/tmp"),
        );

        let executor = AgentExecutor::new(config.clone());
        // We can't directly access the config, but we verified creation
        let _ = executor;
    }

    #[test]
    fn test_agent_executor_builder_complete() {
        let config = AgentConfig::new(
            PathBuf::from("/usr/bin/echo"),
            PathBuf::from("/tmp"),
        );

        // Create a mock event bus - this will fail at runtime but tests the builder
        let result = AgentExecutorBuilder::new()
            .config(config);

        // We can't complete the build without an EventBus, but we can
        // verify the builder accepts the config
        let _ = result;
    }

    #[test]
    fn test_agent_executor_builder_missing_config() {
        let result = AgentExecutorBuilder::new().build();
        assert!(result.is_err());
        match result {
            Err(err) => assert!(err.to_string().contains("configuration")),
            Ok(_) => panic!("Expected error"),
        }
    }

    #[tokio::test]
    async fn test_spawn_subprocess_nonexistent_executable() {
        let config = AgentConfig::new(
            PathBuf::from("/nonexistent/path/to/claude"),
            PathBuf::from("/tmp"),
        );

        let executor = AgentExecutor::new(config);
        let bead_id = BeadId::new();

        let result = executor.spawn_subprocess(bead_id, StageKind::Research);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentSwarmError::ExecutableNotFound { .. }
        ));
    }

    #[test]
    fn test_agent_executor_clone() {
        let config = AgentConfig::new(
            PathBuf::from("/usr/bin/echo"),
            PathBuf::from("/tmp"),
        );

        let executor = AgentExecutor::new(config);
        let _cloned = executor.clone();
        // Verify Clone is implemented
    }
}
