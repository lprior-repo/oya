//! Test for lazy evaluation cache implementation in ProcessPoolActor

use std::sync::Arc;

use oya_pipeline::actor::{ProcessPoolActor, ProcessId, WorkerState};

#[test]
fn test_cache_initialization() {
    let pool = ProcessPoolActor::new();

    // Cache should be empty initially
    assert_eq!(pool.size(), 0);
    assert_eq!(pool.available_count(), 0);
    assert_eq!(pool.busy_count(), 0);
    assert_eq!(pool.needing_attention_count(), 0);
}

#[test]
fn test_cache_populates_on_first_access() {
    let mut pool = ProcessPoolActor::new();

    // Add some workers
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();
    pool.add_worker(ProcessId::new(2), WorkerState::Claimed).unwrap();
    pool.add_worker(ProcessId::new(3), WorkerState::Unhealthy).unwrap();

    // First access should populate cache
    let stats = pool.get_stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.available, 1);  // Only worker 1 is Idle
    assert_eq!(stats.busy, 1);      // Only worker 2 is Claimed
    assert_eq!(stats.needing_attention, 1);  // Worker 3 is Unhealthy

    // Subsequent accesses should use cache
    assert_eq!(pool.available_count(), 1);
    assert_eq!(pool.busy_count(), 1);
    assert_eq!(pool.needing_attention_count(), 1);
}

#[test]
fn test_cache_invalidation_on_worker_add() {
    let mut pool = ProcessPoolActor::new();

    // Add initial workers
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();

    // Access cache to populate it
    let _stats = pool.get_stats();
    assert_eq!(pool.available_count(), 1);

    // Add new worker - should invalidate cache
    pool.add_worker(ProcessId::new(2), WorkerState::Claimed).unwrap();

    // Cache should be repopulated
    assert_eq!(pool.size(), 2);
    assert_eq!(pool.available_count(), 1);  // Only worker 1 is Idle
    assert_eq!(pool.busy_count(), 1);      // Only worker 2 is Claimed
}

#[test]
fn test_cache_invalidation_on_worker_remove() {
    let mut pool = ProcessPoolActor::new();

    // Add workers
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();
    pool.add_worker(ProcessId::new(2), WorkerState::Claimed).unwrap();

    // Access cache to populate it
    let _stats = pool.get_stats();
    assert_eq!(pool.available_count(), 1);

    // Remove worker - should invalidate cache
    let removed = pool.remove_worker(&ProcessId::new(1));
    assert_eq!(removed, Some(WorkerState::Idle));

    // Cache should be repopulated
    assert_eq!(pool.size(), 1);
    assert_eq!(pool.available_count(), 0);  // No Idle workers remaining
    assert_eq!(pool.busy_count(), 1);      // Worker 2 is still Claimed
}

#[test]
fn test_cache_invalidation_on_state_update() {
    let mut pool = ProcessPoolActor::new();

    // Add idle worker
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();

    // Access cache to populate it
    let _stats = pool.get_stats();
    assert_eq!(pool.available_count(), 1);
    assert_eq!(pool.busy_count(), 0);

    // Update state to claimed - should invalidate cache
    pool.update_state(&ProcessId::new(1), WorkerState::Claimed).unwrap();

    // Cache should be repopulated
    assert_eq!(pool.available_count(), 0);  // No Idle workers
    assert_eq!(pool.busy_count(), 1);      // Worker 1 is now busy
}

#[test]
fn test_cache_invalidation_on_claim_worker() {
    let mut pool = ProcessPoolActor::new();

    // Add idle worker
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();

    // Access cache to populate it
    let _stats = pool.get_stats();
    assert_eq!(pool.available_count(), 1);
    assert_eq!(pool.busy_count(), 0);

    // Claim worker - should invalidate cache
    pool.claim_worker(ProcessId::new(1)).unwrap();

    // Cache should be repopulated
    assert_eq!(pool.available_count(), 0);  // No Idle workers
    assert_eq!(pool.busy_count(), 1);      // Worker 1 is now busy
}

#[test]
fn test_cache_invalidation_on_release_worker() {
    let mut pool = ProcessPoolActor::new();

    // Add claimed worker
    pool.add_worker(ProcessId::new(1), WorkerState::Claimed).unwrap();

    // Access cache to populate it
    let _stats = pool.get_stats();
    assert_eq!(pool.available_count(), 0);
    assert_eq!(pool.busy_count(), 1);

    // Release worker - should invalidate cache
    pool.release_worker(ProcessId::new(1)).unwrap();

    // Cache should be repopulated
    assert_eq!(pool.available_count(), 1);  // Worker 1 is now Idle
    assert_eq!(pool.busy_count(), 0);      // No busy workers
}

#[test]
fn test_cache_invalidation_on_clear() {
    let mut pool = ProcessPoolActor::new();

    // Add workers
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();
    pool.add_worker(ProcessId::new(2), WorkerState::Claimed).unwrap();

    // Access cache to populate it
    let _stats = pool.get_stats();
    assert_eq!(pool.available_count(), 1);
    assert_eq!(pool.busy_count(), 1);

    // Clear pool - should invalidate cache
    pool.clear();

    // Cache should be repopulated with empty values
    assert_eq!(pool.size(), 0);
    assert_eq!(pool.available_count(), 0);
    assert_eq!(pool.busy_count(), 0);
    assert_eq!(pool.needing_attention_count(), 0);
}

#[test]
fn test_cache_consistency_with_count_by_state() {
    let mut pool = ProcessPoolActor::new();

    // Add workers in various states
    pool.add_worker(ProcessId::new(1), WorkerState::Idle).unwrap();
    pool.add_worker(ProcessId::new(2), WorkerState::Idle).unwrap();
    pool.add_worker(ProcessId::new(3), WorkerState::Claimed).unwrap();
    pool.add_worker(ProcessId::new(4), WorkerState::Unhealthy).unwrap();
    pool.add_worker(ProcessId::new(5), WorkerState::Dead).unwrap();

    // Use cached methods
    assert_eq!(pool.available_count(), 2);  // Two Idle workers
    assert_eq!(pool.busy_count(), 1);      // One Claimed worker
    assert_eq!(pool.needing_attention_count(), 2);  // One Unhealthy + one Dead

    // Verify with non-cached method
    assert_eq!(pool.count_by_state(WorkerState::Idle), 2);
    assert_eq!(pool.count_by_state(WorkerState::Claimed), 1);
    assert_eq!(pool.count_by_state(WorkerState::Unhealthy), 1);
    assert_eq!(pool.count_by_state(WorkerState::Dead), 1);
}

#[test]
fn test_cache_lazy_evaluation_performance() {
    let mut pool = ProcessPoolActor::new();

    // Add many workers
    for i in 0..1000 {
        let state = if i % 4 == 0 {
            WorkerState::Idle
        } else if i % 4 == 1 {
            WorkerState::Claimed
        } else if i % 4 == 2 {
            WorkerState::Unhealthy
        } else {
            WorkerState::Dead
        };
        pool.add_worker(ProcessId::new(i as u64), state).unwrap();
    }

    // Multiple calls to cached methods should be fast (use cache)
    let start = std::time::Instant::now();
    for _ in 0..100 {
        assert_eq!(pool.available_count(), 250);  // 1000 / 4
        assert_eq!(pool.busy_count(), 250);
        assert_eq!(pool.needing_attention_count(), 500);
    }
    let duration = start.elapsed();

    // Should be very fast (less than 1ms for 100 calls)
    println!("Cache performance test took: {:?}", duration);
    assert!(duration.as_millis() < 10, "Cache should be fast");
}