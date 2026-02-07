//! Event store trait and implementations.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::event::BeadEvent;
use crate::types::{BeadId, EventId};

/// Trait for event storage backends.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append an event to the store.
    async fn append(&self, event: BeadEvent) -> Result<EventId>;

    /// Read events starting from a given event ID.
    async fn read(&self, from: Option<EventId>) -> Result<Vec<BeadEvent>>;

    /// Read events for a specific bead.
    async fn read_for_bead(&self, bead_id: BeadId) -> Result<Vec<BeadEvent>>;

    /// Get the last event ID.
    async fn last_event_id(&self) -> Result<Option<EventId>>;

    /// Get the total number of events.
    async fn count(&self) -> Result<usize>;
}

/// In-memory event store for testing.
#[derive(Default)]
pub struct InMemoryEventStore {
    events: RwLock<Vec<BeadEvent>>,
    bead_index: RwLock<HashMap<BeadId, Vec<usize>>>,
}

impl InMemoryEventStore {
    /// Create a new in-memory event store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new in-memory event store wrapped in an Arc.
    pub fn new_arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, event: BeadEvent) -> Result<EventId> {
        let event_id = event.event_id();
        let bead_id = event.bead_id();

        let mut events = self.events.write().await;
        let index = events.len();
        events.push(event);

        // Update bead index
        let mut bead_index = self.bead_index.write().await;
        bead_index.entry(bead_id).or_default().push(index);

        Ok(event_id)
    }

    async fn read(&self, from: Option<EventId>) -> Result<Vec<BeadEvent>> {
        let events = self.events.read().await;

        if let Some(from_id) = from {
            // Find the position of the event and return all after it
            if let Some(pos) = events.iter().position(|e| e.event_id() == from_id) {
                Ok(events[pos + 1..].to_vec())
            } else {
                // Event not found, return all events
                Ok(events.clone())
            }
        } else {
            Ok(events.clone())
        }
    }

    async fn read_for_bead(&self, bead_id: BeadId) -> Result<Vec<BeadEvent>> {
        let events = self.events.read().await;
        let bead_index = self.bead_index.read().await;

        if let Some(indices) = bead_index.get(&bead_id) {
            Ok(indices
                .iter()
                .filter_map(|&i| events.get(i).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn last_event_id(&self) -> Result<Option<EventId>> {
        let events = self.events.read().await;
        Ok(events.last().map(|e| e.event_id()))
    }

    async fn count(&self) -> Result<usize> {
        let events = self.events.read().await;
        Ok(events.len())
    }
}

/// A wrapper that adds tracing to an event store.
pub struct TracingEventStore<S: EventStore> {
    inner: S,
}

impl<S: EventStore> TracingEventStore<S> {
    /// Create a new tracing event store.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<S: EventStore> EventStore for TracingEventStore<S> {
    async fn append(&self, event: BeadEvent) -> Result<EventId> {
        tracing::debug!(
            event_type = event.event_type(),
            bead_id = %event.bead_id(),
            "Appending event"
        );
        let result = self.inner.append(event).await;
        if let Ok(ref id) = result {
            tracing::trace!(event_id = %id, "Event appended");
        }
        result
    }

    async fn read(&self, from: Option<EventId>) -> Result<Vec<BeadEvent>> {
        tracing::debug!(from = ?from, "Reading events");
        self.inner.read(from).await
    }

    async fn read_for_bead(&self, bead_id: BeadId) -> Result<Vec<BeadEvent>> {
        tracing::debug!(bead_id = %bead_id, "Reading events for bead");
        self.inner.read_for_bead(bead_id).await
    }

    async fn last_event_id(&self) -> Result<Option<EventId>> {
        self.inner.last_event_id().await
    }

    async fn count(&self) -> Result<usize> {
        self.inner.count().await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{BeadSpec, BeadState, Complexity};

    #[tokio::test]
    async fn test_append_and_read() {
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);

        let event = BeadEvent::created(bead_id, spec);
        let event_id = store.append(event).await;
        assert!(event_id.is_ok());

        let events = store.read(None).await;
        assert!(events.is_ok());
        assert_eq!(events.map(|e| e.len()).map_or(0, |v| v), 1);
    }

    #[tokio::test]
    async fn test_read_for_bead() {
        let store = InMemoryEventStore::new();
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();

        // Add events for bead1
        store
            .append(BeadEvent::created(
                bead1,
                BeadSpec::new("Bead 1").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();
        store
            .append(BeadEvent::state_changed(
                bead1,
                crate::types::BeadState::Pending,
                crate::types::BeadState::Scheduled,
            ))
            .await
            .ok();

        // Add event for bead2
        store
            .append(BeadEvent::created(
                bead2,
                BeadSpec::new("Bead 2").with_complexity(Complexity::Medium),
            ))
            .await
            .ok();

        // Read events for bead1
        let events = store.read_for_bead(bead1).await;
        assert!(events.is_ok());
        assert_eq!(events.map(|e| e.len()).map_or(0, |v| v), 2);

        // Read events for bead2
        let events = store.read_for_bead(bead2).await;
        assert!(events.is_ok());
        assert_eq!(events.map(|e| e.len()).map_or(0, |v| v), 1);
    }

    #[tokio::test]
    async fn test_read_from_event() {
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        let event1 = BeadEvent::created(bead_id, spec);
        let event1_id = store.append(event1).await.ok();

        store
            .append(BeadEvent::state_changed(
                bead_id,
                crate::types::BeadState::Pending,
                crate::types::BeadState::Scheduled,
            ))
            .await
            .ok();
        store
            .append(BeadEvent::state_changed(
                bead_id,
                crate::types::BeadState::Scheduled,
                crate::types::BeadState::Ready,
            ))
            .await
            .ok();

        // Read from event1 should return 2 events (after event1)
        let events = store.read(event1_id).await;
        assert!(events.is_ok());
        assert_eq!(events.map(|e| e.len()).map_or(0, |v| v), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        assert_eq!(store.count().await.map_or(0, |v| v), 0);

        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();

        assert_eq!(store.count().await.map_or(0, |v| v), 1);
    }

    // ==========================================================================
    // InMemoryEventStore::new_arc BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_create_arc_wrapped_store() {
        let store: Arc<InMemoryEventStore> = InMemoryEventStore::new_arc();

        // Verify it's an Arc by cloning and using from multiple places
        let store2 = Arc::clone(&store);

        // Both should point to the same store
        let bead_id = BeadId::new();
        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await
            .ok();

        // Count from store2 should see the event appended via store
        let count = store2.count().await.map_or(0, |v| v);
        assert_eq!(count, 1, "Arc-wrapped stores should share state");
    }

    #[tokio::test]
    async fn should_create_empty_store_via_new_arc() {
        let store = InMemoryEventStore::new_arc();

        let count = store.count().await.map_or(999, |v| v);
        assert_eq!(count, 0, "new_arc should create empty store");
    }

    #[tokio::test]
    async fn should_create_functional_store_via_new_arc() -> Result<()> {
        // This tests that new_arc creates a FUNCTIONAL store, not just any Arc
        // Catches mutation: Arc::new(Default::default()) vs Arc::new(Self::new())
        let store = InMemoryEventStore::new_arc();
        let bead_id = BeadId::new();

        // Should be able to append events (proves it's a real InMemoryEventStore)
        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await?;

        // Should be able to read events back
        let events = store.read(None).await?;
        assert_eq!(events.len(), 1, "new_arc store should persist events");

        // Count should reflect the append
        let count = store.count().await?;
        assert_eq!(count, 1, "new_arc store should update count on append");
        Ok(())
    }

    // ==========================================================================
    // last_event_id BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_return_none_when_store_is_empty() -> Result<()> {
        let store = InMemoryEventStore::new();

        let last_id = store.last_event_id().await?;
        assert!(
            last_id.is_none(),
            "Empty store should return None for last_event_id"
        );
        Ok(())
    }

    #[tokio::test]
    async fn should_return_some_when_store_has_events() -> Result<()> {
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        let appended_id = store.append(event).await?;

        let last_id = store.last_event_id().await?;
        assert_eq!(
            last_id,
            Some(appended_id),
            "last_event_id should return the appended event's ID"
        );
        Ok(())
    }

    #[tokio::test]
    async fn should_return_most_recent_event_id() -> Result<()> {
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        // Append first event
        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("First").with_complexity(Complexity::Simple),
            ))
            .await?;

        // Append second event
        let second_id = store
            .append(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            ))
            .await?;

        let last_id = store.last_event_id().await?;
        assert_eq!(
            last_id,
            Some(second_id),
            "last_event_id should return the most recently appended event"
        );
        Ok(())
    }

    // ==========================================================================
    // read_for_bead with unknown bead BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_return_empty_vec_for_unknown_bead() -> Result<()> {
        let store = InMemoryEventStore::new();
        let unknown_bead = BeadId::new();

        let events = store.read_for_bead(unknown_bead).await?;
        assert!(events.is_empty(), "Unknown bead should return empty vec");
        Ok(())
    }

    // ==========================================================================
    // read with non-existent from_id BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_return_all_events_when_from_id_not_found() -> Result<()> {
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        // Add some events
        store
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await?;
        store
            .append(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            ))
            .await?;

        // Read from a non-existent event ID
        let fake_id = crate::types::EventId::new();
        let events = store.read(Some(fake_id)).await?;

        assert_eq!(
            events.len(),
            2,
            "Should return all events when from_id not found"
        );
        Ok(())
    }

    // ==========================================================================
    // TracingEventStore BEHAVIORAL TESTS
    // ==========================================================================

    #[tokio::test]
    async fn should_delegate_append_to_inner_store() -> Result<()> {
        let inner = InMemoryEventStore::new();
        let tracing_store = TracingEventStore::new(inner);
        let bead_id = BeadId::new();

        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        tracing_store.append(event).await?;

        // Verify the event was actually stored
        let count = tracing_store.count().await?;
        assert_eq!(count, 1, "Event should be stored via delegation");
        Ok(())
    }

    #[tokio::test]
    async fn should_delegate_read_to_inner_store() -> Result<()> {
        let inner = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        // Add event directly to inner
        inner
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await?;

        let tracing_store = TracingEventStore::new(inner);
        let events = tracing_store.read(None).await?;

        assert_eq!(events.len(), 1, "Should read events from inner store");
        Ok(())
    }

    #[tokio::test]
    async fn should_delegate_read_for_bead_to_inner_store() -> Result<()> {
        let inner = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        inner
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await?;

        let tracing_store = TracingEventStore::new(inner);
        let events = tracing_store.read_for_bead(bead_id).await?;

        assert_eq!(events.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn should_delegate_last_event_id_to_inner_store() -> Result<()> {
        let inner = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        let expected_id = inner.append(event).await?;

        let tracing_store = TracingEventStore::new(inner);
        let last_id = tracing_store.last_event_id().await?;

        assert_eq!(last_id, Some(expected_id));
        Ok(())
    }

    #[tokio::test]
    async fn should_delegate_count_to_inner_store() -> Result<()> {
        let inner = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        inner
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ))
            .await?;
        inner
            .append(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            ))
            .await?;

        let tracing_store = TracingEventStore::new(inner);
        let count = tracing_store.count().await?;

        assert_eq!(count, 2);
        Ok(())
    }

    #[tokio::test]
    async fn should_preserve_read_from_semantics_through_tracing() -> Result<()> {
        let inner = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        let first_id = inner
            .append(BeadEvent::created(
                bead_id,
                BeadSpec::new("First").with_complexity(Complexity::Simple),
            ))
            .await?;

        inner
            .append(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            ))
            .await?;

        let tracing_store = TracingEventStore::new(inner);
        let events = tracing_store.read(Some(first_id)).await?;

        assert_eq!(events.len(), 1, "Should return events after first_id");
        Ok(())
    }
}
