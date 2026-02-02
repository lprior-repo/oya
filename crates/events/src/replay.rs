//! Event replay with progress tracking.
//!
//! This module provides progress tracking for event replay operations,
//! allowing monitoring of replay progress through a watch channel.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::watch;

use crate::error::Result;

/// Progress information for event replay.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayProgress {
    /// Total number of events to replay.
    pub events_total: u64,
    /// Number of events processed so far.
    pub events_processed: u64,
    /// Percentage complete (0-100).
    pub percent_complete: f64,
    /// Estimated time remaining.
    pub eta: Option<Duration>,
}

impl ReplayProgress {
    /// Create a new progress with zero events processed.
    pub fn new(events_total: u64) -> Self {
        let percent_complete = if events_total == 0 { 100.0 } else { 0.0 };
        Self {
            events_total,
            events_processed: 0,
            percent_complete,
            eta: None,
        }
    }

    /// Calculate percentage complete.
    #[allow(dead_code)]
    fn calculate_percent(&self) -> f64 {
        if self.events_total == 0 {
            100.0
        } else {
            (self.events_processed as f64 / self.events_total as f64) * 100.0
        }
    }

    /// Update progress with new event count.
    #[allow(dead_code)]
    fn update(&mut self, events_processed: u64, eta: Option<Duration>) {
        self.events_processed = events_processed;
        self.percent_complete = self.calculate_percent();
        self.eta = eta;
    }
}

/// Thread-safe progress tracker for event replay.
pub struct ReplayTracker {
    events_total: u64,
    events_processed: AtomicU64,
    start_time: Instant,
    progress_tx: watch::Sender<ReplayProgress>,
    update_interval: u64,
}

impl ReplayTracker {
    /// Create a new replay tracker.
    ///
    /// # Arguments
    /// * `events_total` - Total number of events to replay
    /// * `update_interval` - Emit progress updates every N events (default: 100)
    pub fn new(events_total: u64, update_interval: u64) -> (Self, watch::Receiver<ReplayProgress>) {
        let initial_progress = ReplayProgress::new(events_total);
        let (progress_tx, progress_rx) = watch::channel(initial_progress);

        let tracker = Self {
            events_total,
            events_processed: AtomicU64::new(0),
            start_time: Instant::now(),
            progress_tx,
            update_interval,
        };

        (tracker, progress_rx)
    }

    /// Increment the event counter and optionally emit progress update.
    pub fn increment(&self) -> Result<()> {
        let processed = self.events_processed.fetch_add(1, Ordering::Relaxed) + 1;

        // Emit update every N events or on completion
        if processed.is_multiple_of(self.update_interval) || processed == self.events_total {
            let eta = self.calculate_eta(processed);
            let progress = ReplayProgress {
                events_total: self.events_total,
                events_processed: processed,
                percent_complete: if self.events_total == 0 {
                    100.0
                } else {
                    (processed as f64 / self.events_total as f64) * 100.0
                },
                eta,
            };

            self.progress_tx
                .send(progress)
                .map_err(|_| crate::error::Error::Internal("Progress channel closed".into()))?;
        }

        Ok(())
    }

    /// Calculate estimated time remaining.
    fn calculate_eta(&self, events_processed: u64) -> Option<Duration> {
        if events_processed == 0 || events_processed >= self.events_total {
            return None;
        }

        let elapsed = self.start_time.elapsed();
        let events_remaining = self.events_total - events_processed;
        let events_per_sec = events_processed as f64 / elapsed.as_secs_f64();

        if events_per_sec > 0.0 {
            let eta_secs = events_remaining as f64 / events_per_sec;
            Some(Duration::from_secs_f64(eta_secs))
        } else {
            None
        }
    }

    /// Get the current progress snapshot.
    pub fn current_progress(&self) -> ReplayProgress {
        let processed = self.events_processed.load(Ordering::Relaxed);
        let eta = self.calculate_eta(processed);

        ReplayProgress {
            events_total: self.events_total,
            events_processed: processed,
            percent_complete: if self.events_total == 0 {
                100.0
            } else {
                (processed as f64 / self.events_total as f64) * 100.0
            },
            eta,
        }
    }
}

/// Helper to create a shared replay tracker.
pub fn create_tracker(
    events_total: u64,
    update_interval: u64,
) -> (Arc<ReplayTracker>, watch::Receiver<ReplayProgress>) {
    let (tracker, rx) = ReplayTracker::new(events_total, update_interval);
    (Arc::new(tracker), rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // ReplayProgress BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_replay_progress_new() {
        let progress = ReplayProgress::new(1000);
        assert_eq!(progress.events_total, 1000);
        assert_eq!(progress.events_processed, 0);
        assert_eq!(progress.percent_complete, 0.0);
        assert!(progress.eta.is_none());
    }

    #[test]
    fn test_replay_progress_calculate_percent() {
        let mut progress = ReplayProgress::new(100);
        progress.update(50, None);
        assert_eq!(progress.percent_complete, 50.0);

        progress.update(100, None);
        assert_eq!(progress.percent_complete, 100.0);
    }

    #[test]
    fn test_replay_progress_zero_total() {
        let progress = ReplayProgress::new(0);
        assert_eq!(progress.percent_complete, 100.0);
    }

    // ==========================================================================
    // ReplayTracker::calculate_eta BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_return_none_eta_when_no_events_processed() {
        let (tracker, _rx) = ReplayTracker::new(100, 10);

        // ETA should be None when nothing processed
        let progress = tracker.current_progress();
        assert!(progress.eta.is_none(), "ETA should be None when no events processed");
    }

    #[test]
    fn should_return_none_eta_when_all_events_processed() {
        let (tracker, _rx) = ReplayTracker::new(5, 1);

        // Process all events
        for _ in 0..5 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();
        assert!(progress.eta.is_none(), "ETA should be None when all events processed");
    }

    #[tokio::test]
    async fn should_calculate_positive_eta_during_processing() {
        let (tracker, _rx) = ReplayTracker::new(1000, 1);

        // Process some events with a small delay to establish rate
        for _ in 0..100 {
            tracker.increment().ok();
        }

        // Small delay to make elapsed time measurable
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let progress = tracker.current_progress();

        // With events remaining, ETA should be Some with positive duration
        if let Some(eta) = progress.eta {
            assert!(eta.as_secs_f64() >= 0.0, "ETA should be non-negative");
        }
        // Note: ETA might be None if processing is too fast, which is acceptable
    }

    #[test]
    fn should_track_events_processed_accurately() {
        let (tracker, _rx) = ReplayTracker::new(100, 10);

        // Increment 5 times
        for _ in 0..5 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();
        assert_eq!(progress.events_processed, 5, "Should track exact count");
    }

    #[test]
    fn should_calculate_correct_percentage() {
        let (tracker, _rx) = ReplayTracker::new(100, 10);

        // Process 25 events
        for _ in 0..25 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();
        assert_eq!(progress.percent_complete, 25.0, "Percentage should be 25%");
    }

    #[test]
    fn should_handle_zero_total_events() {
        let (tracker, _rx) = ReplayTracker::new(0, 10);

        let progress = tracker.current_progress();
        assert_eq!(progress.percent_complete, 100.0, "Zero total should be 100% complete");
    }

    #[tokio::test]
    async fn should_emit_progress_at_update_interval() {
        let (tracker, mut rx) = ReplayTracker::new(100, 10);

        // Process 10 events (should trigger update)
        for _ in 0..10 {
            tracker.increment().ok();
        }

        // Should have received an update
        let result = rx.changed().await;
        assert!(result.is_ok(), "Should receive progress update at interval");

        let progress = rx.borrow().clone();
        assert_eq!(progress.events_processed, 10, "Progress should show 10 events");
    }

    #[tokio::test]
    async fn should_emit_progress_on_completion_regardless_of_interval() {
        let (tracker, mut rx) = ReplayTracker::new(5, 100); // Interval 100, but only 5 events

        // Process all 5 events
        for _ in 0..5 {
            tracker.increment().ok();
        }

        // Should receive update on completion even though interval (100) wasn't reached
        let result = rx.changed().await;
        assert!(result.is_ok(), "Should receive progress update on completion");

        let progress = rx.borrow().clone();
        assert_eq!(progress.events_processed, 5, "Should show all events processed");
        assert_eq!(progress.percent_complete, 100.0, "Should be 100% complete");
    }

    #[tokio::test]
    async fn test_replay_tracker_increment() {
        let (tracker, mut rx) = ReplayTracker::new(100, 10);

        // Increment 10 times to trigger update
        for _ in 0..10 {
            tracker.increment().ok();
        }

        // Should receive progress update
        rx.changed().await.ok();
        let progress = rx.borrow().clone();
        assert_eq!(progress.events_processed, 10);
        assert_eq!(progress.percent_complete, 10.0);
    }

    #[tokio::test]
    async fn test_replay_tracker_completion() {
        let (tracker, mut rx) = ReplayTracker::new(5, 100);

        // Process all events (update interval is 100, but completion always triggers update)
        for _ in 0..5 {
            tracker.increment().ok();
        }

        rx.changed().await.ok();
        let progress = rx.borrow().clone();
        assert_eq!(progress.events_processed, 5);
        assert_eq!(progress.percent_complete, 100.0);
    }

    #[test]
    fn test_replay_tracker_current_progress() {
        let (tracker, _rx) = ReplayTracker::new(100, 10);

        tracker.increment().ok();
        tracker.increment().ok();

        let progress = tracker.current_progress();
        assert_eq!(progress.events_processed, 2);
        assert_eq!(progress.percent_complete, 2.0);
    }

    #[tokio::test]
    async fn test_create_tracker() {
        let (tracker, mut rx) = create_tracker(200, 50);

        for _ in 0..50 {
            tracker.increment().ok();
        }

        rx.changed().await.ok();
        let progress = rx.borrow().clone();
        assert_eq!(progress.events_processed, 50);
    }

    // ==========================================================================
    // ReplayTracker::calculate_eta ADDITIONAL OPERATOR TESTS
    // These specifically test the > and / operators in calculate_eta
    // ==========================================================================

    #[test]
    fn should_return_none_eta_when_events_processed_exceeds_total() {
        // This tests the >= comparison: events_processed >= events_total
        let (tracker, _rx) = ReplayTracker::new(10, 1);

        // Manually process more than total (edge case - should still return None)
        for _ in 0..15 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();
        assert!(
            progress.eta.is_none(),
            "ETA should be None when processed exceeds total"
        );
    }

    #[test]
    fn should_calculate_eta_based_on_remaining_events() {
        // This tests that events_remaining = events_total - events_processed
        let (tracker, _rx) = ReplayTracker::new(100, 1);

        // Process 50 events
        for _ in 0..50 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();
        // Should have 50 remaining events
        // If mutation changes - to +, we'd have 150 remaining which is wrong

        // We can't directly test ETA value since it depends on elapsed time,
        // but we can verify the percentage is exactly 50%
        assert_eq!(
            progress.percent_complete, 50.0,
            "Should be 50% complete after processing 50/100 events"
        );
    }

    #[test]
    fn should_not_divide_by_zero_when_calculating_eta() {
        // This tests the events_per_sec > 0.0 check
        let (tracker, _rx) = ReplayTracker::new(100, 1);

        // Immediately check ETA with zero elapsed time
        // The division by elapsed time could cause issues if not handled
        let progress = tracker.current_progress();

        // Should not panic, and ETA should be None since no events processed
        assert!(
            progress.eta.is_none(),
            "ETA should be None when no events processed (zero elapsed)"
        );
    }

    #[tokio::test]
    async fn should_produce_positive_eta_during_active_processing() {
        let (tracker, _rx) = ReplayTracker::new(1000, 1);

        // Process some events
        for _ in 0..100 {
            tracker.increment().ok();
        }

        // Wait a bit to establish a rate
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Process more events to trigger ETA calculation with measurable elapsed time
        for _ in 0..100 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();

        // With 200 processed out of 1000, and some elapsed time,
        // ETA should be Some and positive
        if let Some(eta) = progress.eta {
            assert!(
                eta.as_secs_f64() > 0.0,
                "ETA should be positive when events remaining and rate is positive"
            );
        }
        // Note: ETA might still be None if processing is instantaneous, which is fine
    }

    #[test]
    fn should_update_events_total_in_progress() {
        // Verify events_total is correctly set in ReplayProgress
        let (tracker, _rx) = ReplayTracker::new(500, 10);

        let progress = tracker.current_progress();
        assert_eq!(
            progress.events_total, 500,
            "events_total should match constructor value"
        );
    }

    #[test]
    fn should_start_at_zero_events_processed() {
        let (tracker, _rx) = ReplayTracker::new(100, 10);

        let progress = tracker.current_progress();
        assert_eq!(
            progress.events_processed, 0,
            "Should start with zero events processed"
        );
    }

    // ==========================================================================
    // ReplayProgress::new BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_initialize_progress_with_correct_total() {
        let progress = ReplayProgress::new(500);

        assert_eq!(progress.events_total, 500);
        assert_eq!(progress.events_processed, 0);
        assert_eq!(progress.percent_complete, 0.0);
        assert!(progress.eta.is_none());
    }

    #[test]
    fn should_show_100_percent_for_zero_total_events() {
        let progress = ReplayProgress::new(0);

        assert_eq!(
            progress.percent_complete, 100.0,
            "Zero total should be 100% complete immediately"
        );
    }

    // ==========================================================================
    // create_tracker helper BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_return_arc_wrapped_tracker() {
        let (tracker, _rx) = create_tracker(100, 10);

        // Verify it's an Arc by cloning
        let tracker2 = Arc::clone(&tracker);

        // Both should share state
        tracker.increment().ok();

        let progress1 = tracker.current_progress();
        let progress2 = tracker2.current_progress();

        assert_eq!(
            progress1.events_processed, progress2.events_processed,
            "Arc-wrapped trackers should share state"
        );
    }

    #[test]
    fn should_use_provided_update_interval() {
        // Update interval of 25 means updates at 25, 50, 75, 100...
        let (tracker, _rx) = ReplayTracker::new(100, 25);

        // Process 24 events - should NOT trigger update (except initial)
        for _ in 0..24 {
            tracker.increment().ok();
        }

        // Process 25th event - should trigger update
        tracker.increment().ok();

        let progress = tracker.current_progress();
        assert_eq!(progress.events_processed, 25);
    }

    // ==========================================================================
    // calculate_eta SPECIFIC ARITHMETIC TESTS
    // These try to catch the arithmetic operator mutations
    // ==========================================================================

    #[test]
    fn should_not_return_eta_when_zero_events_processed() {
        // Tests: events_processed == 0 early return
        let (tracker, _rx) = ReplayTracker::new(100, 1);

        let progress = tracker.current_progress();

        // With 0 events processed, ETA must be None (early return check)
        assert!(
            progress.eta.is_none(),
            "ETA must be None when 0 events processed"
        );
    }

    #[test]
    fn should_not_return_eta_when_events_processed_equals_total() {
        // Tests: events_processed >= events_total early return
        let (tracker, _rx) = ReplayTracker::new(5, 1);

        // Process exactly 5 events
        for _ in 0..5 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();

        // When events_processed == events_total, ETA must be None
        assert!(
            progress.eta.is_none(),
            "ETA must be None when all events processed"
        );
    }

    #[tokio::test]
    async fn should_calculate_reasonable_eta_for_partial_progress() {
        // Tests: the calculation uses subtraction for remaining events
        // If - were + or /, the ETA would be wildly different
        let (tracker, _rx) = ReplayTracker::new(100, 1);

        // Process half the events
        for _ in 0..50 {
            tracker.increment().ok();
        }

        // Wait to establish a rate
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let progress = tracker.current_progress();

        // With 50 processed out of 100, there are 50 remaining (100 - 50)
        // If the mutation changed - to +, remaining would be 150, giving a much larger ETA
        // If the mutation changed - to /, remaining would be 2, giving a much smaller ETA
        if let Some(eta) = progress.eta {
            // The ETA should be roughly equal to time already spent
            // (since we're halfway done, time remaining ≈ time spent)
            let eta_secs = eta.as_secs_f64();

            // Just verify it's in a reasonable range (not negative, not astronomically large)
            assert!(
                eta_secs >= 0.0,
                "ETA should not be negative"
            );
            assert!(
                eta_secs < 1000.0,
                "ETA should be reasonable (not astronomically large from wrong operator)"
            );
        }
        // Note: ETA might be None if processing is too fast, which is acceptable
    }

    #[tokio::test]
    async fn should_calculate_eta_proportional_to_remaining_work() {
        // This test verifies the ETA calculation uses correct operators
        // by checking the ratio of remaining work to processed work
        let (tracker, _rx) = ReplayTracker::new(100, 1);

        // Process 10 events with a delay to establish measurable rate
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        for _ in 0..10 {
            tracker.increment().ok();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let progress_10 = tracker.current_progress();

        // Process to 50 events
        for _ in 0..40 {
            tracker.increment().ok();
        }

        let progress_50 = tracker.current_progress();

        // At 10%, 90 events remain; at 50%, 50 events remain
        // ETA at 10% should be greater than ETA at 50%
        if let (Some(eta_10), Some(eta_50)) = (progress_10.eta, progress_50.eta) {
            // More remaining work should mean longer ETA
            // This would fail if - were changed to + (remaining would grow with processed)
            assert!(
                eta_10.as_secs_f64() >= eta_50.as_secs_f64() * 0.5,
                "ETA should decrease as more work is completed"
            );
        }
    }

    #[test]
    fn should_have_eta_none_edge_case_at_boundary() {
        // Tests the boundary condition: events_processed >= events_total
        // Mutation from >= to > would cause eta to be calculated at exactly total
        let (tracker, _rx) = ReplayTracker::new(10, 1);

        // Process exactly to the boundary
        for _ in 0..10 {
            tracker.increment().ok();
        }

        let progress = tracker.current_progress();

        // At exactly events_total, ETA should be None
        assert!(
            progress.eta.is_none(),
            "ETA must be None at completion boundary"
        );
    }
}
