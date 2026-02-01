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
}
