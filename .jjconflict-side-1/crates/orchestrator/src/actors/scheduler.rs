//! SchedulerActor - Actor-based scheduler for workflow DAG management.
//!
//! This module implements the ractor Actor trait for the scheduler,
//! integrating with the EventBus for event-driven coordination and
//! the ShutdownCoordinator for graceful shutdown.

use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

use oya_events::{BeadEvent, EventBus, EventPattern, EventSubscription};

use crate::dag::BeadId;
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
    /// Optional EventBus for subscribing to bead events.
    pub event_bus: Option<Arc<EventBus>>,
    /// Optional ShutdownCoordinator for graceful shutdown.
    pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
}

impl SchedulerArguments {
    /// Create new arguments with no integrations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the EventBus.
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Set the ShutdownCoordinator.
    pub fn with_shutdown_coordinator(mut self, coordinator: Arc<ShutdownCoordinator>) -> Self {
        self.shutdown_coordinator = Some(coordinator);
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
    /// Worker assignments (bead_id -> worker_id).
    pub worker_assignments: HashMap<BeadId, String>,
}

/// Actor state containing core state and integration handles.
pub struct SchedulerState {
    /// Core functional state.
    pub core: CoreSchedulerState,

    // Integration handles
    /// Event subscription ID (for cleanup).
    pub _event_subscription_id: Option<String>,
    /// Shutdown signal receiver.
    pub _shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,
    /// Checkpoint result sender.
    pub checkpoint_tx: Option<mpsc::Sender<CheckpointResult>>,
    /// Whether shutdown has been requested.
    pub shutdown_requested: bool,
}

impl SchedulerState {
    /// Create new empty state.
    fn new() -> Self {
        Self {
            core: CoreSchedulerState::default(),
            _event_subscription_id: None,
            _shutdown_rx: None,
            checkpoint_tx: None,
            shutdown_requested: false,
        }
    }
}

/// Effects produced by the functional core of the SchedulerActor.
pub enum SchedulerEffect {
    /// Reply to an RPC caller.
    ReplyReadyBeads {
        reply: RpcReplyPort<Result<Vec<BeadId>, ActorError>>,
        beads: Vec<BeadId>,
    },
    ReplyStats {
        reply: RpcReplyPort<SchedulerStats>,
        stats: SchedulerStats,
    },
    ReplyIsReady {
        reply: RpcReplyPort<Result<bool, ActorError>>,
        is_ready: bool,
    },
    ReplyWorkflowStatus {
        reply: RpcReplyPort<Option<WorkflowStatus>>,
        status: Option<WorkflowStatus>,
    },
    ReplyAllReady {
        reply: RpcReplyPort<Vec<(WorkflowId, BeadId)>>,
        ready: Vec<(WorkflowId, BeadId)>,
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

        // Subscribe to event bus if provided
        if let Some(bus) = args.event_bus {
            let pattern = EventPattern::bead_all();
            match bus.subscribe(pattern).await {
                Ok(subscription) => {
                    state._event_subscription_id = Some(subscription.id().to_string());
                    // Spawn event forwarder
                    tokio::spawn(Self::event_forwarder(subscription, myself.clone()));
                }
                Err(e) => warn!(error = %e, "Failed to subscribe to event bus"),
            }
        }

        // Subscribe to shutdown coordinator if provided
        if let Some(coordinator) = args.shutdown_coordinator {
            state._shutdown_rx = Some(coordinator.subscribe());
            state.checkpoint_tx = Some(coordinator.checkpoint_tx());

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
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Special case for Shutdown as it's not purely functional in terms of handles
        if matches!(message, SchedulerMessage::Shutdown) {
            info!("Scheduler shutdown requested");
            state.shutdown_requested = true;
            return Ok(());
        }

        let (next_core, effects) = core::handle(state.core.clone(), message);
        state.core = next_core;

        for effect in effects {
            match effect {
                SchedulerEffect::ReplyReadyBeads { reply, beads } => {
                    let _ = reply.send(Ok(beads));
                }
                SchedulerEffect::ReplyStats { reply, stats } => {
                    let _ = reply.send(stats);
                }
                SchedulerEffect::ReplyIsReady { reply, is_ready } => {
                    let _ = reply.send(Ok(is_ready));
                }
                SchedulerEffect::ReplyWorkflowStatus { reply, status } => {
                    let _ = reply.send(status);
                }
                SchedulerEffect::ReplyAllReady { reply, ready } => {
                    let _ = reply.send(ready);
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
    /// Forward events from EventBus to the actor.
    async fn event_forwarder(
        mut subscription: EventSubscription,
        actor_ref: ActorRef<SchedulerMessage>,
    ) {
        loop {
            match subscription.recv().await {
                Ok(event) => {
                    if let Err(_) = Self::forward_event(&actor_ref, event) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }

    fn forward_event(
        actor_ref: &ActorRef<SchedulerMessage>,
        event: BeadEvent,
    ) -> Result<(), ActorError> {
        match event {
            BeadEvent::StateChanged {
                bead_id, from, to, ..
            } => {
                let _ = actor_ref.send_message(SchedulerMessage::OnStateChanged {
                    bead_id: bead_id.to_string(),
                    from: Self::convert_bead_state(&from),
                    to: Self::convert_bead_state(&to),
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn convert_bead_state(state: &oya_events::BeadState) -> MsgBeadState {
        match state {
            oya_events::BeadState::Pending => MsgBeadState::Pending,
            oya_events::BeadState::Ready => MsgBeadState::Ready,
            oya_events::BeadState::Running => MsgBeadState::Running,
            oya_events::BeadState::Completed => MsgBeadState::Completed,
            _ => MsgBeadState::Pending,
        }
    }
}

/// Functional core for SchedulerActor.
mod core {
    use super::*;

    pub fn handle(
        state: CoreSchedulerState,
        msg: SchedulerMessage,
    ) -> (CoreSchedulerState, Vector<SchedulerEffect>) {
        let mut next_state = state;
        let mut effects = Vector::new();

        match msg {
            SchedulerMessage::RegisterWorkflow { workflow_id } => {
                if !next_state.workflows.contains_key(&workflow_id) {
                    next_state
                        .workflows
                        .insert(workflow_id.clone(), WorkflowState::new(workflow_id));
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
                let beads = next_state
                    .workflows
                    .get(&workflow_id)
                    .map_or(Vec::new(), |ws| ws.get_ready_beads());
                effects.push_back(SchedulerEffect::ReplyReadyBeads { reply, beads });
            }
            SchedulerMessage::GetStats { reply } => {
                let stats = build_stats(&next_state);
                effects.push_back(SchedulerEffect::ReplyStats { reply, stats });
            }
            SchedulerMessage::IsBeadReady {
                workflow_id,
                bead_id,
                reply,
            } => {
                let is_ready = next_state
                    .workflows
                    .get(&workflow_id)
                    .and_then(|ws| ws.is_bead_ready(&bead_id).ok())
                    .unwrap_or(false);
                effects.push_back(SchedulerEffect::ReplyIsReady { reply, is_ready });
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
                effects.push_back(SchedulerEffect::ReplyWorkflowStatus { reply, status });
            }
            SchedulerMessage::GetAllReadyBeads { reply } => {
                let mut ready = Vec::new();
                for (wid, ws) in &next_state.workflows {
                    for bid in ws.get_ready_beads() {
                        ready.push((wid.clone(), bid));
                    }
                }
                effects.push_back(SchedulerEffect::ReplyAllReady { reply, ready });
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
        SchedulerStats {
            workflow_count: state.workflows.len(),
            pending_count: state
                .pending_beads
                .values()
                .filter(|b| matches!(b.state, crate::scheduler::BeadScheduleState::Pending))
                .count(),
            ready_count: state.ready_beads.len(),
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
}
