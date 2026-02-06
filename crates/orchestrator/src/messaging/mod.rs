//! Durable message passing for service-to-service communication.
//!
//! This module provides Restate-style durable message channels with
//! exactly-once delivery semantics for reliable inter-workflow communication.
//!
//! # Architecture
//!
//! The messaging system uses event sourcing for durability:
//! 1. Messages are persisted before acknowledgment
//! 2. Delivery tracking ensures exactly-once semantics
//! 3. Message routing handles cross-workflow communication
//!
//! # Key Types
//!
//! - `DurableChannel`: A durable message channel between workflows
//! - `Message`: An envelope containing payload and metadata
//! - `MessageRouter`: Routes messages to appropriate handlers
//! - `DeliveryTracker`: Ensures exactly-once delivery

// Allow dead_code until this module is fully integrated
#![allow(dead_code)]

mod channel;
mod delivery;
mod router;
mod types;

pub use channel::{ChannelConfig, DurableChannel};
pub use delivery::{DeliveryMode, DeliveryStatus, DeliveryTracker};
pub use router::{MessageRouter, RouteConfig};
pub use types::{ChannelId, Message, MessageId, MessagePayload};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id_generation() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_channel_id_from_string() {
        let id: ChannelId = "channel-1".into();
        assert_eq!(id.as_str(), "channel-1");
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::request("test-channel", serde_json::json!({"key": "value"}));
        assert!(matches!(msg, Message::Request { .. }));
    }

    #[test]
    fn test_delivery_mode_default() {
        let mode = DeliveryMode::default();
        assert!(matches!(mode, DeliveryMode::AtLeastOnce));
    }
}
