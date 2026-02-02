//! Durable event store with filtering capabilities.

use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::event::BeadEvent;
use crate::types::{BeadId, EventId};

/// Query parameters for event retrieval.
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    /// Filter by stream (bead) ID.
    pub stream_id: Option<BeadId>,
    /// Filter by event type.
    pub event_type: Option<String>,
    /// Filter events after this timestamp.
    pub after: Option<DateTime<Utc>>,
    /// Filter events before this timestamp.
    pub before: Option<DateTime<Utc>>,
    /// Limit the number of results.
    pub limit: Option<usize>,
}

impl EventQuery {
    /// Create a new query builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by stream (bead) ID.
    pub fn with_stream_id(mut self, stream_id: BeadId) -> Self {
        self.stream_id = Some(stream_id);
        self
    }

    /// Filter by event type.
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    /// Filter events after this timestamp.
    pub fn after(mut self, timestamp: DateTime<Utc>) -> Self {
        self.after = Some(timestamp);
        self
    }

    /// Filter events before this timestamp.
    pub fn before(mut self, timestamp: DateTime<Utc>) -> Self {
        self.before = Some(timestamp);
        self
    }

    /// Limit the number of results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Apply filters to events using iterator combinators.
    pub fn filter_events<'a>(
        &'a self,
        events: impl Iterator<Item = &'a BeadEvent> + 'a,
    ) -> impl Iterator<Item = &'a BeadEvent> + 'a {
        events
            .filter(move |event| {
                // Filter by stream_id
                if let Some(ref sid) = self.stream_id {
                    if event.bead_id() != *sid {
                        return false;
                    }
                }

                // Filter by event_type
                if let Some(ref etype) = self.event_type {
                    if event.event_type() != etype.as_str() {
                        return false;
                    }
                }

                // Filter by after timestamp
                if let Some(ref after) = self.after {
                    if event.timestamp() <= *after {
                        return false;
                    }
                }

                // Filter by before timestamp
                if let Some(ref before) = self.before {
                    if event.timestamp() >= *before {
                        return false;
                    }
                }

                true
            })
            .take(self.limit.unwrap_or(usize::MAX))
    }
}

/// Durable event store with query and filtering capabilities.
pub struct DurableEventStore {
    events: Vec<BeadEvent>,
}

impl DurableEventStore {
    /// Create a new durable event store.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append an event to the store.
    pub fn append(&mut self, event: BeadEvent) -> Result<EventId> {
        let event_id = event.event_id();
        self.events.push(event);
        Ok(event_id)
    }

    /// Get all events for a specific stream (bead).
    pub fn get_events(&self, stream_id: BeadId) -> Result<Vec<BeadEvent>> {
        Ok(self
            .events
            .iter()
            .filter(|e| e.bead_id() == stream_id)
            .cloned()
            .collect())
    }

    /// Get events since a specific event ID (sequence number filtering).
    pub fn get_events_since(&self, since: EventId) -> Result<Vec<BeadEvent>> {
        // Find position of the since event
        let pos = self
            .events
            .iter()
            .position(|e| e.event_id() == since)
            .map(|p| p + 1) // Start from next event
            .unwrap_or(0); // If not found, return all

        Ok(self.events[pos..].to_vec())
    }

    /// Query events with filtering.
    pub fn query(&self, query: &EventQuery) -> Result<Vec<BeadEvent>> {
        Ok(query.filter_events(self.events.iter()).cloned().collect())
    }

    /// Get total count of events.
    pub fn count(&self) -> usize {
        self.events.len()
    }
}

impl Default for DurableEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BeadSpec, BeadState, Complexity};

    #[test]
    fn test_append_and_count() {
        let mut store = DurableEventStore::new();
        assert_eq!(store.count(), 0);

        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);
        let event = BeadEvent::created(bead_id, spec);

        let result = store.append(event);
        assert!(result.is_ok());
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_get_events_for_stream() {
        let mut store = DurableEventStore::new();
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();

        // Add events for bead1
        let _ = store.append(BeadEvent::created(
            bead1,
            BeadSpec::new("Bead 1").with_complexity(Complexity::Simple),
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead1,
            BeadState::Pending,
            BeadState::Scheduled,
        ));

        // Add event for bead2
        let _ = store.append(BeadEvent::created(
            bead2,
            BeadSpec::new("Bead 2").with_complexity(Complexity::Medium),
        ));

        // Get events for bead1
        let events = store.get_events(bead1);
        assert!(events.is_ok());
        if let Ok(events) = events {
            assert_eq!(events.len(), 2);
            assert!(events.iter().all(|e| e.bead_id() == bead1));
        }

        // Get events for bead2
        let events = store.get_events(bead2);
        assert!(events.is_ok());
        if let Ok(events) = events {
            assert_eq!(events.len(), 1);
            assert!(events.iter().all(|e| e.bead_id() == bead2));
        }
    }

    #[test]
    fn test_get_events_since() {
        let mut store = DurableEventStore::new();
        let bead_id = BeadId::new();

        let event1 = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        let event1_id = store.append(event1.clone());
        let event1_id = match event1_id {
            Ok(id) => id,
            Err(_) => return,
        };

        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Scheduled,
            BeadState::Ready,
        ));

        // Get events since event1
        let events = store.get_events_since(event1_id);
        assert!(events.is_ok());
        if let Ok(events) = events {
            // Should return 2 events (after event1)
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].event_type(), "state_changed");
        }
    }

    #[test]
    fn test_query_by_event_type() {
        let mut store = DurableEventStore::new();
        let bead_id = BeadId::new();

        let _ = store.append(BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Scheduled,
            BeadState::Ready,
        ));

        // Query for state_changed events
        let query = EventQuery::new().with_event_type("state_changed");
        let events = store.query(&query);
        assert!(events.is_ok());
        if let Ok(events) = events {
            assert_eq!(events.len(), 2);
            assert!(events.iter().all(|e| e.event_type() == "state_changed"));
        }
    }

    #[test]
    fn test_query_by_timestamp() {
        let mut store = DurableEventStore::new();
        let bead_id = BeadId::new();

        // Create events with some delay
        let event1 = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        let timestamp1 = event1.timestamp();
        let _ = store.append(event1);

        // Add a small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));

        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        ));

        // Query for events after timestamp1
        let query = EventQuery::new().after(timestamp1);
        let events = store.query(&query);
        assert!(events.is_ok());
        if let Ok(events) = events {
            // Should return 1 event (after timestamp1)
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_type(), "state_changed");
        }
    }

    #[test]
    fn test_query_with_limit() {
        let mut store = DurableEventStore::new();
        let bead_id = BeadId::new();

        let _ = store.append(BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead_id,
            BeadState::Scheduled,
            BeadState::Ready,
        ));

        // Query with limit of 2
        let query = EventQuery::new().limit(2);
        let events = store.query(&query);
        assert!(events.is_ok());
        if let Ok(events) = events {
            assert_eq!(events.len(), 2);
        }
    }

    #[test]
    fn test_query_combined_filters() {
        let mut store = DurableEventStore::new();
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();

        // Add events for bead1
        let _ = store.append(BeadEvent::created(
            bead1,
            BeadSpec::new("Bead 1").with_complexity(Complexity::Simple),
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead1,
            BeadState::Pending,
            BeadState::Scheduled,
        ));

        // Add events for bead2
        let _ = store.append(BeadEvent::created(
            bead2,
            BeadSpec::new("Bead 2").with_complexity(Complexity::Medium),
        ));
        let _ = store.append(BeadEvent::state_changed(
            bead2,
            BeadState::Pending,
            BeadState::Scheduled,
        ));

        // Query for state_changed events for bead1
        let query = EventQuery::new()
            .with_stream_id(bead1)
            .with_event_type("state_changed");
        let events = store.query(&query);
        assert!(events.is_ok());
        if let Ok(events) = events {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].bead_id(), bead1);
            assert_eq!(events[0].event_type(), "state_changed");
        }
    }

    #[test]
    fn test_event_query_filter_events() {
        let bead_id = BeadId::new();
        let events = [
            BeadEvent::created(
                bead_id,
                BeadSpec::new("Test").with_complexity(Complexity::Simple),
            ),
            BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled),
            BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready),
        ];

        // Test filtering by event type
        let query = EventQuery::new().with_event_type("state_changed");
        let filtered: Vec<_> = query.filter_events(events.iter()).collect();
        assert_eq!(filtered.len(), 2);

        // Test filtering with limit
        let query = EventQuery::new().limit(2);
        let filtered: Vec<_> = query.filter_events(events.iter()).collect();
        assert_eq!(filtered.len(), 2);
    }

    // ==========================================================================
    // EventQuery Builder BEHAVIORAL TESTS (to catch mutation gaps)
    // ==========================================================================

    #[test]
    fn should_set_before_timestamp_in_builder() {
        use chrono::Utc;

        let timestamp = Utc::now();
        let query = EventQuery::new().before(timestamp);

        assert_eq!(
            query.before,
            Some(timestamp),
            "before() should set the before field"
        );
    }

    #[test]
    fn should_filter_events_before_timestamp() {
        use chrono::{Duration, Utc};

        let bead_id = BeadId::new();
        let now = Utc::now();
        let past = now - Duration::seconds(10);
        let future = now + Duration::seconds(10);

        // Create events at different times (simulated via the created event)
        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );

        let events = [event];

        // Query for events before a timestamp in the far future - should include the event
        let query = EventQuery::new().before(future);
        let filtered: Vec<_> = query.filter_events(events.iter()).collect();
        assert_eq!(
            filtered.len(),
            1,
            "Event before future timestamp should be included"
        );

        // Query for events before a timestamp in the past - should exclude the event
        let query = EventQuery::new().before(past);
        let filtered: Vec<_> = query.filter_events(events.iter()).collect();
        assert_eq!(
            filtered.len(),
            0,
            "Event at/after past timestamp should be excluded"
        );
    }

    #[test]
    fn should_exclude_events_at_exact_before_timestamp() {
        let bead_id = BeadId::new();
        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        let exact_time = event.timestamp();

        let events = [event];

        // Query for events BEFORE the exact timestamp - should exclude (>= means at or after is excluded)
        let query = EventQuery::new().before(exact_time);
        let filtered: Vec<_> = query.filter_events(events.iter()).collect();
        assert_eq!(
            filtered.len(),
            0,
            "Event at exact before timestamp should be excluded"
        );
    }

    #[test]
    fn should_exclude_events_at_exact_after_timestamp() {
        let bead_id = BeadId::new();
        let event = BeadEvent::created(
            bead_id,
            BeadSpec::new("Test").with_complexity(Complexity::Simple),
        );
        let exact_time = event.timestamp();

        let events = [event];

        // Query for events AFTER the exact timestamp - should exclude (<= means at or before is excluded)
        let query = EventQuery::new().after(exact_time);
        let filtered: Vec<_> = query.filter_events(events.iter()).collect();
        assert_eq!(
            filtered.len(),
            0,
            "Event at exact after timestamp should be excluded"
        );
    }

    #[test]
    fn should_return_builder_self_from_before() {
        use chrono::Utc;

        // Verify builder chaining works (catches Default::default() mutation)
        let timestamp = Utc::now();
        let query = EventQuery::new()
            .with_event_type("created")
            .before(timestamp)
            .limit(10);

        assert_eq!(query.event_type, Some("created".to_string()));
        assert_eq!(query.before, Some(timestamp));
        assert_eq!(query.limit, Some(10));
    }
}
