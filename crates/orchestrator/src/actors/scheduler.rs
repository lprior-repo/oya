//! SchedulerActor - Actor-based scheduler for workflow DAG management.
//!
//! This module implements the ractor Actor trait for the scheduler,
//! integrating with the EventBus for event-driven coordination and
//! the ShutdownCoordinator for graceful shutdown.

use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

use oya_events::{BeadEvent, EventBus, EventPattern, EventSubscription};

use crate::dag::BeadId;
use crate::scheduler::{ScheduledBead, SchedulerStats, WorkflowId, WorkflowState};
use crate::shutdown::{CheckpointResult, ShutdownCoordinator, ShutdownSignal};

use super::errors::ActorError;
use super::messages::{BeadState as MsgBeadState, SchedulerMessage, WorkflowStatus};
use super::supervisor::SupervisableActor;

use im::{HashMap, Vector};

/// The scheduler actor definition.
#[derive(Clone, Default)]
pub struct SchedulerActorDef;

impl SupervisableActor for SchedulerActorDef {
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

/// Actor state containing all scheduler data.
pub struct SchedulerState {
    /// Map of workflow IDs to their state (DAG + completed tracking).
    workflows: HashMap<WorkflowId, WorkflowState>,
    /// Pending beads waiting to be scheduled.
    pending_beads: HashMap<BeadId, ScheduledBead>,
    /// Ready beads that can be dispatched.
    ready_beads: Vector<BeadId>,
    /// Worker assignments (bead_id -> worker_id).
    worker_assignments: HashMap<BeadId, String>,

    // Integration handles
    /// Event subscription ID (for cleanup).
    _event_subscription_id: Option<String>,
    /// Shutdown signal receiver.
    _shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,
    /// Checkpoint result sender.
    checkpoint_tx: Option<mpsc::Sender<CheckpointResult>>,
    /// Whether shutdown has been requested.
    shutdown_requested: bool,
}

impl SchedulerState {
    /// Create new empty state.
    fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            pending_beads: HashMap::new(),
            ready_beads: Vector::new(),
            worker_assignments: HashMap::new(),
            _event_subscription_id: None,
            _shutdown_rx: None,
            checkpoint_tx: None,
            shutdown_requested: false,
        }
    }
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

        // Subscribe to EventBus for bead events
        if let Some(bus) = &args.event_bus {
            let pattern =
                EventPattern::ByTypes(vec!["completed".to_string(), "state_changed".to_string()]);
            let (sub_id, subscription): (String, EventSubscription) =
                bus.subscribe_with_pattern(pattern).await;
            state._event_subscription_id = Some(sub_id);

            // Spawn task to forward events to actor
            let myself_clone = myself.clone();
            tokio::spawn(async move {
                Self::event_forwarder(subscription, myself_clone).await;
            });

            debug!("Subscribed to EventBus for bead events");
        }

        // Subscribe to shutdown signals
        if let Some(coordinator) = &args.shutdown_coordinator {
            let shutdown_rx: broadcast::Receiver<ShutdownSignal> = coordinator.subscribe();
            state._shutdown_rx = Some(shutdown_rx);
            state.checkpoint_tx = Some(coordinator.checkpoint_sender());

            // Spawn shutdown listener
            let myself_clone = myself.clone();
            let mut rx: broadcast::Receiver<ShutdownSignal> = coordinator.subscribe();
            tokio::spawn(async move {
                if rx.recv().await.is_ok() {
                    // Send shutdown message to actor
                    let _ = myself_clone.send_message(SchedulerMessage::Shutdown);
                }
            });

            debug!("Subscribed to ShutdownCoordinator");
        }

        Ok(state)
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // ═══════════════════════════════════════════════════════════════
            // COMMANDS
            // ═══════════════════════════════════════════════════════════════
            SchedulerMessage::RegisterWorkflow { workflow_id } => {
                Self::handle_register_workflow(state, workflow_id);
            }

            SchedulerMessage::UnregisterWorkflow { workflow_id } => {
                Self::handle_unregister_workflow(state, &workflow_id);
            }

            SchedulerMessage::ScheduleBead {
                workflow_id,
                bead_id,
            } => {
                if let Err(e) = Self::handle_schedule_bead(state, workflow_id, bead_id) {
                    warn!(error = %e, "Failed to schedule bead");
                }
            }

            SchedulerMessage::AddDependency {
                workflow_id,
                from_bead,
                to_bead,
            } => {
                if let Err(e) = Self::handle_add_dependency(state, &workflow_id, from_bead, to_bead)
                {
                    warn!(error = %e, "Failed to add dependency");
                }
            }

            SchedulerMessage::OnBeadCompleted {
                workflow_id,
                bead_id,
            } => {
                Self::handle_bead_completed(state, &workflow_id, &bead_id);
            }

            SchedulerMessage::OnStateChanged { bead_id, from, to } => {
                debug!(
                    bead_id = %bead_id,
                    from = %from,
                    to = %to,
                    "State change received"
                );
                // State changes are logged but don't affect internal state
                // The actual state is managed via explicit commands
            }

            SchedulerMessage::ClaimBead { bead_id, worker_id } => {
                if let Err(e) = Self::handle_claim_bead(state, &bead_id, worker_id) {
                    warn!(error = %e, "Failed to claim bead");
                }
            }

            SchedulerMessage::ReleaseBead { bead_id } => {
                Self::handle_release_bead(state, &bead_id);
            }

            SchedulerMessage::Shutdown => {
                info!("Shutdown requested, stopping actor");
                state.shutdown_requested = true;
                myself.stop(None);
            }

            // ═══════════════════════════════════════════════════════════════
            // QUERIES
            // ═══════════════════════════════════════════════════════════════
            SchedulerMessage::GetWorkflowReadyBeads { workflow_id, reply } => {
                let result = Self::handle_get_workflow_ready_beads(state, &workflow_id);
                // Ignore send error - caller may have timed out
                let _ = reply.send(result);
            }

            SchedulerMessage::GetStats { reply } => {
                let stats = Self::handle_get_stats(state);
                let _ = reply.send(stats);
            }

            SchedulerMessage::IsBeadReady {
                bead_id,
                workflow_id,
                reply,
            } => {
                let result = Self::handle_is_bead_ready(state, &workflow_id, &bead_id);
                let _ = reply.send(result);
            }

            SchedulerMessage::GetWorkflowStatus { workflow_id, reply } => {
                let status = Self::handle_get_workflow_status(state, &workflow_id);
                let _ = reply.send(status);
            }

            SchedulerMessage::GetAllReadyBeads { reply } => {
                let ready = Self::handle_get_all_ready_beads(state);
                let _ = reply.send(ready);
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
            let result: CheckpointResult = CheckpointResult::success("scheduler", 0);
            let send_res: Result<(), mpsc::error::SendError<CheckpointResult>> =
                tx.send(result).await;
            if send_res.is_err() {
                warn!("Failed to send checkpoint result");
            }
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Implementation
// ═══════════════════════════════════════════════════════════════════════════

impl SchedulerActorDef {
    /// Forward events from EventBus to the actor.
    async fn event_forwarder(
        mut subscription: EventSubscription,
        actor_ref: ActorRef<SchedulerMessage>,
    ) {
        loop {
            match subscription.recv().await {
                Ok(event) => {
                    if let Err(e) = Self::forward_event(&actor_ref, event) {
                        debug!(error = %e, "Failed to forward event to actor");
                        break;
                    }
                }
                Err(_) => {
                    debug!("Event subscription closed");
                    break;
                }
            }
        }
    }

    /// Convert a BeadEvent to a SchedulerMessage and send it.
    fn forward_event(
        actor_ref: &ActorRef<SchedulerMessage>,
        event: BeadEvent,
    ) -> Result<(), ActorError> {
        let message = match event {
            BeadEvent::Completed { bead_id, .. } => {
                // We don't have workflow_id in the event, so we'd need to look it up
                // For now, this is a simplified implementation
                debug!(bead_id = %bead_id, "Received completion event (workflow lookup needed)");
                return Ok(());
            }
            BeadEvent::StateChanged {
                bead_id, from, to, ..
            } => SchedulerMessage::OnStateChanged {
                bead_id: bead_id.to_string(),
                from: Self::convert_bead_state(&from),
                to: Self::convert_bead_state(&to),
            },
            _ => return Ok(()), // Ignore other events
        };

        actor_ref
            .send_message(message)
            .map_err(|_| ActorError::channel_error("Failed to send to actor"))
    }

    /// Convert from oya_events::BeadState to our local BeadState.
    fn convert_bead_state(state: &oya_events::BeadState) -> MsgBeadState {
        match state {
            oya_events::BeadState::Pending => MsgBeadState::Pending,
            oya_events::BeadState::Ready => MsgBeadState::Ready,
            oya_events::BeadState::Running => MsgBeadState::Running,
            oya_events::BeadState::Completed => MsgBeadState::Completed,
            // Map other states to appropriate local states
            oya_events::BeadState::Scheduled => MsgBeadState::Ready,
            oya_events::BeadState::Suspended => MsgBeadState::Pending,
            oya_events::BeadState::BackingOff => MsgBeadState::Pending,
            oya_events::BeadState::Paused => MsgBeadState::Pending,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Command Handlers
    // ═══════════════════════════════════════════════════════════════════════

    fn handle_register_workflow(state: &mut SchedulerState, workflow_id: WorkflowId) {
        if state.workflows.contains_key(&workflow_id) {
            debug!(workflow_id = %workflow_id, "Workflow already registered (idempotent)");
            return;
        }

        state
            .workflows
            .insert(workflow_id.clone(), WorkflowState::new(workflow_id.clone()));
        debug!(workflow_id = %workflow_id, "Workflow registered");
    }

    fn handle_unregister_workflow(state: &mut SchedulerState, workflow_id: &WorkflowId) {
        if state.workflows.remove(workflow_id).is_some() {
            debug!(workflow_id = %workflow_id, "Workflow unregistered");
        } else {
            debug!(workflow_id = %workflow_id, "Workflow not found for unregister");
        }
    }

    fn handle_schedule_bead(
        state: &mut SchedulerState,
        workflow_id: WorkflowId,
        bead_id: BeadId,
    ) -> Result<(), ActorError> {
        let workflow_state = state
            .workflows
            .get_mut(&workflow_id)
            .ok_or_else(|| ActorError::workflow_not_found(&workflow_id))?;

        workflow_state
            .add_bead(bead_id.clone())
            .map_err(ActorError::from)?;

        let scheduled_bead = ScheduledBead::new(bead_id.clone(), workflow_id);
        state.pending_beads.insert(bead_id.clone(), scheduled_bead);

        debug!(bead_id = %bead_id, "Bead scheduled");
        Ok(())
    }

    fn handle_add_dependency(
        state: &mut SchedulerState,
        workflow_id: &WorkflowId,
        from_bead: BeadId,
        to_bead: BeadId,
    ) -> Result<(), ActorError> {
        let workflow_state = state
            .workflows
            .get_mut(workflow_id)
            .ok_or_else(|| ActorError::workflow_not_found(workflow_id))?;

        workflow_state
            .add_dependency(
                from_bead.clone(),
                to_bead.clone(),
                crate::dag::DependencyType::BlockingDependency,
            )
            .map_err(ActorError::from)?;

        debug!(
            from = %from_bead,
            to = %to_bead,
            "Dependency added"
        );
        Ok(())
    }

    fn handle_bead_completed(
        state: &mut SchedulerState,
        workflow_id: &WorkflowId,
        bead_id: &BeadId,
    ) {
        if let Some(workflow_state) = state.workflows.get_mut(workflow_id) {
            workflow_state.mark_completed(bead_id);
            debug!(
                workflow_id = %workflow_id,
                bead_id = %bead_id,
                "Bead marked completed"
            );
        }

        // Update pending bead state
        if let Some(bead) = state.pending_beads.get_mut(bead_id) {
            bead.set_state(crate::scheduler::BeadScheduleState::Completed);
        }

        // Remove from ready list
        state.ready_beads.retain(|id| id != bead_id);

        // Remove worker assignment
        state.worker_assignments.remove(bead_id);
    }

    fn handle_claim_bead(
        state: &mut SchedulerState,
        bead_id: &BeadId,
        worker_id: String,
    ) -> Result<(), ActorError> {
        // Check if already claimed
        if let Some(existing_worker) = state.worker_assignments.get(bead_id) {
            return Err(ActorError::bead_already_claimed(bead_id, existing_worker));
        }

        // Verify bead exists
        if !state.pending_beads.contains_key(bead_id) {
            return Err(ActorError::bead_not_found(bead_id));
        }

        state
            .worker_assignments
            .insert(bead_id.clone(), worker_id.clone());

        if let Some(bead) = state.pending_beads.get_mut(bead_id) {
            bead.set_state(crate::scheduler::BeadScheduleState::Assigned);
        }

        debug!(bead_id = %bead_id, worker_id = %worker_id, "Bead claimed");
        Ok(())
    }

    fn handle_release_bead(state: &mut SchedulerState, bead_id: &BeadId) {
        if state.worker_assignments.remove(bead_id).is_some() {
            if let Some(bead) = state.pending_beads.get_mut(bead_id) {
                bead.set_state(crate::scheduler::BeadScheduleState::Ready);
            }
            debug!(bead_id = %bead_id, "Bead released");
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Query Handlers
    // ═══════════════════════════════════════════════════════════════════════

    fn handle_get_workflow_ready_beads(
        state: &SchedulerState,
        workflow_id: &WorkflowId,
    ) -> Result<Vec<BeadId>, ActorError> {
        let workflow_state = state
            .workflows
            .get(workflow_id)
            .ok_or_else(|| ActorError::workflow_not_found(workflow_id))?;

        Ok(workflow_state.get_ready_beads())
    }

    fn handle_get_stats(state: &SchedulerState) -> SchedulerStats {
        let pending_count = state
            .pending_beads
            .values()
            .filter(|b| matches!(b.state, crate::scheduler::BeadScheduleState::Pending))
            .count();

        SchedulerStats {
            workflow_count: state.workflows.len(),
            pending_count,
            ready_count: state.ready_beads.len(),
            assigned_count: state.worker_assignments.len(),
            queue_count: 0, // Queues are not managed in actor state
        }
    }

    fn handle_is_bead_ready(
        state: &SchedulerState,
        workflow_id: &WorkflowId,
        bead_id: &BeadId,
    ) -> Result<bool, ActorError> {
        let workflow_state = state
            .workflows
            .get(workflow_id)
            .ok_or_else(|| ActorError::workflow_not_found(workflow_id))?;

        workflow_state
            .is_bead_ready(bead_id)
            .map_err(ActorError::from)
    }

    fn handle_get_workflow_status(
        state: &SchedulerState,
        workflow_id: &WorkflowId,
    ) -> Option<WorkflowStatus> {
        state.workflows.get(workflow_id).map(|ws| WorkflowStatus {
            workflow_id: ws.workflow_id().clone(),
            total_beads: ws.len(),
            completed_beads: ws.completed_count(),
            ready_beads: ws.get_ready_beads().len(),
            is_complete: ws.is_complete(),
        })
    }

    fn handle_get_all_ready_beads(state: &SchedulerState) -> Vec<(WorkflowId, BeadId)> {
        let mut all_ready = Vec::new();

        for (workflow_id, workflow_state) in &state.workflows {
            for bead_id in workflow_state.get_ready_beads() {
                // Only include if not already claimed
                if !state.worker_assignments.contains_key(&bead_id) {
                    all_ready.push((workflow_id.clone(), bead_id));
                }
            }
        }

        all_ready
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unnecessary_get_then_check,
    clippy::unnecessary_to_owned
)]
mod tests {
    use super::*;

    #[test]
    fn should_create_scheduler_arguments() {
        let args = SchedulerArguments::new();
        assert!(args.event_bus.is_none());
        assert!(args.shutdown_coordinator.is_none());
    }

    #[test]
    fn should_create_scheduler_state() {
        let state = SchedulerState::new();
        assert!(state.workflows.is_empty());
        assert!(state.pending_beads.is_empty());
        assert!(state.ready_beads.is_empty());
        assert!(!state.shutdown_requested);
    }

    #[test]
    fn should_register_workflow() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());

        assert!(state.workflows.contains_key("wf-1"));
    }

    #[test]
    fn should_be_idempotent_on_duplicate_register() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());

        assert_eq!(state.workflows.len(), 1);
    }

    #[test]
    fn should_unregister_workflow() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_unregister_workflow(&mut state, &"wf-1".to_string());

        assert!(!state.workflows.contains_key("wf-1"));
    }

    #[test]
    fn should_schedule_bead_in_workflow() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());

        let result = SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        );

        assert!(result.is_ok());
        assert!(state.pending_beads.contains_key("bead-1"));
    }

    #[test]
    fn should_fail_to_schedule_bead_in_unknown_workflow() {
        let mut state = SchedulerState::new();

        let result = SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "unknown".to_string(),
            "bead-1".to_string(),
        );

        assert!(matches!(result, Err(ActorError::WorkflowNotFound(_))));
    }

    #[test]
    fn should_get_workflow_ready_beads() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();

        let result =
            SchedulerActorDef::handle_get_workflow_ready_beads(&state, &"wf-1".to_string());

        assert!(result.is_ok());
        // Root bead with no dependencies should be ready
        assert!(
            result
                .as_ref()
                .map(|v| v.contains(&"bead-1".to_string()))
                .unwrap_or(false)
        );
    }

    #[test]
    fn should_claim_bead() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();

        let result = SchedulerActorDef::handle_claim_bead(
            &mut state,
            &"bead-1".to_string(),
            "worker-1".to_string(),
        );

        assert!(result.is_ok());
        assert_eq!(
            state.worker_assignments.get("bead-1"),
            Some(&"worker-1".to_string())
        );
    }

    #[test]
    fn should_fail_to_claim_already_claimed_bead() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();
        SchedulerActorDef::handle_claim_bead(
            &mut state,
            &"bead-1".to_string(),
            "worker-1".to_string(),
        )
        .ok();

        let result = SchedulerActorDef::handle_claim_bead(
            &mut state,
            &"bead-1".to_string(),
            "worker-2".to_string(),
        );

        assert!(matches!(result, Err(ActorError::BeadAlreadyClaimed { .. })));
    }

    #[test]
    fn should_release_bead() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();
        SchedulerActorDef::handle_claim_bead(
            &mut state,
            &"bead-1".to_string(),
            "worker-1".to_string(),
        )
        .ok();

        SchedulerActorDef::handle_release_bead(&mut state, &"bead-1".to_string());

        assert!(state.worker_assignments.get("bead-1").is_none());
    }

    #[test]
    fn should_get_stats() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();

        let stats = SchedulerActorDef::handle_get_stats(&state);

        assert_eq!(stats.workflow_count, 1);
        assert_eq!(stats.pending_count, 1);
    }

    #[test]
    fn should_mark_bead_completed() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();
        SchedulerActorDef::handle_claim_bead(
            &mut state,
            &"bead-1".to_string(),
            "worker-1".to_string(),
        )
        .ok();

        SchedulerActorDef::handle_bead_completed(
            &mut state,
            &"wf-1".to_string(),
            &"bead-1".to_string(),
        );

        // Bead should be removed from worker assignments
        assert!(state.worker_assignments.get("bead-1").is_none());
    }

    #[test]
    fn should_get_all_ready_beads() {
        let mut state = SchedulerState::new();
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-1".to_string());
        SchedulerActorDef::handle_register_workflow(&mut state, "wf-2".to_string());
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-1".to_string(),
            "bead-1".to_string(),
        )
        .ok();
        SchedulerActorDef::handle_schedule_bead(
            &mut state,
            "wf-2".to_string(),
            "bead-2".to_string(),
        )
        .ok();

        let ready = SchedulerActorDef::handle_get_all_ready_beads(&state);

        assert_eq!(ready.len(), 2);
    }
}
