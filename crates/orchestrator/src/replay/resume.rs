//! Bridges orchestrator persistence with the checkpoint resume helpers.
//!
//! This module adapts `OrchestratorStore` to the `CheckpointStore` and `EventLog`
//! traits provided by the `oya_events::replay::resume` module so that the replay
//! engine can validate checkpoints and inspect the event log before applying events.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use surrealdb::sql::Datetime as SurrealDatetime;
use tokio::runtime::Runtime;

use oya_events::replay::resume::{
    CheckpointData, CheckpointId, CheckpointStore, EventLog, EventMetadata, ReplayState,
};
use oya_events::Error as OyaError;

use crate::persistence::{OrchestratorStore, PersistenceError};

/// Checkpoint store backed by the orchestrator persistence layer.
#[derive(Clone, Debug)]
pub struct OrchestratorCheckpointStore {
    store: Arc<OrchestratorStore>,
}

impl OrchestratorCheckpointStore {
    /// Create a new checkpoint bridge.
    #[must_use]
    pub fn new(store: Arc<OrchestratorStore>) -> Self {
        Self { store }
    }

    fn new_runtime() -> Result<Runtime, OyaError> {
        Runtime::new()
            .map_err(|err| OyaError::Internal(format!("tokio runtime creation failed: {err}")))
    }
}

impl CheckpointStore for OrchestratorCheckpointStore {
    fn load_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> Result<Option<(CheckpointData, DateTime<Utc>)>, OyaError> {
        let runtime = Self::new_runtime()?;
        runtime
            .block_on(self.store.get_checkpoint(checkpoint_id.as_str()))
            .map_err(|e| OyaError::Internal(format!("checkpoint load failed: {e}")))
            .map(|record| {
                let checkpoint_data = CheckpointData {
                    state: record.scheduler_state.into_bytes(),
                    sequence_number: record.event_sequence,
                    compressed: false,
                };
                Some((checkpoint_data, record.created_at))
            })
    }

    fn validate_timestamp(
        &self,
        checkpoint_id: &CheckpointId,
        checkpoint_timestamp: DateTime<Utc>,
    ) -> Result<bool, OyaError> {
        let runtime = Self::new_runtime()?;
        runtime
            .block_on(self.store.get_checkpoint(checkpoint_id.as_str()))
            .map_err(|e| OyaError::Internal(format!("checkpoint validation failed: {e}")))
            .map(|record| record.created_at == checkpoint_timestamp)
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
    pub fn new(store: Arc<OrchestratorStore>) -> Self {
        Self { store }
    }

    fn new_runtime() -> Result<Runtime, ResumeError> {
        Runtime::new().map_err(|err| ResumeError::EventLoadFailed {
            reason: err.to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct EventRow {
    event_id: String,
    sequence: u64,
    timestamp: SurrealDatetime,
}

impl EventLog for OrchestratorEventLog {
    fn load_events_after(
        &self,
        timestamp: DateTime<Utc>,
    ) -> Result<Vec<EventMetadata>, ResumeError> {
        let runtime = Self::new_runtime()?;
        let response = runtime.block_on(
            self.store
                .db()
                .query("SELECT event_id, sequence, timestamp FROM orchestrator_event WHERE timestamp > $timestamp ORDER BY timestamp ASC, sequence ASC")
                .bind(("timestamp", SurrealDatetime::from(timestamp)))
                .await,
        );
        let rows: Vec<EventRow> = match response {
            Ok(rows) => rows.take(0).map_err(|err| ResumeError::EventLoadFailed {
                reason: err.to_string(),
            })?,
            Err(err) => {
                return Err(ResumeError::EventLoadFailed {
                    reason: err.to_string(),
                })
            }
        };

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
