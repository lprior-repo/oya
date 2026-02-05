//! Core types for durable messaging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a message channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(String);

impl ChannelId {
    /// Create a new channel ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the channel ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ChannelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ChannelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(String);

impl MessageId {
    /// Create a new unique message ID.
    #[must_use]
    pub fn new() -> Self {
        Self(format!("msg-{}", Uuid::new_v4()))
    }

    /// Create a message ID from an existing string.
    #[must_use]
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the message ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message payload as JSON value.
pub type MessagePayload = serde_json::Value;

/// A durable message with exactly-once delivery semantics.
///
/// Messages can be:
/// - **Request**: Expects a response
/// - **Response**: Reply to a request
/// - **OneWay**: Fire-and-forget
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    /// A request message expecting a response.
    Request {
        /// Unique message identifier
        id: MessageId,
        /// Message payload
        payload: MessagePayload,
        /// Channel to reply on
        reply_to: ChannelId,
        /// When the message was created
        created_at: DateTime<Utc>,
        /// Correlation ID for tracing
        correlation_id: Option<String>,
    },

    /// A response to a request.
    Response {
        /// Unique message identifier
        id: MessageId,
        /// ID of the request this responds to
        request_id: MessageId,
        /// Response payload
        payload: MessagePayload,
        /// When the response was created
        created_at: DateTime<Utc>,
    },

    /// A one-way fire-and-forget message.
    OneWay {
        /// Unique message identifier
        id: MessageId,
        /// Message payload
        payload: MessagePayload,
        /// When the message was created
        created_at: DateTime<Utc>,
        /// Correlation ID for tracing
        correlation_id: Option<String>,
    },
}

impl Message {
    /// Create a new request message.
    #[must_use]
    pub fn request(reply_to: impl Into<ChannelId>, payload: MessagePayload) -> Self {
        Self::Request {
            id: MessageId::new(),
            payload,
            reply_to: reply_to.into(),
            created_at: Utc::now(),
            correlation_id: None,
        }
    }

    /// Create a new request with correlation ID.
    #[must_use]
    pub fn request_with_correlation(
        reply_to: impl Into<ChannelId>,
        payload: MessagePayload,
        correlation_id: impl Into<String>,
    ) -> Self {
        Self::Request {
            id: MessageId::new(),
            payload,
            reply_to: reply_to.into(),
            created_at: Utc::now(),
            correlation_id: Some(correlation_id.into()),
        }
    }

    /// Create a response message.
    #[must_use]
    pub fn response(request_id: MessageId, payload: MessagePayload) -> Self {
        Self::Response {
            id: MessageId::new(),
            request_id,
            payload,
            created_at: Utc::now(),
        }
    }

    /// Create a one-way message.
    #[must_use]
    pub fn one_way(payload: MessagePayload) -> Self {
        Self::OneWay {
            id: MessageId::new(),
            payload,
            created_at: Utc::now(),
            correlation_id: None,
        }
    }

    /// Create a one-way message with correlation ID.
    #[must_use]
    pub fn one_way_with_correlation(
        payload: MessagePayload,
        correlation_id: impl Into<String>,
    ) -> Self {
        Self::OneWay {
            id: MessageId::new(),
            payload,
            created_at: Utc::now(),
            correlation_id: Some(correlation_id.into()),
        }
    }

    /// Get the message ID.
    #[must_use]
    pub fn id(&self) -> &MessageId {
        match self {
            Self::Request { id, .. } | Self::Response { id, .. } | Self::OneWay { id, .. } => id,
        }
    }

    /// Get the payload.
    #[must_use]
    pub fn payload(&self) -> &MessagePayload {
        match self {
            Self::Request { payload, .. }
            | Self::Response { payload, .. }
            | Self::OneWay { payload, .. } => payload,
        }
    }

    /// Get the creation timestamp.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::Request { created_at, .. }
            | Self::Response { created_at, .. }
            | Self::OneWay { created_at, .. } => *created_at,
        }
    }

    /// Get the correlation ID if present.
    #[must_use]
    pub fn correlation_id(&self) -> Option<&str> {
        match self {
            Self::Request { correlation_id, .. } | Self::OneWay { correlation_id, .. } => {
                correlation_id.as_deref()
            }
            Self::Response { .. } => None,
        }
    }

    /// Check if this is a request.
    #[must_use]
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request { .. })
    }

    /// Check if this is a response.
    #[must_use]
    pub fn is_response(&self) -> bool {
        matches!(self, Self::Response { .. })
    }

    /// Check if this is a one-way message.
    #[must_use]
    pub fn is_one_way(&self) -> bool {
        matches!(self, Self::OneWay { .. })
    }
}

/// Metadata for tracking message delivery.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Source workflow ID
    pub source_workflow: Option<String>,
    /// Source bead ID
    pub source_bead: Option<String>,
    /// Target workflow ID
    pub target_workflow: Option<String>,
    /// Target bead ID
    pub target_bead: Option<String>,
    /// Number of delivery attempts
    pub delivery_attempts: u32,
    /// Last delivery attempt time
    pub last_attempt_at: Option<DateTime<Utc>>,
}

impl MessageMetadata {
    /// Create metadata with source information.
    #[must_use]
    pub fn with_source(mut self, workflow_id: &str, bead_id: Option<&str>) -> Self {
        self.source_workflow = Some(workflow_id.to_string());
        self.source_bead = bead_id.map(String::from);
        self
    }

    /// Create metadata with target information.
    #[must_use]
    pub fn with_target(mut self, workflow_id: &str, bead_id: Option<&str>) -> Self {
        self.target_workflow = Some(workflow_id.to_string());
        self.target_bead = bead_id.map(String::from);
        self
    }

    /// Record a delivery attempt.
    pub fn record_attempt(&mut self) {
        self.delivery_attempts = self.delivery_attempts.saturating_add(1);
        self.last_attempt_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_id_display() {
        let id = ChannelId::new("test-channel");
        assert_eq!(format!("{}", id), "test-channel");
    }

    #[test]
    fn test_message_id_uniqueness() {
        let ids: Vec<MessageId> = (0..100).map(|_| MessageId::new()).collect();
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 100);
    }

    #[test]
    fn test_request_message() {
        let msg = Message::request("reply-channel", serde_json::json!({"test": true}));
        assert!(msg.is_request());
        assert!(!msg.is_response());
        assert!(!msg.is_one_way());
    }

    #[test]
    fn test_response_message() {
        let request_id = MessageId::new();
        let msg = Message::response(request_id.clone(), serde_json::json!({"result": "ok"}));
        assert!(msg.is_response());

        if let Message::Response {
            request_id: ref rid,
            ..
        } = msg
        {
            assert_eq!(rid, &request_id);
        }
    }

    #[test]
    fn test_one_way_message() {
        let msg = Message::one_way(serde_json::json!({"event": "happened"}));
        assert!(msg.is_one_way());
    }

    #[test]
    fn test_message_with_correlation() {
        let msg = Message::request_with_correlation("reply", serde_json::json!({}), "corr-123");
        assert_eq!(msg.correlation_id(), Some("corr-123"));
    }

    #[test]
    fn test_metadata_with_source() {
        let meta = MessageMetadata::default().with_source("wf-1", Some("bead-1"));
        assert_eq!(meta.source_workflow, Some("wf-1".to_string()));
        assert_eq!(meta.source_bead, Some("bead-1".to_string()));
    }

    #[test]
    fn test_metadata_delivery_attempts() {
        let mut meta = MessageMetadata::default();
        assert_eq!(meta.delivery_attempts, 0);

        meta.record_attempt();
        assert_eq!(meta.delivery_attempts, 1);
        assert!(meta.last_attempt_at.is_some());

        meta.record_attempt();
        assert_eq!(meta.delivery_attempts, 2);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::request("channel", serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&msg);
        assert!(json.is_ok());

        if let Ok(serialized) = json {
            let deserialized: Result<Message, _> = serde_json::from_str(&serialized);
            assert!(deserialized.is_ok());
        }
    }
}
