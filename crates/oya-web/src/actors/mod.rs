//! Actor placeholders (to be replaced by actual implementations in future beads)

use axum::extract::State;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use ulid::Ulid;

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
            }
        }
    });

    tx
}

/// Helper to extract state in handlers
pub type AppStateRef = State<Arc<AppState>>;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod broadcast_tests;
