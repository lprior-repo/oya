//! Bridges orchestrator persistence with the checkpoint resume helpers.
//!
//! This module adapts `OrchestratorStore` to the `CheckpointStore` and `EventLog`
//! traits provided by the `oya_events::replay::resume` module so that the replay
//! engine can validate checkpoints and inspect the event log before applying events.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use surrealdb::sql::Datetime as SurrealDatetime;
use tokio::runtime::{Handle, Runtime};

use oya_events::replay::resume::{
    CheckpointData, CheckpointId, CheckpointStore, EventLog, EventMetadata,
};
use oya_events::Error as OyaError;

use crate::persistence::OrchestratorStore;

/// Checkpoint store backed by the orchestrator persistence layer.
#[derive(Clone, Debug)]
pub struct OrchestratorCheckpointStore {
    store: Arc<OrchestratorStore>,
}

impl OrchestratorCheckpointStore {
    /// Create a new checkpoint bridge.
    #[must_use]
    pub const fn new(store: Arc<OrchestratorStore>) -> Self {
        Self { store }
    }
}

fn run_async<F, T>(future: F) -> Result<T, OyaError>
where
    F: std::future::Future<Output = T>,
{
    if let Ok(handle) = Handle::try_current() {
        Ok(tokio::task::block_in_place(|| handle.block_on(future)))
    } else {
        Runtime::new()
            .map_err(|err| OyaError::Internal(format!("tokio runtime creation failed: {err}")))
            .map(|rt| rt.block_on(future))
    }
}

impl CheckpointStore for OrchestratorCheckpointStore {
    fn load_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> Result<Option<(CheckpointData, DateTime<Utc>)>, OyaError> {
        let store = self.store.clone();
        let id = checkpoint_id.as_str().to_string();
        let record = run_async(async move { store.get_checkpoint(&id).await })
            .map_err(|e| OyaError::Internal(format!("checkpoint load failed: {e}")))?
            .map_err(|e| OyaError::Internal(format!("checkpoint fetch failed: {e}")))?;
        
        let checkpoint_data = CheckpointData {
            state: record.scheduler_state.into_bytes(),
            sequence_number: record.event_sequence,
            compressed: false,
        };
        Ok(Some((checkpoint_data, record.created_at)))
    }

    fn validate_timestamp(
        &self,
        checkpoint_id: &CheckpointId,
        checkpoint_timestamp: DateTime<Utc>,
    ) -> Result<bool, OyaError> {
        let store = self.store.clone();
        let id = checkpoint_id.as_str().to_string();
        let record = run_async(async move { store.get_checkpoint(&id).await })
            .map_err(|e| OyaError::Internal(format!("checkpoint validation failed: {e}")))?
            .map_err(|e| OyaError::Internal(format!("checkpoint fetch failed: {e}")))?;
        Ok(record.created_at == checkpoint_timestamp)
    }
}

/// Event log adapter that exposes orchestrator events as metadata.
#[derive(Clone, Debug)]
pub struct OrchestratorEventLog {
    store: Arc<OrchestratorStore>,
}

impl OrchestratorEventLog {
    /// Create a new event log adapter.
    #[must_use]
    pub const fn new(store: Arc<OrchestratorStore>) -> Self {
        Self { store }
    }
}

#[derive(Debug, Deserialize)]
struct EventRow {
    event_id: String,
    sequence: u64,
    timestamp: SurrealDatetime,
}

impl EventLog for OrchestratorEventLog {
    fn load_events_after(&self, timestamp: DateTime<Utc>) -> Result<Vec<EventMetadata>, OyaError> {
        let surreal_timestamp = SurrealDatetime::from(timestamp);
        let store = self.store.clone();
        let mut response = run_async(async move {
            store
                .db()
                .query("SELECT event_id, sequence, timestamp FROM orchestrator_event WHERE timestamp > $timestamp ORDER BY timestamp ASC, sequence ASC")
                .bind(("timestamp", surreal_timestamp))
                .await
        })
        .map_err(|e| OyaError::Internal(format!("async execution failed: {e}")))?
        .map_err(|e| OyaError::Internal(format!("query execution failed: {e}")))?;

        let rows: Vec<EventRow> = response
            .take(0)
            .map_err(|e| OyaError::Internal(format!("failed to extract event rows: {e}")))?;

        let events = rows
            .into_iter()
            .map(|row| EventMetadata {
                event_id: row.event_id,
                timestamp: DateTime::<Utc>::from(row.timestamp),
                sequence_number: row.sequence,
            })
            .collect();

        Ok(events)
    }
}
