//! Actor placeholders (to be replaced by actual implementations in future beads)

use axum::extract::State;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use ulid::Ulid;

use crate::agent_service::{
    AgentLauncher, AgentProcessHandle, AgentService, AgentServiceConfig, AgentServiceError,
};
use async_trait::async_trait;

/// Placeholder message to SchedulerActor
#[derive(Debug, Clone)]
pub enum SchedulerMessage {
    CreateBead { id: Ulid, spec: String },
    CancelBead { id: Ulid },
    RetryBead { id: Ulid },
}

/// Placeholder response from SchedulerActor
#[derive(Debug, Clone)]
pub enum SchedulerResponse {
    Created { id: Ulid },
    Cancelled { id: Ulid },
    Retried { id: Ulid },
    Error { message: String },
}

/// Placeholder message to StateManagerActor
#[derive(Debug)]
pub enum StateManagerMessage {
    QueryBead {
        id: Ulid,
        response: tokio::sync::oneshot::Sender<Option<BeadState>>,
    },
    QueryAllAgents {
        response: tokio::sync::oneshot::Sender<Vec<AgentSummary>>,
    },
    QueryAllBeads {
        response: tokio::sync::oneshot::Sender<Vec<BeadState>>,
    },
    QueryBeadPipeline {
        id: Ulid,
        response: tokio::sync::oneshot::Sender<Vec<StageInfo>>,
    },
    QueryGraph {
        response: tokio::sync::oneshot::Sender<GraphResponse>,
    },
}

/// Placeholder response from StateManagerActor
#[derive(Debug, Clone)]
pub struct BeadState {
    pub id: Ulid,
    pub status: String,
    pub phase: String,
    pub events: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub title: Option<String>,
    pub dependencies: Vec<String>,
    pub progress: f32,
}

/// Information about a pipeline stage
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageInfo {
    pub name: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
}

/// Node in the dependency graph
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub status: String,
}

/// Edge in the dependency graph
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
}

/// Response for graph queries
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Summary of an agent for listing
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentSummary {
    pub id: Ulid,
    pub status: String,
    pub bead_id: Option<String>,
}

/// Placeholder for SchedulerActor sender
pub type SchedulerSender = mpsc::UnboundedSender<SchedulerMessage>;

/// Placeholder for StateManagerActor sender
pub type StateManagerSender = mpsc::UnboundedSender<StateManagerMessage>;

/// Broadcast event type sent to all WebSocket clients
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BroadcastEvent {
    /// Bead status changed
    BeadStatusChanged {
        bead_id: String,
        status: String,
        phase: String,
    },
    /// Bead event occurred
    BeadEvent { bead_id: String, event: String },
    /// General system event
    SystemEvent { message: String },
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub scheduler: Arc<SchedulerSender>,
    pub state_manager: Arc<StateManagerSender>,
    pub agent_service: Arc<AgentService>,
    /// Broadcast channel for sending events to all connected WebSocket clients
    pub broadcast_tx: broadcast::Sender<BroadcastEvent>,
}

impl AppState {
    /// Broadcast an event to all connected WebSocket clients
    ///
    /// Returns Ok(count) where count is the number of receivers that received the event.
    /// Returns Err if there are no active receivers (not an error in practice).
    ///
    /// This is a fire-and-forget operation - slow clients will lag and fast clients
    /// will receive the event immediately.
    pub fn broadcast_event(
        &self,
        event: BroadcastEvent,
    ) -> Result<usize, broadcast::error::SendError<BroadcastEvent>> {
        self.broadcast_tx.send(event)
    }
}

/// Mock scheduler with working receiver (will be replaced with actual actor)
pub fn mock_scheduler() -> SchedulerSender {
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                SchedulerMessage::CreateBead { id, spec } => {
                    tracing::debug!("Scheduler: Creating bead {} with spec: {}", id, spec);
                }
                SchedulerMessage::CancelBead { id } => {
                    tracing::debug!("Scheduler: Cancelling bead {}", id);
                }
                SchedulerMessage::RetryBead { id } => {
                    tracing::debug!("Scheduler: Retrying bead {}", id);
                }
            }
        }
    });

    tx
}

/// Mock state manager with working receiver (will be replaced with actual actor)
pub fn mock_state_manager() -> StateManagerSender {
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                StateManagerMessage::QueryBead { id, response } => {
                    tracing::debug!("State manager: Querying bead {}", id);
                    // Return mock data for testing
                    let mock_state = BeadState {
                        id,
                        status: "pending".to_string(),
                        phase: "initializing".to_string(),
                        events: vec![],
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        updated_at: "2024-01-01T00:00:00Z".to_string(),
                        title: None,
                        dependencies: vec![],
                        progress: 0.45,
                    };
                    let _ = response.send(Some(mock_state));
                }
                StateManagerMessage::QueryAllAgents { response } => {
                    tracing::debug!("State manager: Querying all agents");
                    // Return mock agent list for testing
                    let agents = vec![
                        AgentSummary {
                            id: Ulid::new(),
                            status: "running".to_string(),
                            bead_id: Some("bead-001".to_string()),
                        },
                        AgentSummary {
                            id: Ulid::new(),
                            status: "idle".to_string(),
                            bead_id: Some("bead-002".to_string()),
                        },
                    ];
                    let _ = response.send(agents);
                }
                StateManagerMessage::QueryAllBeads { response } => {
                    tracing::debug!("State manager: Querying all beads");
                    let beads = vec![
                        BeadState {
                            id: Ulid::new(),
                            status: "in_progress".to_string(),
                            phase: "implementing".to_string(),
                            events: vec![],
                            created_at: "2024-01-01T00:00:00Z".to_string(),
                            updated_at: "2024-01-01T00:00:00Z".to_string(),
                            title: Some("Implement agent view".to_string()),
                            dependencies: vec![],
                            progress: 0.65,
                        },
                        BeadState {
                            id: Ulid::new(),
                            status: "pending".to_string(),
                            phase: "planning".to_string(),
                            events: vec![],
                            created_at: "2024-01-01T00:00:00Z".to_string(),
                            updated_at: "2024-01-01T00:00:00Z".to_string(),
                            title: Some("Add dashboard tests".to_string()),
                            dependencies: vec![],
                            progress: 0.0,
                        },
                    ];
                    let _ = response.send(beads);
                }
                StateManagerMessage::QueryBeadPipeline { id, response } => {
                    tracing::debug!("State manager: Querying pipeline for bead {}", id);
                    let stages = vec![
                        StageInfo {
                            name: "planning".to_string(),
                            status: "passed".to_string(),
                            duration_ms: Some(1200),
                            exit_code: None,
                        },
                        StageInfo {
                            name: "implementing".to_string(),
                            status: "running".to_string(),
                            duration_ms: None,
                            exit_code: None,
                        },
                        StageInfo {
                            name: "testing".to_string(),
                            status: "pending".to_string(),
                            duration_ms: None,
                            exit_code: None,
                        },
                    ];
                    let _ = response.send(stages);
                }
                StateManagerMessage::QueryGraph { response } => {
                    tracing::debug!("State manager: Querying global graph");
                    let nodes = vec![
                        GraphNode {
                            id: "bead-1".to_string(),
                            label: "Requirement Analysis".to_string(),
                            status: "completed".to_string(),
                        },
                        GraphNode {
                            id: "bead-2".to_string(),
                            label: "Implementation".to_string(),
                            status: "in_progress".to_string(),
                        },
                        GraphNode {
                            id: "bead-3".to_string(),
                            label: "Testing".to_string(),
                            status: "pending".to_string(),
                        },
                        GraphNode {
                            id: "bead-4".to_string(),
                            label: "Documentation".to_string(),
                            status: "pending".to_string(),
                        },
                        GraphNode {
                            id: "bead-5".to_string(),
                            label: "Deployment".to_string(),
                            status: "pending".to_string(),
                        },
                    ];
                    let edges = vec![
                        GraphEdge {
                            from: "bead-1".to_string(),
                            to: "bead-2".to_string(),
                            edge_type: "blocks".to_string(),
                        },
                        GraphEdge {
                            from: "bead-2".to_string(),
                            to: "bead-3".to_string(),
                            edge_type: "blocks".to_string(),
                        },
                        GraphEdge {
                            from: "bead-2".to_string(),
                            to: "bead-4".to_string(),
                            edge_type: "blocks".to_string(),
                        },
                        GraphEdge {
                            from: "bead-3".to_string(),
                            to: "bead-5".to_string(),
                            edge_type: "blocks".to_string(),
                        },
                        GraphEdge {
                            from: "bead-4".to_string(),
                            to: "bead-5".to_string(),
                            edge_type: "blocks".to_string(),
                        },
                    ];
                    let _ = response.send(GraphResponse { nodes, edges });
                }
            }
        }
    });

    tx
}

/// Mock agent service with noop launcher
pub fn mock_agent_service() -> AgentService {
    struct NoopLauncher;

    #[async_trait]
    impl AgentLauncher for NoopLauncher {
        async fn launch(&self, _agent_id: &str) -> Result<AgentProcessHandle, AgentServiceError> {
            Ok(AgentProcessHandle::Noop)
        }
    }

    AgentService::new_with_launcher(AgentServiceConfig::default(), Arc::new(NoopLauncher))
}

/// Helper to extract state in handlers
pub type AppStateRef = State<Arc<AppState>>;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod broadcast_tests;
