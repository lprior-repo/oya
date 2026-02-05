//! Durable message channel implementation.

use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::delivery::{DeliveryMode, DeliveryTracker};
use super::types::{ChannelId, Message, MessageId, MessageMetadata};
use crate::persistence::{OrchestratorStore, PersistenceError, PersistenceResult};

/// Configuration for a durable channel.
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// Maximum queue depth (0 = unlimited).
    pub max_queue_depth: usize,
    /// Default delivery mode for messages.
    pub default_delivery_mode: DeliveryMode,
    /// Whether to persist messages.
    pub persist_messages: bool,
    /// Time-to-live for messages (seconds, 0 = unlimited).
    pub message_ttl_secs: u64,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            max_queue_depth: 10_000,
            default_delivery_mode: DeliveryMode::AtLeastOnce,
            persist_messages: true,
            message_ttl_secs: 0, // No expiry by default
        }
    }
}

/// A queued message with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedMessage {
    message: Message,
    metadata: MessageMetadata,
    queued_at: DateTime<Utc>,
    delivery_mode: DeliveryMode,
}

/// Persistent channel record for SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelRecord {
    channel_id: String,
    sender_workflow: Option<String>,
    receiver_workflow: Option<String>,
    created_at: DateTime<Utc>,
    message_count: u64,
}

/// A durable message channel between workflows.
///
/// Channels provide reliable message delivery with configurable
/// semantics (at-most-once, at-least-once, exactly-once).
pub struct DurableChannel {
    id: ChannelId,
    config: ChannelConfig,
    store: Option<OrchestratorStore>,
    delivery_tracker: Option<Arc<DeliveryTracker>>,

    /// Sender workflow ID
    sender_workflow: Option<String>,
    /// Receiver workflow ID
    receiver_workflow: Option<String>,

    /// In-memory message queue
    queue: Arc<RwLock<VecDeque<QueuedMessage>>>,
    /// Message count
    message_count: Arc<RwLock<u64>>,
}

impl DurableChannel {
    /// Create a new in-memory durable channel.
    #[must_use]
    pub fn new(id: impl Into<ChannelId>, config: ChannelConfig) -> Self {
        Self {
            id: id.into(),
            config,
            store: None,
            delivery_tracker: None,
            sender_workflow: None,
            receiver_workflow: None,
            queue: Arc::new(RwLock::new(VecDeque::new())),
            message_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Create a channel with persistent storage.
    #[must_use]
    pub fn with_store(
        id: impl Into<ChannelId>,
        config: ChannelConfig,
        store: OrchestratorStore,
        delivery_tracker: Arc<DeliveryTracker>,
    ) -> Self {
        Self {
            id: id.into(),
            config,
            store: Some(store),
            delivery_tracker: Some(delivery_tracker),
            sender_workflow: None,
            receiver_workflow: None,
            queue: Arc::new(RwLock::new(VecDeque::new())),
            message_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Set the sender workflow.
    #[must_use]
    pub fn with_sender(mut self, workflow_id: impl Into<String>) -> Self {
        self.sender_workflow = Some(workflow_id.into());
        self
    }

    /// Set the receiver workflow.
    #[must_use]
    pub fn with_receiver(mut self, workflow_id: impl Into<String>) -> Self {
        self.receiver_workflow = Some(workflow_id.into());
        self
    }

    /// Get the channel ID.
    #[must_use]
    pub fn id(&self) -> &ChannelId {
        &self.id
    }

    /// Get the current queue depth.
    pub async fn queue_depth(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Get the total message count.
    pub async fn message_count(&self) -> u64 {
        *self.message_count.read().await
    }

    /// Send a message through the channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Queue is full
    /// - Persistence fails
    pub async fn send(&self, message: Message) -> PersistenceResult<MessageId> {
        self.send_with_mode(message, self.config.default_delivery_mode)
            .await
    }

    /// Send a message with a specific delivery mode.
    ///
    /// # Errors
    ///
    /// Returns an error if queue is full or persistence fails.
    pub async fn send_with_mode(
        &self,
        message: Message,
        delivery_mode: DeliveryMode,
    ) -> PersistenceResult<MessageId> {
        let message_id = message.id().clone();

        // Build metadata
        let metadata = MessageMetadata::default()
            .with_source(self.sender_workflow.as_deref().unwrap_or("unknown"), None)
            .with_target(self.receiver_workflow.as_deref().unwrap_or("unknown"), None);

        // Track delivery if tracker available
        if let Some(tracker) = &self.delivery_tracker {
            let idempotency_key = message.correlation_id();
            let _ = tracker
                .track(message_id.clone(), delivery_mode, idempotency_key)
                .await?;
        }

        // Queue the message
        let queued = QueuedMessage {
            message,
            metadata,
            queued_at: Utc::now(),
            delivery_mode,
        };

        {
            let mut queue = self.queue.write().await;
            let queue_depth = queue.len();
            if self.config.max_queue_depth > 0 && queue_depth >= self.config.max_queue_depth {
                return Err(PersistenceError::query_failed(format!(
                    "Channel queue full (max: {})",
                    self.config.max_queue_depth
                )));
            }
            queue.push_back(queued.clone());
        }

        {
            let mut count = self.message_count.write().await;
            *count = count.saturating_add(1);
        }

        // Persist if enabled
        if self.config.persist_messages {
            if let Some(store) = &self.store {
                if let Err(err) = self.persist_message(store, &queued).await {
                    self.rollback_enqueue(&message_id).await;
                    return Err(err);
                }
            }
        }

        Ok(message_id)
    }

    async fn rollback_enqueue(&self, message_id: &MessageId) {
        {
            let mut queue = self.queue.write().await;
            if let Some(pos) = queue.iter().position(|q| q.message.id() == message_id) {
                queue.remove(pos);
            }
        }

        let mut count = self.message_count.write().await;
        *count = count.saturating_sub(1);
    }

    /// Receive the next message from the channel.
    ///
    /// Returns `None` if the queue is empty.
    pub async fn receive(&self) -> Option<(Message, MessageMetadata)> {
        loop {
            let mut queue = self.queue.write().await;
            let queued = queue.pop_front()?;

            // Expire old messages if TTL is set
            if self.config.message_ttl_secs > 0 {
                let age = Utc::now()
                    .signed_duration_since(queued.queued_at)
                    .num_seconds();

                if age > self.config.message_ttl_secs as i64 {
                    // Message expired, try next
                    drop(queue);
                    continue;
                }
            }

            return Some((queued.message, queued.metadata));
        }
    }

    /// Receive a message and mark as delivered.
    ///
    /// For exactly-once semantics, this acknowledges the message.
    ///
    /// # Errors
    ///
    /// Returns an error if delivery tracking fails.
    pub async fn receive_and_ack(&self) -> PersistenceResult<Option<Message>> {
        let result = self.receive().await;

        if let Some((message, _metadata)) = result {
            // Mark as delivered if tracker available
            if let Some(tracker) = &self.delivery_tracker {
                tracker.mark_delivered(message.id()).await?;
            }
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    /// Peek at the next message without removing it.
    pub async fn peek(&self) -> Option<Message> {
        let queue = self.queue.read().await;
        queue.front().map(|q| q.message.clone())
    }

    /// Clear all messages from the channel.
    pub async fn clear(&self) {
        let mut queue = self.queue.write().await;
        queue.clear();
    }

    /// Persist a message to storage.
    async fn persist_message(
        &self,
        store: &OrchestratorStore,
        queued: &QueuedMessage,
    ) -> PersistenceResult<()> {
        #[derive(Serialize)]
        struct MessageInput {
            channel_id: String,
            message_id: String,
            message_data: String,
            metadata: String,
            queued_at: chrono::DateTime<Utc>,
            delivery_mode: String,
        }

        let message_data = serde_json::to_string(&queued.message)
            .map_err(|e| PersistenceError::serialization_error(e.to_string()))?;

        let metadata = serde_json::to_string(&queued.metadata)
            .map_err(|e| PersistenceError::serialization_error(e.to_string()))?;

        let input = MessageInput {
            channel_id: self.id.as_str().to_string(),
            message_id: queued.message.id().as_str().to_string(),
            message_data,
            metadata,
            queued_at: queued.queued_at,
            delivery_mode: format!("{:?}", queued.delivery_mode),
        };

        let _: Option<serde_json::Value> = store
            .db()
            .create(("channel_message", queued.message.id().as_str()))
            .content(input)
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?;

        Ok(())
    }

    /// Initialize the channel schema in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    pub async fn initialize_schema(store: &OrchestratorStore) -> PersistenceResult<()> {
        let schema = r"
            DEFINE TABLE IF NOT EXISTS channel_message SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS channel_id ON channel_message TYPE string;
            DEFINE FIELD IF NOT EXISTS message_id ON channel_message TYPE string;
            DEFINE FIELD IF NOT EXISTS message_data ON channel_message TYPE string;
            DEFINE FIELD IF NOT EXISTS metadata ON channel_message TYPE string;
            DEFINE FIELD IF NOT EXISTS queued_at ON channel_message TYPE datetime;
            DEFINE FIELD IF NOT EXISTS delivery_mode ON channel_message TYPE string;
            DEFINE INDEX IF NOT EXISTS channel_message_channel ON channel_message FIELDS channel_id;
        ";

        store
            .db()
            .query(schema)
            .await
            .map_err(|e| PersistenceError::query_failed(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_config_default() {
        let config = ChannelConfig::default();
        assert_eq!(config.max_queue_depth, 10_000);
        assert!(matches!(
            config.default_delivery_mode,
            DeliveryMode::AtLeastOnce
        ));
        assert!(config.persist_messages);
    }

    #[tokio::test]
    async fn test_channel_send_receive() {
        let channel = DurableChannel::new("test-channel", ChannelConfig::default());

        let msg = Message::one_way(serde_json::json!({"data": "hello"}));
        let msg_id = msg.id().clone();

        let send_result = channel.send(msg).await;
        assert!(send_result.is_ok());
        if let Ok(sent_id) = send_result {
            assert_eq!(sent_id.as_str(), msg_id.as_str());
        }

        let received = channel.receive().await;
        assert!(received.is_some());

        if let Some((message, _metadata)) = received {
            assert!(message.is_one_way());
        }
    }

    #[tokio::test]
    async fn test_channel_queue_depth() {
        let channel = DurableChannel::new("test-channel", ChannelConfig::default());

        assert_eq!(channel.queue_depth().await, 0);

        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;
        assert_eq!(channel.queue_depth().await, 1);

        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;
        assert_eq!(channel.queue_depth().await, 2);

        let _ = channel.receive().await;
        assert_eq!(channel.queue_depth().await, 1);
    }

    #[tokio::test]
    async fn test_channel_max_queue_depth() {
        let config = ChannelConfig {
            max_queue_depth: 2,
            ..Default::default()
        };
        let channel = DurableChannel::new("test-channel", config);

        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;
        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;

        let result = channel.send(Message::one_way(serde_json::json!({}))).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_channel_peek() {
        let channel = DurableChannel::new("test-channel", ChannelConfig::default());

        let msg = Message::one_way(serde_json::json!({"peek": "test"}));
        let msg_id = msg.id().clone();
        let _ = channel.send(msg).await;

        let peeked = channel.peek().await;
        assert!(peeked.is_some());
        if let Some(peeked_msg) = peeked {
            assert_eq!(peeked_msg.id().as_str(), msg_id.as_str());
        }

        // Should still be in queue
        assert_eq!(channel.queue_depth().await, 1);
    }

    #[tokio::test]
    async fn test_channel_clear() {
        let channel = DurableChannel::new("test-channel", ChannelConfig::default());

        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;
        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;
        assert_eq!(channel.queue_depth().await, 2);

        channel.clear().await;
        assert_eq!(channel.queue_depth().await, 0);
    }

    #[tokio::test]
    async fn test_channel_with_workflows() {
        let channel = DurableChannel::new("test-channel", ChannelConfig::default())
            .with_sender("sender-wf")
            .with_receiver("receiver-wf");

        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;

        let received = channel.receive().await;
        assert!(received.is_some());

        if let Some((_msg, metadata)) = received {
            assert_eq!(metadata.source_workflow, Some("sender-wf".to_string()));
            assert_eq!(metadata.target_workflow, Some("receiver-wf".to_string()));
        }
    }

    #[tokio::test]
    async fn test_channel_message_count() {
        let channel = DurableChannel::new("test-channel", ChannelConfig::default());

        assert_eq!(channel.message_count().await, 0);

        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;
        let _ = channel.send(Message::one_way(serde_json::json!({}))).await;

        assert_eq!(channel.message_count().await, 2);

        // Message count doesn't decrease on receive (it's a total count)
        let _ = channel.receive().await;
        assert_eq!(channel.message_count().await, 2);
    }
}
