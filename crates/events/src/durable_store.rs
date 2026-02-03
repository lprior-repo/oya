#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use chrono::{DateTime, Utc};
use std::sync::Arc;
use surrealdb::engine::any::Any;
use surrealdb::opt::RecordId;
use surrealdb::Surreal;

use crate::error::Result;
use crate::event::BeadEvent;
use crate::types::{BeadId, EventId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SerializedEvent {
    event_id: String,
    bead_id: String,
    event_type: String,
    data: Vec<u8>,
    timestamp: DateTime<Utc>,
}

impl SerializedEvent {
    fn from_bead_event(event: &BeadEvent) -> Result<Self> {
        let data = bincode::serialize(event).map_err(|e| {
            crate::error::Error::serialization(format!("failed to serialize event: {}", e))
        })?;

        Ok(Self {
            event_id: event.event_id().to_string(),
            bead_id: event.bead_id().to_string(),
            event_type: event.event_type().to_string(),
            data,
            timestamp: event.timestamp(),
        })
    }

    fn to_bead_event(&self) -> Result<BeadEvent> {
        bincode::deserialize(&self.data).map_err(|e| {
            crate::error::Error::serialization(format!("failed to deserialize event: {}", e))
        })
    }
}

pub struct DurableEventStore {
    db: Arc<Surreal<Any>>,
}

impl DurableEventStore {
    pub async fn new(db: Arc<Surreal<Any>>) -> Result<Self> {
        Ok(Self { db })
    }

    pub async fn append_event(&self, event: &BeadEvent) -> Result<()> {
        let serialized = SerializedEvent::from_bead_event(event)?;

        let bead_id_str = serialized.bead_id.clone();
        let timestamp = serialized.timestamp;

        self.db
            .create(("state_transition", serialized.event_id.clone()))
            .content(serialized)
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "append_event",
                    format!("failed to create record: {}", e),
                )
            })?;

        self.db
            .query("SELECT record::id FROM type::thing($table, $id)")
            .bind(("table", "state_transition"))
            .bind(("id", format!("{}", event.event_id())))
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "append_event",
                    format!("failed to verify write: {}", e),
                )
            })?;

        Ok(())
    }

    pub async fn read_events(&self, bead_id: &BeadId) -> Result<Vec<BeadEvent>> {
        let bead_id_str = bead_id.to_string();

        let mut result = self
            .db
            .query("SELECT * FROM state_transition WHERE bead_id = $bead_id ORDER BY timestamp ASC")
            .bind(("bead_id", bead_id_str))
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "read_events",
                    format!("failed to query events: {}", e),
                )
            })?;

        let serialized_events: Vec<SerializedEvent> = result.take(0).map_err(|e| {
            crate::error::Error::store_failed(
                "read_events",
                format!("failed to extract results: {}", e),
            )
        })?;

        serialized_events
            .iter()
            .map(|se| se.to_bead_event())
            .collect()
    }

    pub async fn replay_from(&self, checkpoint_id: &str) -> Result<Vec<BeadEvent>> {
        let mut result = self
            .db
            .query(
                "SELECT * FROM state_transition WHERE timestamp > (SELECT timestamp FROM state_transition WHERE event_id = $checkpoint_id LIMIT 1) ORDER BY timestamp ASC"
            )
            .bind(("checkpoint_id", checkpoint_id))
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "replay_from",
                    format!("failed to query events from checkpoint: {}", e),
                )
            })?;

        let serialized_events: Vec<SerializedEvent> = result.take(0).map_err(|e| {
            crate::error::Error::store_failed(
                "replay_from",
                format!("failed to extract results: {}", e),
            )
        })?;

        serialized_events
            .iter()
            .map(|se| se.to_bead_event())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialized_event_roundtrip() {
        let bead_id = BeadId::new();
        let event = BeadEvent::created(
            bead_id,
            crate::types::BeadSpec::new("Test").with_complexity(crate::types::Complexity::Simple),
        );

        let serialized = SerializedEvent::from_bead_event(&event);
        assert!(serialized.is_ok());

        let serialized = serialized.unwrap();
        assert_eq!(serialized.event_type, "created");

        let deserialized = serialized.to_bead_event();
        assert!(deserialized.is_ok());

        let deserialized = deserialized.unwrap();
        assert_eq!(deserialized.event_id(), event.event_id());
        assert_eq!(deserialized.bead_id(), event.bead_id());
        assert_eq!(deserialized.event_type(), "created");
    }

    #[test]
    fn test_serialized_event_all_types() {
        let bead_id = BeadId::new();

        let events = [
            BeadEvent::created(
                bead_id,
                crate::types::BeadSpec::new("Test")
                    .with_complexity(crate::types::Complexity::Simple),
            ),
            BeadEvent::state_changed(
                bead_id,
                crate::types::BeadState::Pending,
                crate::types::BeadState::Scheduled,
            ),
            BeadEvent::failed(bead_id, "test error"),
            BeadEvent::completed(
                bead_id,
                crate::types::BeadResult::success(vec![1, 2, 3], 1000),
            ),
        ];

        for event in events {
            let serialized = SerializedEvent::from_bead_event(&event);
            assert!(
                serialized.is_ok(),
                "failed to serialize event type: {}",
                event.event_type()
            );

            let serialized = serialized.unwrap();
            let deserialized = serialized.to_bead_event();
            assert!(
                deserialized.is_ok(),
                "failed to deserialize event type: {}",
                event.event_type()
            );

            let deserialized = deserialized.unwrap();
            assert_eq!(deserialized.event_id(), event.event_id());
            assert_eq!(deserialized.bead_id(), event.bead_id());
            assert_eq!(deserialized.event_type(), event.event_type());
        }
    }

    #[test]
    fn test_serialized_event_with_complex_data() {
        let bead_id = BeadId::new();
        let phase_id = crate::types::PhaseId::new();

        let event = BeadEvent::phase_completed(
            bead_id,
            phase_id,
            "test_phase",
            crate::types::PhaseOutput::success(vec![1, 2, 3, 4, 5]),
        );

        let serialized = SerializedEvent::from_bead_event(&event);
        assert!(serialized.is_ok());

        let serialized = serialized.unwrap();
        assert_eq!(serialized.event_type, "phase_completed");

        let deserialized = serialized.to_bead_event();
        assert!(deserialized.is_ok());

        let deserialized = deserialized.unwrap();
        assert_eq!(deserialized.event_id(), event.event_id());
        assert_eq!(deserialized.bead_id(), event.bead_id());
        assert_eq!(deserialized.event_type(), "phase_completed");
    }
}
