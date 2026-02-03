//! Message routing for cross-workflow communication.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::channel::{ChannelConfig, DurableChannel};
use super::delivery::{DeliveryTracker, DeliveryTrackerConfig};
use super::types::{ChannelId, Message, MessageId};
use crate::persistence::{OrchestratorStore, PersistenceResult};

/// Configuration for a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// Source channel pattern (supports wildcards).
    pub source: String,
    /// Target channel ID.
    pub target: ChannelId,
    /// Whether to copy messages (vs move).
    pub copy: bool,
    /// Optional filter expression (JSON path).
    pub filter: Option<String>,
}

impl RouteConfig {
    /// Create a new route from source to target.
    #[must_use]
    pub fn new(source: impl Into<String>, target: impl Into<ChannelId>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            copy: false,
            filter: None,
        }
    }

    /// Set copy mode.
    #[must_use]
    pub fn with_copy(mut self, copy: bool) -> Self {
        self.copy = copy;
        self
    }

    /// Set filter expression.
    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Check if a channel ID matches this route's source pattern.
    #[must_use]
    pub fn matches(&self, channel_id: &str) -> bool {
        // Simple wildcard matching
        if self.source == "*" {
            return true;
        }

        if self.source.ends_with('*') {
            let prefix = &self.source[..self.source.len() - 1];
            return channel_id.starts_with(prefix);
        }

        self.source == channel_id
    }
}

/// Router configuration.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Whether to enable message deduplication.
    pub enable_deduplication: bool,
    /// Default channel configuration.
    pub default_channel_config: ChannelConfig,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enable_deduplication: true,
            default_channel_config: ChannelConfig::default(),
        }
    }
}

/// Routes messages between channels.
///
/// The router maintains a registry of channels and routing rules,
/// enabling flexible message delivery patterns.
pub struct MessageRouter {
    config: RouterConfig,
    store: Option<OrchestratorStore>,
    delivery_tracker: Arc<DeliveryTracker>,

    /// Registered channels
    channels: Arc<RwLock<HashMap<String, Arc<DurableChannel>>>>,
    /// Routing rules
    routes: Arc<RwLock<Vec<RouteConfig>>>,
}

impl MessageRouter {
    /// Create a new in-memory message router.
    #[must_use]
    pub fn new(config: RouterConfig) -> Self {
        let delivery_tracker = Arc::new(DeliveryTracker::new(DeliveryTrackerConfig::default()));

        Self {
            config,
            store: None,
            delivery_tracker,
            channels: Arc::new(RwLock::new(HashMap::new())),
            routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a router with persistent storage.
    #[must_use]
    pub fn with_store(config: RouterConfig, store: OrchestratorStore) -> Self {
        let delivery_tracker = Arc::new(DeliveryTracker::with_store(
            DeliveryTrackerConfig::default(),
            store.clone(),
        ));

        Self {
            config,
            store: Some(store),
            delivery_tracker,
            channels: Arc::new(RwLock::new(HashMap::new())),
            routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new channel.
    pub async fn register_channel(&self, channel_id: impl Into<ChannelId>) -> Arc<DurableChannel> {
        let id = channel_id.into();
        let id_str = id.as_str().to_string();

        // Check if already exists
        {
            let channels = self.channels.read().await;
            if let Some(channel) = channels.get(&id_str) {
                return Arc::clone(channel);
            }
        }

        // Create new channel
        let channel = if let Some(store) = &self.store {
            Arc::new(DurableChannel::with_store(
                id,
                self.config.default_channel_config.clone(),
                store.clone(),
                Arc::clone(&self.delivery_tracker),
            ))
        } else {
            Arc::new(DurableChannel::new(
                id,
                self.config.default_channel_config.clone(),
            ))
        };

        // Register
        {
            let mut channels = self.channels.write().await;
            channels.insert(id_str, Arc::clone(&channel));
        }

        channel
    }

    /// Get a channel by ID.
    pub async fn get_channel(&self, channel_id: &str) -> Option<Arc<DurableChannel>> {
        let channels = self.channels.read().await;
        channels.get(channel_id).cloned()
    }

    /// Add a routing rule.
    pub async fn add_route(&self, route: RouteConfig) {
        let mut routes = self.routes.write().await;
        routes.push(route);
    }

    /// Remove routes matching a source pattern.
    pub async fn remove_routes(&self, source: &str) {
        let mut routes = self.routes.write().await;
        routes.retain(|r| r.source != source);
    }

    /// Route a message from a source channel.
    ///
    /// # Errors
    ///
    /// Returns an error if routing fails.
    pub async fn route(
        &self,
        source_channel: &str,
        message: Message,
    ) -> PersistenceResult<Vec<MessageId>> {
        let routes = self.routes.read().await;
        let mut routed_ids = Vec::new();

        for route in routes.iter() {
            if route.matches(source_channel) {
                // Apply filter if present
                if let Some(ref filter) = route.filter {
                    if !self.apply_filter(filter, &message) {
                        continue;
                    }
                }

                // Get or create target channel
                let target_channel = self.register_channel(route.target.clone()).await;

                // Send message (always clone since there may be multiple routes)
                let msg = message.clone();

                let id = target_channel.send(msg).await?;
                routed_ids.push(id);
            }
        }

        Ok(routed_ids)
    }

    /// Send a message directly to a channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel doesn't exist or send fails.
    pub async fn send(&self, channel_id: &str, message: Message) -> PersistenceResult<MessageId> {
        let channel = self.register_channel(channel_id).await;
        channel.send(message).await
    }

    /// Receive a message from a channel.
    pub async fn receive(&self, channel_id: &str) -> Option<Message> {
        let channels = self.channels.read().await;
        if let Some(channel) = channels.get(channel_id) {
            channel.receive().await.map(|(msg, _)| msg)
        } else {
            None
        }
    }

    /// Get the number of registered channels.
    pub async fn channel_count(&self) -> usize {
        let channels = self.channels.read().await;
        channels.len()
    }

    /// Get the number of registered routes.
    pub async fn route_count(&self) -> usize {
        let routes = self.routes.read().await;
        routes.len()
    }

    /// Get the delivery tracker.
    #[must_use]
    pub fn delivery_tracker(&self) -> &Arc<DeliveryTracker> {
        &self.delivery_tracker
    }

    /// Apply a filter to a message.
    ///
    /// Currently supports simple JSON path expressions.
    fn apply_filter(&self, filter: &str, message: &Message) -> bool {
        let payload = message.payload();

        // Simple equality filter: "field=value"
        if let Some((path, value)) = filter.split_once('=') {
            let actual = payload.get(path.trim());
            return actual
                .map(|v| v.to_string().trim_matches('"') == value.trim())
                .unwrap_or(false);
        }

        // Simple existence filter: "field"
        payload.get(filter).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_config_matches_exact() {
        let route = RouteConfig::new("channel-1", "target");
        assert!(route.matches("channel-1"));
        assert!(!route.matches("channel-2"));
    }

    #[test]
    fn test_route_config_matches_wildcard() {
        let route = RouteConfig::new("events/*", "target");
        assert!(route.matches("events/created"));
        assert!(route.matches("events/updated"));
        assert!(!route.matches("commands/create"));
    }

    #[test]
    fn test_route_config_matches_all() {
        let route = RouteConfig::new("*", "target");
        assert!(route.matches("any-channel"));
        assert!(route.matches("another"));
    }

    #[tokio::test]
    async fn test_router_register_channel() {
        let router = MessageRouter::new(RouterConfig::default());

        let channel = router.register_channel("test-channel").await;
        assert_eq!(channel.id().as_str(), "test-channel");

        // Should return same channel on re-register
        let channel2 = router.register_channel("test-channel").await;
        assert_eq!(channel.id().as_str(), channel2.id().as_str());
    }

    #[tokio::test]
    async fn test_router_get_channel() {
        let router = MessageRouter::new(RouterConfig::default());

        let _ = router.register_channel("test-channel").await;

        let channel = router.get_channel("test-channel").await;
        assert!(channel.is_some());

        let missing = router.get_channel("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_router_send_receive() {
        let router = MessageRouter::new(RouterConfig::default());

        let _ = router.register_channel("test-channel").await;

        let msg = Message::one_way(serde_json::json!({"data": "test"}));
        let result = router.send("test-channel", msg).await;
        assert!(result.is_ok());

        let received = router.receive("test-channel").await;
        assert!(received.is_some());
        assert!(received.map(|m| m.is_one_way()).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_router_add_route() {
        let router = MessageRouter::new(RouterConfig::default());

        router.add_route(RouteConfig::new("source", "target")).await;
        assert_eq!(router.route_count().await, 1);

        router
            .add_route(RouteConfig::new("source2", "target2"))
            .await;
        assert_eq!(router.route_count().await, 2);
    }

    #[tokio::test]
    async fn test_router_remove_routes() {
        let router = MessageRouter::new(RouterConfig::default());

        router
            .add_route(RouteConfig::new("source1", "target1"))
            .await;
        router
            .add_route(RouteConfig::new("source2", "target2"))
            .await;
        assert_eq!(router.route_count().await, 2);

        router.remove_routes("source1").await;
        assert_eq!(router.route_count().await, 1);
    }

    #[tokio::test]
    async fn test_router_route_message() {
        let router = MessageRouter::new(RouterConfig::default());

        router.add_route(RouteConfig::new("input", "output")).await;

        let _ = router.register_channel("input").await;
        let _ = router.register_channel("output").await;

        let msg = Message::one_way(serde_json::json!({"routed": true}));
        let result = router.route("input", msg).await;
        assert!(result.is_ok());

        let ids = result.unwrap_or_else(|_| Vec::new());
        assert_eq!(ids.len(), 1);

        // Message should be in output channel
        let received = router.receive("output").await;
        assert!(received.is_some());
    }

    #[tokio::test]
    async fn test_router_route_with_filter() {
        let router = MessageRouter::new(RouterConfig::default());

        router
            .add_route(RouteConfig::new("input", "output").with_filter("type=event"))
            .await;

        let _ = router.register_channel("input").await;
        let _ = router.register_channel("output").await;

        // Message that matches filter
        let msg1 = Message::one_way(serde_json::json!({"type": "event", "data": "test"}));
        let result1 = router.route("input", msg1).await;
        assert_eq!(result1.map(|v| v.len()).unwrap_or(0), 1);

        // Message that doesn't match filter
        let msg2 = Message::one_way(serde_json::json!({"type": "command", "data": "test"}));
        let result2 = router.route("input", msg2).await;
        assert_eq!(result2.map(|v| v.len()).unwrap_or(0), 0);
    }

    #[tokio::test]
    async fn test_router_wildcard_route() {
        let router = MessageRouter::new(RouterConfig::default());

        router
            .add_route(RouteConfig::new("events/*", "all-events"))
            .await;

        let _ = router.register_channel("all-events").await;

        let msg1 = Message::one_way(serde_json::json!({"event": "created"}));
        let _ = router.route("events/user", msg1).await;

        let msg2 = Message::one_way(serde_json::json!({"event": "updated"}));
        let _ = router.route("events/order", msg2).await;

        // Both should be routed to all-events
        let channel = router.get_channel("all-events").await;
        assert!(channel.is_some());
        if let Some(ch) = channel {
            assert_eq!(ch.id().as_str(), "all-events");
        }
    }
}
