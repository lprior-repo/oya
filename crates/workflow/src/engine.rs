//! Workflow execution engine.

use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, error, info, warn};

use crate::error::{Error, Result};
use crate::handler::HandlerRegistry;
use crate::storage::WorkflowStorage;
use crate::types::{
    Checkpoint, JournalEntry, PhaseContext, PhaseId, PhaseOutput, Workflow, WorkflowId,
    WorkflowResult, WorkflowState,
};

/// Configuration for the workflow engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Whether to create checkpoints after each phase.
    pub checkpoint_enabled: bool,
    /// Whether to rollback on failure.
    pub rollback_on_failure: bool,
    /// Maximum concurrent workflows.
    pub max_concurrent: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            checkpoint_enabled: true,
            rollback_on_failure: true,
            max_concurrent: 10,
        }
    }
}

/// Workflow execution engine.
///
/// The engine is responsible for:
/// - Executing workflow phases in order
/// - Managing retries with exponential backoff
/// - Creating checkpoints after successful phases
/// - Handling rollback on failure
/// - Supporting rewind to previous checkpoints
/// - Replaying from journal
pub struct WorkflowEngine {
    /// Storage backend.
    storage: Arc<dyn WorkflowStorage>,
    /// Handler registry.
    handlers: Arc<HandlerRegistry>,
    /// Engine configuration.
    config: EngineConfig,
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new(
        storage: Arc<dyn WorkflowStorage>,
        handlers: Arc<HandlerRegistry>,
        config: EngineConfig,
    ) -> Self {
        Self {
            storage,
            handlers,
            config,
        }
    }

    /// Run a workflow to completion.
    pub async fn run(&self, mut workflow: Workflow) -> Result<WorkflowResult> {
        info!(workflow_id = %workflow.id, name = %workflow.name, "Starting workflow");

        // Validate workflow
        if workflow.phases.is_empty() {
            return Ok(WorkflowResult::success(workflow.id, Vec::new()));
        }

        // Check all handlers exist
        for phase in &workflow.phases {
            if !self.handlers.has(&phase.name) {
                return Err(Error::handler_not_found(&phase.name));
            }
        }

        // Transition to running
        self.transition_state(&mut workflow, WorkflowState::Running)
            .await?;

        let start = Instant::now();
        let mut phase_outputs: Vec<(PhaseId, PhaseOutput)> = Vec::new();
        let mut last_output: Option<Vec<u8>> = None;

        // Execute each phase
        while !workflow.is_complete() {
            let phase = match workflow.current_phase() {
                Some(p) => p.clone(),
                None => break,
            };

            info!(
                workflow_id = %workflow.id,
                phase = %phase.name,
                progress = workflow.progress(),
                "Executing phase"
            );

            // Record phase start
            self.storage
                .append_journal(
                    workflow.id,
                    JournalEntry::phase_started(phase.id, &phase.name),
                )
                .await?;

            // Execute with retries
            let result = self
                .execute_phase_with_retries(&workflow, &phase, last_output.clone())
                .await;

            match result {
                Ok(output) => {
                    info!(
                        workflow_id = %workflow.id,
                        phase = %phase.name,
                        duration_ms = output.duration_ms,
                        "Phase completed"
                    );

                    // Record completion
                    self.storage
                        .append_journal(
                            workflow.id,
                            JournalEntry::phase_completed(
                                phase.id,
                                &phase.name,
                                output.data.clone(),
                            ),
                        )
                        .await?;

                    // Create checkpoint
                    if self.config.checkpoint_enabled {
                        self.create_checkpoint(&workflow, &phase, &output).await?;
                    }

                    // Store output for next phase
                    last_output = Some(output.data.clone());
                    phase_outputs.push((phase.id, output));

                    // Advance to next phase
                    workflow.advance();
                    self.storage.save_workflow(&workflow).await?;
                }
                Err(e) => {
                    error!(
                        workflow_id = %workflow.id,
                        phase = %phase.name,
                        error = %e,
                        "Phase failed"
                    );

                    // Record failure
                    self.storage
                        .append_journal(
                            workflow.id,
                            JournalEntry::phase_failed(phase.id, &phase.name, e.to_string()),
                        )
                        .await?;

                    // Rollback if configured
                    if self.config.rollback_on_failure {
                        self.rollback_phases(&workflow, &phase_outputs).await?;
                    }

                    // Transition to failed
                    self.transition_state(&mut workflow, WorkflowState::Failed)
                        .await?;

                    return Ok(WorkflowResult::failure(
                        workflow.id,
                        phase_outputs,
                        e.to_string(),
                    ));
                }
            }
        }

        // Transition to completed
        self.transition_state(&mut workflow, WorkflowState::Completed)
            .await?;

        let duration = start.elapsed();
        info!(
            workflow_id = %workflow.id,
            duration_ms = duration.as_millis(),
            phases = phase_outputs.len(),
            "Workflow completed"
        );

        Ok(WorkflowResult::success(workflow.id, phase_outputs))
    }

    /// Execute a phase with retry logic.
    async fn execute_phase_with_retries(
        &self,
        workflow: &Workflow,
        phase: &crate::types::Phase,
        previous_output: Option<Vec<u8>>,
    ) -> Result<PhaseOutput> {
        let handler = self
            .handlers
            .get(&phase.name)
            .ok_or_else(|| Error::handler_not_found(&phase.name))?;

        let mut attempt = 1u32;
        let max_attempts = phase.retries + 1;

        loop {
            let mut ctx = PhaseContext::new(workflow.id, phase.clone()).with_attempt(attempt);

            if let Some(ref output) = previous_output {
                ctx = ctx.with_previous_output(output.clone());
            }

            if let Some(ref metadata) = workflow.metadata {
                ctx = ctx.with_metadata(metadata.clone());
            }

            debug!(
                phase = %phase.name,
                attempt = attempt,
                max_attempts = max_attempts,
                "Executing phase attempt"
            );

            let start = Instant::now();

            // Execute with timeout
            let result = tokio::time::timeout(phase.timeout, handler.execute(&ctx)).await;

            let duration = start.elapsed();

            match result {
                Ok(Ok(mut output)) => {
                    output.duration_ms = duration.as_millis() as u64;
                    return Ok(output);
                }
                Ok(Err(e)) => {
                    warn!(
                        phase = %phase.name,
                        attempt = attempt,
                        error = %e,
                        "Phase attempt failed"
                    );

                    if attempt >= max_attempts {
                        return Err(Error::max_retries_exceeded(&phase.name, attempt));
                    }

                    // Exponential backoff
                    let backoff =
                        std::time::Duration::from_millis(100 * 2u64.saturating_pow(attempt - 1));
                    tokio::time::sleep(backoff).await;

                    attempt += 1;
                }
                Err(_) => {
                    warn!(
                        phase = %phase.name,
                        timeout_secs = phase.timeout.as_secs(),
                        "Phase timed out"
                    );

                    if attempt >= max_attempts {
                        return Err(Error::phase_timeout(&phase.name, phase.timeout.as_secs()));
                    }

                    attempt += 1;
                }
            }
        }
    }

    /// Create a checkpoint after a successful phase.
    async fn create_checkpoint(
        &self,
        workflow: &Workflow,
        phase: &crate::types::Phase,
        output: &PhaseOutput,
    ) -> Result<()> {
        let checkpoint =
            Checkpoint::new(phase.id, Vec::new(), Vec::new()).with_outputs(output.data.clone());

        self.storage
            .save_checkpoint(workflow.id, &checkpoint)
            .await?;

        self.storage
            .append_journal(workflow.id, JournalEntry::checkpoint_created(phase.id))
            .await?;

        debug!(phase = %phase.name, "Checkpoint created");

        Ok(())
    }

    /// Rollback executed phases in reverse order.
    async fn rollback_phases(
        &self,
        workflow: &Workflow,
        phase_outputs: &[(PhaseId, PhaseOutput)],
    ) -> Result<()> {
        info!(
            workflow_id = %workflow.id,
            phases = phase_outputs.len(),
            "Rolling back phases"
        );

        for (phase_id, _) in phase_outputs.iter().rev() {
            // Find the phase
            let phase = workflow
                .phases
                .iter()
                .find(|p| p.id == *phase_id)
                .ok_or_else(|| Error::phase_not_found(phase_id.to_string()))?;

            if let Some(handler) = self.handlers.get(&phase.name) {
                let ctx = PhaseContext::new(workflow.id, phase.clone());
                if let Err(e) = handler.rollback(&ctx).await {
                    warn!(
                        phase = %phase.name,
                        error = %e,
                        "Rollback failed"
                    );
                    // Continue rolling back other phases
                }
            }
        }

        Ok(())
    }

    /// Rewind a workflow to a previous checkpoint.
    pub async fn rewind(&self, workflow_id: WorkflowId, to_phase: PhaseId) -> Result<Workflow> {
        info!(
            workflow_id = %workflow_id,
            to_phase = %to_phase,
            "Rewinding workflow"
        );

        // Load workflow
        let mut workflow = self
            .storage
            .load_workflow(workflow_id)
            .await?
            .ok_or_else(|| Error::workflow_not_found(workflow_id.to_string()))?;

        // Find the phase index
        let phase_idx = workflow
            .phases
            .iter()
            .position(|p| p.id == to_phase)
            .ok_or_else(|| Error::phase_not_found(to_phase.to_string()))?;

        // Verify checkpoint exists
        let _checkpoint = self
            .storage
            .load_checkpoint(workflow_id, to_phase)
            .await?
            .ok_or_else(|| Error::checkpoint_not_found(to_phase.to_string()))?;

        // Record rewind in journal
        self.storage
            .append_journal(
                workflow_id,
                JournalEntry::rewind_initiated(to_phase, "User requested rewind"),
            )
            .await?;

        // Clear checkpoints after this phase
        self.storage
            .clear_checkpoints_after(workflow_id, to_phase)
            .await?;

        // Update workflow state
        workflow.current_phase = phase_idx + 1; // Start from next phase
        workflow.state = WorkflowState::Paused;
        self.storage.save_workflow(&workflow).await?;

        Ok(workflow)
    }

    /// Replay a workflow from its journal.
    pub async fn replay(&self, workflow_id: WorkflowId) -> Result<WorkflowResult> {
        info!(workflow_id = %workflow_id, "Replaying workflow from journal");

        let workflow = self
            .storage
            .load_workflow(workflow_id)
            .await?
            .ok_or_else(|| Error::workflow_not_found(workflow_id.to_string()))?;

        let journal = self.storage.load_journal(workflow_id).await?;

        // Reconstruct outputs from journal
        let mut phase_outputs: Vec<(PhaseId, PhaseOutput)> = Vec::new();

        for entry in journal.entries() {
            if let JournalEntry::PhaseCompleted {
                phase_id, output, ..
            } = entry
            {
                phase_outputs.push((*phase_id, PhaseOutput::success(output.clone())));
            }
        }

        Ok(WorkflowResult::success(workflow.id, phase_outputs))
    }

    /// Resume a paused workflow.
    pub async fn resume(&self, workflow_id: WorkflowId) -> Result<WorkflowResult> {
        let workflow = self
            .storage
            .load_workflow(workflow_id)
            .await?
            .ok_or_else(|| Error::workflow_not_found(workflow_id.to_string()))?;

        if workflow.state != WorkflowState::Paused {
            return Err(Error::invalid_transition(
                workflow.state.to_string(),
                "running".to_string(),
            ));
        }

        self.run(workflow).await
    }

    /// Create a checkpoint at the current state.
    pub async fn checkpoint(&self, workflow_id: WorkflowId) -> Result<Checkpoint> {
        let workflow = self
            .storage
            .load_workflow(workflow_id)
            .await?
            .ok_or_else(|| Error::workflow_not_found(workflow_id.to_string()))?;

        let phase = workflow
            .current_phase()
            .ok_or_else(|| Error::checkpoint_failed("No current phase"))?;

        let checkpoint = Checkpoint::new(phase.id, Vec::new(), Vec::new());

        self.storage
            .save_checkpoint(workflow_id, &checkpoint)
            .await?;

        Ok(checkpoint)
    }

    /// Transition workflow state.
    async fn transition_state(&self, workflow: &mut Workflow, to: WorkflowState) -> Result<()> {
        let from = workflow.state;

        if !from.can_transition_to(to) {
            return Err(Error::invalid_transition(from.to_string(), to.to_string()));
        }

        workflow.state = to;
        self.storage.save_workflow(workflow).await?;

        self.storage
            .append_journal(workflow.id, JournalEntry::state_changed(from, to))
            .await?;

        debug!(from = %from, to = %to, "Workflow state transitioned");

        Ok(())
    }

    /// Get workflow by ID.
    pub async fn get_workflow(&self, workflow_id: WorkflowId) -> Result<Option<Workflow>> {
        self.storage.load_workflow(workflow_id).await
    }

    /// List all workflows.
    pub async fn list_workflows(&self) -> Result<Vec<Workflow>> {
        self.storage.list_workflows().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::NoOpHandler;
    use crate::storage::InMemoryStorage;
    use crate::types::Phase;
    use std::time::Duration;

    fn setup_engine() -> (WorkflowEngine, Arc<InMemoryStorage>) {
        let storage = Arc::new(InMemoryStorage::new());
        let mut registry = HandlerRegistry::new();
        registry.register("build", Arc::new(NoOpHandler::new("build")));
        registry.register("test", Arc::new(NoOpHandler::new("test")));
        registry.register("deploy", Arc::new(NoOpHandler::new("deploy")));

        let engine =
            WorkflowEngine::new(storage.clone(), Arc::new(registry), EngineConfig::default());

        (engine, storage)
    }

    #[tokio::test]
    async fn test_run_empty_workflow() {
        let (engine, _) = setup_engine();
        let workflow = Workflow::new("empty");

        let result = engine.run(workflow).await;
        assert!(result.is_ok());
        let result = result.ok();
        assert!(result
            .as_ref()
            .map(|r| r.state == WorkflowState::Completed)
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn test_run_single_phase() {
        let (engine, _) = setup_engine();
        let workflow = Workflow::new("single")
            .add_phase(Phase::new("build").with_timeout(Duration::from_secs(10)));

        let result = engine.run(workflow).await;
        assert!(result.is_ok());
        let result = result.ok();
        assert!(result
            .as_ref()
            .map(|r| r.state == WorkflowState::Completed)
            .unwrap_or(false));
        assert_eq!(result.map(|r| r.phase_outputs.len()).unwrap_or(0), 1);
    }

    #[tokio::test]
    async fn test_run_multiple_phases() {
        let (engine, _) = setup_engine();
        let workflow = Workflow::new("multi")
            .add_phase(Phase::new("build"))
            .add_phase(Phase::new("test"))
            .add_phase(Phase::new("deploy"));

        let result = engine.run(workflow).await;
        assert!(result.is_ok());
        let result = result.ok();
        assert!(result
            .as_ref()
            .map(|r| r.state == WorkflowState::Completed)
            .unwrap_or(false));
        assert_eq!(result.map(|r| r.phase_outputs.len()).unwrap_or(0), 3);
    }

    #[tokio::test]
    async fn test_missing_handler() {
        let (engine, _) = setup_engine();
        let workflow = Workflow::new("unknown").add_phase(Phase::new("unknown_phase"));

        let result = engine.run(workflow).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_persisted() {
        let (engine, storage) = setup_engine();
        let workflow = Workflow::new("persist").add_phase(Phase::new("build"));

        let workflow_id = workflow.id;
        let _ = engine.run(workflow).await;

        let loaded = storage.load_workflow(workflow_id).await;
        assert!(loaded.is_ok());
        assert!(loaded.ok().flatten().is_some());
    }

    #[tokio::test]
    async fn test_checkpoints_created() {
        let (engine, storage) = setup_engine();
        let workflow = Workflow::new("checkpoints")
            .add_phase(Phase::new("build"))
            .add_phase(Phase::new("test"));

        let workflow_id = workflow.id;
        let _ = engine.run(workflow).await;

        let checkpoints = storage.load_checkpoints(workflow_id).await;
        assert!(checkpoints.is_ok());
        assert_eq!(checkpoints.map(|c| c.len()).unwrap_or(0), 2);
    }

    #[tokio::test]
    async fn test_journal_recorded() {
        let (engine, storage) = setup_engine();
        let workflow = Workflow::new("journal").add_phase(Phase::new("build"));

        let workflow_id = workflow.id;
        let _ = engine.run(workflow).await;

        let journal = storage.load_journal(workflow_id).await;
        assert!(journal.is_ok());
        // Should have: state change, phase started, phase completed, checkpoint, state change
        assert!(journal.map(|j| j.len()).unwrap_or(0) >= 4);
    }
}
