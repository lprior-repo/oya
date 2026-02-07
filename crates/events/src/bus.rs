//! Event bus for pub/sub coordination.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use tracing::debug;

use crate::error::{Error, Result};
use crate::event::BeadEvent;
use crate::store::EventStore;
use crate::types::EventId;

/// Circuit breaker to prevent cascading failures.
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    threshold: u32,
    last_failure: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given failure threshold.
    pub fn new(threshold: u32) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            threshold,
            last_failure: AtomicU64::new(0),
        }
    }

    /// Check if a request should be allowed.
    pub fn allow_request(&self) -> bool {
        self.failure_count.load(Ordering::Relaxed) < self.threshold
    }

    /// Record a successful request.
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the current failure count.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Reset the circuit breaker.
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
    }
}

/// Enhanced subscriber information with circuit breaker.
struct Subscriber {
    sender: broadcast::Sender<BeadEvent>,
    pattern: EventPattern,
    breaker: Arc<CircuitBreaker>,
}

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
    /// Failure threshold for circuit breakers.
    failure_threshold: u32,
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
            failure_threshold: 5, // Default: 5 consecutive failures before opening circuit
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

        // Send to pattern-based subscribers with circuit breaker protection
        let subscribers = self.subscribers.read().await;
        for (_, sub) in subscribers
            .iter()
            .filter(|(_, sub)| sub.pattern.matches(&event))
        {
            // Check if subscriber is in circuit breaker state
            if !sub.breaker.allow_request() {
                debug!(
                    event_type = event.event_type(),
                    bead_id = %event.bead_id(),
                    subscriber_failures = sub.breaker.failure_count(),
                    "Skipping subscriber due to circuit breaker"
                );
                continue;
            }

            // Attempt to send event to subscriber
            match sub.sender.send(event.clone()) {
                Ok(_) => {
                    // Success - reset failure count
                    sub.breaker.record_success();
                    debug!(
                        event_type = event.event_type(),
                        bead_id = %event.bead_id(),
                        "Event delivered to subscriber successfully"
                    );
                }
                Err(broadcast::error::SendError(_)) => {
                    // Subscriber dropped - treat as failure
                    sub.breaker.record_failure();
                    debug!(
                        event_type = event.event_type(),
                        bead_id = %event.bead_id(),
                        subscriber_failures = sub.breaker.failure_count(),
                        "Failed to deliver event to subscriber"
                    );
                }
            }
        }

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
        subscribers.insert(id.clone(), Subscriber {
            sender,
            pattern,
            breaker: self.create_circuit_breaker(),
        });

        (id, EventSubscription { receiver })
    }

    /// Unsubscribe a pattern-based subscriber.
    pub async fn unsubscribe(&self, subscriber_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(subscriber_id);
    }

    /// Replay events from a given event ID.
    pub async fn replay_from(&self, from: Option<EventId>) -> Result<Arc<[BeadEvent]>> {
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
    failure_threshold: u32,
}

impl EventBusBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            store: None,
            channel_capacity: 1000,
            failure_threshold: 5,
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

    /// Set the circuit breaker failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
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
            failure_threshold: self.failure_threshold,
        })
    }
}

impl Default for EventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Get the circuit breaker failure threshold.
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold
    }

    /// Create a new circuit breaker for subscribers.
    fn create_circuit_breaker(&self) -> Arc<CircuitBreaker> {
        Arc::new(CircuitBreaker::new(self.failure_threshold))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
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
        assert_eq!(events.map(|e| e.len()).map_or(0, |len| len), 2); // Events after first
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

    // ==========================================================================
    // Circuit Breaker Tests
    // ==========================================================================

    #[test]
    fn test_circuit_breaker_initial_state() {
        let breaker = CircuitBreaker::new(3);
        assert!(breaker.allow_request(), "New breaker should allow requests");
        assert_eq!(breaker.failure_count(), 0, "New breaker should have 0 failures");
    }

    #[test]
    fn test_circuit_breaker_record_success() {
        let breaker = CircuitBreaker::new(3);
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 2, "Should have 2 failures");

        breaker.record_success();
        assert_eq!(breaker.failure_count(), 0, "Success should reset failure count");
        assert!(breaker.allow_request(), "Should allow requests after success");
    }

    #[test]
    fn test_circuit_breaker_record_failure() {
        let breaker = CircuitBreaker::new(3);
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 1, "Should have 1 failure");
        assert!(breaker.allow_request(), "Should allow requests below threshold");

        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 3, "Should have 3 failures");
        assert!(!breaker.allow_request(), "Should block requests at threshold");
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let breaker = CircuitBreaker::new(3);
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 2, "Should have 2 failures");

        breaker.reset();
        assert_eq!(breaker.failure_count(), 0, "Reset should clear failure count");
        assert!(breaker.allow_request(), "Should allow requests after reset");
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_failing_subscriber() {
        let bus = EventBusBuilder::new()
            .with_store(Arc::new(InMemoryEventStore::new()))
            .with_failure_threshold(2)
            .build()
            .unwrap();

        let bead_id = BeadId::new();

        // Subscribe to state_changed events
        let pattern = EventPattern::ByType("state_changed".to_string());
        let (sub_id, mut sub) = bus.subscribe_with_pattern(pattern).await;

        // Create a dropping receiver to simulate failure
        let (_handle, mut test_sub) = bus.subscribe_with_pattern(pattern).await;
        std::mem::drop(test_sub); // Drop to cause send failures

        // First event - should be sent but fail, incrementing failure count
        let result1 = bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        )).await;
        assert!(result1.is_ok(), "Publish should succeed even with failing subscriber");

        // Second event - subscriber should be blocked
        let result2 = bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Scheduled,
            BeadState::Ready,
        )).await;
        assert!(result2.is_ok(), "Publish should succeed with circuit breaker");

        // Give time for delivery
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Only the first event should have been delivered (before circuit breaker opened)
        let received1 = sub.try_recv();
        assert!(received1.is_ok(), "Should receive first event");

        let received2 = sub.try_recv();
        assert!(received2.is_err(), "Should not receive second event due to circuit breaker");

        // Unsubscribe
        bus.unsubscribe(&sub_id).await;
    }

    #[tokio::test]
    async fn test_circuit_breaker_resets_after_success() {
        let bus = EventBusBuilder::new()
            .with_store(Arc::new(InMemoryEventStore::new()))
            .with_failure_threshold(2)
            .build()
            .unwrap();

        let bead_id = BeadId::new();

        // Subscribe to state_changed events
        let pattern = EventPattern::ByType("state_changed".to_string());
        let (sub_id, mut sub) = bus.subscribe_with_pattern(pattern).await;

        // Create a dropping receiver to simulate failure
        let (_handle, mut test_sub) = bus.subscribe_with_pattern(pattern).await;
        std::mem::drop(test_sub); // Drop to cause send failures

        // First event - failure, count = 1
        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        )).await.unwrap();

        // Second event - failure, count = 2 (circuit opens)
        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Scheduled,
            BeadState::Ready,
        )).await.unwrap();

        // Third event - should be blocked
        bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Ready,
            BeadState::Completed,
        )).await.unwrap();

        // Create a new subscriber that should work
        let (sub_id2, mut sub2) = bus.subscribe_with_pattern(pattern).await;

        // Publish another event - should be delivered to working subscriber
        let result = bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Completed,
            BeadState::Archived,
        )).await;
        assert!(result.is_ok());

        // Give time for delivery
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // New subscriber should receive the event
        let received = sub2.try_recv();
        assert!(received.is_ok(), "Working subscriber should receive event");

        // Unsubscribe
        bus.unsubscribe(&sub_id).await;
        bus.unsubscribe(&sub_id2).await;
    }

    #[tokio::test]
    async fn test_custom_failure_threshold() {
        let bus = EventBusBuilder::new()
            .with_store(Arc::new(InMemoryEventStore::new()))
            .with_failure_threshold(1)
            .build()
            .unwrap();

        assert_eq!(bus.failure_threshold(), 1, "Should use custom threshold");

        let bead_id = BeadId::new();

        // Subscribe to state_changed events
        let pattern = EventPattern::ByType("state_changed".to_string());
        let (sub_id, mut sub) = bus.subscribe_with_pattern(pattern).await;

        // Create a dropping receiver to simulate failure
        let (_handle, mut test_sub) = bus.subscribe_with_pattern(pattern).await;
        std::mem::drop(test_sub); // Drop to cause send failures

        // First event - should fail and open circuit (threshold = 1)
        let result = bus.publish(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        )).await;
        assert!(result.is_ok(), "Publish should succeed even with failing subscriber");

        // Give time for delivery
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should not receive the event due to circuit breaker
        let received = sub.try_recv();
        assert!(received.is_err(), "Should not receive event due to circuit breaker");

        // Unsubscribe
        bus.unsubscribe(&sub_id).await;
    }
}
