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
                    // For now, return None (not found) - actual implementation will query DB
                    let _ = response.send(None);
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
