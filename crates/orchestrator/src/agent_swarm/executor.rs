//! High-level orchestration of agent subprocesses.

use std::sync::Arc;
use std::time::Duration;

use oya_events::{BeadEvent, BeadId, EventBus, StageKind};
use tokio::process::Command;

use super::{
    AgentSwarmError, AgentSwarmResult,
    config::AgentConfig,
    subprocess_handle::{AgentOutput, SubprocessHandle},
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
    pub const fn new(config: AgentConfig) -> Self {
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
    /// Emits the following events to the configured `EventBus`:
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
            return Err(AgentSwarmError::ExecutableNotFound { path: path.clone() });
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

    /// Emit process started event to `EventBus`.
    fn emit_process_started_event(&self, bead_id: BeadId, stage: StageKind, bus: &Arc<EventBus>) {
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

    /// Emit completion event to `EventBus`.
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
