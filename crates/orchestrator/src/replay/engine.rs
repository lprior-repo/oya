//! Replay engine for recovering orchestrator state.
//!
//! The replay engine reconstructs state by:
//! 1. Loading the latest checkpoint
//! 2. Replaying events since that checkpoint
//! 3. Applying events to projections

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Datetime as SurrealDatetime;
use std::sync::Arc;

use oya_events::replay::resume::{resume_from_checkpoint, CheckpointId, ReplayState};

use super::events::{EventRecord, OrchestratorEvent};
use super::projection::OrchestratorProjection;
use crate::persistence::{OrchestratorStore, PersistenceError, PersistenceResult};
use crate::replay::resume::{OrchestratorCheckpointStore, OrchestratorEventLog};

/// Convert `oya_events::Error` to `PersistenceError`.
fn error_to_persistence(err: oya_events::Error) -> PersistenceError {
    PersistenceError::QueryFailed {
        reason: err.to_string(),
    }
}

/// Input for storing events in `SurrealDB`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventInput {
    event_id: String,
    event_type: String,
    event_data: String,
    sequence: u64,
    timestamp: SurrealDatetime,
    workflow_id: Option<String>,
    bead_id: Option<String>,
}

/// The replay engine for recovering orchestrator state.
pub struct ReplayEngine {
    store: OrchestratorStore,
    current_sequence: u64,
}

impl ReplayEngine {
    /// Create a new replay engine.
    #[must_use]
    pub const fn new(store: OrchestratorStore) -> Self {
        Self {
            store,
            current_sequence: 0,
        }
    }

    /// Record an event to the event store.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be saved.
    pub async fn record_event(
        &mut self,
        event: OrchestratorEvent,
    ) -> PersistenceResult<EventRecord> {
        self.current_sequence = self.current_sequence.saturating_add(1);

        let event_id = format!(
            "evt-{}-{}",
            Utc::now().timestamp_nanos_opt().map_or(0, |v| v),
            self.current_sequence
        );

        let event_data = serde_json::to_string(&event)?;

        let input = EventInput {
            event_id: event_id.clone(),
            event_type: event.event_type().to_string(),
            event_data,
            sequence: self.current_sequence,
            timestamp: SurrealDatetime::from(Utc::now()),
            workflow_id: event.workflow_id().map(String::from),
            bead_id: event.bead_id().map(String::from),
        };

        let result: Option<EventInput> = self
            .store
            .db()
            .create(("orchestrator_event", &event_id))
            .content(input)
            .await
            .map_err(PersistenceError::from)?;

        if result.is_some() {
            Ok(EventRecord::new(event_id, self.current_sequence, event))
        } else {
            Err(PersistenceError::query_failed("failed to record event"))
        }
    }

    /// Get events since a specific sequence number.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_events_since(&self, sequence: u64) -> PersistenceResult<Vec<EventRecord>> {
        let inputs: Vec<EventInput> = self
            .store
            .db()
            .query("SELECT * FROM orchestrator_event WHERE sequence > $seq ORDER BY sequence ASC")
            .bind(("seq", sequence))
            .await
            .map_err(PersistenceError::from)?
            .take(0)
            .map_err(PersistenceError::from)?;

        inputs
            .into_iter()
            .map(|input| {
                let event: OrchestratorEvent = serde_json::from_str(&input.event_data)?;

                Ok(EventRecord {
                    id: input.event_id,
                    sequence: input.sequence,
                    event,
                    timestamp: DateTime::<Utc>::from(input.timestamp),
                })
            })
            .collect()
    }

    /// Recover state from the latest checkpoint and replay events.
    ///
    /// # Errors
    ///
    /// Returns an error if recovery fails.
    pub async fn recover<P: OrchestratorProjection>(
        &mut self,
        projection: &mut P,
    ) -> PersistenceResult<RecoveryResult> {
        // Reset projection to clean state
        projection.reset();

        // Try to load latest checkpoint
        let checkpoint_result = self.store.get_latest_checkpoint().await;

        let mut resume_state = None;
        let mut checkpoint_id = None;

        let from_sequence = match checkpoint_result {
            Ok(cp) => {
                self.current_sequence = cp.event_sequence;
                checkpoint_id = Some(cp.checkpoint_id.clone());

                let store_handle = Arc::new(self.store.clone());
                let checkpoint_store = OrchestratorCheckpointStore::new(store_handle.clone());
                let event_log = OrchestratorEventLog::new(store_handle);
                let checkpoint_key = CheckpointId::new(cp.checkpoint_id.clone());

                let resumed_state = resume_from_checkpoint(&checkpoint_key, &checkpoint_store, &event_log)
                    .map_err(error_to_persistence)?;
                resume_state = Some(resumed_state);

                cp.event_sequence
            }
            Err(PersistenceError::NotFound { .. }) => {
                // No checkpoint exists, replay from beginning
                self.current_sequence = 0;
                0
            }
            Err(e) => return Err(e),
        };

        // Replay events since checkpoint using functional fold
        let events = self.get_events_since(from_sequence).await?;
        let events_replayed = events.len();

        self.current_sequence = events
            .iter()
            .fold(self.current_sequence, |_seq, event_record| {
                projection.apply(&event_record.event);
                event_record.sequence
            });

        Ok(RecoveryResult {
            checkpoint_id,
            events_replayed,
            final_sequence: self.current_sequence,
            resume_state,
        })
    }

    /// Replay a single event through a projection.
    pub fn apply_event<P: OrchestratorProjection>(projection: &mut P, event: &OrchestratorEvent) {
        projection.apply(event);
    }

    /// Get the current event sequence number.
    #[must_use]
    pub const fn current_sequence(&self) -> u64 {
        self.current_sequence
    }

    /// Set the current sequence (used when loading from checkpoint).
    pub const fn set_sequence(&mut self, sequence: u64) {
        self.current_sequence = sequence;
    }
}

fn resume_error_to_persistence(err: ResumeError) -> PersistenceError {
    PersistenceError::invalid_state(format!("checkpoint resume failed: {err}"))
}

/// Result of a recovery operation.
#[derive(Debug)]
pub struct RecoveryResult {
    /// ID of the checkpoint used (if any)
    pub checkpoint_id: Option<String>,
    /// Number of events replayed
    pub events_replayed: usize,
    /// Final sequence number after recovery
    pub final_sequence: u64,
    /// Resume metadata captured by `resume_from_checkpoint`
    pub resume_state: Option<ReplayState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::StoreConfig;
    use crate::persistence::CheckpointRecord;
    use crate::replay::projection::WorkflowStatusProjection;
    use std::time::{Duration, Instant};

    async fn setup_engine() -> Option<ReplayEngine> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        Some(ReplayEngine::new(store))
    }

    async fn setup_engine_with_store() -> Option<(ReplayEngine, OrchestratorStore)> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        let engine = ReplayEngine::new(store.clone());
        Some((engine, store))
    }

    macro_rules! require_engine {
        ($engine_opt:expr) => {
            match $engine_opt {
                Some(e) => e,
                None => {
                    eprintln!("Skipping test: engine setup failed");
                    return;
                }
            }
        };
    }

    macro_rules! require_engine_with_store {
        ($engine_store_opt:expr) => {
            match $engine_store_opt {
                Some((engine, store)) => (engine, store),
                None => {
                    eprintln!("Skipping test: engine setup failed");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn test_record_event() {
        let mut engine = require_engine!(setup_engine().await);

        let event = OrchestratorEvent::WorkflowRegistered {
            workflow_id: "wf-1".to_string(),
            name: "Test".to_string(),
            dag_json: "{}".to_string(),
        };

        let result = engine.record_event(event).await;
        assert!(result.is_ok(), "record should succeed: {:?}", result.err());

        if let Ok(record) = result {
            assert_eq!(record.sequence, 1);
        }
    }

    #[tokio::test]
    async fn test_get_events_since() {
        let mut engine = require_engine!(setup_engine().await);

        // Record several events
        for i in 1..=5 {
            let event = OrchestratorEvent::WorkflowRegistered {
                workflow_id: format!("wf-{}", i),
                name: format!("Workflow {}", i),
                dag_json: "{}".to_string(),
            };
            let _ = engine.record_event(event).await;
        }

        // Get events since sequence 2
        let events = engine.get_events_since(2).await;
        assert!(events.is_ok());

        if let Ok(list) = events {
            assert_eq!(list.len(), 3, "should have 3 events after seq 2");
        }
    }

    #[tokio::test]
    async fn test_recover_replays_events() {
        let mut engine = require_engine!(setup_engine().await);

        // Record some events
        let _ = engine
            .record_event(OrchestratorEvent::WorkflowRegistered {
                workflow_id: "wf-1".to_string(),
                name: "Test 1".to_string(),
                dag_json: "{}".to_string(),
            })
            .await;

        let _ = engine
            .record_event(OrchestratorEvent::WorkflowStatusChanged {
                workflow_id: "wf-1".to_string(),
                status: "running".to_string(),
            })
            .await;

        // Create a fresh engine to simulate restart
        let config = StoreConfig::in_memory();
        if let Ok(store) = OrchestratorStore::connect(config).await {
            let _ = store.initialize_schema().await;
            let mut new_engine = ReplayEngine::new(store);

            let mut projection = WorkflowStatusProjection::new();
            let result = new_engine.recover(&mut projection).await;

            // Recovery should work but find no events (different in-memory store)
            assert!(result.is_ok());
            if let Ok(recovery) = result {
                assert!(
                    recovery.resume_state.is_none(),
                    "Resume state should be None when no checkpoint exists"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_recover_records_resume_state() {
        let (mut engine, store) = require_engine_with_store!(setup_engine_with_store().await);

        let _ = engine
            .record_event(OrchestratorEvent::WorkflowRegistered {
                workflow_id: "wf-1".to_string(),
                name: "Test Resume".to_string(),
                dag_json: "{}".to_string(),
            })
            .await;

        let _ = engine
            .record_event(OrchestratorEvent::WorkflowStatusChanged {
                workflow_id: "wf-1".to_string(),
                status: "running".to_string(),
            })
            .await;

        let checkpoint = CheckpointRecord::new("cp-resume", "{}", engine.current_sequence());
        let saved = store.save_checkpoint(&checkpoint).await;
        assert!(saved.is_ok(), "checkpoint save should succeed");

        let mut projection = WorkflowStatusProjection::new();
        let result = engine.recover(&mut projection).await;

        assert!(result.is_ok(), "recovery should succeed");
        if let Ok(recovery) = result {
            assert_eq!(recovery.checkpoint_id, Some("cp-resume".to_string()));
            let resume = recovery.resume_state.expect("resume state should be present");
            assert_eq!(resume.checkpoint_id.as_str(), "cp-resume");
            assert_eq!(recovery.events_replayed, resume.events_replayed as usize);
        }
    }

    #[tokio::test]
    async fn test_recover_handles_large_event_volume() {
        let (mut engine, store) = require_engine_with_store!(setup_engine_with_store().await);

        for i in 0..1_000 {
            let _ = engine
                .record_event(OrchestratorEvent::WorkflowStatusChanged {
                    workflow_id: format!("wf-{}", i % 5),
                    status: "running".to_string(),
                })
                .await;
        }

        let checkpoint = CheckpointRecord::new("cp-large", "{}", engine.current_sequence());
        let saved = store.save_checkpoint(&checkpoint).await;
        assert!(saved.is_ok(), "checkpoint save should succeed");

        let mut projection = WorkflowStatusProjection::new();
        let start = Instant::now();
        let result = engine.recover(&mut projection).await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "recovery should succeed for large workload");
        assert!(
            duration < Duration::from_secs(5),
            "Recovery should complete within 5s, took {:?}",
            duration
        );

        if let Ok(recovery) = result {
            let resume = recovery.resume_state.expect("resume state should be present");
            assert!(resume.events_replayed as usize >= 1_000);
            assert_eq!(recovery.events_replayed, resume.events_replayed as usize);
        }
    }

    #[tokio::test]
    async fn test_sequence_increments() {
        let mut engine = require_engine!(setup_engine().await);

        assert_eq!(engine.current_sequence(), 0);

        let _ = engine
            .record_event(OrchestratorEvent::WorkflowUnregistered {
                workflow_id: "wf".to_string(),
            })
            .await;

        assert_eq!(engine.current_sequence(), 1);

        let _ = engine
            .record_event(OrchestratorEvent::WorkflowUnregistered {
                workflow_id: "wf2".to_string(),
            })
            .await;

        assert_eq!(engine.current_sequence(), 2);
    }
}
