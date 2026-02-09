//! IPC Worker Actor - Zellij plugin communication bridge.
//!
//! This actor manages communication between the Zellij guest plugin (UI)
//! and the OYA orchestrator (host). It handles GuestMessage commands,
//! queries the orchestrator state, and broadcasts HostMessage events.
//!
//! # Architecture
//!
//! ```text
//! Zellij Guest Plugin (UI)
//!        │
//!        │ GuestMessage (stdin/stdout)
//!        ↓
//! ┌─────────────────────────────┐
//! │   IpcWorker Actor           │
//! │  ────────────────────────   │
//! │  • transport: IpcTransport  │
//! │  • orchestrator: references │
//! │  • event_tx: broadcast      │
//! └─────────────────────────────┘
//!        │
//!        │ HostMessage (events, responses)
//!        ↓
//!    Subscribers
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;

use chrono::Utc;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use oya_events::{BeadEvent, EventBus, EventPattern, EventSubscription, Severity};
use oya_pipeline::{
    apply_stage_plan, approve_task, list_all_tasks, load_task_record, resolve_stage_range,
    run_full_pipeline, save_task_record,
};
use oya_events::StageKind;

use crate::ipc_messages::{
    AlertLevel, BeadDetail as IpcBeadDetail, BeadSummary, ComponentHealth, GuestMessage,
    HealthStatus, HostMessage, TaskDetail, TaskSummary, TaskUpdate,
};

use crate::actors::SchedulerState;
use crate::actors::errors::ActorError;
use crate::agent_swarm::{AgentPool, PoolStats};
use crate::persistence::{BeadRecord, BeadState, OrchestratorStore, StoreConfig};

/// IPC worker actor definition.
#[derive(Clone, Default)]
pub struct IpcWorkerActorDef;

/// Arguments passed to the IPC worker on startup.
#[derive(Default, Clone)]
pub struct IpcWorkerArguments {
    /// EventBus for subscribing to bead events.
    pub event_bus: Option<Arc<EventBus>>,
    /// AgentPool for querying agent statistics.
    pub agent_pool: Option<Arc<AgentPool>>,
    /// Optional SchedulerState for workflow queries.
    pub scheduler_state: Option<Arc<SchedulerState>>,
    /// Optional OrchestratorStore for bead persistence queries.
    pub store: Option<Arc<OrchestratorStore>>,
}

impl IpcWorkerArguments {
    /// Create new arguments with no integrations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the EventBus.
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Set the AgentPool.
    pub fn with_agent_pool(mut self, pool: Arc<AgentPool>) -> Self {
        self.agent_pool = Some(pool);
        self
    }

    /// Set the SchedulerState.
    pub fn with_scheduler_state(mut self, state: Arc<SchedulerState>) -> Self {
        self.scheduler_state = Some(state);
        self
    }

    /// Set the OrchestratorStore.
    pub fn with_store(mut self, store: Arc<OrchestratorStore>) -> Self {
        self.store = Some(store);
        self
    }
}

/// IPC worker state.
#[derive(Clone)]
pub struct IpcWorkerState {
    /// Event subscription ID (for cleanup).
    _event_subscription_id: Option<String>,
    /// Broadcast sender for HostMessage events.
    event_tx: broadcast::Sender<HostMessage>,
    /// EventBus for subscribing to events.
    event_bus: Option<Arc<EventBus>>,
    /// AgentPool for querying agent statistics.
    agent_pool: Option<Arc<AgentPool>>,
    /// SchedulerState for workflow queries.
    scheduler_state: Option<Arc<SchedulerState>>,
    /// OrchestratorStore for bead persistence queries.
    store: Option<Arc<OrchestratorStore>>,
    /// Whether shutdown has been requested.
    shutdown_requested: bool,
}

impl IpcWorkerState {
    /// Create new empty state.
    pub(crate) fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            _event_subscription_id: None,
            event_tx,
            event_bus: None,
            agent_pool: None,
            scheduler_state: None,
            store: None,
            shutdown_requested: false,
        }
    }

    /// Create state with store for testing.
    #[cfg(test)]
    fn with_store(store: Arc<OrchestratorStore>) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            _event_subscription_id: None,
            event_tx,
            event_bus: None,
            agent_pool: None,
            scheduler_state: None,
            store: Some(store),
            shutdown_requested: false,
        }
    }
}

/// Messages for the IPC worker actor.
#[derive(Debug)]
pub enum IpcWorkerMessage {
    /// Handle a guest message (from Zellij plugin).
    HandleGuestMessage {
        /// Guest message to process
        message: GuestMessage,
        /// Reply port for the response
        reply: ractor::RpcReplyPort<Result<HostMessage, ActorError>>,
    },

    /// Subscribe to host events.
    Subscribe {
        /// Sender for host messages
        sender: mpsc::Sender<HostMessage>,
    },

    /// Initiate graceful shutdown.
    Shutdown,
}

/// Effects produced by the IPC worker.
pub enum IpcWorkerEffect {
    /// Reply to a guest message.
    ReplyGuestMessage {
        reply: ractor::RpcReplyPort<Result<HostMessage, ActorError>>,
        response: Result<HostMessage, ActorError>,
    },
}

/// Errors that can occur during IPC bridge operations.
#[derive(Debug, thiserror::Error)]
pub enum IpcBridgeError {
    /// Event serialization failed.
    #[error("Event serialization failed: {event_type} - {reason}")]
    EventSerializationFailed {
        event_type: String,
        reason: String,
    },

    /// Invalid event payload (missing required fields).
    #[error("Invalid event payload for bead {bead_id}: {event_type} - missing {missing_field}")]
    InvalidEventPayload {
        bead_id: String,
        event_type: String,
        missing_field: String,
    },

    /// Stage kind not recognized.
    #[error("Unknown stage kind: {stage_name}")]
    UnknownStageKind {
        stage_name: String,
    },

    /// Attempt count overflow.
    #[error("Attempt count overflow for bead {bead_id}: {current_count}")]
    AttemptCountOverflow {
        bead_id: String,
        current_count: u32,
    },

    /// EventBus not ready.
    #[error("EventBus not ready: unavailable for {since:?}")]
    EventBusNotReady {
        since: std::time::Duration,
    },
}

/// Convert StageKind to string for IPC.
fn stage_kind_to_string(stage: StageKind) -> String {
    match stage {
        StageKind::Research => "research",
        StageKind::Plan => "plan",
        StageKind::Implement => "implement",
        StageKind::Review => "review",
        StageKind::Validate => "validate",
        StageKind::Accept => "accept",
    }
    .to_string()
}

/// Convert Severity to string for IPC.
fn severity_to_string(severity: Severity) -> String {
    match severity {
        Severity::Minor => "minor",
        Severity::Major => "major",
        Severity::Fundamental => "fundamental",
    }
    .to_string()
}

/// Truncate string to maximum length with indicator.
fn truncate_with_indicator(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Convert BeadEvent to HostMessage for stage updates.
pub fn event_to_host_message(event: &BeadEvent) -> Result<HostMessage, IpcBridgeError> {
    match event {
        BeadEvent::StageStarted {
            bead_id,
            stage,
            attempt,
            timestamp,
            ..
        } => {
            let stage_str = stage_kind_to_string(*stage);
            Ok(HostMessage::StageStarted {
                bead_id: bead_id.to_string(),
                stage: stage_str,
                attempt: *attempt,
                timestamp: timestamp.timestamp() as u64,
            })
        }

        BeadEvent::StageCompleted {
            bead_id,
            stage,
            artifact_ref,
            timestamp,
            ..
        } => {
            let stage_str = stage_kind_to_string(*stage);
            Ok(HostMessage::StageCompleted {
                bead_id: bead_id.to_string(),
                stage: stage_str,
                artifact_ref: artifact_ref.clone(),
                timestamp: timestamp.timestamp() as u64,
            })
        }

        BeadEvent::StageFailed {
            bead_id,
            stage,
            feedback,
            severity,
            timestamp,
            ..
        } => {
            let stage_str = stage_kind_to_string(*stage);
            let severity_str = severity_to_string(*severity);
            let truncated_feedback = truncate_with_indicator(feedback, 256);
            Ok(HostMessage::StageFailed {
                bead_id: bead_id.to_string(),
                stage: stage_str,
                feedback: truncated_feedback,
                severity: severity_str,
                timestamp: timestamp.timestamp() as u64,
            })
        }

        BeadEvent::StageReentry {
            bead_id,
            from_stage,
            to_stage,
            reason,
            attempt,
            timestamp,
            ..
        } => {
            let from_str = stage_kind_to_string(*from_stage);
            let to_str = stage_kind_to_string(*to_stage);
            let truncated_reason = truncate_with_indicator(reason, 256);
            Ok(HostMessage::StageReentry {
                bead_id: bead_id.to_string(),
                from_stage: from_str,
                to_stage: to_str,
                reason: truncated_reason,
                attempt: *attempt,
                timestamp: timestamp.timestamp() as u64,
            })
        }

        BeadEvent::ValidationRan {
            bead_id,
            passed,
            output,
            command,
            exit_code,
            timestamp,
            ..
        } => {
            let truncated_output = truncate_with_indicator(output, 256);
            Ok(HostMessage::ValidationRan {
                bead_id: bead_id.to_string(),
                passed: *passed,
                output: truncated_output,
                command: command.clone(),
                exit_code: *exit_code,
                timestamp: timestamp.timestamp() as u64,
            })
        }

        BeadEvent::RecursionExhausted {
            bead_id,
            total_attempts,
            last_stage,
            timestamp,
            ..
        } => {
            let stage_str = stage_kind_to_string(*last_stage);
            Ok(HostMessage::RecursionExhausted {
                bead_id: bead_id.to_string(),
                total_attempts: *total_attempts,
                last_stage: stage_str,
                timestamp: timestamp.timestamp() as u64,
            })
        }

        // Non-stage events are not handled by this function
        _ => Err(IpcBridgeError::EventSerializationFailed {
            event_type: event.event_type().to_string(),
            reason: "Not a stage lifecycle event".to_string(),
        }),
    }
}

impl Actor for IpcWorkerActorDef {
    type Msg = IpcWorkerMessage;
    type State = IpcWorkerState;
    type Arguments = IpcWorkerArguments;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("IpcWorker starting");

        let mut state = IpcWorkerState::new();

        // Store EventBus
        if let Some(bus) = args.event_bus {
            state.event_bus = Some(bus.clone());
        }

        // Store AgentPool
        if let Some(pool) = args.agent_pool {
            state.agent_pool = Some(pool);
        }

        // Store SchedulerState
        if let Some(scheduler) = args.scheduler_state {
            state.scheduler_state = Some(scheduler);
        }

        // Store OrchestratorStore
        if let Some(store) = args.store {
            state.store = Some(store);
        }

        // Subscribe to event bus if provided
        if let Some(bus) = &state.event_bus {
            let (subscription_id, _subscription) =
                bus.subscribe_with_pattern(EventPattern::All).await;
            state._event_subscription_id = Some(subscription_id);

            // Spawn event forwarder
            let event_tx = state.event_tx.clone();
            tokio::spawn(Self::event_forwarder(_subscription, event_tx));
        }

        Ok(state)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        if let IpcWorkerMessage::HandleGuestMessage {
            message:
                GuestMessage::RunStage {
                    slug,
                    stage,
                    from,
                    to,
                    dry_run,
                },
            reply,
        } = message
        {
            let response =
                Self::handle_run_stage(&slug, &stage, from.as_deref(), to.as_deref(), dry_run)
                    .await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::RunPipeline { slug, dry_run },
            reply,
        } = message
        {
            let response = Self::handle_run_pipeline(&slug, dry_run).await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::RunPipelineBatch { slugs, dry_run },
            reply,
        } = message
        {
            let response = Self::handle_run_pipeline_batch(&slugs, dry_run).await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::ApproveTask { slug, force },
            reply,
        } = message
        {
            let response = Self::handle_approve_task(&slug, force).await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::GetTaskList,
            reply,
        } = message
        {
            let response = Self::handle_get_task_list().await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::GetTaskDetail { slug },
            reply,
        } = message
        {
            let response = Self::handle_get_task_detail(&slug).await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::GetBeadList,
            reply,
        } = message
        {
            let response = Self::handle_get_task_list().await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::GetBeadDetail { bead_id },
            reply,
        } = message
        {
            let response = Self::handle_get_task_detail(&bead_id).await;
            let _ = reply.send(response);
            return Ok(());
        }

        // Bead operation commands (async, require persistence)
        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead { bead_id },
            reply,
        } = message
        {
            let response = Self::handle_start_bead(state, &bead_id).await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::CancelBead { bead_id },
            reply,
        } = message
        {
            let response = Self::handle_cancel_bead(state, &bead_id).await;
            let _ = reply.send(response);
            return Ok(());
        }

        if let IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::RetryBead { bead_id },
            reply,
        } = message
        {
            let response = Self::handle_retry_bead(state, &bead_id).await;
            let _ = reply.send(response);
            return Ok(());
        }

        // Special case for Shutdown
        if matches!(message, IpcWorkerMessage::Shutdown) {
            info!("IpcWorker shutdown requested");
            state.shutdown_requested = true;
            _myself.stop(Some("IpcWorker shutdown requested".to_string()));
            return Ok(());
        }

        let (next_state, effects) = core::handle(state.clone(), message);
        *state = next_state;

        for effect in effects {
            match effect {
                IpcWorkerEffect::ReplyGuestMessage { reply, response } => {
                    let _ = reply.send(response);
                }
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("IpcWorker stopping");
        Ok(())
    }
}

/// Functional core for IpcWorker.
mod core {
    use super::*;

    pub fn handle(
        state: IpcWorkerState,
        msg: IpcWorkerMessage,
    ) -> (IpcWorkerState, Vec<IpcWorkerEffect>) {
        let mut effects = Vec::new();

        match msg {
            IpcWorkerMessage::HandleGuestMessage { message, reply } => {
                let response = handle_guest_message(&state, message);
                effects.push(IpcWorkerEffect::ReplyGuestMessage { reply, response });
            }
            IpcWorkerMessage::Subscribe { sender } => {
                // Subscribe sender to broadcast events
                let mut rx = state.event_tx.subscribe();
                tokio::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        if sender.send(msg).await.is_err() {
                            break; // Receiver closed
                        }
                    }
                });
            }
            IpcWorkerMessage::Shutdown => {} // Handled by shell
        }

        (state, effects)
    }

    fn handle_guest_message(
        state: &IpcWorkerState,
        message: GuestMessage,
    ) -> Result<HostMessage, ActorError> {
        match message {
            // QUERIES
            // ═══════
            GuestMessage::GetBeadList => {
                // TODO: Query actual bead list from BeadStore
                let beads = vec![];
                Ok(HostMessage::BeadList { beads })
            }

            GuestMessage::GetTaskList | GuestMessage::GetTaskDetail { .. } => Err(
                ActorError::internal("Task queries are handled asynchronously".to_string()),
            ),

            GuestMessage::GetBeadDetail { bead_id } => {
                // TODO: Query actual bead details from BeadStore
                return Err(ActorError::not_found(
                    format!("bead {}", bead_id),
                    "Bead not found",
                ));
            }

            GuestMessage::GetWorkflowGraph { workflow_id } => {
                // TODO: Query actual workflow graph from DAG
                let nodes = vec![];
                let edges = vec![];
                Ok(HostMessage::WorkflowGraph {
                    workflow_id,
                    nodes,
                    edges,
                })
            }

            GuestMessage::GetAgentPool => {
                let stats = get_agent_pool_stats(state)?;
                Ok(HostMessage::AgentPoolStats {
                    total_agents: stats.total,
                    active_agents: stats.working,
                    idle_agents: stats.idle,
                    beads_assigned: 0,  // TODO: Track assigned beads
                    beads_completed: 0, // TODO: Track completed beads
                })
            }

            GuestMessage::GetSystemHealth => {
                let health = get_system_health(state);
                Ok(HostMessage::SystemHealth {
                    status: health.overall_status,
                    components: health.components,
                })
            }

            // COMMANDS
            // ════════
            GuestMessage::StartBead { .. }
            | GuestMessage::CancelBead { .. }
            | GuestMessage::RetryBead { .. } => Err(ActorError::internal(
                "Bead commands are handled asynchronously".to_string(),
            )),

            GuestMessage::RunStage { .. } | GuestMessage::ApproveTask { .. } => Err(
                ActorError::internal("Task commands are handled asynchronously".to_string()),
            ),
            GuestMessage::RunPipeline { .. } | GuestMessage::RunPipelineBatch { .. } => Err(
                ActorError::internal("Task commands are handled asynchronously".to_string()),
            ),
        }
    }

    fn get_agent_pool_stats(state: &IpcWorkerState) -> Result<PoolStats, ActorError> {
        if let Some(_pool) = &state.agent_pool {
            // TODO: Call pool.get_stats() via async
            // For now, return default stats
            Ok(PoolStats {
                total: 0,
                idle: 0,
                working: 0,
                unhealthy: 0,
                shutting_down: 0,
                terminated: 0,
                beads_assigned: 0,
                beads_completed: 0,
            })
        } else {
            Ok(PoolStats {
                total: 0,
                idle: 0,
                working: 0,
                unhealthy: 0,
                shutting_down: 0,
                terminated: 0,
                beads_assigned: 0,
                beads_completed: 0,
            })
        }
    }

    #[derive(Debug)]
    struct SystemHealthReport {
        overall_status: HealthStatus,
        components: Vec<ComponentHealth>,
    }

    fn get_system_health(state: &IpcWorkerState) -> SystemHealthReport {
        let now = Utc::now().timestamp() as u64;
        let mut components = Vec::new();
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;

        // Check EventBus health
        let event_bus_health = check_event_bus(state, now);
        degraded_count += if matches!(event_bus_health.status, HealthStatus::Degraded) {
            1
        } else {
            0
        };
        unhealthy_count += if matches!(event_bus_health.status, HealthStatus::Unhealthy) {
            1
        } else {
            0
        };
        components.push(event_bus_health);

        // Check AgentPool health
        let agent_pool_health = check_agent_pool(state, now);
        degraded_count += if matches!(agent_pool_health.status, HealthStatus::Degraded) {
            1
        } else {
            0
        };
        unhealthy_count += if matches!(agent_pool_health.status, HealthStatus::Unhealthy) {
            1
        } else {
            0
        };
        components.push(agent_pool_health);

        // Check SchedulerState health
        let scheduler_health = check_scheduler_state(state, now);
        degraded_count += if matches!(scheduler_health.status, HealthStatus::Degraded) {
            1
        } else {
            0
        };
        unhealthy_count += if matches!(scheduler_health.status, HealthStatus::Unhealthy) {
            1
        } else {
            0
        };
        components.push(scheduler_health);

        // Check Persistence health
        let persistence_health = check_persistence(now);
        degraded_count += if matches!(persistence_health.status, HealthStatus::Degraded) {
            1
        } else {
            0
        };
        unhealthy_count += if matches!(persistence_health.status, HealthStatus::Unhealthy) {
            1
        } else {
            0
        };
        components.push(persistence_health);

        // Determine overall health status
        let overall_status = if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        SystemHealthReport {
            overall_status,
            components,
        }
    }

    fn check_event_bus(state: &IpcWorkerState, timestamp: u64) -> ComponentHealth {
        match &state.event_bus {
            Some(_) => ComponentHealth {
                name: "EventBus".to_string(),
                status: HealthStatus::Healthy,
                message: "Operational: Event bus is connected and accepting events".to_string(),
                last_check: timestamp,
            },
            None => ComponentHealth {
                name: "EventBus".to_string(),
                status: HealthStatus::Degraded,
                message: "Degraded: Event bus not initialized".to_string(),
                last_check: timestamp,
            },
        }
    }

    fn check_agent_pool(state: &IpcWorkerState, timestamp: u64) -> ComponentHealth {
        match &state.agent_pool {
            Some(_) => {
                let pool_stats =
                    get_agent_pool_stats(state).unwrap_or_else(|_| PoolStats::default());
                let total = pool_stats.total;
                let working = pool_stats.working;
                let unhealthy = pool_stats.unhealthy;
                let idle = pool_stats.idle;

                let status = if unhealthy > 0 {
                    HealthStatus::Unhealthy
                } else if total == 0 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                let message = if total == 0 {
                    "Empty pool: No agents registered".to_string()
                } else {
                    format!(
                        "Operational: {}/{} agents active, {} idle, {} unhealthy",
                        working, total, idle, unhealthy
                    )
                };

                ComponentHealth {
                    name: "AgentPool".to_string(),
                    status,
                    message,
                    last_check: timestamp,
                }
            }
            None => ComponentHealth {
                name: "AgentPool".to_string(),
                status: HealthStatus::Degraded,
                message: "Degraded: Agent pool not initialized".to_string(),
                last_check: timestamp,
            },
        }
    }

    fn check_scheduler_state(state: &IpcWorkerState, timestamp: u64) -> ComponentHealth {
        match &state.scheduler_state {
            Some(scheduler) => {
                let workflow_count = scheduler.core.workflows.len();
                let pending_beads = scheduler.core.pending_beads.len();
                let ready_beads = scheduler.core.ready_beads.len();

                let status = if scheduler.shutdown_requested {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                let message = if scheduler.shutdown_requested {
                    format!(
                        "Shutdown in progress: {} workflows, {} pending beads",
                        workflow_count, pending_beads
                    )
                } else {
                    format!(
                        "Operational: {} workflows, {} pending beads, {} ready beads",
                        workflow_count, pending_beads, ready_beads
                    )
                };

                ComponentHealth {
                    name: "SchedulerState".to_string(),
                    status,
                    message,
                    last_check: timestamp,
                }
            }
            None => ComponentHealth {
                name: "SchedulerState".to_string(),
                status: HealthStatus::Degraded,
                message: "Degraded: Scheduler not initialized".to_string(),
                last_check: timestamp,
            },
        }
    }

    fn check_persistence(timestamp: u64) -> ComponentHealth {
        ComponentHealth {
            name: "Persistence".to_string(),
            status: HealthStatus::Healthy,
            message: "Operational: File system storage accessible".to_string(),
            last_check: timestamp,
        }
    }
}

impl IpcWorkerActorDef {
    async fn handle_run_stage(
        slug: &str,
        stage: &str,
        from: Option<&str>,
        to: Option<&str>,
        dry_run: bool,
    ) -> Result<HostMessage, ActorError> {
        let repo_root = locate_repo_root()?;
        let stages = resolve_stage_range(stage, from, to).map_err(map_pipeline_error)?;

        let task = load_task_record(slug, &repo_root)
            .await
            .map_err(map_pipeline_error)?;
        let updated = apply_stage_plan(task, &stages).map_err(map_pipeline_error)?;

        if dry_run {
            return Ok(HostMessage::TaskUpdated {
                slug: slug.to_string(),
                status: updated.status.to_string(),
                message: "Dry run: task status not persisted".to_string(),
            });
        }

        save_task_record(&updated, &repo_root)
            .await
            .map_err(map_pipeline_error)?;

        Ok(HostMessage::TaskUpdated {
            slug: slug.to_string(),
            status: updated.status.to_string(),
            message: "Task updated successfully".to_string(),
        })
    }

    async fn handle_run_pipeline(slug: &str, dry_run: bool) -> Result<HostMessage, ActorError> {
        let repo_root = locate_repo_root()?;
        let task = load_task_record(slug, &repo_root)
            .await
            .map_err(map_pipeline_error)?;
        let updated = run_full_pipeline(task).map_err(map_pipeline_error)?;

        if dry_run {
            return Ok(HostMessage::TaskUpdated {
                slug: slug.to_string(),
                status: updated.status.to_string(),
                message: "Dry run: task status not persisted".to_string(),
            });
        }

        save_task_record(&updated, &repo_root)
            .await
            .map_err(map_pipeline_error)?;

        Ok(HostMessage::TaskUpdated {
            slug: slug.to_string(),
            status: updated.status.to_string(),
            message: "Task updated successfully".to_string(),
        })
    }

    async fn handle_run_pipeline_batch(
        slugs: &[String],
        dry_run: bool,
    ) -> Result<HostMessage, ActorError> {
        let repo_root = locate_repo_root()?;
        let mut updated = Vec::new();
        let mut failed = Vec::new();

        for slug in slugs {
            match run_pipeline_for_slug(slug, dry_run, &repo_root).await {
                Ok(update) => updated.push(update),
                Err(update) => failed.push(update),
            }
        }

        Ok(HostMessage::TaskBatchUpdated { updated, failed })
    }

    async fn handle_approve_task(slug: &str, force: bool) -> Result<HostMessage, ActorError> {
        let repo_root = locate_repo_root()?;
        let task = load_task_record(slug, &repo_root)
            .await
            .map_err(map_pipeline_error)?;
        let updated = approve_task(task, force).map_err(map_pipeline_error)?;

        save_task_record(&updated, &repo_root)
            .await
            .map_err(map_pipeline_error)?;

        Ok(HostMessage::TaskUpdated {
            slug: slug.to_string(),
            status: updated.status.to_string(),
            message: "Task approved successfully".to_string(),
        })
    }

    async fn handle_get_task_list() -> Result<HostMessage, ActorError> {
        let repo_root = locate_repo_root()?;
        let tasks = list_all_tasks(&repo_root)
            .await
            .map_err(map_pipeline_error)?;
        let summaries = tasks.into_iter().map(task_to_summary).collect();
        Ok(HostMessage::TaskList { tasks: summaries })
    }

    async fn handle_get_task_detail(slug: &str) -> Result<HostMessage, ActorError> {
        let repo_root = locate_repo_root()?;
        let task = load_task_record(slug, &repo_root)
            .await
            .map_err(map_pipeline_error)?;
        Ok(HostMessage::TaskDetail {
            task: task_to_detail(task),
        })
    }

    /// Handle start bead command.
    ///
    /// Transitions a bead from a non-terminal state to Running.
    /// This operation is idempotent: calling start on an already-running bead
    /// succeeds without modification.
    pub(crate) async fn handle_start_bead(
        state: &IpcWorkerState,
        bead_id: &str,
    ) -> Result<HostMessage, ActorError> {
        let store = state
            .store
            .as_ref()
            .ok_or_else(|| ActorError::internal("Store not initialized"))?;

        if bead_id.is_empty() {
            return Err(ActorError::internal("Bead ID cannot be empty"));
        }

        let current = match store.get_bead(bead_id).await {
            Ok(record) => record,
            Err(_) => {
                return Err(ActorError::BeadNotFound(bead_id.to_string()));
            }
        };

        match current.state {
            BeadState::Completed | BeadState::Failed | BeadState::Cancelled => {
                return Err(ActorError::invalid_state_transition(format!(
                    "Cannot start bead in terminal state: {}",
                    current.state
                )));
            }
            BeadState::Running => {
                return Ok(HostMessage::Ack {
                    command: "StartBead".to_string(),
                    message: format!("Bead {} is already running", bead_id),
                });
            }
            BeadState::Pending | BeadState::Ready | BeadState::Dispatched | BeadState::Assigned => {
                // Valid transition to Running
            }
        }

        match store.update_bead_state(bead_id, BeadState::Running).await {
            Ok(_) => (),
            Err(e) => {
                return Err(ActorError::internal(format!(
                    "Failed to update bead state: {}",
                    e
                )));
            }
        };

        Ok(HostMessage::Ack {
            command: "StartBead".to_string(),
            message: format!("Bead {} started successfully", bead_id),
        })
    }

    /// Handle cancel bead command.
    ///
    /// Transitions a bead from any non-terminal state to Cancelled.
    /// This operation is idempotent: calling cancel on an already-cancelled bead
    /// succeeds without modification.
    pub(crate) async fn handle_cancel_bead(
        state: &IpcWorkerState,
        bead_id: &str,
    ) -> Result<HostMessage, ActorError> {
        let store = state
            .store
            .as_ref()
            .ok_or_else(|| ActorError::internal("Store not initialized"))?;

        if bead_id.is_empty() {
            return Err(ActorError::internal("Bead ID cannot be empty"));
        }

        let current = match store.get_bead(bead_id).await {
            Ok(record) => record,
            Err(_) => {
                return Err(ActorError::BeadNotFound(bead_id.to_string()));
            }
        };

        match current.state {
            BeadState::Completed | BeadState::Failed => {
                return Err(ActorError::invalid_state_transition(format!(
                    "Cannot cancel bead in terminal state: {}",
                    current.state
                )));
            }
            BeadState::Cancelled => {
                return Ok(HostMessage::Ack {
                    command: "CancelBead".to_string(),
                    message: format!("Bead {} is already cancelled", bead_id),
                });
            }
            BeadState::Pending
            | BeadState::Ready
            | BeadState::Dispatched
            | BeadState::Assigned
            | BeadState::Running => {
                // Valid transition to Cancelled
            }
        }

        match store.update_bead_state(bead_id, BeadState::Cancelled).await {
            Ok(_) => (),
            Err(e) => {
                return Err(ActorError::internal(format!(
                    "Failed to update bead state: {}",
                    e
                )));
            }
        };

        Ok(HostMessage::Ack {
            command: "CancelBead".to_string(),
            message: format!("Bead {} cancelled successfully", bead_id),
        })
    }

    /// Handle retry bead command.
    ///
    /// Resets a Failed bead to Ready state for re-execution.
    /// Increments retry_count and clears error information.
    pub(crate) async fn handle_retry_bead(
        state: &IpcWorkerState,
        bead_id: &str,
    ) -> Result<HostMessage, ActorError> {
        let store = state
            .store
            .as_ref()
            .ok_or_else(|| ActorError::internal("Store not initialized"))?;

        if bead_id.is_empty() {
            return Err(ActorError::internal("Bead ID cannot be empty"));
        }

        let current = match store.get_bead(bead_id).await {
            Ok(record) => record,
            Err(_) => {
                return Err(ActorError::BeadNotFound(bead_id.to_string()));
            }
        };

        if current.state != BeadState::Failed {
            return Err(ActorError::invalid_state_transition(format!(
                "Cannot retry bead in state: {} (only Failed beads can be retried)",
                current.state
            )));
        }

        let new_retry_count = current.retry_count + 1;

        match store.update_bead_state(bead_id, BeadState::Ready).await {
            Ok(_) => (),
            Err(e) => {
                return Err(ActorError::internal(format!(
                    "Failed to update bead state: {}",
                    e
                )));
            }
        };

        Ok(HostMessage::Ack {
            command: "RetryBead".to_string(),
            message: format!(
                "Bead {} reset for retry (attempt {})",
                bead_id, new_retry_count
            ),
        })
    }

    /// Forward events from EventBus to broadcast subscribers.
    pub async fn event_forwarder(
        mut subscription: EventSubscription,
        event_tx: broadcast::Sender<HostMessage>,
    ) {
        while let Ok(event) = subscription.recv().await {
            let msg = Self::convert_event_to_host_message(event);
            if let Some(host_msg) = msg {
                let _ = event_tx.send(host_msg);
            }
        }
    }

    fn convert_event_to_host_message(event: BeadEvent) -> Option<HostMessage> {
        // Try to convert stage lifecycle events first
        match event_to_host_message(&event) {
            Ok(msg) => return Some(msg),
            Err(IpcBridgeError::EventSerializationFailed { .. }) => {
                // Not a stage event, continue to other conversions
            }
            Err(e) => {
                tracing::warn!("Failed to convert event to HostMessage: {}", e);
                return None;
            }
        }

        // Handle non-stage events
        match event {
            BeadEvent::StateChanged {
                bead_id,
                from,
                to,
                timestamp,
                ..
            } => Some(HostMessage::BeadStateChanged {
                bead_id: bead_id.to_string(),
                from_state: from.to_string(),
                to_state: to.to_string(),
                timestamp: timestamp.timestamp() as u64,
            }),
            BeadEvent::PhaseCompleted {
                bead_id,
                phase_id,
                phase_name,
                timestamp,
                ..
            } => Some(HostMessage::PhaseProgress {
                bead_id: bead_id.to_string(),
                phase_id: phase_id.to_string(),
                progress: 100, // Phase completed means 100%
                current_step: format!("Completed: {}", phase_name),
            }),
            BeadEvent::Failed {
                bead_id,
                error,
                timestamp,
                ..
            } => Some(HostMessage::SystemAlert {
                level: AlertLevel::Error,
                message: format!("Bead failed: {}", error),
                component: Some(bead_id.to_string()),
                timestamp: timestamp.timestamp() as u64,
            }),
            BeadEvent::WorkerUnhealthy {
                worker_id,
                reason,
                timestamp,
                ..
            } => Some(HostMessage::SystemAlert {
                level: AlertLevel::Warning,
                message: format!("Worker unhealthy: {}", reason),
                component: Some(worker_id),
                timestamp: timestamp.timestamp() as u64,
            }),
            _ => None,
        }
    }
}

fn locate_repo_root() -> Result<std::path::PathBuf, ActorError> {
    let current = std::env::current_dir().map_err(|err| ActorError::internal(err.to_string()))?;
    let mut path = current.as_path();

    loop {
        let oya_dir = path.join(".oya");
        let git_dir = path.join(".git");

        if oya_dir.exists() || git_dir.exists() {
            return Ok(path.to_path_buf());
        }

        match path.parent() {
            Some(parent) if parent != path => path = parent,
            _ => {
                return Ok(current);
            }
        }
    }
}

fn map_pipeline_error(error: oya_pipeline::Error) -> ActorError {
    match error {
        oya_pipeline::Error::InvalidTransition { .. }
        | oya_pipeline::Error::InvalidStageSequence(_) => {
            ActorError::invalid_state_transition(error.to_string())
        }
        oya_pipeline::Error::TaskNotFound(task) => ActorError::not_found("task", task),
        _ => ActorError::internal(error.to_string()),
    }
}

fn task_to_summary(task: oya_pipeline::domain::Task) -> TaskSummary {
    let (status, stage) = task_status_fields(&task.status);
    TaskSummary {
        slug: task.slug.as_str().to_string(),
        status,
        stage,
        priority: task.priority.to_string(),
        language: task.language.to_string(),
        branch: task.branch,
    }
}

fn task_to_detail(task: oya_pipeline::domain::Task) -> TaskDetail {
    let (status, stage) = task_status_fields(&task.status);
    TaskDetail {
        slug: task.slug.as_str().to_string(),
        status,
        stage,
        priority: task.priority.to_string(),
        language: task.language.to_string(),
        branch: task.branch,
    }
}

fn task_status_fields(status: &oya_pipeline::domain::TaskStatus) -> (String, Option<String>) {
    match status {
        oya_pipeline::domain::TaskStatus::Created => ("created".to_string(), None),
        oya_pipeline::domain::TaskStatus::InProgress { stage } => {
            ("in_progress".to_string(), Some(stage.clone()))
        }
        oya_pipeline::domain::TaskStatus::PassedPipeline => ("passed".to_string(), None),
        oya_pipeline::domain::TaskStatus::FailedPipeline { stage, reason } => {
            ("failed".to_string(), Some(format!("{stage}: {reason}")))
        }
        oya_pipeline::domain::TaskStatus::Integrated => ("integrated".to_string(), None),
    }
}

async fn run_pipeline_for_slug(
    slug: &str,
    dry_run: bool,
    repo_root: &std::path::Path,
) -> Result<TaskUpdate, TaskUpdate> {
    let task = load_task_record(slug, repo_root).await;
    let update = match task {
        Ok(task) => match run_full_pipeline(task) {
            Ok(updated) if dry_run => Ok(TaskUpdate {
                slug: slug.to_string(),
                status: Some(updated.status.to_string()),
                message: "Dry run: task status not persisted".to_string(),
            }),
            Ok(updated) => save_task_record(&updated, repo_root)
                .await
                .map(|_| TaskUpdate {
                    slug: slug.to_string(),
                    status: Some(updated.status.to_string()),
                    message: "Task updated successfully".to_string(),
                })
                .map_err(|err| TaskUpdate {
                    slug: slug.to_string(),
                    status: None,
                    message: err.to_string(),
                }),
            Err(err) => Err(TaskUpdate {
                slug: slug.to_string(),
                status: None,
                message: err.to_string(),
            }),
        },
        Err(err) => Err(TaskUpdate {
            slug: slug.to_string(),
            status: None,
            message: err.to_string(),
        }),
    };

    update
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_worker_arguments_construction() {
        let args = IpcWorkerArguments::new();
        assert!(args.event_bus.is_none());
        assert!(args.agent_pool.is_none());
    }

    #[test]
    fn test_ipc_worker_state_construction() {
        let state = IpcWorkerState::new();
        // Verify broadcast channel exists
        let receiver_count = state.event_tx.receiver_count();
        assert_eq!(receiver_count, 0);
    }
}

// Include bead operations tests from separate file
#[cfg(test)]
include!("ipc_worker_bead_ops_tests.rs");
