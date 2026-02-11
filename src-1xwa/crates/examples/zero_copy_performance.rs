use oya_events::{InMemoryEventStore, BeadEvent, BeadId, BeadSpec, Complexity};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("Testing zero-copy optimization in event store...");

    // Create a store and add many events
    let store = Arc::new(InMemoryEventStore::new());
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Performance Test").with_complexity(Complexity::Simple);

    // Add 1000 events
    println!("Adding 1000 events...");
    let start = Instant::now();
    for i in 0..1000 {
        let event = BeadEvent::created(
            bead_id,
            spec.clone().with_name(format!("Event {}", i)),
        );
        store.append(event).await.unwrap();
    }
    let append_time = start.elapsed();
    println!("Append time: {:?}", append_time);

    // Test event retrieval - this should now be zero-copy
    println!("Testing event retrieval...");
    let start = Instant::now();
    let events = store.read_for_bead(bead_id).await.unwrap();
    let read_time = start.elapsed();
    println!("Retrieved {} events in {:?}", events.len(), read_time);

    // Test multiple retrievals - should be fast due to Arc sharing
    println!("Testing multiple retrievals...");
    let start = Instant::now();
    for _ in 0..100 {
        let events_clone = store.read_for_bead(bead_id).await.unwrap();
        // Arc::clone is O(1), so this should be very fast
        assert_eq!(events_clone.len(), 1000);
    }
    let multi_read_time = start.elapsed();
    println!("100 retrievals in {:?}", multi_read_time);

    // Verify we're getting the same events (zero-copy)
    let first_retrieval = store.read_for_bead(bead_id).await.unwrap();
    let second_retrieval = store.read_for_bead(bead_id).await.unwrap();

    // These should be the same Arc instance (same pointer)
    let first_ptr = Arc::as_ptr(&first_retrieval) as *const ();
    let second_ptr = Arc::as_ptr(&second_retrieval) as *const ();
    println!("Same Arc instance: {}", first_ptr == second_ptr);

    println!("\nPerformance Summary:");
    println!("  - Append 1000 events: {:?}", append_time);
    println!("  - Read 1000 events: {:?}", read_time);
    println!("  - 100x reads (zero-copy): {:?}", multi_read_time);
    println!("  - Average read time: {:?}", multi_read_time / 100);
}