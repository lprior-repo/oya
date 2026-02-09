//! AgentSlotActor for recursive bead stage execution.
//!
//! This actor manages the lifecycle of a single bead through its recursive
//! stage execution, handling gate decisions, reentry with feedback, and
//! artifact tracking.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use oya_events::{
    BeadEvent, BeadId, BeadStateMachine, EventBus, Severity, StageKind, TransitionReason,
};

use crate::context_builder::{BeadContext, StageContextBuilder, StagePrompt};
use crate::stage_gate::{GateDecision, StageGate, StageOutput};

/// Messages for AgentSlotActor.
#[derive(Debug)]
pub enum AgentSlotMessage {
    /// Start executing a bead through its stage lifecycle.
    StartBead {
        bead_id: BeadId,
        spec: String,
        relevant_files: Vec<PathBuf>,
        upstream_artifacts: Vec<String>,
        reply: oneshot::Sender<Result<BeadCompletion, SlotError>>,
    },

    /// Internal message to execute the next stage.
    ExecuteNextStage {
        reply: oneshot::Sender<Result<(), SlotError>>,
    },

    /// Internal timeout handler for stage execution.
    StageTimeout { stage: StageKind },

    /// Query current slot state.
    GetState { reply: oneshot::Sender<SlotState> },
}

/// Result of completing a bead execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadCompletion {
    /// Bead completed successfully.
    Accepted,
    /// Bead failed with reason.
    Failed { reason: String },
    /// Bead exhausted retry limits and needs human intervention.
    Parked { reason: String },
}

/// Current state of an agent slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotState {
    /// Slot is idle.
    Idle,
    /// Slot is executing a bead.
    Executing {
        bead_id: BeadId,
        current_stage: StageKind,
    },
    /// Slot completed a bead.
    Completed {
        bead_id: BeadId,
        result: BeadCompletion,
    },
}

/// Errors from slot operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SlotError {
    /// Stage execution failed.
    #[error("stage execution failed: {0}")]
    StageExecutionFailed(String),

    /// Gate evaluation failed.
    #[error("gate evaluation failed: {0}")]
    GateEvaluationFailed(String),

    /// Context builder error.
    #[error("context builder error: {0}")]
    ContextError(String),

    /// State machine error.
    #[error("state machine error: {0}")]
    StateMachineError(String),

    /// Timeout occurred.
    #[error("stage timeout: {0:?}")]
    Timeout(StageKind),

    /// Invalid state transition.
    #[error("invalid state transition")]
    InvalidTransition,

    /// Bead ID not available.
    #[error("bead ID not available")]
    BeadIdNotAvailable,
}

/// State for AgentSlotActor.
#[derive(Debug)]
pub struct AgentSlotState {
    bead_id: Option<BeadId>,
    state_machine: Option<BeadStateMachine>,
    artifacts: HashMap<StageKind, String>,
    stage_gate: Option<StageGate>,
    context_builder: Option<StageContextBuilder>,
    event_bus: Option<Arc<EventBus>>,
    project_root: PathBuf,
    pending_feedback: Option<String>,
    current_stage: Option<StageKind>,
}

impl AgentSlotState {
    /// Create a new slot state.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            bead_id: None,
            state_machine: None,
            artifacts: HashMap::new(),
            stage_gate: None,
            context_builder: None,
            event_bus: None,
            project_root,
            pending_feedback: None,
            current_stage: None,
        }
    }

    /// Check if slot is idle.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.bead_id.is_none()
    }

    /// Get current bead ID.
    #[must_use]
    pub fn current_bead(&self) -> Option<&BeadId> {
        self.bead_id.as_ref()
    }

    /// Get bead ID or return error.
    fn require_bead_id(&self) -> Result<&BeadId, SlotError> {
        self.bead_id.as_ref().ok_or(SlotError::BeadIdNotAvailable)
    }
}

/// Actor definition for AgentSlotActor.
pub struct AgentSlotActorDef;

impl AgentSlotActorDef {
    /// Spawn a new agent slot actor.
    pub async fn spawn(
        project_root: PathBuf,
        event_bus: Option<Arc<EventBus>>,
    ) -> Result<ActorRef<AgentSlotMessage>, SlotError> {
        let initial_state = AgentSlotState::new(project_root);
        let (actor, _) = Actor::spawn(None, AgentSlotActorDef, initial_state)
            .await
            .map_err(|e| SlotError::StageExecutionFailed(format!("spawn failed: {e}")))?;

        // Configure the actor after spawning
        actor.send_message(AgentSlotMessage::GetState {
            reply: oneshot::channel().0,
        })?;

        Ok(actor)
    }
}

#[async_trait::async_trait]
impl Actor for AgentSlotActorDef {
    type Msg = AgentSlotMessage;
    type State = AgentSlotState;
    type Arguments = AgentSlotState;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: Self::State,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("AgentSlotActor starting");
        Ok(state)
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: Self::State,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("AgentSlotActor stopping");
        Ok(state)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<Self::State, ActorProcessingErr> {
        match message {
            AgentSlotMessage::StartBead {
                bead_id,
                spec,
                relevant_files,
                upstream_artifacts,
                reply,
            } => {
                let result = self.handle_start_bead(
                    state,
                    bead_id,
                    spec,
                    relevant_files,
                    upstream_artifacts,
                );

                let _ = reply.send(result);
                Ok(state.clone())
            }

            AgentSlotMessage::ExecuteNextStage { reply } => {
                let result = self.handle_execute_next_stage(state);
                let _ = reply.send(result);
                Ok(state.clone())
            }

            AgentSlotMessage::StageTimeout { stage } => {
                warn!("Stage timeout occurred for: {:?}", stage);
                // Handle timeout by failing the current stage
                self.handle_stage_timeout(state, stage);
                Ok(state.clone())
            }

            AgentSlotMessage::GetState { reply } => {
                let slot_state = if let Some(ref bead_id) = state.bead_id {
                    if let Some(current_stage) = state.current_stage {
                        SlotState::Executing {
                            bead_id: bead_id.clone(),
                            current_stage,
                        }
                    } else {
                        SlotState::Completed {
                            bead_id: bead_id.clone(),
                            result: BeadCompletion::Accepted,
                        }
                    }
                } else {
                    SlotState::Idle
                };
                let _ = reply.send(slot_state);
                Ok(state.clone())
            }
        }
    }
}

impl AgentSlotActorDef {
    fn handle_start_bead(
        &self,
        state: &mut AgentSlotState,
        bead_id: BeadId,
        spec: String,
        relevant_files: Vec<PathBuf>,
        upstream_artifacts: Vec<String>,
    ) -> Result<BeadCompletion, SlotError> {
        if !state.is_idle() {
            return Err(SlotError::InvalidTransition);
        }

        info!("Starting bead execution: {}", bead_id);

        // Initialize state machine
        let state_machine = BeadStateMachine::new(bead_id.clone());
        let policy = state_machine.policy();
        let stage_gate = StageGate::new(policy);
        let context_builder = StageContextBuilder::new(state.project_root.clone())
            .with_claude_md(state.project_root.join("CLAUDE.md"));

        state.bead_id = Some(bead_id.clone());
        state.state_machine = Some(state_machine);
        state.stage_gate = Some(stage_gate);
        state.context_builder = Some(context_builder);
        state.current_stage = Some(StageKind::Research);

        // Emit bead started event
        self.emit_event(
            state,
            BeadEvent::BeadStarted {
                bead_id: bead_id.clone(),
                timestamp: chrono::Utc::now(),
            },
        );

        // Store initial context
        state.artifacts.clear();

        Ok(BeadCompletion::Accepted) // Will be updated through execution
    }

    fn handle_execute_next_stage(&self, state: &mut AgentSlotState) -> Result<(), SlotError> {
        let state_machine = state
            .state_machine
            .as_ref()
            .ok_or_else(|| SlotError::InvalidTransition)?;
        let context_builder = state
            .context_builder
            .as_ref()
            .ok_or_else(|| SlotError::InvalidTransition)?;
        let stage_gate = state
            .stage_gate
            .as_ref()
            .ok_or_else(|| SlotError::InvalidTransition)?;

        let current_stage = state_machine.current_stage();
        state.current_stage = Some(current_stage);

        info!(
            "Executing stage: {:?} for bead: {:?}",
            current_stage, state.bead_id
        );

        // Enter stage (increments counters)
    // TODO: FIX THIS LINE - /* TODO: FIX THIS */ machine_clone = state_machine.clone();
        machine_clone
            .enter_stage()
            .map_err(|e| SlotError::StateMachineError(format!("enter_stage: {e}")))?;
        state.state_machine = Some(machine_clone.clone());

        // Build context for this stage
        let bead_id = state.require_bead_id()?;
        let bead_context = BeadContext {
            bead_id: bead_id.clone(),
            spec: format!("Bead {}", bead_id), // Simplified
            relevant_files: vec![],
            upstream_artifacts: vec![],
        };

        // Build prompt for this stage
        let feedback = state.pending_feedback.as_deref();
        let stage_prompt = context_builder
            .build_prompt(current_stage, &bead_context, &state.artifacts, feedback)
            .map_err(|e| SlotError::ContextError(format!("{e}")))?;
        self.emit_event(
            state,
            BeadEvent::StageStarted {
                bead_id: bead_id.clone(),
                stage: current_stage,
                timestamp: chrono::Utc::now(),
            },
        );

        // Execute the stage (simplified for now - would invoke agent)
        let output = if current_stage.requires_agent() {
            self.execute_agent_stage(&stage_prompt, current_stage)
        } else {
            self.execute_non_agent_stage(current_stage)
        };

        // Evaluate output through gate
        let decision = stage_gate.evaluate(&machine_clone, output.clone());

        // Handle gate decision
        match decision {
            GateDecision::Proceed { next_stage } => {
                info!(
                    "Stage {:?} succeeded, proceeding to {:?}",
                    current_stage, next_stage
                );

                // Store artifact for this stage
                state
                    .artifacts
                    .insert(current_stage, "artifact-placeholder".to_string());

                // Advance state machine
    // TODO: FIX THIS LINE - /* TODO: FIX THIS */ machine_clone = state_machine.clone();
                let transition = machine_clone
                    .advance()
                    .map_err(|e| SlotError::StateMachineError(format!("advance: {e}")))?;
                state.state_machine = Some(machine_clone);

                // Emit transition event
                self.emit_transition_event(state, &transition);

                // Check if complete
                if state_machine.current_stage() == StageKind::Accept {
                    self.complete_bead(state, BeadCompletion::Accepted);
                }
            }
            GateDecision::Reenter {
                stage: target,
                feedback,
                severity,
            } => {
                warn!(
                    "Stage {:?} failed, reentering {:?} with severity: {:?}",
                    current_stage, target, severity
                );

                // Reenter target stage
    // TODO: FIX THIS LINE - /* TODO: FIX THIS */ machine_clone = state_machine.clone();
                let transition = machine_clone
                    .reenter(target, feedback.clone(), severity)
                    .map_err(|e| SlotError::StateMachineError(format!("reenter: {e}")))?;
                state.state_machine = Some(machine_clone);

                // Emit transition event
                self.emit_transition_event(state, &transition);

                // Store feedback for next execution
                state.pending_feedback = Some(feedback);
            }
            GateDecision::Fail { reason } => {
                error!("Bead failed: {}", reason);
                self.complete_bead(state, BeadCompletion::Failed { reason });
            }
            GateDecision::Exhausted { policy } => {
                warn!("Retry limits exhausted, policy: {:?}", policy);
                self.complete_bead(
                    state,
                    BeadCompletion::Parked {
                        reason: format!("Retry limits exhausted: {:?}", policy),
                    },
                );
            }
        }

        Ok(())
    }

    fn execute_agent_stage(&self, _prompt: &StagePrompt, stage: StageKind) -> StageOutput {
        // Placeholder: Would invoke agent IPC here
        StageOutput {
            stage,
            success: true,
            output: format!("Agent execution for {:?}", stage),
            exit_code: Some(0),
            duration_ms: 100,
        }
    }

    fn execute_non_agent_stage(&self, stage: StageKind) -> StageOutput {
        // Placeholder: Would run validation/acceptance logic here
        match stage {
            StageKind::Validate => StageOutput {
                stage,
                success: true,
                output: "Validation passed".to_string(),
                exit_code: Some(0),
                duration_ms: 50,
            },
            StageKind::Accept => StageOutput {
                stage,
                success: true,
                output: "Bead accepted".to_string(),
                exit_code: Some(0),
                duration_ms: 10,
            },
            _ => StageOutput {
                stage,
                success: false,
                output: format!("Non-agent stage {:?}", stage),
                exit_code: Some(1),
                duration_ms: 10,
            },
        }
    }

    fn handle_stage_timeout(&self, state: &mut AgentSlotState, stage: StageKind) {
        if let Some(ref bead_id) = state.bead_id {
            error!("Stage timeout for bead {}: {:?}", bead_id, stage);

            // Emit timeout event
            self.emit_event(
                state,
                BeadEvent::StageFailed {
                    bead_id: bead_id.clone(),
                    stage,
                    reason: format!("Timeout after {:?}", Duration::from_secs(60)),
                    timestamp: chrono::Utc::now(),
                },
            );

            // Mark as failed
            self.complete_bead(
                state,
                BeadCompletion::Failed {
                    reason: format!("Stage timeout: {:?}", stage),
                },
            );
        }
    }

    fn complete_bead(&self, state: &mut AgentSlotState, result: BeadCompletion) {
        let bead_id = match state.require_bead_id() {
            Ok(id) => id.clone(),
            Err(e) => {
                error!("Cannot complete bead: {}", e);
                return;
            }
        };

        info!("Bead {:?} completed with result: {:?}", bead_id, result);

        // Emit completion event
        match result {
            BeadCompletion::Accepted => {
                self.emit_event(
                    state,
                    BeadEvent::BeadCompleted {
                        bead_id: bead_id.clone(),
                        timestamp: chrono::Utc::now(),
                    },
                );
            }
            BeadCompletion::Failed { ref reason } => {
                self.emit_event(
                    state,
                    BeadEvent::BeadFailed {
                        bead_id: bead_id.clone(),
                        reason: reason.clone(),
                        timestamp: chrono::Utc::now(),
                    },
                );
            }
            BeadCompletion::Parked { ref reason } => {
                self.emit_event(
                    state,
                    BeadEvent::BeadParked {
                        bead_id: bead_id.clone(),
                        reason: reason.clone(),
                        timestamp: chrono::Utc::now(),
                    },
                );
            }
        }

        // Reset state
        state.bead_id = None;
        state.state_machine = None;
        state.stage_gate = None;
        state.context_builder = None;
        state.artifacts.clear();
        state.pending_feedback = None;
        state.current_stage = None;
    }

    fn emit_transition_event(
        &self,
        state: &AgentSlotState,
        transition: &oya_events::StageTransition,
    ) {
        if let Some(ref bead_id) = state.bead_id {
            let event = BeadEvent::StageTransition {
                bead_id: bead_id.clone(),
                transition: transition.clone(),
            };
            self.emit_event(state, event);
        }
    }

    fn emit_event(&self, state: &AgentSlotState, event: BeadEvent) {
        if let Some(ref bus) = state.event_bus {
            if let Err(e) = bus.publish(event) {
                warn!("Failed to emit event: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_slot_state_initialization() {
        let state = AgentSlotState::new(PathBuf::from("/tmp"));
        assert!(state.is_idle());
        assert!(state.current_bead().is_none());
    }

    #[test]
    fn test_bead_completion_accepted() {
        let completion = BeadCompletion::Accepted;
        assert_eq!(completion, BeadCompletion::Accepted);
    }

    #[test]
    fn test_bead_completion_failed() {
        let completion = BeadCompletion::Failed {
            reason: "test failure".to_string(),
        };
        assert!(matches!(completion, BeadCompletion::Failed { .. }));
    }

    #[test]
    fn test_bead_completion_parked() {
        let completion = BeadCompletion::Parked {
            reason: "exhausted".to_string(),
        };
        assert!(matches!(completion, BeadCompletion::Parked { .. }));
    }

    #[test]
    fn test_slot_error_display() {
        let err = SlotError::Timeout(StageKind::Implement);
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_agent_stage_output() {
        let actor = AgentSlotActorDef;
        let prompt = StagePrompt {
            stage: StageKind::Implement,
            prompt_text: "test".to_string(),
            allowed_tools: vec!["read".to_string()],
            timeout: Duration::from_secs(60),
        };

        let output = actor.execute_agent_stage(&prompt, StageKind::Implement);
        assert!(output.success);
        assert_eq!(output.stage, StageKind::Implement);
    }

    #[test]
    fn test_validate_stage_output() {
        let actor = AgentSlotActorDef;
        let output = actor.execute_non_agent_stage(StageKind::Validate);
        assert!(output.success);
        assert_eq!(output.stage, StageKind::Validate);
    }

    #[test]
    fn test_accept_stage_output() {
        let actor = AgentSlotActorDef;
        let output = actor.execute_non_agent_stage(StageKind::Accept);
        assert!(output.success);
        assert_eq!(output.stage, StageKind::Accept);
    }

    #[test]
    fn test_require_bead_id_when_present() {
    // TODO: FIX THIS LINE - /* TODO: FIX THIS */ state = AgentSlotState::new(PathBuf::from("/tmp"));
        state.bead_id = Some(BeadId::new());
        assert!(state.require_bead_id().is_ok());
    }

    #[test]
    fn test_require_bead_id_when_missing() {
        let state = AgentSlotState::new(PathBuf::from("/tmp"));
        let result = state.require_bead_id();
        assert!(matches!(result, Err(SlotError::BeadIdNotAvailable)));
    }
}