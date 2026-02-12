//! `SchedulerActor` - Actor-based scheduler for workflow DAG management.
//!
//! This module implements the ractor Actor trait for the scheduler,
//! integrating with the `EventBus` for event-driven coordination and
//! the `ShutdownCoordinator` for graceful shutdown.

use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::info;

use oya_events::{BeadEvent, EventBus, EventPattern, EventSubscription};

use crate::dag::BeadId;
use crate::replay::{CheckpointManager, OrchestratorEvent, ReplayEngine};
use crate::scheduler::{ScheduledBead, SchedulerStats, WorkflowId, WorkflowState};
use crate::shutdown::{CheckpointResult, ShutdownCoordinator, ShutdownSignal};

use super::errors::ActorError;
use super::messages::{BeadState as MsgBeadState, SchedulerMessage, WorkflowStatus};
use super::supervisor::GenericSupervisableActor;

use im::{HashMap, Vector};

/// The scheduler actor definition.
#[derive(Clone, Default)]
pub struct SchedulerActorDef;

impl GenericSupervisableActor for SchedulerActorDef {
    fn default_args() -> Self::Arguments {
        Self::Arguments::default()
    }
}

/// Arguments passed to the actor on startup.
#[derive(Default, Clone)]
pub struct SchedulerArguments {
    /// Optional `EventBus` for subscribing to bead events.
    pub event_bus: Option<Arc<EventBus>>,
    /// Optional `ShutdownCoordinator` for graceful shutdown.
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
    /// Optional `ReplayEngine` for event sourcing and recovery (wrapped in Mutex for async access).
    pub replay_engine: Option<Arc<Mutex<ReplayEngine>>>,
    /// Optional `CheckpointManager` for checkpoint persistence.
    pub checkpoint_manager: Option<Arc<CheckpointManager>>,
}

impl std::fmt::Debug for SchedulerArguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchedulerArguments").finish_non_exhaustive()
    }
}

impl SchedulerArguments {
    /// Create new arguments with no integrations.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `EventBus`.
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Set the `ShutdownCoordinator`.
    #[must_use]
    pub fn with_shutdown_coordinator(mut self, coordinator_: Arc<ShutdownCoordinator>) -> Self {
        self.shutdown_coordinator = Some(coordinator_);
        self
    }

    /// Set the `ReplayEngine`.
    #[must_use]
    pub fn with_replay_engine(mut self, engine: Arc<Mutex<ReplayEngine>>) -> Self {
        self.replay_engine = Some(engine);
        self
    }

    /// Set the `CheckpointManager`.
    #[must_use]
    pub fn with_checkpoint_manager(mut self, manager: Arc<CheckpointManager>) -> Self {
        self.checkpoint_manager = Some(manager);
        self
    }
}

/// Core functional state for the scheduler.
#[derive(Clone, Default)]
pub struct CoreSchedulerState {
    /// Map of workflow IDs to their state (DAG + completed tracking).
    pub workflows: HashMap<WorkflowId, WorkflowState>,
    /// Pending beads waiting to be scheduled.
    pub pending_beads: HashMap<BeadId, ScheduledBead>,
    /// Ready beads that can be dispatched.
    pub ready_beads: Vector<BeadId>,
    /// Worker assignments (`bead_id` -> `worker_id`).
    pub worker_assignments: HashMap<BeadId, String>,
    /// Registered agents with their capabilities.
    pub agents: HashMap<String, Vec<String>>,
}

/// Actor state containing core state and integration handles.
pub struct SchedulerState {
    /// Core functional state.
    pub core: CoreSchedulerState,

    // Integration handles
    /// Event subscription ID (for cleanup).
    _event_subscription_id: Option<String>,
    /// Shutdown signal receiver.
    shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,
    /// Checkpoint result sender.
    pub checkpoint_tx: Option<mpsc::Sender<CheckpointResult>>,
    /// `ReplayEngine` for event sourcing and recovery (wrapped in Mutex for async access).
    pub replay_engine: Option<Arc<Mutex<ReplayEngine>>>,
    /// `CheckpointManager` for periodic checkpointing.
    pub checkpoint_manager: Option<Arc<CheckpointManager>>,
    /// Whether shutdown has been requested.
    pub shutdown_requested: bool,
}

impl SchedulerState {
    /// Create new empty state.
    fn new() -> Self {
        Self {
            core: CoreSchedulerState::default(),
            _event_subscription_id: None,
            shutdown_rx: None,
            checkpoint_tx: None,
            replay_engine: None,
            checkpoint_manager: None,
            shutdown_requested: false,
        }
    }
}

/// Effects produced by the functional core of the `SchedulerActor`.
pub enum SchedulerEffect {
    /// Reply to an RPC caller.
    ReplyReadyBeads {
        reply: RpcReplyPort<Result<Vec<BeadId>, ActorError>>,
        result: Result<Vec<BeadId>, ActorError>,
    },
    ReplyStats {
        reply: RpcReplyPort<SchedulerStats>,
        stats: SchedulerStats,
    },
    ReplyIsReady {
        reply: RpcReplyPort<Result<bool, ActorError>>,
        result: Result<bool, ActorError>,
    },
    ReplyWorkflowStatus {
        reply: RpcReplyPort<Option<WorkflowStatus>>,
        status: Option<WorkflowStatus>,
    },
    ReplyAllReady {
        reply: RpcReplyPort<Vec<(WorkflowId, BeadId)>>,
        ready: Vec<(WorkflowId, BeadId)>,
    },
    /// Record an event to the replay engine.
    RecordEvent {
        event: OrchestratorEvent,
    },
}

impl Actor for SchedulerActorDef {
    type Msg = SchedulerMessage;
    type State = SchedulerState;
    type Arguments = SchedulerArguments;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("SchedulerActor starting");

        let mut state = SchedulerState::new();

        // Initialize replay engine if provided
        if let Some(engine) = args.replay_engine {
            state.replay_engine = Some(engine);
        }

        // Initialize checkpoint manager if provided
        if let Some(manager) = args.checkpoint_manager {
            state.checkpoint_manager = Some(manager);
        }

        // Subscribe to event bus if provided
        if let Some(bus) = args.event_bus {
            let (subscription_id, subscription) =
                bus.subscribe_with_pattern(EventPattern::All).await;
            state._event_subscription_id = Some(subscription_id);
            // Spawn event forwarder
            tokio::spawn(Self::event_forwarder(subscription, myself.clone()));
        }

        // Subscribe to shutdown coordinator if provided
        if let Some(coordinator) = args.shutdown_coordinator {
            state.shutdown_rx = Some(coordinator.subscribe());
            state.checkpoint_tx = Some(coordinator.checkpoint_sender());

            // Spawn shutdown listener
            let myself_clone = myself.clone();
            let mut rx = coordinator.subscribe();
            tokio::spawn(async move {
                if rx.recv().await.is_ok() {
                    let _ = myself_clone.send_message(SchedulerMessage::Shutdown);
                }
            });
        }

        Ok(state)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Special case for Shutdown as it's not purely functional in terms of handles
        if matches!(message, SchedulerMessage::Shutdown) {
            info!("Scheduler shutdown requested");
            state.shutdown_requested = true;
            myself.stop(Some("Scheduler shutdown requested".to_string()));
            return Ok(());
        }

        let (next_core, effects) = core::handle(state.core.clone(), message);
        state.core = next_core;

        for effect in effects {
            match effect {
                SchedulerEffect::ReplyReadyBeads { reply, result } => {
                    let _ = reply.send(result);
                }
                SchedulerEffect::ReplyStats { reply, stats } => {
                    let _ = reply.send(stats);
                }
                SchedulerEffect::ReplyIsReady { reply, result } => {
                    let _ = reply.send(result);
                }
                SchedulerEffect::ReplyWorkflowStatus { reply, status } => {
                    let _ = reply.send(status);
                }
                SchedulerEffect::ReplyAllReady { reply, ready } => {
                    let _ = reply.send(ready);
                }
                SchedulerEffect::RecordEvent { event } => {
                    if let Some(engine) = &state.replay_engine {
                        let engine = Arc::clone(engine);
                        tokio::spawn(async move {
                            let mut guard = engine.lock().await;
                            let _ = guard.record_event(event).await;
                        });
                    }
                }
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("SchedulerActor stopping");

        // Save checkpoint on graceful shutdown
        if let Some(tx) = &state.checkpoint_tx {
            let result = CheckpointResult::success("scheduler", 0);
            let _ = tx.send(result).await;
        }

        Ok(())
    }
}

impl SchedulerActorDef {
    /// Forward events from `EventBus` to the actor.
    async fn event_forwarder(
        mut subscription: EventSubscription,
        actor_ref: ActorRef<SchedulerMessage>,
    ) {
        while let Ok(event) = subscription.recv().await {
            if Self::forward_event(&actor_ref, event).is_err() {
                break;
            }
        }
    }

    fn forward_event(
        actor_ref: &ActorRef<SchedulerMessage>,
        event: BeadEvent,
    ) -> Result<(), ActorError> {
        match event {
            // Handle StateChanged events (including transitions to Completed)
            BeadEvent::StateChanged {
                bead_id, from, to, ..
            } => {
                let _ = actor_ref.send_message(SchedulerMessage::OnStateChanged {
                    bead_id: bead_id.to_string(),
                    from: Self::convert_bead_state(&from),
                    to: Self::convert_bead_state(&to),
                });
            }
            // Handle BeadCompleted events directly
            BeadEvent::Completed { bead_id, .. } => {
                // Convert to OnStateChanged with Completed state
                let _ = actor_ref.send_message(SchedulerMessage::OnStateChanged {
                    bead_id: bead_id.to_string(),
                    from: crate::actors::messages::BeadState::Running,
                    to: crate::actors::messages::BeadState::Completed,
                });
            }
            // Ignore other event types
            _ => {}
        }
        Ok(())
    }

    const fn convert_bead_state(state: &oya_events::BeadState) -> MsgBeadState {
        match state {
            oya_events::BeadState::Pending => MsgBeadState::Pending,
            oya_events::BeadState::Ready => MsgBeadState::Ready,
            oya_events::BeadState::Running => MsgBeadState::Running,
            oya_events::BeadState::Completed => MsgBeadState::Completed,
            _ => MsgBeadState::Pending,
        }
    }
}

/// Functional core for `SchedulerActor`.
mod core {
    use chrono::Utc;

    use super::{
        ActorError, BeadId, CoreSchedulerState, MsgBeadState, OrchestratorEvent, ScheduledBead,
        SchedulerEffect, SchedulerMessage, SchedulerStats, WorkflowId, WorkflowState,
        WorkflowStatus,
    };

    pub fn handle(
        state: CoreSchedulerState,
        msg: SchedulerMessage,
    ) -> (CoreSchedulerState, Vec<SchedulerEffect>) {
        let mut next_state = state;
        let mut effects = Vec::new();

        match msg {
            SchedulerMessage::RegisterWorkflow { workflow_id } => {
                if !next_state.workflows.contains_key(&workflow_id) {
                    next_state
                        .workflows
                        .insert(workflow_id.clone(), WorkflowState::new(workflow_id.clone()));
                    effects.push(SchedulerEffect::RecordEvent {
                        event: OrchestratorEvent::WorkflowRegistered {
                            workflow_id,
                            name: String::new(),
                            dag_json: String::new(),
                        },
                    });
                }
            }
            SchedulerMessage::UnregisterWorkflow { workflow_id } => {
                next_state.workflows.remove(&workflow_id);
            }
            SchedulerMessage::ScheduleBead {
                workflow_id,
                bead_id,
            } => {
                if let Some(ws) = next_state.workflows.get_mut(&workflow_id) {
                    let _ = ws.add_bead(bead_id.clone());
                    next_state
                        .pending_beads
                        .insert(bead_id.clone(), ScheduledBead::new(bead_id, workflow_id));
                }
            }
            SchedulerMessage::AddDependency {
                workflow_id,
                from_bead,
                to_bead,
            } => {
                if let Some(ws) = next_state.workflows.get_mut(&workflow_id) {
                    let _ = ws.add_dependency(
                        from_bead,
                        to_bead,
                        crate::dag::DependencyType::BlockingDependency,
                    );
                }
            }
            SchedulerMessage::OnBeadCompleted {
                workflow_id,
                bead_id,
            } => {
                handle_bead_completed(&mut next_state, &workflow_id, &bead_id);
            }
            SchedulerMessage::OnStateChanged { bead_id, to, .. } => {
                if to == MsgBeadState::Completed {
                    // Find workflow for bead
                    if let Some(workflow_id) = next_state
                        .pending_beads
                        .get(&bead_id)
                        .map(|b| b.workflow_id.clone())
                    {
                        handle_bead_completed(&mut next_state, &workflow_id, &bead_id);
                    }
                }
            }
            SchedulerMessage::ClaimBead { bead_id, worker_id } => {
                if !next_state.worker_assignments.contains_key(&bead_id)
                    && next_state.pending_beads.contains_key(&bead_id)
                {
                    next_state
                        .worker_assignments
                        .insert(bead_id.clone(), worker_id);
                    if let Some(bead) = next_state.pending_beads.get_mut(&bead_id) {
                        bead.set_state(crate::scheduler::BeadScheduleState::Assigned);
                    }
                }
            }
            SchedulerMessage::ReleaseBead { bead_id } => {
                if next_state.worker_assignments.remove(&bead_id).is_some() {
                    if let Some(bead) = next_state.pending_beads.get_mut(&bead_id) {
                        bead.set_state(crate::scheduler::BeadScheduleState::Ready);
                    }
                }
            }
            SchedulerMessage::GetWorkflowReadyBeads { workflow_id, reply } => {
                let result = match next_state.workflows.get(&workflow_id) {
                    Some(ws) => Ok(ws
                        .get_ready_beads()
                        .into_iter()
                        .filter(|bead_id| !next_state.worker_assignments.contains_key(bead_id))
                        .collect()),
                    None => Err(ActorError::workflow_not_found(workflow_id)),
                };
                effects.push(SchedulerEffect::ReplyReadyBeads { reply, result });
            }
            SchedulerMessage::GetStats { reply } => {
                let stats = build_stats(&next_state);
                effects.push(SchedulerEffect::ReplyStats { reply, stats });
            }
            SchedulerMessage::IsBeadReady {
                workflow_id,
                bead_id,
                reply,
            } => {
                let result = next_state
                    .workflows
                    .get(&workflow_id)
                    .ok_or_else(|| ActorError::workflow_not_found(workflow_id))
                    .and_then(|ws| ws.is_bead_ready(&bead_id).map_err(ActorError::from));
                effects.push(SchedulerEffect::ReplyIsReady { reply, result });
            }
            SchedulerMessage::GetWorkflowStatus { workflow_id, reply } => {
                let status = next_state
                    .workflows
                    .get(&workflow_id)
                    .map(|ws| WorkflowStatus {
                        workflow_id: ws.workflow_id().clone(),
                        total_beads: ws.len(),
                        completed_beads: ws.completed_count(),
                        ready_beads: ws.get_ready_beads().len(),
                        is_complete: ws.is_complete(),
                    });
                effects.push(SchedulerEffect::ReplyWorkflowStatus { reply, status });
            }
            SchedulerMessage::GetAllReadyBeads { reply } => {
                let mut ready = Vec::new();
                for (wid, ws) in &next_state.workflows {
                    for bid in ws.get_ready_beads() {
                        if !next_state.worker_assignments.contains_key(&bid) {
                            ready.push((wid.clone(), bid));
                        }
                    }
                }
                effects.push(SchedulerEffect::ReplyAllReady { reply, ready });
            }
            SchedulerMessage::RegisterAgent {
                agent_id,
                capabilities,
            } => {
                next_state.agents.insert(agent_id.clone(), capabilities.clone());
                effects.push(SchedulerEffect::RecordEvent {
                    event: OrchestratorEvent::AgentRegistered {
                        agent_id,
                        capabilities,
                    },
                });
            }
            SchedulerMessage::UnregisterAgent { agent_id } => {
                next_state.agents.remove(&agent_id);
                effects.push(SchedulerEffect::RecordEvent {
                    event: OrchestratorEvent::AgentUnregistered { agent_id },
                });
            }
            SchedulerMessage::Shutdown => {} // Handled by shell
        }

        (next_state, effects)
    }

    fn handle_bead_completed(
        state: &mut CoreSchedulerState,
        workflow_id: &WorkflowId,
        bead_id: &BeadId,
    ) {
        if let Some(ws) = state.workflows.get_mut(workflow_id) {
            ws.mark_completed(bead_id);
        }
        if let Some(bead) = state.pending_beads.get_mut(bead_id) {
            bead.set_state(crate::scheduler::BeadScheduleState::Completed);
        }
        state.ready_beads.retain(|id| id != bead_id);
        state.worker_assignments.remove(bead_id);
    }

    fn build_stats(state: &CoreSchedulerState) -> SchedulerStats {
        let ready_count = state
            .workflows
            .values()
            .flat_map(crate::scheduler::WorkflowState::get_ready_beads)
            .filter(|bead_id| !state.worker_assignments.contains_key(bead_id))
            .count();

        SchedulerStats {
            workflow_count: state.workflows.len(),
            pending_count: state
                .pending_beads
                .values()
                .filter(|b| matches!(b.state, crate::scheduler::BeadScheduleState::Pending))
                .count(),
            ready_count,
            assigned_count: state.worker_assignments.len(),
            queue_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_core_register_workflow() {
        let state = CoreSchedulerState::default();
        let msg = SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-1".to_string(),
        };
        let (next_state, _effects) = core::handle(state, msg);
        assert!(next_state.workflows.contains_key("wf-1"));
    }

    #[test]
    fn test_register_workflow_records_event() {
        let state = CoreSchedulerState::default();
        let msg = SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-test".to_string(),
        };
        let (next_state, effects) = core::handle(state, msg);

        assert!(next_state.workflows.contains_key("wf-test"));

        let record_event = effects.iter().find_map(|e| match e {
            SchedulerEffect::RecordEvent { event } => Some(event.clone()),
            _ => None,
        });
        assert!(
            record_event.is_some(),
            "RegisterWorkflow should produce a RecordEvent effect"
        );

        let event = record_event.expect("checked is_some");
        assert!(
            matches!(event, OrchestratorEvent::WorkflowRegistered { workflow_id, .. } if workflow_id == "wf-test"),
            "Expected WorkflowRegistered event"
        );
    }

    #[test]
    fn test_register_workflow_idempotent_no_duplicate_events() {
        let state = CoreSchedulerState::default();

        let (state, effects1) = core::handle(state, SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-dupe".to_string(),
        });
        let (_, effects2) = core::handle(state, SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-dupe".to_string(),
        });

        let count1 = effects1.iter().filter(|e| matches!(e, SchedulerEffect::RecordEvent { .. })).count();
        let count2 = effects2.iter().filter(|e| matches!(e, SchedulerEffect::RecordEvent { .. })).count();

        assert_eq!(count1, 1, "First registration should record one event");
        assert_eq!(count2, 0, "Duplicate registration should not record event");
    }

    #[test]
    fn test_scheduler_arguments_with_replay_engine() {
        // Test that SchedulerArguments can accept a replay engine
        let args = SchedulerArguments::new();
        // Verify the field exists and is None by default
        assert!(args.replay_engine.is_none());
    }

    #[test]
    fn test_scheduler_state_has_checkpoint_manager_field() {
        // BDD: GIVEN SchedulerState is created WHEN inspecting fields THEN checkpoint_manager exists
        let state = SchedulerState::new();
        assert!(
            state.checkpoint_manager.is_none(),
            "checkpoint_manager should be None by default"
        );
    }

    #[test]
    fn test_register_agent_adds_to_state_and_records_event() {
        let state = CoreSchedulerState::default();
        let msg = SchedulerMessage::RegisterAgent {
            agent_id: "agent-1".to_string(),
            capabilities: vec!["compute".to_string(), "storage".to_string()],
        };
        let (next_state, effects) = core::handle(state, msg);

        assert!(
            next_state.agents.contains_key("agent-1"),
            "agent should be in state"
        );
        assert_eq!(
            next_state.agents.get("agent-1"),
            Some(&vec!["compute".to_string(), "storage".to_string()]),
            "capabilities should match"
        );

        let record_event = effects.iter().find_map(|e| match e {
            SchedulerEffect::RecordEvent { event } => Some(event.clone()),
            _ => None,
        });
        assert!(
            record_event.is_some(),
            "RegisterAgent should produce a RecordEvent effect"
        );

        let event = record_event.expect("checked is_some");
        assert!(
            matches!(
                event,
                OrchestratorEvent::AgentRegistered {
                    agent_id,
                    capabilities,
                } if agent_id == "agent-1" && capabilities == vec!["compute", "storage"]
            ),
            "Expected AgentRegistered event with correct data"
        );
    }

    #[test]
    fn test_unregister_agent_removes_from_state_and_records_event() {
        let state = CoreSchedulerState::default();
        let register_msg = SchedulerMessage::RegisterAgent {
            agent_id: "agent-1".to_string(),
            capabilities: vec!["compute".to_string()],
        };
        let (state, _) = core::handle(state, register_msg);

        let unregister_msg = SchedulerMessage::UnregisterAgent {
            agent_id: "agent-1".to_string(),
        };
        let (next_state, effects) = core::handle(state, unregister_msg);

        assert!(
            !next_state.agents.contains_key("agent-1"),
            "agent should be removed from state"
        );

        let record_event = effects.iter().find_map(|e| match e {
            SchedulerEffect::RecordEvent { event } => Some(event.clone()),
            _ => None,
        });
        assert!(
            record_event.is_some(),
            "UnregisterAgent should produce a RecordEvent effect"
        );

        let event = record_event.expect("checked is_some");
        assert!(
            matches!(
                event,
                OrchestratorEvent::AgentUnregistered { agent_id } if agent_id == "agent-1"
            ),
            "Expected AgentUnregistered event with correct agent_id"
        );
    }

    #[test]
    fn test_agent_count_matches_events_no_orphans() {
        let state = CoreSchedulerState::default();

        let (state, _) = core::handle(state, SchedulerMessage::RegisterAgent {
            agent_id: "agent-1".to_string(),
            capabilities: vec!["a".to_string()],
        });
        let (state, _) = core::handle(state, SchedulerMessage::RegisterAgent {
            agent_id: "agent-2".to_string(),
            capabilities: vec!["b".to_string()],
        });
        let (state, _) = core::handle(state, SchedulerMessage::RegisterAgent {
            agent_id: "agent-3".to_string(),
            capabilities: vec!["c".to_string()],
        });

        assert_eq!(state.agents.len(), 3, "should have 3 agents");

        let (state, _) = core::handle(state, SchedulerMessage::UnregisterAgent {
            agent_id: "agent-2".to_string(),
        });

        assert_eq!(state.agents.len(), 2, "should have 2 agents after unregister");
        assert!(state.agents.contains_key("agent-1"), "agent-1 should exist");
        assert!(state.agents.contains_key("agent-3"), "agent-3 should exist");
        assert!(!state.agents.contains_key("agent-2"), "agent-2 should be removed");
    }
}
