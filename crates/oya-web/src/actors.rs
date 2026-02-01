//! Actor placeholders (to be replaced by actual implementations in future beads)

use axum::extract::State;
use std::sync::Arc;
use tokio::sync::mpsc;
use ulid::Ulid;

/// Placeholder message to SchedulerActor
#[derive(Debug, Clone)]
pub enum SchedulerMessage {
    CreateBead { spec: String },
    CancelBead { id: Ulid },
}

/// Placeholder response from SchedulerActor
#[derive(Debug, Clone)]
pub enum SchedulerResponse {
    Created { id: Ulid },
    Cancelled { id: Ulid },
    Error { message: String },
}

/// Placeholder message to StateManagerActor
#[derive(Debug, Clone)]
pub enum StateManagerMessage {
    QueryBead { id: Ulid },
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
}

/// Placeholder for SchedulerActor sender
pub type SchedulerSender = mpsc::UnboundedSender<SchedulerMessage>;

/// Placeholder for StateManagerActor sender
pub type StateManagerSender = mpsc::UnboundedSender<StateManagerMessage>;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub scheduler: Arc<SchedulerSender>,
    pub state_manager: Arc<StateManagerSender>,
}

/// Mock scheduler with working receiver (will be replaced with actual actor)
pub fn mock_scheduler() -> SchedulerSender {
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                SchedulerMessage::CreateBead { spec } => {
                    tracing::debug!("Scheduler: Creating bead with spec: {}", spec);
                }
                SchedulerMessage::CancelBead { id } => {
                    tracing::debug!("Scheduler: Cancelling bead {}", id);
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
                StateManagerMessage::QueryBead { id } => {
                    tracing::debug!("State manager: Querying bead {}", id);
                }
            }
        }
    });

    tx
}

/// Helper to extract state in handlers
pub type AppStateRef = State<Arc<AppState>>;
