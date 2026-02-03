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
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Result;
    use crate::store::InMemoryEventStore;
    use crate::types::{BeadId, BeadSpec, BeadState, Complexity};

    fn setup_bus() -> EventBus {
        let store = Arc::new(InMemoryEventStore::new());
        EventBus::new(store)
    }

    // ==========================================================================
    // EventPattern BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_match_all_events_with_all_pattern() {
        let event = BeadEvent::created(
            BeadId::new(),
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );

        assert!(
            EventPattern::All.matches(&event),
            "All pattern should match any event"
        );
    }

    #[test]
    fn should_match_event_by_exact_type() {
        let bead_id = BeadId::new();
        let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Ready);

        // Should match correct type
        assert!(
            EventPattern::ByType("state_changed".to_string()).matches(&event),
            "ByType should match correct event type"
        );

        // Should NOT match wrong type
        assert!(
            !EventPattern::ByType("created".to_string()).matches(&event),
            "ByType should not match wrong event type"
        );
    }

    #[test]
    fn should_match_event_by_exact_bead_id() {
        let matching_id = BeadId::new();
        let different_id = BeadId::new();

        let event = BeadEvent::created(
            matching_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );

        // Should match correct bead ID
        assert!(
            EventPattern::ByBead(matching_id).matches(&event),
            "ByBead should match correct bead ID"
        );

        // Should NOT match different bead ID
        assert!(
            !EventPattern::ByBead(different_id).matches(&event),
            "ByBead should not match different bead ID"
        );
    }

    #[test]
    fn should_match_event_by_multiple_types() {
        let bead_id = BeadId::new();
        let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Ready);

        let pattern =
            EventPattern::ByTypes(vec!["created".to_string(), "state_changed".to_string()]);

        // Should match when type is in list
        assert!(
            pattern.matches(&event),
            "ByTypes should match when event type is in list"
        );

        let non_matching_pattern =
            EventPattern::ByTypes(vec!["created".to_string(), "completed".to_string()]);

        // Should NOT match when type is not in list
        assert!(
            !non_matching_pattern.matches(&event),
            "ByTypes should not match when event type is not in list"
        );
    }

    // ==========================================================================
    // EventBusBuilder BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_fail_to_build_without_store() {
        let result = EventBusBuilder::new().build();

        assert!(result.is_err(), "Building without store should fail");
    }

    #[test]
    fn should_build_with_store() {
        let store = Arc::new(InMemoryEventStore::new());
        let result = EventBusBuilder::new().with_store(store).build();

        assert!(result.is_ok(), "Building with store should succeed");
    }

    #[test]
    fn should_use_configured_store() -> crate::Result<()> {
        let store: Arc<dyn crate::store::EventStore> = Arc::new(InMemoryEventStore::new());
        let bus = EventBusBuilder::new().with_store(store.clone()).build()?;

        // Verify the store is the one we configured (compare by address)
        let store_ptr = Arc::as_ptr(&store) as *const ();
        let bus_store_ptr = Arc::as_ptr(bus.store()) as *const ();
        assert_eq!(
            store_ptr, bus_store_ptr,
            "Bus should use the configured store"
        );
        Ok(())
    }

    #[tokio::test]
    async fn should_use_configured_channel_capacity() -> crate::Result<()> {
        let store = Arc::new(InMemoryEventStore::new());
        let bus = EventBusBuilder::new()
            .with_store(store)
            .with_channel_capacity(5) // Small capacity
            .build()?;

        // Subscribe to create a receiver
        let _sub = bus.subscribe();

        // Publish events - with capacity 5, we should be able to publish without blocking
        for i in 0..5 {
            let event = BeadEvent::created(
                BeadId::new(),
                BeadSpec::new(format!("Test {}", i)).with_complexity(Complexity::Simple),
            );
            let result = bus.publish(event).await;
            assert!(result.is_ok(), "Should publish within capacity");
        }
        Ok(())
    }

    // ==========================================================================
    // EventBus Subscription BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_assign_unique_subscriber_ids() {
        let bus = setup_bus();

        let (id1, _sub1) = bus.subscribe_with_pattern(EventPattern::All).await;
        let (id2, _sub2) = bus.subscribe_with_pattern(EventPattern::All).await;
        let (id3, _sub3) = bus.subscribe_with_pattern(EventPattern::All).await;

        // All IDs should be unique
        assert_ne!(id1, id2, "Subscriber IDs should be unique");
        assert_ne!(id2, id3, "Subscriber IDs should be unique");
        assert_ne!(id1, id3, "Subscriber IDs should be unique");
    }

    #[tokio::test]
    async fn should_remove_subscriber_on_unsubscribe() {
        let bus = setup_bus();

        // Subscribe
        let (sub_id, _sub) = bus.subscribe_with_pattern(EventPattern::All).await;

        // Verify subscriber exists (internal check via subscriber count)
        {
            let subscribers = bus.subscribers.read().await;
            assert!(subscribers.contains_key(&sub_id), "Subscriber should exist");
        }

        // Unsubscribe
        bus.unsubscribe(&sub_id).await;

        // Verify subscriber is removed
        {
            let subscribers = bus.subscribers.read().await;
            assert!(
                !subscribers.contains_key(&sub_id),
                "Subscriber should be removed"
            );
        }
    }

    #[tokio::test]
    async fn should_deliver_events_only_to_matching_pattern_subscribers() {
        let bus = setup_bus();
        let bead_id = BeadId::new();

        // Subscribe to only "state_changed" events
        let (_sub_id, mut state_sub) = bus
            .subscribe_with_pattern(EventPattern::ByType("state_changed".to_string()))
            .await;

        // Publish a "created" event (should NOT be delivered)
        bus.publish(BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        ))
        .await
        .ok();

        // Give a moment for delivery
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should NOT have received the created event
        let result = state_sub.try_recv();
        assert!(result.is_err(), "Should not receive non-matching event");

        // Publish a "state_changed" event (SHOULD be delivered)
        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Ready,
        ))
        .await
        .ok();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should have received the state_changed event
        let result = state_sub.try_recv();
        assert!(result.is_ok(), "Should receive matching event");
    }

    // ==========================================================================
    // Original Tests
    // ==========================================================================

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
