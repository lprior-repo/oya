//! Message delivery tracking and exactly-once semantics.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::types::{MessageId, MessageMetadata};
use crate::persistence::{OrchestratorStore, PersistenceError, PersistenceResult};

/// Type alias for the idempotency cache.
type IdempotencyCache = Arc<RwLock<HashMap<String, (MessageId, DateTime<Utc>)>>>;

/// Delivery mode for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// Message delivered at most once (may be lost).
    AtMostOnce,
    /// Message delivered at least once (may be duplicated).
    #[default]
    AtLeastOnce,
    /// Message delivered exactly once (strongest guarantee).
    ExactlyOnce,
}

/// Status of a message delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Message is pending delivery.
    Pending,
    /// Message has been sent but not acknowledged.
    Sent,
    /// Message was delivered and acknowledged.
    Delivered,
    /// Message delivery failed.
    Failed {
        /// Error description
        error: String,
    },
    /// Message was expired before delivery.
    Expired,
    /// Message was deduplicated (already delivered).
    Deduplicated,
}

impl DeliveryStatus {
    /// Check if the delivery is terminal (won't change).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Delivered | Self::Failed { .. } | Self::Expired | Self::Deduplicated
        )
    }

    /// Check if the delivery was successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Delivered | Self::Deduplicated)
    }
}

/// Record of a message delivery attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    /// Message ID
    pub message_id: MessageId,
    /// Current delivery status
    pub status: DeliveryStatus,
    /// Delivery mode
    pub mode: DeliveryMode,
    /// Metadata about the message
    pub metadata: MessageMetadata,
    /// When the message was first received
    pub received_at: DateTime<Utc>,
    /// When the message was last updated
    pub updated_at: DateTime<Utc>,
    /// Idempotency key for deduplication
    pub idempotency_key: Option<String>,
}

impl DeliveryRecord {
    /// Create a new delivery record.
    #[must_use]
    pub fn new(message_id: MessageId, mode: DeliveryMode) -> Self {
        let now = Utc::now();
        Self {
            message_id,
            status: DeliveryStatus::Pending,
            mode,
            metadata: MessageMetadata::default(),
            received_at: now,
            updated_at: now,
            idempotency_key: None,
        }
    }

    /// Set the idempotency key.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Set the metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Update the status.
    pub fn update_status(&mut self, status: DeliveryStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Record a delivery attempt.
    pub fn record_attempt(&mut self) {
        self.metadata.record_attempt();
        self.updated_at = Utc::now();
    }
}

/// Configuration for the delivery tracker.
#[derive(Debug, Clone)]
pub struct DeliveryTrackerConfig {
    /// Maximum delivery attempts before marking as failed.
    pub max_attempts: u32,
    /// Enable deduplication based on idempotency keys.
    pub enable_deduplication: bool,
    /// Time-to-live for deduplication cache (seconds).
    pub dedup_ttl_secs: u64,
}

impl Default for DeliveryTrackerConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            enable_deduplication: true,
            dedup_ttl_secs: 3600, // 1 hour
        }
    }
}

/// Tracks message delivery and ensures exactly-once semantics.
pub struct DeliveryTracker {
    config: DeliveryTrackerConfig,
    store: Option<OrchestratorStore>,
    /// In-memory cache of delivery records
    records: Arc<RwLock<HashMap<String, DeliveryRecord>>>,
    /// Idempotency key -> message ID mapping for deduplication
    idempotency_cache: IdempotencyCache,
}

impl DeliveryTracker {
    /// Create a new delivery tracker with in-memory storage.
    #[must_use]
    pub fn new(config: DeliveryTrackerConfig) -> Self {
        Self {
            config,
            store: None,
            records: Arc::new(RwLock::new(HashMap::new())),
            idempotency_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a delivery tracker with persistent storage.
    #[must_use]
    pub fn with_store(config: DeliveryTrackerConfig, store: OrchestratorStore) -> Self {
        Self {
            config,
            store: Some(store),
            records: Arc::new(RwLock::new(HashMap::new())),
            idempotency_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Track a new message for delivery.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn track(
        &self,
        message_id: MessageId,
        mode: DeliveryMode,
        idempotency_key: Option<&str>,
    ) -> PersistenceResult<TrackResult> {
        // Check for duplicate if idempotency key provided
        if let Some(key) = idempotency_key {
            if self.config.enable_deduplication {
                let cache = self.idempotency_cache.read().await;
                if let Some((existing_id, cached_at)) = cache.get(key) {
                    // Check if cache entry is still valid
                    let age = Utc::now().signed_duration_since(*cached_at).num_seconds();
                    if age < self.config.dedup_ttl_secs as i64 {
                        return Ok(TrackResult::Duplicate(existing_id.clone()));
                    }
                }
            }
        }

        // Create new delivery record
        let mut record = DeliveryRecord::new(message_id.clone(), mode);
        if let Some(key) = idempotency_key {
            record = record.with_idempotency_key(key);
        }

        // Store in memory
        {
            let mut records = self.records.write().await;
            records.insert(message_id.as_str().to_string(), record.clone());
        }

        // Update idempotency cache
        if let Some(key) = idempotency_key {
            let mut cache = self.idempotency_cache.write().await;
            cache.insert(key.to_string(), (message_id.clone(), Utc::now()));
        }

        // Persist if store available
        if let Some(store) = &self.store {
            self.persist_record(store, &record).await?;
        }

        Ok(TrackResult::Tracked(message_id))
    }

    /// Mark a message as sent.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is not found or persistence fails.
    pub async fn mark_sent(&self, message_id: &MessageId) -> PersistenceResult<()> {
        self.update_status(message_id, DeliveryStatus::Sent).await
    }

    /// Mark a message as delivered.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is not found or persistence fails.
    pub async fn mark_delivered(&self, message_id: &MessageId) -> PersistenceResult<()> {
        self.update_status(message_id, DeliveryStatus::Delivered)
            .await
    }

    /// Mark a message as failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is not found or persistence fails.
    pub async fn mark_failed(
        &self,
        message_id: &MessageId,
        error: impl Into<String>,
    ) -> PersistenceResult<()> {
        self.update_status(
            message_id,
            DeliveryStatus::Failed {
                error: error.into(),
            },
        )
        .await
    }

    /// Record a delivery attempt.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is not found or max attempts exceeded.
    pub async fn record_attempt(&self, message_id: &MessageId) -> PersistenceResult<AttemptResult> {
        let mut records = self.records.write().await;

        let record = records
            .get_mut(message_id.as_str())
            .ok_or_else(|| PersistenceError::not_found("delivery_record", message_id.as_str()))?;

        record.record_attempt();

        if record.metadata.delivery_attempts >= self.config.max_attempts {
            record.update_status(DeliveryStatus::Failed {
                error: format!(
                    "Max delivery attempts ({}) exceeded",
                    self.config.max_attempts
                ),
            });

            // Persist if store available
            if let Some(store) = &self.store {
                drop(records); // Release lock before async operation
                self.persist_record_by_id(store, message_id).await?;
            }

            return Ok(AttemptResult::MaxAttemptsExceeded);
        }

        // Persist if store available
        if let Some(store) = &self.store {
            let record_clone = record.clone();
            drop(records); // Release lock before async operation
            self.persist_record(store, &record_clone).await?;
        }

        Ok(AttemptResult::Recorded)
    }

    /// Get the delivery status of a message.
    pub async fn get_status(&self, message_id: &MessageId) -> Option<DeliveryStatus> {
        let records = self.records.read().await;
        records.get(message_id.as_str()).map(|r| r.status.clone())
    }

    /// Get a delivery record.
    pub async fn get_record(&self, message_id: &MessageId) -> Option<DeliveryRecord> {
        let records = self.records.read().await;
        records.get(message_id.as_str()).cloned()
    }

    /// Clean up expired entries from the deduplication cache.
    pub async fn cleanup_dedup_cache(&self) {
        let now = Utc::now();
        let ttl = self.config.dedup_ttl_secs as i64;

        let mut cache = self.idempotency_cache.write().await;
        cache.retain(|_, (_, cached_at)| now.signed_duration_since(*cached_at).num_seconds() < ttl);
    }

    /// Update status helper.
    async fn update_status(
        &self,
        message_id: &MessageId,
        status: DeliveryStatus,
    ) -> PersistenceResult<()> {
        let mut records = self.records.write().await;

        let record = records
            .get_mut(message_id.as_str())
            .ok_or_else(|| PersistenceError::not_found("delivery_record", message_id.as_str()))?;

        record.update_status(status);

        // Persist if store available
        if let Some(store) = &self.store {
            let record_clone = record.clone();
            drop(records); // Release lock before async operation
            self.persist_record(store, &record_clone).await?;
        }

        Ok(())
    }

    /// Persist a delivery record.
    async fn persist_record(
        &self,
        store: &OrchestratorStore,
        record: &DeliveryRecord,
    ) -> PersistenceResult<()> {
        let json = serde_json::to_string(record)?;

        let record_id = record.message_id.as_str().to_string();

        let _: Option<serde_json::Value> = store
            .db()
            .query("UPSERT type::thing('delivery_record', $id) CONTENT $data")
            .bind(("id", record_id))
            .bind(("data", json))
            .await
            .map_err(PersistenceError::from)?
            .take(0)
            .map_err(PersistenceError::from)?;

        Ok(())
    }

    /// Persist a delivery record by ID.
    async fn persist_record_by_id(
        &self,
        store: &OrchestratorStore,
        message_id: &MessageId,
    ) -> PersistenceResult<()> {
        let records = self.records.read().await;
        if let Some(record) = records.get(message_id.as_str()) {
            let record_clone = record.clone();
            drop(records);
            self.persist_record(store, &record_clone).await?;
        }
        Ok(())
    }
}

/// Result of tracking a message.
#[derive(Debug, Clone)]
pub enum TrackResult {
    /// Message was successfully tracked.
    Tracked(MessageId),
    /// Message was a duplicate (idempotency key matched).
    Duplicate(MessageId),
}

/// Result of recording a delivery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptResult {
    /// Attempt was recorded.
    Recorded,
    /// Max attempts exceeded; message marked as failed.
    MaxAttemptsExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_mode_default() {
        let mode = DeliveryMode::default();
        assert!(matches!(mode, DeliveryMode::AtLeastOnce));
    }

    #[test]
    fn test_delivery_status_terminal() {
        assert!(!DeliveryStatus::Pending.is_terminal());
        assert!(!DeliveryStatus::Sent.is_terminal());
        assert!(DeliveryStatus::Delivered.is_terminal());
        assert!(
            DeliveryStatus::Failed {
                error: "test".into()
            }
            .is_terminal()
        );
        assert!(DeliveryStatus::Expired.is_terminal());
        assert!(DeliveryStatus::Deduplicated.is_terminal());
    }

    #[test]
    fn test_delivery_status_success() {
        assert!(DeliveryStatus::Delivered.is_success());
        assert!(DeliveryStatus::Deduplicated.is_success());
        assert!(
            !DeliveryStatus::Failed {
                error: "err".into()
            }
            .is_success()
        );
    }

    #[test]
    fn test_delivery_record_creation() {
        let id = MessageId::new();
        let record = DeliveryRecord::new(id.clone(), DeliveryMode::ExactlyOnce);

        assert_eq!(record.message_id, id);
        assert!(matches!(record.status, DeliveryStatus::Pending));
        assert!(matches!(record.mode, DeliveryMode::ExactlyOnce));
    }

    #[test]
    fn test_delivery_record_with_idempotency() {
        let id = MessageId::new();
        let record =
            DeliveryRecord::new(id, DeliveryMode::AtLeastOnce).with_idempotency_key("idem-123");

        assert_eq!(record.idempotency_key, Some("idem-123".to_string()));
    }

    #[tokio::test]
    async fn test_tracker_track_message() {
        let tracker = DeliveryTracker::new(DeliveryTrackerConfig::default());
        let message_id = MessageId::new();

        let result = tracker
            .track(message_id.clone(), DeliveryMode::AtLeastOnce, None)
            .await;

        assert!(result.is_ok());
        assert!(matches!(result, Ok(TrackResult::Tracked(_))));

        let status = tracker.get_status(&message_id).await;
        assert!(matches!(status, Some(DeliveryStatus::Pending)));
    }

    #[tokio::test]
    async fn test_tracker_deduplication() {
        let tracker = DeliveryTracker::new(DeliveryTrackerConfig::default());
        let id1 = MessageId::new();
        let id2 = MessageId::new();

        // First message with idempotency key
        let result1 = tracker
            .track(id1.clone(), DeliveryMode::ExactlyOnce, Some("key-1"))
            .await;
        assert!(matches!(result1, Ok(TrackResult::Tracked(_))));

        // Second message with same idempotency key should be duplicate
        let result2 = tracker
            .track(id2, DeliveryMode::ExactlyOnce, Some("key-1"))
            .await;

        assert!(matches!(result2, Ok(TrackResult::Duplicate(_))));
    }

    #[tokio::test]
    async fn test_tracker_mark_delivered() {
        let tracker = DeliveryTracker::new(DeliveryTrackerConfig::default());
        let message_id = MessageId::new();

        let _ = tracker
            .track(message_id.clone(), DeliveryMode::AtLeastOnce, None)
            .await;
        let _ = tracker.mark_sent(&message_id).await;
        let result = tracker.mark_delivered(&message_id).await;

        assert!(result.is_ok());

        let status = tracker.get_status(&message_id).await;
        assert!(matches!(status, Some(DeliveryStatus::Delivered)));
    }

    #[tokio::test]
    async fn test_tracker_max_attempts() {
        let config = DeliveryTrackerConfig {
            max_attempts: 2,
            ..Default::default()
        };
        let tracker = DeliveryTracker::new(config);
        let message_id = MessageId::new();

        let _ = tracker
            .track(message_id.clone(), DeliveryMode::AtLeastOnce, None)
            .await;

        let attempt1 = tracker.record_attempt(&message_id).await;
        assert!(matches!(attempt1, Ok(AttemptResult::Recorded)));

        let attempt2 = tracker.record_attempt(&message_id).await;
        assert!(matches!(attempt2, Ok(AttemptResult::MaxAttemptsExceeded)));

        let status = tracker.get_status(&message_id).await;
        assert!(matches!(status, Some(DeliveryStatus::Failed { .. })));
    }
}
