//! Event bus for pub/sub coordination.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use tracing::debug;

use crate::error::{Error, Result};
use crate::event::BeadEvent;
use crate::store::EventStore;
use crate::types::EventId;

/// Subscription handle for receiving events.
pub struct EventSubscription {
    receiver: broadcast::Receiver<BeadEvent>,
}

impl EventSubscription {
    /// Receive the next event.
    pub async fn recv(&mut self) -> Result<BeadEvent> {
        self.receiver.recv().await.map_err(|_| Error::ChannelClosed)
    }

    /// Try to receive an event without waiting.
    pub fn try_recv(&mut self) -> Result<BeadEvent> {
        self.receiver.try_recv().map_err(|_| Error::ChannelClosed)
    }
}

/// Pattern for filtering events.
#[derive(Debug, Clone)]
pub enum EventPattern {
    /// Match all events.
    All,
    /// Match events by type.
    ByType(String),
    /// Match events by bead ID.
    ByBead(crate::types::BeadId),
    /// Match events by multiple types.
    ByTypes(Vec<String>),
}

impl EventPattern {
    /// Check if an event matches this pattern.
    pub fn matches(&self, event: &BeadEvent) -> bool {
        match self {
            Self::All => true,
            Self::ByType(t) => event.event_type() == t,
            Self::ByBead(id) => event.bead_id() == *id,
            Self::ByTypes(types) => types.iter().any(|t| event.event_type() == t),
        }
    }
}

/// Subscriber information.
struct Subscriber {
    sender: broadcast::Sender<BeadEvent>,
    pattern: EventPattern,
}

/// Event bus for publishing and subscribing to events.
pub struct EventBus {
    /// Underlying event store.
    store: Arc<dyn EventStore>,
    /// Broadcast sender for all events.
    broadcast: broadcast::Sender<BeadEvent>,
    /// Pattern-based subscribers.
    subscribers: RwLock<HashMap<String, Subscriber>>,
    /// Next subscriber ID.
    next_id: RwLock<u64>,
}

impl EventBus {
    /// Create a new event bus with the given store.
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        let (broadcast, _) = broadcast::channel(1000);
        Self {
            store,
            broadcast,
            subscribers: RwLock::new(HashMap::new()),
            next_id: RwLock::new(0),
        }
    }

    /// Publish an event.
    ///
    /// The event is stored and broadcast to all subscribers.
    pub async fn publish(&self, event: BeadEvent) -> Result<EventId> {
        // Store the event
        let event_id = self.store.append(event.clone()).await?;

        debug!(
            event_id = %event_id,
            event_type = event.event_type(),
            bead_id = %event.bead_id(),
            "Publishing event"
        );

        // Broadcast to global subscribers
        let _ = self.broadcast.send(event.clone());

        // Send to pattern-based subscribers
        let subscribers = self.subscribers.read().await;
        subscribers
            .iter()
            .filter(|(_, sub)| sub.pattern.matches(&event))
            .for_each(|(_, sub)| {
                let _ = sub.sender.send(event.clone());
            });

        Ok(event_id)
    }

    /// Subscribe to all events.
    pub fn subscribe(&self) -> EventSubscription {
        EventSubscription {
            receiver: self.broadcast.subscribe(),
        }
    }

    /// Subscribe to events matching a pattern.
    pub async fn subscribe_with_pattern(
        &self,
        pattern: EventPattern,
    ) -> (String, EventSubscription) {
        let (sender, receiver) = broadcast::channel(100);

        let mut next_id = self.next_id.write().await;
        let id = format!("sub_{}", *next_id);
        *next_id += 1;

        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(id.clone(), Subscriber { sender, pattern });

        (id, EventSubscription { receiver })
    }

    /// Unsubscribe a pattern-based subscriber.
    pub async fn unsubscribe(&self, subscriber_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(subscriber_id);
    }

    /// Replay events from a given event ID.
    pub async fn replay_from(&self, from: Option<EventId>) -> Result<Vec<BeadEvent>> {
        self.store.read(from).await
    }

    /// Get the underlying event store.
    pub fn store(&self) -> &Arc<dyn EventStore> {
        &self.store
    }
}

/// Builder for EventBus.
pub struct EventBusBuilder {
    store: Option<Arc<dyn EventStore>>,
    channel_capacity: usize,
}

impl EventBusBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            store: None,
            channel_capacity: 1000,
        }
    }

    /// Set the event store.
    pub fn with_store(mut self, store: Arc<dyn EventStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Set the broadcast channel capacity.
    pub fn with_channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }

    /// Build the event bus.
    pub fn build(self) -> Result<EventBus> {
        let store = self
            .store
            .ok_or_else(|| Error::invalid_event("No event store configured"))?;

        let (broadcast, _) = broadcast::channel(self.channel_capacity);

        Ok(EventBus {
            store,
            broadcast,
            subscribers: RwLock::new(HashMap::new()),
            next_id: RwLock::new(0),
        })
    }
}

impl Default for EventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryEventStore;
    use crate::types::{BeadId, BeadSpec, BeadState, Complexity};

    fn setup_bus() -> EventBus {
        let store = Arc::new(InMemoryEventStore::new());
        EventBus::new(store)
    }

    #[tokio::test]
    async fn test_publish_and_subscribe() {
        let bus = setup_bus();
        let mut sub = bus.subscribe();

        let bead_id = BeadId::new();
        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );

        let result = bus.publish(event).await;
        assert!(result.is_ok());

        // Give broadcast a moment to deliver
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let received = sub.try_recv();
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_pattern_subscription() {
        let bus = setup_bus();
        let bead_id = BeadId::new();

        // Subscribe to state_changed events only
        let pattern = EventPattern::ByType("state_changed".to_string());
        let (sub_id, mut sub) = bus.subscribe_with_pattern(pattern).await;

        // Publish a created event (should not match)
        bus.publish(BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        ))
        .await
        .ok();

        // Publish a state_changed event (should match)
        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        ))
        .await
        .ok();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should receive the state_changed event
        let received = sub.try_recv();
        assert!(received.is_ok());
        assert_eq!(received.ok().map(|e| e.event_type()), Some("state_changed"));

        // Unsubscribe
        bus.unsubscribe(&sub_id).await;
    }

    #[tokio::test]
    async fn test_replay() {
        let bus = setup_bus();
        let bead_id = BeadId::new();

        // Publish some events
        let first_id = bus
            .publish(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();

        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        ))
        .await
        .ok();

        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Scheduled,
            BeadState::Ready,
        ))
        .await
        .ok();

        // Replay from the first event
        let events = bus.replay_from(first_id).await;
        assert!(events.is_ok());
        assert_eq!(events.map(|e| e.len()).unwrap_or(0), 2); // Events after first
    }

    #[tokio::test]
    async fn test_event_pattern_matching() {
        let bead_id = BeadId::new();
        let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);

        assert!(EventPattern::All.matches(&event));
        assert!(EventPattern::ByType("state_changed".to_string()).matches(&event));
        assert!(!EventPattern::ByType("created".to_string()).matches(&event));
        assert!(EventPattern::ByBead(bead_id).matches(&event));
        assert!(!EventPattern::ByBead(BeadId::new()).matches(&event));
    }
}
