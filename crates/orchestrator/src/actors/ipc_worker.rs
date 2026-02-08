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

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::{broadcast, mpsc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

use oya_events::{BeadEvent, EventBus, EventPattern, EventSubscription};
use oya_ipc::{
    AlertLevel, BeadDetail, BeadSummary, ComponentHealth, GraphEdge, GraphNode, GuestMessage,
    HealthStatus, HostMessage, IpcTransport,
};

use crate::agent_swarm::{AgentPool, PoolStats};
use crate::actors::errors::ActorError;
use crate::dag::Dag;
use crate::scheduler::SchedulerState;

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
}

/// IPC worker state.
pub struct IpcWorkerState {
    /// Event subscription ID (for cleanup).
    _event_subscription_id: Option<String>,
    /// Event subscription for receiving orchestrator events.
    _event_subscription: Option<EventSubscription>,
    /// Broadcast sender for HostMessage events.
    event_tx: broadcast::Sender<HostMessage>,
    /// EventBus for subscribing to events.
    event_bus: Option<Arc<EventBus>>,
    /// AgentPool for querying agent statistics.
    agent_pool: Option<Arc<AgentPool>>,
    /// SchedulerState for workflow queries.
    scheduler_state: Option<Arc<SchedulerState>>,
    /// Whether shutdown has been requested.
    shutdown_requested: bool,
}

impl IpcWorkerState {
    /// Create new empty state.
    fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            _event_subscription_id: None,
            _event_subscription: None,
            event_tx,
            event_bus: None,
            agent_pool: None,
            scheduler_state: None,
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
            state.event_bus = Some(bus);
        }

        // Store AgentPool
        if let Some(pool) = args.agent_pool {
            state.agent_pool = Some(pool);
        }

        // Store SchedulerState
        if let Some(scheduler) = args.scheduler_state {
            state.scheduler_state = Some(scheduler);
        }

        // Subscribe to event bus if provided
        if let Some(bus) = &state.event_bus {
            let (subscription_id, subscription) =
                bus.subscribe_with_pattern(EventPattern::All).await;
            state._event_subscription_id = Some(subscription_id);
            state._event_subscription = Some(subscription);
        }

        Ok(state)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
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
                effects.push(IpcWorkerEffect::ReplyGuestMessage {
                    reply,
                    response,
                });
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
                    total_agents: stats.total_agents,
                    active_agents: stats.active_agents,
                    idle_agents: stats.idle_agents,
                    beads_assigned: stats.beads_assigned,
                    beads_completed: stats.beads_completed,
                })
            }

            GuestMessage::GetSystemHealth => {
                // TODO: Query actual system health
                let components = vec![
                    ComponentHealth {
                        name: "EventBus".to_string(),
                        status: HealthStatus::Healthy,
                        message: "Operational".to_string(),
                        last_check: chrono::Utc::now().timestamp() as u64,
                    },
                    ComponentHealth {
                        name: "AgentPool".to_string(),
                        status: HealthStatus::Healthy,
                        message: "Operational".to_string(),
                        last_check: chrono::Utc::now().timestamp() as u64,
                    },
                ];
                Ok(HostMessage::SystemHealth {
                    status: HealthStatus::Healthy,
                    components,
                })
            }

            // COMMANDS
            // ════════
            GuestMessage::StartBead { bead_id } => {
                // TODO: Execute start bead command
                Ok(HostMessage::Ack {
                    command: format!("start_bead {}", bead_id),
                    message: "Bead started".to_string(),
                })
            }

            GuestMessage::CancelBead { bead_id } => {
                // TODO: Execute cancel bead command
                Ok(HostMessage::Ack {
                    command: format!("cancel_bead {}", bead_id),
                    message: "Bead cancelled".to_string(),
                })
            }

            GuestMessage::RetryBead { bead_id } => {
                // TODO: Execute retry bead command
                Ok(HostMessage::Ack {
                    command: format!("retry_bead {}", bead_id),
                    message: "Bead retry queued".to_string(),
                })
            }
        }
    }

    fn get_agent_pool_stats(state: &IpcWorkerState) -> Result<PoolStats, ActorError> {
        if let Some(pool) = &state.agent_pool {
            // TODO: Call pool.get_stats() via async
            Ok(PoolStats {
                total_agents: 0,
                active_agents: 0,
                idle_agents: 0,
                beads_assigned: 0,
                beads_completed: 0,
            })
        } else {
            Ok(PoolStats {
                total_agents: 0,
                active_agents: 0,
                idle_agents: 0,
                beads_assigned: 0,
                beads_completed: 0,
            })
        }
    }
}

impl IpcWorkerActorDef {
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
