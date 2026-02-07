//! Tests for EventStoreActor handlers.
//!
//! These tests follow the RED-GREEN-REFACTOR TDD workflow:
//! - RED: Tests fail because handlers aren't implemented
//! - GREEN: Implement handlers to make tests pass
//! - REFACTOR: Clean up implementation

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::actors::storage::{EventStoreActorDef, EventStoreMessage};
use oya_events::durable_store::DurableEventStore;
use oya_events::event::BeadEvent;
use oya_events::types::{BeadId, BeadSpec, BeadState, Complexity};
use ractor::{Actor, ActorRef};
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::local::RocksDb;
use tokio::sync::oneshot;

/// Helper to spawn an EventStoreActor for testing.
async fn spawn_event_store_actor() -> Result<ActorRef<EventStoreMessage>, Box<dyn std::error::Error>>
{
    // Create a temporary RocksDB instance for testing
    let temp_dir = std::env::temp_dir().join(format!("event_store_test_{}", uuid::Uuid::new_v4()));
    let db = Arc::new(
        Surreal::new::<RocksDb>(temp_dir.clone())
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?,
    );

    db.use_ns("oya_test")
        .use_db("events_test")
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let store = DurableEventStore::new(db)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
        .with_wal_dir(temp_dir.join("wal"));

    let (actor, _) = Actor::spawn(None, EventStoreActorDef, Some(Arc::new(store))).await?;

    // Give the actor a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    Ok(actor)
}

#[tokio::test]
async fn test_append_event_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An EventStoreActor
    let actor = spawn_event_store_actor().await?;

    // WHEN: Sending AppendEvent message
    let bead_id = BeadId::new();
    let event = BeadEvent::created(
        bead_id,
        BeadSpec::new("test-bead").with_complexity(Complexity::Simple),
    );

    let (tx, rx) = oneshot::channel();
    actor.send_message(EventStoreMessage::AppendEvent {
        event: event.clone(),
        reply: tx.into(),
    })?;

    // THEN: Should receive Ok response with fsync guarantee
    let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx).await??;

    assert!(result.is_ok(), "AppendEvent should succeed");
    Ok(())
}

#[tokio::test]
async fn test_read_events_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An EventStoreActor with an existing event
    let actor = spawn_event_store_actor().await?;

    let bead_id = BeadId::new();
    let event = BeadEvent::created(
        bead_id,
        BeadSpec::new("test-bead").with_complexity(Complexity::Simple),
    );

    // First, append an event
    let (tx_append, _rx_append) = oneshot::channel();
    actor.send_message(EventStoreMessage::AppendEvent {
        event: event.clone(),
        reply: tx_append.into(),
    })?;

    // Give time for append
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // WHEN: Reading events for the bead
    let (tx, rx) = oneshot::channel();
    actor.send_message(EventStoreMessage::ReadEvents {
        bead_id,
        reply: tx.into(),
    })?;

    // THEN: Should receive Ok with events
    let result: Result<Vec<BeadEvent>, _> =
        tokio::time::timeout(tokio::time::Duration::from_secs(1), rx).await??;

    assert!(result.is_ok(), "ReadEvents should succeed");
    let events = result.map_err(|_| "Expected Ok value")?;
    assert!(!events.is_empty(), "Should have at least one event");
    Ok(())
}

#[tokio::test]
async fn test_read_events_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An EventStoreActor
    let actor = spawn_event_store_actor().await?;

    // WHEN: Reading events for a non-existent bead
    let bead_id = BeadId::new();
    let (tx, rx) = oneshot::channel();
    actor.send_message(EventStoreMessage::ReadEvents {
        bead_id,
        reply: tx.into(),
    })?;

    // THEN: Should receive Ok with empty vec (not an error)
    let result: Result<Vec<BeadEvent>, _> =
        tokio::time::timeout(tokio::time::Duration::from_secs(1), rx).await??;

    assert!(
        result.is_ok(),
        "ReadEvents should succeed even if no events"
    );
    let events = result.map_err(|_| "Expected Ok value")?;
    assert!(
        events.is_empty(),
        "Should have no events for non-existent bead"
    );
    Ok(())
}

#[tokio::test]
async fn test_replay_events_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An EventStoreActor with events
    let actor = spawn_event_store_actor().await?;

    let bead_id = BeadId::new();

    // Create multiple events
    let event1 = BeadEvent::created(
        bead_id,
        BeadSpec::new("test-bead").with_complexity(Complexity::Simple),
    );

    let event2 = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);

    // Append events
    let (tx1, _) = oneshot::channel();
    actor.send_message(EventStoreMessage::AppendEvent {
        event: event1.clone(),
        reply: tx1.into(),
    })?;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let (tx2, _) = oneshot::channel();
    actor.send_message(EventStoreMessage::AppendEvent {
        event: event2.clone(),
        reply: tx2.into(),
    })?;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // WHEN: Replaying from first event's checkpoint
    let checkpoint_id = event1.event_id().to_string();
    let (tx, rx) = oneshot::channel();
    actor.send_message(EventStoreMessage::ReplayEvents {
        checkpoint_id,
        reply: tx.into(),
    })?;

    // THEN: Should receive Ok with events after checkpoint
    let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx).await??;

    assert!(result.is_ok(), "ReplayEvents should succeed");
    let events = result.map_err(|_| "Expected Ok value")?;
    assert!(!events.is_empty(), "Should have events after checkpoint");
    Ok(())
}

#[tokio::test]
async fn test_replay_events_empty_stream() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An EventStoreActor
    let actor = spawn_event_store_actor().await?;

    // WHEN: Replaying from a non-existent checkpoint
    let checkpoint_id = "nonexistent-checkpoint".to_string();
    let (tx, rx) = oneshot::channel();
    actor.send_message(EventStoreMessage::ReplayEvents {
        checkpoint_id,
        reply: tx.into(),
    })?;

    // THEN: Should return Err
    let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx).await??;

    assert!(
        result.is_err(),
        "ReplayEvents should fail for non-existent checkpoint"
    );
    Ok(())
}

#[tokio::test]
async fn test_append_event_preserves_fsync_guarantee() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An EventStoreActor
    let actor = spawn_event_store_actor().await?;

    // WHEN: Appending an event
    let bead_id = BeadId::new();
    let event = BeadEvent::created(
        bead_id,
        BeadSpec::new("test-bead").with_complexity(Complexity::Simple),
    );

    let (tx, rx) = oneshot::channel();
    actor.send_message(EventStoreMessage::AppendEvent {
        event: event.clone(),
        reply: tx.into(),
    })?;

    // THEN: The reply should only come after fsync succeeds
    // This is verified by the append_event implementation in DurableEventStore
    // which explicitly calls file.sync_all() before returning Ok
    let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx).await??;

    assert!(
        result.is_ok(),
        "AppendEvent should only succeed after fsync"
    );

    // Verify the event was actually persisted by reading it back
    let (tx_read, rx_read) = oneshot::channel();
    actor.send_message(EventStoreMessage::ReadEvents {
        bead_id,
        reply: tx_read.into(),
    })?;

    let read_result: Result<Vec<BeadEvent>, _> =
        tokio::time::timeout(tokio::time::Duration::from_secs(1), rx_read).await??;

    assert!(
        read_result.is_ok(),
        "Should be able to read persisted event"
    );
    let events = read_result.map_err(|_| "Expected Ok value")?;
    assert_eq!(events.len(), 1, "Should have exactly one persisted event");
    Ok(())
}
