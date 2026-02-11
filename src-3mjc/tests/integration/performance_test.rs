//! Performance validation tests for event sourcing.
//!
//! This test module validates that event replay performance meets
//! the required thresholds for production use.
//!
//! ## Performance Targets
//!
//! - **1000 events replay**: Must complete in <5 seconds
//! - **Throughput**: Must maintain >200 events/second
//!
//! ## Test Strategy
//!
//! These tests use TDD approach:
//! 1. Create 1000 diverse events (mix of all event types)
//! 2. Store them in DurableEventStore
//! 3. Replay all events and measure time
//! 4. Assert performance targets are met
//! 5. Report throughput metrics

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_events::{
    connect, ConnectionConfig, DurableEventStore, BeadEvent, BeadId, BeadSpec,
    BeadState, BeadResult, PhaseId, PhaseOutput, Complexity, EventId,
};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Number of events to use for performance testing.
const EVENT_COUNT: usize = 1000;

/// Maximum allowed duration for replaying 1000 events (5 seconds).
const MAX_REPLAY_DURATION: Duration = Duration::from_secs(5);

/// Minimum required throughput (events per second).
const MIN_THROUGHPUT: f64 = 200.0;

/// Helper struct to hold test context.
struct TestContext {
    _temp_dir: TempDir,
    store: DurableEventStore,
    bead_ids: Vec<BeadId>,
}

impl TestContext {
    /// Create a new test context with a temporary database.
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("events");

        let config = ConnectionConfig::new(storage_path)
            .with_namespace("performance_test")
            .with_database("events");

        let db = connect(config).await?;
        let store = DurableEventStore::new(db).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            store,
            bead_ids: Vec::new(),
        })
    }

    /// Generate a diverse set of 1000 events.
    async fn generate_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create 100 bead IDs to work with
        self.bead_ids = (0..100)
            .map(|_| BeadId::new())
            .collect();

        // Generate 1000 diverse events
        for i in 0..EVENT_COUNT {
            let bead_id = self.bead_ids[i % 100];
            let event = self.create_event(i, bead_id)?;
            self.store.append_event(&event).await?;
        }

        Ok(())
    }

    /// Create a diverse event based on index.
    fn create_event(&self, index: usize, bead_id: BeadId) -> Result<BeadEvent, Box<dyn std::error::Error>> {
        match index % 10 {
            0 => Ok(BeadEvent::created(
                bead_id,
                BeadSpec::new(&format!("Task {}", index))
                    .with_complexity(Complexity::Medium)
                    .with_priority(50 + (index % 50) as u32)
            )),
            1 => Ok(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled
            )),
            2 => Ok(BeadEvent::state_changed(
                bead_id,
                BeadState::Scheduled,
                BeadState::Ready
            )),
            3 => Ok(BeadEvent::claimed(bead_id, &format!("agent-{}", index % 10))),
            4 => {
                let phase_id = PhaseId::new();
                Ok(BeadEvent::phase_completed(
                    bead_id,
                    phase_id,
                    &format!("phase-{}", index % 5),
                    PhaseOutput::success(vec![1, 2, 3, 4])
                ))
            },
            5 => Ok(BeadEvent::state_changed(
                bead_id,
                BeadState::Ready,
                BeadState::Running
            )),
            6 => {
                let phase_id = PhaseId::new();
                Ok(BeadEvent::phase_completed(
                    bead_id,
                    phase_id,
                    &format!("phase-{}", index % 5),
                    PhaseOutput::success(vec![5, 6, 7, 8])
                ))
            },
            7 => Ok(BeadEvent::dependency_resolved(
                bead_id,
                self.bead_ids[(index + 1) % 100]
            )),
            8 => Ok(BeadEvent::completed(
                bead_id,
                BeadResult::success(vec![9, 10], 1000 + index as u64)
            )),
            9 => Ok(BeadEvent::priority_changed(
                bead_id,
                50,
                100
            )),
            _ => Err("Invalid event index".into()),
        }
    }

    /// Replay all events and measure performance.
    async fn replay_and_measure(&self) -> Result<PerformanceMetrics, Box<dyn std::error::Error>> {
        let start = Instant::now();

        // Replay events for all beads
        let mut total_events = 0;
        for bead_id in &self.bead_ids {
            let events = self.store.read_events(bead_id).await?;
            total_events += events.len();
        }

        let duration = start.elapsed();

        Ok(PerformanceMetrics {
            event_count: total_events,
            duration,
            throughput: if duration.as_secs_f64() > 0.0 {
                total_events as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
        })
    }
}

/// Performance metrics from replay operation.
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    event_count: usize,
    duration: Duration,
    throughput: f64,
}

impl PerformanceMetrics {
    /// Check if performance meets requirements.
    fn meets_requirements(&self) -> bool {
        self.duration <= MAX_REPLAY_DURATION && self.throughput >= MIN_THROUGHPUT
    }

    /// Get a human-readable report.
    fn report(&self) -> String {
        format!(
            "Performance Report:\n\
             - Events replayed: {}\n\
             - Duration: {:.2}s\n\
             - Throughput: {:.2} events/sec\n\
             - Target: <{}s, >{} events/sec\n\
             - Status: {}",
            self.event_count,
            self.duration.as_secs_f64(),
            self.throughput,
            MAX_REPLAY_DURATION.as_secs(),
            MIN_THROUGHPUT,
            if self.meets_requirements() { "PASS" } else { "FAIL" }
        )
    }
}

#[tokio::test]
async fn test_replay_1000_events_completes_under_5_seconds() -> Result<(), Box<dyn std::error::Error>> {
    // Setup: Create test context and generate events
    let mut context = TestContext::new().await?;
    context.generate_events().await?;

    // Exercise: Replay events and measure performance
    let metrics = context.replay_and_measure().await?;

    // Verify: Assert performance requirements
    assert!(
        metrics.duration <= MAX_REPLAY_DURATION,
        "Performance regression: replay took {:.2}s, expected <{}s\n{}",
        metrics.duration.as_secs_f64(),
        MAX_REPLAY_DURATION.as_secs(),
        metrics.report()
    );

    // Verify: Assert throughput requirements
    assert!(
        metrics.throughput >= MIN_THROUGHPUT,
        "Throughput regression: {:.2} events/sec, expected >{} events/sec\n{}",
        metrics.throughput,
        MIN_THROUGHPUT,
        metrics.report()
    );

    // Verify: Assert all events were replayed
    assert_eq!(
        metrics.event_count, EVENT_COUNT,
        "Event count mismatch: expected {}, got {}",
        EVENT_COUNT, metrics.event_count
    );

    println!("{}", metrics.report());
    Ok(())
}

#[tokio::test]
async fn test_performance_metrics_are_accurate() -> Result<(), Box<dyn std::error::Error>> {
    // This test validates that our performance measurement logic is correct
    let mut context = TestContext::new().await?;

    // Generate a smaller set for quick validation
    for i in 0..50 {
        let bead_id = BeadId::new();
        let event = context.create_event(i, bead_id)?;
        context.store.append_event(&event).await?;
        context.bead_ids.push(bead_id);
    }

    let metrics = context.replay_and_measure().await?;

    // Verify count accuracy
    assert_eq!(
        metrics.event_count, 50,
        "Should replay exactly 50 events"
    );

    // Verify duration is reasonable (should be very fast for 50 events)
    assert!(
        metrics.duration < Duration::from_secs(1),
        "50 events should complete in <1s, took {:.2}s",
        metrics.duration.as_secs_f64()
    );

    // Verify throughput calculation
    if metrics.duration.as_secs_f64() > 0.0 {
        let expected_throughput = 50.0 / metrics.duration.as_secs_f64();
        assert!(
            (metrics.throughput - expected_throughput).abs() < 0.01,
            "Throughput calculation mismatch: expected {:.2}, got {:.2}",
            expected_throughput, metrics.throughput
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_replay_handles_empty_store_gracefully() -> Result<(), Box<dyn std::error::Error>> {
    // Test edge case: empty store should return instantly
    let context = TestContext::new().await?;

    let start = Instant::now();
    let metrics = context.replay_and_measure().await?;
    let duration = start.elapsed();

    assert_eq!(metrics.event_count, 0, "Empty store should have 0 events");
    assert!(
        duration < Duration::from_millis(100),
        "Empty replay should complete in <100ms, took {:.2}ms",
        duration.as_millis()
    );

    Ok(())
}

#[tokio::test]
async fn test_performance_is_consistent_across_multiple_runs() -> Result<(), Box<dyn std::error::Error>> {
    // This test runs the replay multiple times to ensure consistency
    let mut context = TestContext::new().await?;
    context.generate_events().await?;

    let mut durations = Vec::new();
    let mut throughputs = Vec::new();

    // Run 3 times to check consistency
    for _ in 0..3 {
        let metrics = context.replay_and_measure().await?;
        durations.push(metrics.duration);
        throughputs.push(metrics.throughput);
    }

    // All runs should meet performance requirements
    for (i, duration) in durations.iter().enumerate() {
        assert!(
            duration <= &MAX_REPLAY_DURATION,
            "Run {} failed: duration {:.2}s exceeds {}s",
            i + 1,
            duration.as_secs_f64(),
            MAX_REPLAY_DURATION.as_secs()
        );
    }

    // Throughput should be relatively consistent (within 50% variation)
    let max_throughput = throughputs.iter().fold(f64::NAN, |a, &b| a.max(b));
    let min_throughput = throughputs.iter().fold(f64::NAN, |a, &b| a.min(b));

    if max_throughput > 0.0 && min_throughput > 0.0 {
        let variation = (max_throughput - min_throughput) / max_throughput;
        assert!(
            variation < 0.5,
            "Throughput variation too high: {:.1}%, min={:.2}, max={:.2}",
            variation * 100.0,
            min_throughput,
            max_throughput
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_replay_maintains_event_order() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that replay preserves event ordering
    let mut context = TestContext::new().await?;

    // Create events with known timestamps
    let bead_id = BeadId::new();
    let event_ids = (0..10)
        .map(|_| {
            let event = BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled
            );
            let id = event.event_id();
            context.store.append_event(&event)
                .await
                .map(|_| id)
        })
        .collect::<Result<Vec<_>, _>>()?;

    context.bead_ids.push(bead_id);

    // Replay and verify order
    let events = context.store.read_events(&bead_id).await?;

    assert_eq!(
        events.len(),
        10,
        "Should replay all 10 events"
    );

    // Verify events are in the same order as created
    for (i, event) in events.iter().enumerate() {
        assert_eq!(
            event.event_id(),
            event_ids[i],
            "Event {} has wrong ID",
            i
        );
    }

    Ok(())
}

#[cfg(test)]
mod performance_benchmarks {
    use super::*;

    #[tokio::test]
    async fn benchmark_event_append_throughput() -> Result<(), Box<dyn std::error::Error>> {
        // Measure pure append throughput
        let context = TestContext::new().await?;

        let bead_id = BeadId::new();
        let start = Instant::now();

        for i in 0..EVENT_COUNT {
            let event = BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled
            );
            context.store.append_event(&event).await?;
        }

        let duration = start.elapsed();
        let append_throughput = EVENT_COUNT as f64 / duration.as_secs_f64();

        println!("Append throughput: {:.2} events/sec", append_throughput);

        // Append should be fast (>500 events/sec)
        assert!(
            append_throughput > 500.0,
            "Append throughput too low: {:.2} events/sec",
            append_throughput
        );

        Ok(())
    }

    #[tokio::test]
    async fn benchmark_single_bead_replay() -> Result<(), Box<dyn std::error::Error>> {
        // Benchmark replay of a single bead with many events
        let context = TestContext::new().await?;

        let bead_id = BeadId::new();

        // Append 1000 events for a single bead
        for i in 0..EVENT_COUNT {
            let event = BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                if i % 2 == 0 { BeadState::Scheduled } else { BeadState::Ready }
            );
            context.store.append_event(&event).await?;
        }

        // Measure replay time
        let start = Instant::now();
        let events = context.store.read_events(&bead_id).await?;
        let duration = start.elapsed();

        assert_eq!(events.len(), EVENT_COUNT);

        let throughput = EVENT_COUNT as f64 / duration.as_secs_f64();
        println!("Single bead replay throughput: {:.2} events/sec", throughput);

        assert!(
            duration <= MAX_REPLAY_DURATION,
            "Single bead replay took {:.2}s, expected <{}s",
            duration.as_secs_f64(),
            MAX_REPLAY_DURATION.as_secs()
        );

        Ok(())
    }
}
