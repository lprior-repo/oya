//! Backpressure-aware pipe handling for Zellij IPC
//!
//! This module provides flow control mechanisms to handle high-volume
//! log streaming without overwhelming the consumer.

use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for backpressure handling
#[derive(Debug, Clone, Copy)]
pub struct BackpressureConfig {
    /// Maximum number of messages to buffer before applying backpressure
    pub max_buffered: usize,
    /// Delay to apply when backpressure is triggered (in milliseconds)
    pub throttle_delay_ms: u64,
    /// Whether to enable backpressure signaling
    pub enabled: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_buffered: 1000,
            throttle_delay_ms: 10,
            enabled: true,
        }
    }
}

impl BackpressureConfig {
    /// Create a new config with custom settings
    pub fn new(max_buffered: usize, throttle_delay_ms: u64) -> Self {
        Self {
            max_buffered,
            throttle_delay_ms,
            enabled: true,
        }
    }

    /// Disable backpressure
    pub fn disabled() -> Self {
        Self {
            max_buffered: usize::MAX,
            throttle_delay_ms: 0,
            enabled: false,
        }
    }
}

/// Shared state for tracking backpressure between producer and consumer
#[derive(Debug)]
pub struct BackpressureState {
    /// Current number of buffered messages
    buffered_count: AtomicUsize,
    /// Whether backpressure is currently active
    is_throttled: AtomicBool,
    /// Last time backpressure was checked
    last_check: Mutex<Instant>,
    /// Configuration
    config: BackpressureConfig,
}

impl BackpressureState {
    /// Create a new backpressure state with default config
    pub fn new() -> Self {
        Self::with_config(BackpressureConfig::default())
    }

    /// Create a new backpressure state with custom config
    pub fn with_config(config: BackpressureConfig) -> Self {
        Self {
            buffered_count: AtomicUsize::new(0),
            is_throttled: AtomicBool::new(false),
            last_check: Mutex::new(Instant::now()),
            config,
        }
    }

    /// Increment the buffered message count
    pub fn increment_buffered(&self) {
        self.buffered_count.fetch_add(1, Ordering::SeqCst);
        self.check_backpressure();
    }

    /// Decrement the buffered message count (when message is consumed)
    pub fn decrement_buffered(&self) {
        let prev = self.buffered_count.fetch_sub(1, Ordering::SeqCst);
        if prev <= self.config.max_buffered && self.is_throttled.load(Ordering::SeqCst) {
            self.is_throttled.store(false, Ordering::SeqCst);
        }
    }

    /// Check if backpressure should be applied
    fn check_backpressure(&self) {
        if !self.config.enabled {
            return;
        }

        let buffered = self.buffered_count.load(Ordering::SeqCst);
        let should_throttle = buffered >= self.config.max_buffered;

        self.is_throttled.store(should_throttle, Ordering::SeqCst);

        if let Ok(mut last_check) = self.last_check.lock() {
            *last_check = Instant::now();
        }
    }

    /// Check if currently throttled
    pub fn is_throttled(&self) -> bool {
        self.is_throttled.load(Ordering::SeqCst)
    }

    /// Get current buffered count
    pub fn buffered_count(&self) -> usize {
        self.buffered_count.load(Ordering::SeqCst)
    }

    /// Apply throttling delay if needed
    pub fn apply_throttle(&self) {
        if self.is_throttled() && self.config.throttle_delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(self.config.throttle_delay_ms));
        }
    }
}

impl Default for BackpressureState {
    fn default() -> Self {
        Self::new()
    }
}

/// A writer that implements backpressure
pub struct BackpressureWriter<W: Write> {
    inner: W,
    state: Arc<BackpressureState>,
}

impl<W: Write> BackpressureWriter<W> {
    /// Create a new backpressure writer
    pub fn new(inner: W, state: Arc<BackpressureState>) -> Self {
        Self { inner, state }
    }

    /// Get a reference to the backpressure state
    pub fn state(&self) -> &Arc<BackpressureState> {
        &self.state
    }
}

impl<W: Write> Write for BackpressureWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Apply throttling if needed
        self.state.apply_throttle();

        let result = self.inner.write(buf);

        // Track successful writes
        if result.is_ok() {
            self.state.increment_buffered();
        }

        result
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A reader that implements backpressure
pub struct BackpressureReader<R: Read> {
    inner: R,
    state: Arc<BackpressureState>,
}

impl<R: Read> BackpressureReader<R> {
    /// Create a new backpressure reader
    pub fn new(inner: R, state: Arc<BackpressureState>) -> Self {
        Self { inner, state }
    }

    /// Get a reference to the backpressure state
    pub fn state(&self) -> &Arc<BackpressureState> {
        &self.state
    }
}

impl<R: Read> Read for BackpressureReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.inner.read(buf);

        // Track successful reads (messages consumed)
        if let Ok(n) = result {
            if n > 0 {
                self.state.decrement_buffered();
            }
        }

        result
    }
}

/// Create a backpressure-controlled pipe pair
///
/// Returns a (reader, writer) pair that shares backpressure state
#[cfg(test)]
pub fn backpressure_pipe(
    config: BackpressureConfig,
) -> io::Result<(
    BackpressureReader<os_pipe::PipeReader>,
    BackpressureWriter<os_pipe::PipeWriter>,
)> {
    let (reader, writer) = os_pipe::pipe()?;
    let state = Arc::new(BackpressureState::with_config(config));

    Ok((
        BackpressureReader::new(reader, Arc::clone(&state)),
        BackpressureWriter::new(writer, state),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_backpressure_state_tracks_count() {
        let state = BackpressureState::with_config(BackpressureConfig::disabled());

        assert_eq!(state.buffered_count(), 0);

        state.increment_buffered();
        assert_eq!(state.buffered_count(), 1);

        state.increment_buffered();
        assert_eq!(state.buffered_count(), 2);

        state.decrement_buffered();
        assert_eq!(state.buffered_count(), 1);
    }

    #[test]
    fn test_backpressure_triggers_when_threshold_reached() {
        let config = BackpressureConfig::new(5, 1);
        let state = BackpressureState::with_config(config);

        // Should not be throttled initially
        assert!(!state.is_throttled());

        // Increment to threshold
        for _ in 0..5 {
            state.increment_buffered();
        }

        // Should now be throttled
        assert!(state.is_throttled());
    }

    #[test]
    fn test_backpressure_clears_when_count_drops() {
        let config = BackpressureConfig::new(3, 1);
        let state = BackpressureState::with_config(config);

        // Trigger backpressure
        for _ in 0..3 {
            state.increment_buffered();
        }
        assert!(state.is_throttled());

        // Decrement below threshold
        for _ in 0..3 {
            state.decrement_buffered();
        }

        // Should no longer be throttled
        assert!(!state.is_throttled());
    }

    #[test]
    fn test_backpressure_writer_tracks_writes() {
        let (reader, writer) = os_pipe::pipe().unwrap();
        let state = Arc::new(BackpressureState::with_config(
            BackpressureConfig::disabled(),
        ));
        let mut bp_writer = BackpressureWriter::new(writer, Arc::clone(&state));

        bp_writer.write_all(b"test").unwrap();

        assert_eq!(state.buffered_count(), 1);

        // Read to clear
        let mut bp_reader = BackpressureReader::new(reader, Arc::clone(&state));
        let mut buf = [0u8; 4];
        bp_reader.read_exact(&mut buf).unwrap();

        assert_eq!(state.buffered_count(), 0);
    }

    #[test]
    fn test_disabled_backpressure_never_throttles() {
        let config = BackpressureConfig::disabled();
        let state = BackpressureState::with_config(config);

        // Increment many times
        for _ in 0..10000 {
            state.increment_buffered();
        }

        // Should never be throttled
        assert!(!state.is_throttled());
    }
}
