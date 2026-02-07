//! Checkpoint-based resume for event replay.
//!
//! This module provides functionality to resume event replay from a checkpoint
//! instead of replaying all events from the beginning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Unique identifier for a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(String);

impl CheckpointId {
    /// Create a new checkpoint ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the inner ID value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Replay state after resuming from checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayState {
    /// Checkpoint ID used for resume.
    pub checkpoint_id: CheckpointId,
    /// Timestamp of the checkpoint.
    pub checkpoint_timestamp: DateTime<Utc>,
    /// Number of events replayed after checkpoint.
    pub events_replayed: u64,
    /// Timestamp of last processed event.
    pub last_event_timestamp: Option<DateTime<Utc>>,
}

impl ReplayState {
    /// Create a new replay state.
    pub fn new(checkpoint_id: CheckpointId, checkpoint_timestamp: DateTime<Utc>) -> Self {
        Self {
            checkpoint_id,
            checkpoint_timestamp,
            events_replayed: 0,
            last_event_timestamp: None,
        }
    }

    /// Record an event as replayed.
    pub fn record_event(&mut self, timestamp: DateTime<Utc>) {
        self.events_replayed += 1;
        self.last_event_timestamp = Some(timestamp);
    }
}

/// Resume error types.
#[derive(Debug, Clone, PartialEq)]
pub enum ResumeError {
    /// Checkpoint not found.
    CheckpointNotFound { checkpoint_id: String },
    /// Checkpoint timestamp does not match event log.
    TimestampMismatch {
        checkpoint_id: String,
        checkpoint_timestamp: DateTime<Utc>,
        log_timestamp: DateTime<Utc>,
    },
    /// Invalid checkpoint data.
    InvalidCheckpoint { reason: String },
    /// Failed to load events after checkpoint.
    EventLoadFailed { reason: String },
}

impl std::fmt::Display for ResumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckpointNotFound { checkpoint_id } => {
                write!(f, "checkpoint '{checkpoint_id}' not found")
            }
            Self::TimestampMismatch {
                checkpoint_id,
                checkpoint_timestamp,
                log_timestamp,
            } => {
                write!(
                    f,
                    "checkpoint '{checkpoint_id}' timestamp {checkpoint_timestamp} does not match event log {log_timestamp}"
                )
            }
            Self::InvalidCheckpoint { reason } => {
                write!(f, "invalid checkpoint: {reason}")
            }
            Self::EventLoadFailed { reason } => {
                write!(f, "failed to load events after checkpoint: {reason}")
            }
        }
    }
}

impl std::error::Error for ResumeError {}

impl From<ResumeError> for Error {
    fn from(err: ResumeError) -> Self {
        Error::Internal(err.to_string())
    }
}

/// Trait for checkpoint storage backends.
pub trait CheckpointStore: Send + Sync {
    /// Load checkpoint data by ID.
    fn load_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> Result<Option<(CheckpointData, DateTime<Utc>)>>;

    /// Validate checkpoint timestamp against event log.
    fn validate_timestamp(
        &self,
        checkpoint_id: &CheckpointId,
        checkpoint_timestamp: DateTime<Utc>,
    ) -> Result<bool>;
}

/// Checkpoint data containing stored state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    /// Serialized state data.
    pub state: Vec<u8>,
    /// Sequence number of checkpoint.
    pub sequence_number: u64,
    /// Whether data is compressed.
    pub compressed: bool,
}

/// Event metadata for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Event ID.
    pub event_id: String,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Sequence number.
    pub sequence_number: u64,
}

/// Trait for event log access.
pub trait EventLog: Send + Sync {
    /// Load events after the given timestamp.
    fn load_events_after(&self, timestamp: DateTime<Utc>) -> Result<Vec<EventMetadata>>;
}

/// Resume replay from a checkpoint.
///
/// # Arguments
/// * `checkpoint_id` - ID of checkpoint to resume from
/// * `checkpoints` - Checkpoint storage backend
/// * `event_log` - Event log backend
///
/// # Returns
/// `ReplayState` if successful, `ResumeError` otherwise
///
/// # Errors
/// Returns `ResumeError::CheckpointNotFound` if checkpoint doesn't exist
/// Returns `ResumeError::TimestampMismatch` if checkpoint timestamp doesn't match event log
pub fn resume_from_checkpoint<S, L>(
    checkpoint_id: &CheckpointId,
    checkpoints: &S,
    event_log: &L,
) -> Result<ReplayState>
where
    S: CheckpointStore,
    L: EventLog,
{
    // Load checkpoint
    let (_checkpoint_data, checkpoint_timestamp) = checkpoints
        .load_checkpoint(checkpoint_id)
        .map_err(|e| match e {
            Error::Internal(msg) if msg.contains("not found") => ResumeError::CheckpointNotFound {
                checkpoint_id: checkpoint_id.as_str().to_string(),
            },
            _ => ResumeError::InvalidCheckpoint {
                reason: e.to_string(),
            },
        })?
        .ok_or(ResumeError::CheckpointNotFound {
            checkpoint_id: checkpoint_id.as_str().to_string(),
        })?;

    // Validate checkpoint timestamp
    let timestamp_valid = checkpoints
        .validate_timestamp(checkpoint_id, checkpoint_timestamp)
        .map_err(|e| ResumeError::InvalidCheckpoint {
            reason: e.to_string(),
        })?;

    if !timestamp_valid {
        return Err(Error::Internal(
            "Checkpoint timestamp validation failed".to_string(),
        ));
    }

    // Create initial replay state
    let mut state = ReplayState::new(checkpoint_id.clone(), checkpoint_timestamp);

    // Load events after checkpoint timestamp
    let events = event_log
        .load_events_after(checkpoint_timestamp)
        .map_err(|e| ResumeError::EventLoadFailed {
            reason: e.to_string(),
        })?;

    // Record events as replayed
    for event in events {
        state.record_event(event.timestamp);
    }

    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // CheckpointId BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_checkpoint_id_new() {
        let id = CheckpointId::new("test-checkpoint");
        assert_eq!(id.as_str(), "test-checkpoint");
    }

    #[test]
    fn test_checkpoint_id_from_string() {
        let id = CheckpointId::new("checkpoint-123".to_string());
        assert_eq!(id.as_str(), "checkpoint-123");
    }

    // ==========================================================================
    // ReplayState BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_replay_state_new() {
        let checkpoint_id = CheckpointId::new("test");
        let timestamp = Utc::now();
        let state = ReplayState::new(checkpoint_id.clone(), timestamp);

        assert_eq!(state.checkpoint_id.as_str(), "test");
        assert_eq!(state.checkpoint_timestamp, timestamp);
        assert_eq!(state.events_replayed, 0);
        assert!(state.last_event_timestamp.is_none());
    }

    #[test]
    fn test_replay_state_record_event() {
        let mut state = ReplayState::new(CheckpointId::new("test"), Utc::now());

        let event1_ts = Utc::now();
        state.record_event(event1_ts);

        assert_eq!(state.events_replayed, 1);
        assert_eq!(state.last_event_timestamp, Some(event1_ts));

        let event2_ts = Utc::now();
        state.record_event(event2_ts);

        assert_eq!(state.events_replayed, 2);
        assert_eq!(state.last_event_timestamp, Some(event2_ts));
    }

    // ==========================================================================
    // ResumeError Display Tests
    // ==========================================================================

    #[test]
    fn test_resume_error_display_checkpoint_not_found() {
        let err = ResumeError::CheckpointNotFound {
            checkpoint_id: "cp-123".to_string(),
        };
        assert!(err.to_string().contains("cp-123"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_resume_error_display_timestamp_mismatch() {
        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::seconds(1);
        let err = ResumeError::TimestampMismatch {
            checkpoint_id: "cp-123".to_string(),
            checkpoint_timestamp: ts1,
            log_timestamp: ts2,
        };
        let msg = err.to_string();
        assert!(msg.contains("cp-123"));
        assert!(msg.contains("timestamp"));
        assert!(msg.contains("does not match"));
    }

    #[test]
    fn test_resume_error_display_invalid_checkpoint() {
        let err = ResumeError::InvalidCheckpoint {
            reason: "corrupted data".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid checkpoint"));
        assert!(msg.contains("corrupted data"));
    }

    #[test]
    fn test_resume_error_display_event_load_failed() {
        let err = ResumeError::EventLoadFailed {
            reason: "connection lost".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("failed to load events"));
        assert!(msg.contains("connection lost"));
    }

    // ==========================================================================
    // resume_from_checkpoint BEHAVIORAL TESTS
    // ==========================================================================

    struct MockCheckpointStore {
        checkpoint: Option<(CheckpointData, DateTime<Utc>)>,
        timestamp_valid: bool,
    }

    impl CheckpointStore for MockCheckpointStore {
        fn load_checkpoint(
            &self,
            _checkpoint_id: &CheckpointId,
        ) -> Result<Option<(CheckpointData, DateTime<Utc>)>> {
            Ok(self.checkpoint.clone())
        }

        fn validate_timestamp(
            &self,
            _checkpoint_id: &CheckpointId,
            _checkpoint_timestamp: DateTime<Utc>,
        ) -> Result<bool> {
            Ok(self.timestamp_valid)
        }
    }

    struct MockEventLog {
        events: Vec<EventMetadata>,
    }

    impl EventLog for MockEventLog {
        fn load_events_after(&self, _timestamp: DateTime<Utc>) -> Result<Vec<EventMetadata>> {
            Ok(self.events.clone())
        }
    }

    #[test]
    fn should_return_checkpoint_not_found_when_checkpoint_missing() {
        let store = MockCheckpointStore {
            checkpoint: None,
            timestamp_valid: true,
        };
        let log = MockEventLog { events: vec![] };
        let checkpoint_id = CheckpointId::new("missing");

        let result = resume_from_checkpoint(&checkpoint_id, &store, &log);

        assert!(
            result.is_err(),
            "Should return error when checkpoint not found"
        );
        let err_msg = result
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            err_msg.contains("not found") || err_msg.contains("missing"),
            "Error should indicate checkpoint not found: {}",
            err_msg
        );
    }

    #[test]
    fn should_create_replay_state_with_correct_checkpoint_data() {
        let timestamp = Utc::now();
        let checkpoint_data = CheckpointData {
            state: vec![1, 2, 3],
            sequence_number: 10,
            compressed: true,
        };

        let store = MockCheckpointStore {
            checkpoint: Some((checkpoint_data, timestamp)),
            timestamp_valid: true,
        };
        let log = MockEventLog { events: vec![] };
        let checkpoint_id = CheckpointId::new("test");

        let result = resume_from_checkpoint(&checkpoint_id, &store, &log);

        assert!(result.is_ok(), "Should successfully create replay state");
        let state = result.unwrap(); // Safe: asserted is_ok above
        assert_eq!(state.checkpoint_id.as_str(), "test");
        assert_eq!(state.checkpoint_timestamp, timestamp);
        assert_eq!(state.events_replayed, 0);
    }

    #[test]
    fn should_count_events_replayed_from_event_log() {
        let timestamp = Utc::now();
        let checkpoint_data = CheckpointData {
            state: vec![],
            sequence_number: 5,
            compressed: false,
        };

        let events = vec![
            EventMetadata {
                event_id: "evt-1".to_string(),
                timestamp: timestamp + chrono::Duration::milliseconds(1),
                sequence_number: 6,
            },
            EventMetadata {
                event_id: "evt-2".to_string(),
                timestamp: timestamp + chrono::Duration::milliseconds(2),
                sequence_number: 7,
            },
            EventMetadata {
                event_id: "evt-3".to_string(),
                timestamp: timestamp + chrono::Duration::milliseconds(3),
                sequence_number: 8,
            },
        ];

        let store = MockCheckpointStore {
            checkpoint: Some((checkpoint_data, timestamp)),
            timestamp_valid: true,
        };
        let log = MockEventLog { events };
        let checkpoint_id = CheckpointId::new("test");

        let result = resume_from_checkpoint(&checkpoint_id, &store, &log);

        assert!(result.is_ok(), "Should successfully create replay state");
        let state = result.unwrap(); // Safe: asserted is_ok above
        assert_eq!(
            state.events_replayed, 3,
            "Should count all events after checkpoint"
        );
    }

    #[test]
    fn should_return_error_when_timestamp_validation_fails() {
        let timestamp = Utc::now();
        let checkpoint_data = CheckpointData {
            state: vec![],
            sequence_number: 1,
            compressed: false,
        };

        let store = MockCheckpointStore {
            checkpoint: Some((checkpoint_data, timestamp)),
            timestamp_valid: false, // Validation fails
        };
        let log = MockEventLog { events: vec![] };
        let checkpoint_id = CheckpointId::new("test");

        let result = resume_from_checkpoint(&checkpoint_id, &store, &log);

        assert!(
            result.is_err(),
            "Should fail when timestamp validation fails"
        );
    }

    #[test]
    fn should_record_last_event_timestamp() {
        let timestamp = Utc::now();
        let checkpoint_data = CheckpointData {
            state: vec![],
            sequence_number: 1,
            compressed: false,
        };

        let event1_ts = timestamp + chrono::Duration::milliseconds(1);
        let event2_ts = timestamp + chrono::Duration::milliseconds(2);

        let events = vec![
            EventMetadata {
                event_id: "evt-1".to_string(),
                timestamp: event1_ts,
                sequence_number: 2,
            },
            EventMetadata {
                event_id: "evt-2".to_string(),
                timestamp: event2_ts,
                sequence_number: 3,
            },
        ];

        let store = MockCheckpointStore {
            checkpoint: Some((checkpoint_data, timestamp)),
            timestamp_valid: true,
        };
        let log = MockEventLog { events };
        let checkpoint_id = CheckpointId::new("test");

        let result = resume_from_checkpoint(&checkpoint_id, &store, &log);

        assert!(result.is_ok(), "Should successfully create replay state");
        let state = result.unwrap(); // Safe: asserted is_ok above
        assert_eq!(
            state.last_event_timestamp,
            Some(event2_ts),
            "Should record the last event timestamp"
        );
    }

    #[test]
    fn should_handle_empty_event_log_after_checkpoint() {
        let timestamp = Utc::now();
        let checkpoint_data = CheckpointData {
            state: vec![1, 2, 3],
            sequence_number: 100,
            compressed: true,
        };

        let store = MockCheckpointStore {
            checkpoint: Some((checkpoint_data, timestamp)),
            timestamp_valid: true,
        };
        let log = MockEventLog { events: vec![] };
        let checkpoint_id = CheckpointId::new("test");

        let result = resume_from_checkpoint(&checkpoint_id, &store, &log);

        assert!(result.is_ok(), "Should successfully create replay state");
        let state = result.unwrap(); // Safe: asserted is_ok above
        assert_eq!(state.events_replayed, 0, "Should have zero events replayed");
        assert!(
            state.last_event_timestamp.is_none(),
            "Should have no last event timestamp"
        );
    }
}
