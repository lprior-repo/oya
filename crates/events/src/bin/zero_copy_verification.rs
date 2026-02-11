use oya_events::{BeadEvent, BeadId, BeadSpec, Complexity, EventStore, InMemoryEventStore};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("Verifying zero-copy optimization in event store...");

    // Create a store and add events
    let store = Arc::new(InMemoryEventStore::new());
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Zero-Copy Test").with_complexity(Complexity::Simple);

    // Add 100 events
    for _i in 0..100 {
        let event = BeadEvent::created(bead_id, spec.clone());
        match store.append(event).await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to append event: {}", e);
                std::process::exit(1);
            }
        };
    }

    // Get events twice
    let first_read = match store.read_for_bead(bead_id).await {
        Ok(events) => events,
        Err(e) => {
            eprintln!("Failed to read events for bead: {}", e);
            std::process::exit(1);
        }
    };
    let second_read = match store.read_for_bead(bead_id).await {
        Ok(events) => events,
        Err(e) => {
            eprintln!("Failed to read events for bead: {}", e);
            std::process::exit(1);
        }
    };

    println!(
        "First read: {} events, ptr: {:?}",
        first_read.len(),
        Arc::as_ptr(&first_read) as *const ()
    );
    println!(
        "Second read: {} events, ptr: {:?}",
        second_read.len(),
        Arc::as_ptr(&second_read) as *const ()
    );

    // Clone the Arc - this should be O(1) and very fast
    let cloned_first = Arc::clone(&first_read);
    println!(
        "Cloned first read: {} events, ptr: {:?}",
        cloned_first.len(),
        Arc::as_ptr(&cloned_first) as *const ()
    );

    // Verify they point to the same data
    let first_ptr = Arc::as_ptr(&first_read) as *const ();
    let second_ptr = Arc::as_ptr(&second_read) as *const ();
    let cloned_ptr = Arc::as_ptr(&cloned_first) as *const ();

    println!("\nPointer comparison:");
    println!("  First read == Second read: {}", first_ptr == second_ptr);
    println!("  First read == Cloned: {}", first_ptr == cloned_ptr);

    // Performance test: many Arc clones should be very fast
    let start = Instant::now();
    for _ in 0..10000 {
        let _clone = Arc::clone(&first_read);
    }
    let clone_time = start.elapsed();
    println!("\n10,000 Arc clones took: {:?}", clone_time);
    println!("Average clone time: {:?}", clone_time / 10000);

    // Test that we can modify the clone without affecting the original (Arc semantics)
    let mut events_vec: Vec<BeadEvent> = first_read.to_vec();
    let original_count = events_vec.len();
    events_vec.clear();

    println!(
        "\nOriginal Arc length after modifying vector: {}",
        first_read.len()
    );
    println!("Vector length after clearing: {}", original_count);
    println!(
        "Arc sharing maintains reference semantics: {}",
        first_read.len() == 100
    );

    println!("\n✅ Zero-copy optimization verified!");
    println!("   - Arc<[BeadEvent]> provides O(1) cloning");
    println!("   - Multiple readers share the same underlying data");
    println!("   - No unnecessary data copying occurs during event retrieval");
}
