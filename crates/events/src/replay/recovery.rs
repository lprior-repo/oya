//! Error recovery and retry logic for event replay.
//!
//! Provides retry strategies with exponential backoff and dead letter queue
//! for handling poison events.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::{Error, Result};
use std::time::Duration;
use tracing::{error, warn};

/// Maximum number of retry attempts for transient errors.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (in milliseconds).
const BASE_BACKOFF_MS: u64 = 100;

/// Maximum backoff delay (in milliseconds).
const MAX_BACKOFF_MS: u64 = 5000;

/// A poison event that failed during replay and was sent to the dead letter queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoisonEvent {
    /// Unique identifier for the event.
    pub event_id: String,
    /// Number of retry attempts before sending to DLQ.
    pub attempt_count: u32,
    /// Error message describing why the event failed.
    pub error: String,
    /// Timestamp when the event was sent to DLQ.
    pub timestamp: DateTime<Utc>,
    /// Optional serialized event payload for inspection/replay.
    pub event_data: Option<Vec<u8>>,
}

impl PoisonEvent {
    /// Create a new poison event.
    #[must_use]
    pub fn new(
        event_id: impl Into<String>,
        attempt_count: u32,
        error: impl Into<String>,
        event_data: Option<Vec<u8>>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            attempt_count,
            error: error.into(),
            timestamp: Utc::now(),
            event_data,
        }
    }

    /// Create a poison event with a specific timestamp.
    #[must_use]
    pub const fn with_timestamp(
        event_id: String,
        attempt_count: u32,
        error: String,
        timestamp: DateTime<Utc>,
        event_data: Option<Vec<u8>>,
    ) -> Self {
        Self {
            event_id,
            attempt_count,
            error,
            timestamp,
            event_data,
        }
    }

    /// Increment attempt count and update timestamp.
    #[must_use]
    pub fn increment_attempt(mut self) -> Self {
        self.attempt_count += 1;
        self.timestamp = Utc::now();
        self
    }
}

/// Trait for dead letter queue storage backends.
///
/// A dead letter queue stores events that failed during replay
/// and could not be recovered after retry attempts.
pub trait DeadLetterQueue: Send + Sync {
    /// Push a poison event to the dead letter queue.
    ///
    /// # Errors
    ///
    /// Returns `Error` if the event cannot be stored.
    fn push_poison_event(&self, event: PoisonEvent) -> Result<()>;

    /// Get all poison events from the dead letter queue.
    ///
    /// # Errors
    ///
    /// Returns `Error` if events cannot be retrieved.
    fn get_poison_events(&self) -> Result<Vec<PoisonEvent>>;

    /// Clear all poison events from the dead letter queue.
    ///
    /// # Errors
    ///
    /// Returns `Error` if the queue cannot be cleared.
    fn clear(&self) -> Result<()>;

    /// Get the count of poison events in the queue.
    ///
    /// # Errors
    ///
    /// Returns `Error` if the count cannot be retrieved.
    fn count(&self) -> Result<usize>;
}

/// In-memory dead letter queue implementation using `Arc<Mutex<VecDeque>>`.
///
/// This implementation uses interior mutability to provide thread-safe
/// access to the poison event storage.
#[derive(Debug, Default, Clone)]
pub struct InMemoryDeadLetterQueue {
    events: Arc<std::sync::Mutex<VecDeque<PoisonEvent>>>,
}

impl InMemoryDeadLetterQueue {
    /// Create a new in-memory dead letter queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(std::sync::Mutex::new(VecDeque::new())),
        }
    }

    /// Create a new in-memory dead letter queue with initial capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(std::sync::Mutex::new(VecDeque::with_capacity(capacity))),
        }
    }
}

impl DeadLetterQueue for InMemoryDeadLetterQueue {
    fn push_poison_event(&self, event: PoisonEvent) -> Result<()> {
        self.events
            .lock()
            .map_err(|e| Error::Internal(format!("DLQ lock poisoned: {e}")))
            .map(|mut events| {
                events.push_back(event);
                ()
            })
    }

    fn get_poison_events(&self) -> Result<Vec<PoisonEvent>> {
        self.events
            .lock()
            .map_err(|e| Error::Internal(format!("DLQ lock poisoned: {e}")))
            .map(|events| events.iter().cloned().collect())
    }

    fn clear(&self) -> Result<()> {
        self.events
            .lock()
            .map_err(|e| Error::Internal(format!("DLQ lock poisoned: {e}")))
            .map(|mut events| {
                events.clear();
            })
    }

    fn count(&self) -> Result<usize> {
        self.events
            .lock()
            .map_err(|e| Error::Internal(format!("DLQ lock poisoned: {e}")))
            .map(|events| events.len())
    }
}

/// Configuration for error recovery during event replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryConfig {
    /// Maximum number of retry attempts for transient errors.
    pub max_retries: u32,
    /// Base delay for exponential backoff.
    pub base_backoff_ms: u64,
    /// Maximum backoff delay.
    pub max_backoff_ms: u64,
    /// Whether to enable dead letter queue for poison events.
    pub enable_dlq: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: MAX_RETRIES,
            base_backoff_ms: BASE_BACKOFF_MS,
            max_backoff_ms: MAX_BACKOFF_MS,
            enable_dlq: true,
        }
    }
}

impl RecoveryConfig {
    /// Create a new recovery configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retries.
    #[must_use]
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the base backoff delay in milliseconds.
    #[must_use]
    pub const fn with_base_backoff(mut self, base_backoff_ms: u64) -> Self {
        self.base_backoff_ms = base_backoff_ms;
        self
    }

    /// Set the maximum backoff delay in milliseconds.
    #[must_use]
    pub const fn with_max_backoff(mut self, max_backoff_ms: u64) -> Self {
        self.max_backoff_ms = max_backoff_ms;
        self
    }

    /// Enable or disable the dead letter queue.
    #[must_use]
    pub const fn with_dlq(mut self, enable_dlq: bool) -> Self {
        self.enable_dlq = enable_dlq;
        self
    }

    /// Calculate the backoff delay for a given retry attempt.
    ///
    /// Uses exponential backoff with jitter: delay = base * 2^attempt
    /// Capped at `max_backoff_ms`.
    #[must_use]
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let exponential_delay = self.base_backoff_ms * 2_u64.pow(attempt);
        let delay_ms = exponential_delay.min(self.max_backoff_ms);
        Duration::from_millis(delay_ms)
    }
}

/// Strategy for recovering from errors during event replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff.
    Retry { attempt: u32, delay: Duration },
    /// Skip the event and send to dead letter queue.
    SkipToDlq,
    /// Fail the entire replay operation.
    Fail,
}

/// Policy for retrying failed event operations.
pub struct RetryPolicy {
    config: RecoveryConfig,
}

impl RetryPolicy {
    /// Create a new retry policy with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: RecoveryConfig::default(),
        }
    }

    /// Create a new retry policy with custom configuration.
    #[must_use]
    pub const fn with_config(config: RecoveryConfig) -> Self {
        Self { config }
    }

    /// Determine the recovery strategy for a given error and attempt number.
    #[must_use]
    pub fn should_retry(&self, error: &Error, attempt: u32) -> RecoveryStrategy {
        // Check if we've exceeded max retries
        if attempt >= self.config.max_retries {
            if self.config.enable_dlq {
                return RecoveryStrategy::SkipToDlq;
            }
            return RecoveryStrategy::Fail;
        }

        // Check if error is transient
        if is_transient_error(error) {
            let delay = self.config.calculate_backoff(attempt);
            RecoveryStrategy::Retry {
                attempt: attempt + 1,
                delay,
            }
        } else {
            // Non-transient errors should not be retried
            if self.config.enable_dlq {
                RecoveryStrategy::SkipToDlq
            } else {
                RecoveryStrategy::Fail
            }
        }
    }

    /// Get a reference to the configuration.
    #[must_use]
    pub const fn config(&self) -> &RecoveryConfig {
        &self.config
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine if an error is transient (retryable) or permanent.
///
/// Transient errors include:
/// - Network errors (temporary connection issues)
/// - Lock contention (resource temporarily unavailable)
/// - Timeouts (operation took too long but might succeed on retry)
///
/// Permanent errors include:
/// - Invalid event data (data corruption)
/// - Event not found (missing data)
/// - Invalid state transitions (logic errors)
#[must_use]
pub fn is_transient_error(error: &Error) -> bool {
    match error {
        // Network and connection issues are transient
        Error::Connection(_) => true,

        // Timeouts might succeed on retry
        Error::StoreFailed { operation, reason } => {
            operation.to_lowercase().contains("timeout")
                || operation.to_lowercase().contains("network")
                || reason.to_lowercase().contains("lock")
                || reason.to_lowercase().contains("timeout")
                || reason.to_lowercase().contains("temporary")
        }

        // Serialization errors are permanent (data corruption)
        Error::Serialization { .. } => false,

        // Invalid events are permanent
        Error::InvalidEvent { .. } => false,

        // Event not found is permanent
        Error::EventNotFound { .. } => false,

        // Projection failures might be transient
        Error::ProjectionFailed { reason, .. } => {
            reason.to_lowercase().contains("timeout")
                || reason.to_lowercase().contains("lock")
                || reason.to_lowercase().contains("temporary")
        }

        // Invalid transitions are permanent
        Error::InvalidTransition { .. } => false,

        // Channel closed is permanent
        Error::ChannelClosed => false,

        // Internal errors are permanent by default
        Error::Internal(_) => false,

        // Other errors are not transient
        _ => false,
    }
}

/// Type alias for async fallible operation factory.
///
/// Using a factory function (Fn -> Future) instead of a direct Future
/// allows us to retry the operation multiple times, since futures
/// consume themselves when polled.
pub type AsyncOperation<T, E> = Pin<Box<dyn Future<Output = std::result::Result<T, E>> + Send>>;

/// Type alias for async operation factory.
///
/// The factory creates a new Future for each retry attempt,
/// working around the fact that Futures consume themselves when polled.
pub type AsyncOperationFactory<T, E> = dyn (Fn() -> AsyncOperation<T, E>) + Send + Sync;

/// Retry an async operation with exponential backoff and DLQ on exhaustion.
///
/// This function implements Railway-Oriented Programming by chaining
/// async operations with proper error handling. Transient errors are
/// retried with exponential backoff, while permanent errors or
/// exhausted retries are sent to the dead letter queue.
///
/// Uses a factory function to create fresh futures for each retry attempt,
/// avoiding the "future consumed after first poll" issue.
///
/// # Arguments
///
/// * `operation_factory` - Factory function that creates async operations to retry
/// * `policy` - Retry policy with backoff configuration
/// * `dlq` - Dead letter queue for exhausted retries
/// * `event_id` - Event identifier for logging and DLQ
/// * `event_data` - Optional serialized event payload for DLQ
///
/// # Errors
///
/// Returns `Error` if:
/// - Operation fails after max retries (with DLQ disabled)
/// - Dead letter queue push fails
/// - Operation fails with non-transient error (with DLQ disabled)
///
/// # Example
///
/// ```no_run
/// use oya_events::replay::recovery::{retry_with_policy, RetryPolicy, InMemoryDeadLetterQueue};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let policy = RetryPolicy::new();
/// let dlq = InMemoryDeadLetterQueue::new();
///
/// let result = retry_with_policy(
///     &|| Box::pin(async { Ok::<_, oya_events::Error>(42) }),
///     &policy,
///     &dlq,
///     "event-123",
///     None,
/// ).await;
/// # Ok(())
/// # }
/// ```
pub async fn retry_with_policy<T>(
    operation_factory: &AsyncOperationFactory<T, Error>,
    policy: &RetryPolicy,
    dlq: &dyn DeadLetterQueue,
    event_id: &str,
    event_data: Option<Vec<u8>>,
) -> Result<T> {
    let mut attempt: u32 = 0;

    loop {
        // Create a new operation from the factory for this attempt
        let operation = operation_factory();

        // Attempt the operation
        match operation.await {
            Ok(value) => {
                // Success: return the value
                return Ok(value);
            }
            Err(err) => {
                // Determine recovery strategy
                let strategy = policy.should_retry(&err, attempt);

                match strategy {
                    RecoveryStrategy::Retry {
                        attempt: next_attempt,
                        delay,
                    } => {
                        // Transient error: log and retry after backoff
                        log_failed_event(event_id, attempt, &err);

                        tokio::time::sleep(delay).await;
                        attempt = next_attempt;
                    }
                    RecoveryStrategy::SkipToDlq => {
                        // Exhausted retries or permanent error: send to DLQ
                        let poison_event =
                            PoisonEvent::new(event_id, attempt, err.to_string(), event_data);
                        dlq.push_poison_event(poison_event)?;
                        log_poison_event(event_id, attempt, &err);
                        return Err(err);
                    }
                    RecoveryStrategy::Fail => {
                        // DLQ disabled: fail immediately
                        log_failed_event(event_id, attempt, &err);
                        return Err(err);
                    }
                }
            }
        }
    }
}

/// Log a poison event that was sent to the dead letter queue.
///
/// Uses structured logging at WARN level with tracing.
/// Includes event_id, attempt_count, and error context.
///
/// # Arguments
///
/// * `event_id` - Identifier of the poison event
/// * `attempt_count` - Number of retry attempts before DLQ
/// * `error` - The error that caused the event to be poisoned
pub fn log_poison_event(event_id: &str, attempt_count: u32, error: &Error) {
    warn!(
        event_id = %event_id,
        attempt_count = attempt_count,
        error = %error,
        "Event sent to dead letter queue"
    );
}

/// Log a failed event retry attempt.
///
/// Uses structured logging at ERROR level with tracing.
/// Includes event_id, attempt_count, and error context.
///
/// # Arguments
///
/// * `event_id` - Identifier of the failed event
/// * `attempt_count` - Current retry attempt number
/// * `error` - The error that caused the retry to fail
pub fn log_failed_event(event_id: &str, attempt_count: u32, error: &Error) {
    error!(
        event_id = %event_id,
        attempt_count = attempt_count,
        error = %error,
        is_transient = is_transient_error(error),
        "Event operation failed, retrying"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ConnectionError;

    // ==========================================================================
    // PoisonEvent BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_poison_event_new() {
        let event = PoisonEvent::new("evt-123", 3, "timeout", None);
        assert_eq!(event.event_id, "evt-123");
        assert_eq!(event.attempt_count, 3);
        assert_eq!(event.error, "timeout");
        assert!(event.event_data.is_none());
        // Timestamp should be very recent (within last second)
        let now = Utc::now();
        let diff = now - event.timestamp;
        assert!(diff.num_seconds() >= 0 && diff.num_seconds() <= 1);
    }

    #[test]
    fn test_poison_event_new_with_data() {
        let data = vec![1, 2, 3, 4];
        let event = PoisonEvent::new("evt-456", 1, "serialization failed", Some(data.clone()));
        assert_eq!(event.event_id, "evt-456");
        assert_eq!(event.attempt_count, 1);
        assert_eq!(event.error, "serialization failed");
        assert_eq!(event.event_data, Some(data));
    }

    #[test]
    fn test_poison_event_with_timestamp() {
        let timestamp = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .map(|dt| dt.with_timezone(&Utc))
            .expect("valid timestamp");
        let event = PoisonEvent::with_timestamp(
            "evt-789".to_string(),
            5,
            "permanent error".to_string(),
            timestamp,
            None,
        );
        assert_eq!(event.event_id, "evt-789");
        assert_eq!(event.attempt_count, 5);
        assert_eq!(event.error, "permanent error");
        assert_eq!(event.timestamp, timestamp);
        assert!(event.event_data.is_none());
    }

    #[test]
    fn test_poison_event_increment_attempt() {
        let timestamp = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .map(|dt| dt.with_timezone(&Utc))
            .expect("valid timestamp");
        let event = PoisonEvent::with_timestamp(
            "evt-123".to_string(),
            2,
            "error".to_string(),
            timestamp,
            None,
        );

        let incremented = event.increment_attempt();

        assert_eq!(incremented.event_id, "evt-123");
        assert_eq!(incremented.attempt_count, 3); // Incremented
        assert_eq!(incremented.error, "error");
        // Timestamp should be updated to now
        assert!(incremented.timestamp > timestamp);
    }

    #[test]
    fn test_poison_event_equality() {
        let timestamp = Utc::now();
        let event1 = PoisonEvent::with_timestamp(
            "evt-123".to_string(),
            2,
            "error".to_string(),
            timestamp,
            None,
        );
        let event2 = PoisonEvent::with_timestamp(
            "evt-123".to_string(),
            2,
            "error".to_string(),
            timestamp,
            None,
        );
        assert_eq!(event1, event2);
    }

    #[test]
    fn test_poison_event_not_equal_different_id() {
        let timestamp = Utc::now();
        let event1 = PoisonEvent::with_timestamp(
            "evt-123".to_string(),
            2,
            "error".to_string(),
            timestamp,
            None,
        );
        let event2 = PoisonEvent::with_timestamp(
            "evt-456".to_string(),
            2,
            "error".to_string(),
            timestamp,
            None,
        );
        assert_ne!(event1, event2);
    }

    #[test]
    fn test_poison_event_not_equal_different_attempt() {
        let timestamp = Utc::now();
        let event1 = PoisonEvent::with_timestamp(
            "evt-123".to_string(),
            2,
            "error".to_string(),
            timestamp,
            None,
        );
        let event2 = PoisonEvent::with_timestamp(
            "evt-123".to_string(),
            3,
            "error".to_string(),
            timestamp,
            None,
        );
        assert_ne!(event1, event2);
    }

    // ==========================================================================
    // InMemoryDeadLetterQueue BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_in_memory_dlq_new() {
        let dlq = InMemoryDeadLetterQueue::new();
        assert_eq!(dlq.count().ok(), Some(0));
    }

    #[test]
    fn test_in_memory_dlq_with_capacity() {
        let dlq = InMemoryDeadLetterQueue::with_capacity(10);
        assert_eq!(dlq.count().ok(), Some(0));
    }

    #[test]
    fn test_push_single_poison_event() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();
        let event = PoisonEvent::new("evt-1", 3, "timeout", None);

        dlq.push_poison_event(event)?;

        assert_eq!(dlq.count()?, 1);
        Ok(())
    }

    #[test]
    fn test_push_multiple_poison_events() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "error1", None))?;
        dlq.push_poison_event(PoisonEvent::new("evt-2", 2, "error2", None))?;
        dlq.push_poison_event(PoisonEvent::new("evt-3", 3, "error3", None))?;

        assert_eq!(dlq.count()?, 3);
        Ok(())
    }

    #[test]
    fn test_get_poison_events_empty() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();
        let events = dlq.get_poison_events()?;
        assert!(events.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_poison_events_after_push() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();
        let event1 = PoisonEvent::new("evt-1", 1, "error1", None);
        let event2 = PoisonEvent::new("evt-2", 2, "error2", None);

        dlq.push_poison_event(event1.clone())?;
        dlq.push_poison_event(event2.clone())?;

        let events = dlq.get_poison_events()?;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event1);
        assert_eq!(events[1], event2);
        Ok(())
    }

    #[test]
    fn test_get_poison_events_preserves_order() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "first", None))?;
        dlq.push_poison_event(PoisonEvent::new("evt-2", 2, "second", None))?;
        dlq.push_poison_event(PoisonEvent::new("evt-3", 3, "third", None))?;

        let events = dlq.get_poison_events()?;

        assert_eq!(events[0].event_id, "evt-1");
        assert_eq!(events[1].event_id, "evt-2");
        assert_eq!(events[2].event_id, "evt-3");
        Ok(())
    }

    #[test]
    fn test_clear_empty_queue() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();
        dlq.clear()?;
        assert_eq!(dlq.count()?, 0);
        Ok(())
    }

    #[test]
    fn test_clear_non_empty_queue() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "error1", None))?;
        dlq.push_poison_event(PoisonEvent::new("evt-2", 2, "error2", None))?;

        assert_eq!(dlq.count()?, 2);

        dlq.clear()?;

        assert_eq!(dlq.count()?, 0);
        let events = dlq.get_poison_events()?;
        assert!(events.is_empty());
        Ok(())
    }

    #[test]
    fn test_count_increments_with_push() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();

        assert_eq!(dlq.count()?, 0);

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "error1", None))?;
        assert_eq!(dlq.count()?, 1);

        dlq.push_poison_event(PoisonEvent::new("evt-2", 2, "error2", None))?;
        assert_eq!(dlq.count()?, 2);

        dlq.push_poison_event(PoisonEvent::new("evt-3", 3, "error3", None))?;
        assert_eq!(dlq.count()?, 3);

        Ok(())
    }

    #[test]
    fn test_clone_creates_shared_state() -> Result<()> {
        let dlq1 = InMemoryDeadLetterQueue::new();
        let dlq2 = dlq1.clone();

        dlq1.push_poison_event(PoisonEvent::new("evt-1", 1, "error1", None))?;

        // Both clones should see the same event
        assert_eq!(dlq1.count()?, 1);
        assert_eq!(dlq2.count()?, 1);

        let events1 = dlq1.get_poison_events()?;
        let events2 = dlq2.get_poison_events()?;
        assert_eq!(events1, events2);

        Ok(())
    }

    #[test]
    fn test_push_with_event_data() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();
        let data = vec![10, 20, 30, 40];

        let event = PoisonEvent::new("evt-1", 1, "corrupted", Some(data.clone()));
        dlq.push_poison_event(event)?;

        let events = dlq.get_poison_events()?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_data, Some(data));
        Ok(())
    }

    #[test]
    fn test_multiple_gets_return_same_events() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();
        let event = PoisonEvent::new("evt-1", 1, "error", None);

        dlq.push_poison_event(event.clone())?;

        let events1 = dlq.get_poison_events()?;
        let events2 = dlq.get_poison_events()?;

        assert_eq!(events1, events2);
        assert_eq!(events1.len(), 1);
        Ok(())
    }

    #[test]
    fn test_clear_then_add() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::new();

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "error1", None))?;
        dlq.clear()?;

        assert_eq!(dlq.count()?, 0);

        dlq.push_poison_event(PoisonEvent::new("evt-2", 1, "error2", None))?;

        assert_eq!(dlq.count()?, 1);

        let events = dlq.get_poison_events()?;
        assert_eq!(events[0].event_id, "evt-2");
        Ok(())
    }

    #[test]
    fn test_large_number_of_events() -> Result<()> {
        let dlq = InMemoryDeadLetterQueue::with_capacity(1000);

        for i in 0..100 {
            dlq.push_poison_event(PoisonEvent::new(
                format!("evt-{}", i),
                1,
                format!("error {}", i),
                None,
            ))?;
        }

        assert_eq!(dlq.count()?, 100);

        let events = dlq.get_poison_events()?;
        assert_eq!(events.len(), 100);
        assert_eq!(events[0].event_id, "evt-0");
        assert_eq!(events[99].event_id, "evt-99");
        Ok(())
    }

    // ==========================================================================
    // DeadLetterQueue trait constraint tests
    // ==========================================================================

    #[test]
    fn test_dlq_trait_object_push_and_get() -> Result<()> {
        // Test that trait can be used as trait object
        let dlq: Box<dyn DeadLetterQueue> = Box::new(InMemoryDeadLetterQueue::new());
        let event = PoisonEvent::new("evt-1", 1, "error", None);

        dlq.push_poison_event(event)?;
        let events = dlq.get_poison_events()?;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "evt-1");
        Ok(())
    }

    #[test]
    fn test_dlq_trait_object_clear() -> Result<()> {
        let dlq: Box<dyn DeadLetterQueue> = Box::new(InMemoryDeadLetterQueue::new());

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "error", None))?;
        assert_eq!(dlq.count()?, 1);

        dlq.clear()?;
        assert_eq!(dlq.count()?, 0);
        Ok(())
    }

    #[test]
    fn test_dlq_reference_can_be_shared() -> Result<()> {
        // Test that Arc<impl DeadLetterQueue> works for sharing across threads
        let dlq = Arc::new(InMemoryDeadLetterQueue::new());
        let dlq_clone = Arc::clone(&dlq);

        dlq.push_poison_event(PoisonEvent::new("evt-1", 1, "error", None))?;

        // Both references should see the same event
        assert_eq!(dlq.count()?, 1);
        assert_eq!(dlq_clone.count()?, 1);

        Ok(())
    }

    // ==========================================================================
    // RecoveryConfig BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 5000);
        assert!(config.enable_dlq);
    }

    #[test]
    fn test_recovery_config_builder() {
        let config = RecoveryConfig::new()
            .with_max_retries(5)
            .with_base_backoff(200)
            .with_max_backoff(10000)
            .with_dlq(false);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_backoff_ms, 200);
        assert_eq!(config.max_backoff_ms, 10000);
        assert!(!config.enable_dlq);
    }

    #[test]
    fn test_calculate_backoff_exponential() {
        let config = RecoveryConfig::new()
            .with_base_backoff(100)
            .with_max_backoff(10000);

        // Attempt 0: 100ms * 2^0 = 100ms
        assert_eq!(config.calculate_backoff(0), Duration::from_millis(100));

        // Attempt 1: 100ms * 2^1 = 200ms
        assert_eq!(config.calculate_backoff(1), Duration::from_millis(200));

        // Attempt 2: 100ms * 2^2 = 400ms
        assert_eq!(config.calculate_backoff(2), Duration::from_millis(400));

        // Attempt 3: 100ms * 2^3 = 800ms
        assert_eq!(config.calculate_backoff(3), Duration::from_millis(800));
    }

    #[test]
    fn test_calculate_backoff_capped() {
        let config = RecoveryConfig::new()
            .with_base_backoff(100)
            .with_max_backoff(500);

        // Attempt 0: 100ms (below cap)
        assert_eq!(config.calculate_backoff(0), Duration::from_millis(100));

        // Attempt 1: 200ms (below cap)
        assert_eq!(config.calculate_backoff(1), Duration::from_millis(200));

        // Attempt 2: 400ms (below cap)
        assert_eq!(config.calculate_backoff(2), Duration::from_millis(400));

        // Attempt 3: 800ms would exceed cap, should be 500ms
        assert_eq!(config.calculate_backoff(3), Duration::from_millis(500));

        // Attempt 10: Would be huge, but capped at 500ms
        assert_eq!(config.calculate_backoff(10), Duration::from_millis(500));
    }

    // ==========================================================================
    // is_transient_error TESTS
    // ==========================================================================

    #[test]
    fn test_connection_errors_are_transient() {
        let err = Error::Connection(crate::error::ConnectionError::Timeout { timeout_ms: 5000 });
        assert!(
            is_transient_error(&err),
            "Connection timeout should be transient"
        );

        let err = Error::Connection(crate::error::ConnectionError::PoolExhausted {
            max_connections: 10,
        });
        assert!(
            is_transient_error(&err),
            "Pool exhausted should be transient"
        );
    }

    #[test]
    fn test_store_failed_timeout_is_transient() {
        let err = Error::StoreFailed {
            operation: "append".to_string(),
            reason: "operation timeout".to_string(),
        };
        assert!(is_transient_error(&err), "Timeout should be transient");

        let err = Error::StoreFailed {
            operation: "timeout".to_string(),
            reason: "database busy".to_string(),
        };
        assert!(
            is_transient_error(&err),
            "Timeout operation should be transient"
        );
    }

    #[test]
    fn test_store_failed_lock_contention_is_transient() {
        let err = Error::StoreFailed {
            operation: "append".to_string(),
            reason: "database lock contention".to_string(),
        };
        assert!(
            is_transient_error(&err),
            "Lock contention should be transient"
        );

        let err = Error::StoreFailed {
            operation: "read".to_string(),
            reason: "resource locked by another process".to_string(),
        };
        assert!(
            is_transient_error(&err),
            "Locked resource should be transient"
        );
    }

    #[test]
    fn test_store_failed_temporary_is_transient() {
        let err = Error::StoreFailed {
            operation: "append".to_string(),
            reason: "temporary network glitch".to_string(),
        };
        assert!(
            is_transient_error(&err),
            "Temporary error should be transient"
        );
    }

    #[test]
    fn test_serialization_errors_are_not_transient() {
        let err = Error::Serialization {
            reason: "invalid data format".to_string(),
        };
        assert!(
            !is_transient_error(&err),
            "Serialization errors should be permanent"
        );
    }

    #[test]
    fn test_invalid_event_errors_are_not_transient() {
        let err = Error::InvalidEvent {
            reason: "missing required field".to_string(),
        };
        assert!(
            !is_transient_error(&err),
            "Invalid event should be permanent"
        );
    }

    #[test]
    fn test_event_not_found_is_not_transient() {
        let err = Error::EventNotFound {
            event_id: "evt-123".to_string(),
        };
        assert!(
            !is_transient_error(&err),
            "Event not found should be permanent"
        );
    }

    #[test]
    fn test_invalid_transition_is_not_transient() {
        let err = Error::InvalidTransition {
            from: "open".to_string(),
            to: "completed".to_string(),
        };
        assert!(
            !is_transient_error(&err),
            "Invalid transition should be permanent"
        );
    }

    #[test]
    fn test_channel_closed_is_not_transient() {
        let err = Error::ChannelClosed;
        assert!(
            !is_transient_error(&err),
            "Channel closed should be permanent"
        );
    }

    #[test]
    fn test_internal_errors_are_not_transient() {
        let err = Error::Internal("critical failure".to_string());
        assert!(
            !is_transient_error(&err),
            "Internal errors should be permanent"
        );
    }

    #[test]
    fn test_projection_failed_timeout_is_transient() {
        let err = Error::ProjectionFailed {
            projection: "user-view".to_string(),
            reason: "query timeout".to_string(),
        };
        assert!(
            is_transient_error(&err),
            "Projection timeout should be transient"
        );
    }

    #[test]
    fn test_projection_failed_lock_is_transient() {
        let err = Error::ProjectionFailed {
            projection: "order-summary".to_string(),
            reason: "resource lock contention".to_string(),
        };
        assert!(
            is_transient_error(&err),
            "Projection lock should be transient"
        );
    }

    #[test]
    fn test_projection_failed_permanent_is_not_transient() {
        let err = Error::ProjectionFailed {
            projection: "metrics".to_string(),
            reason: "invalid projection configuration".to_string(),
        };
        assert!(
            !is_transient_error(&err),
            "Invalid projection should be permanent"
        );
    }

    #[test]
    fn test_store_failed_permanent_is_not_transient() {
        let err = Error::StoreFailed {
            operation: "append".to_string(),
            reason: "disk full".to_string(),
        };
        assert!(!is_transient_error(&err), "Disk full should be permanent");

        let err = Error::StoreFailed {
            operation: "read".to_string(),
            reason: "corrupted data".to_string(),
        };
        assert!(
            !is_transient_error(&err),
            "Corrupted data should be permanent"
        );
    }

    // ==========================================================================
    // RetryPolicy TESTS
    // ==========================================================================

    #[test]
    fn test_retry_policy_default_config() {
        let policy = RetryPolicy::new();
        let config = policy.config();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_backoff_ms, 100);
        assert!(config.enable_dlq);
    }

    #[test]
    fn test_retry_policy_custom_config() {
        let config = RecoveryConfig::new().with_max_retries(5).with_dlq(false);
        let policy = RetryPolicy::with_config(config.clone());

        assert_eq!(policy.config().max_retries, 5);
        assert!(!policy.config().enable_dlq);
    }

    #[test]
    fn test_should_retry_transient_error_first_attempt() {
        let policy = RetryPolicy::new();
        let err = Error::Connection(crate::error::ConnectionError::Timeout { timeout_ms: 5000 });

        match policy.should_retry(&err, 0) {
            RecoveryStrategy::Retry { attempt, delay } => {
                assert_eq!(attempt, 1, "Should increment attempt to 1");
                assert_eq!(delay, Duration::from_millis(100), "Should use base backoff");
            }
            other => assert!(
                matches!(other, RecoveryStrategy::Retry { .. }),
                "Expected Retry strategy for transient error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_retry_transient_error_second_attempt() {
        let policy = RetryPolicy::new();
        let err = Error::StoreFailed {
            operation: "append".to_string(),
            reason: "lock contention".to_string(),
        };

        match policy.should_retry(&err, 1) {
            RecoveryStrategy::Retry { attempt, delay } => {
                assert_eq!(attempt, 2, "Should increment attempt to 2");
                assert_eq!(delay, Duration::from_millis(200), "Should double backoff");
            }
            other => assert!(
                matches!(other, RecoveryStrategy::Retry { .. }),
                "Expected Retry strategy for transient error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_retry_transient_error_third_attempt() {
        let policy = RetryPolicy::new();
        let err = Error::ProjectionFailed {
            projection: "view".to_string(),
            reason: "timeout".to_string(),
        };

        match policy.should_retry(&err, 2) {
            RecoveryStrategy::Retry { attempt, delay } => {
                assert_eq!(attempt, 3, "Should increment attempt to 3");
                assert_eq!(
                    delay,
                    Duration::from_millis(400),
                    "Should quadruple backoff"
                );
            }
            other => assert!(
                matches!(other, RecoveryStrategy::Retry { .. }),
                "Expected Retry strategy for transient error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_send_to_dlq_after_max_retries() {
        let policy = RetryPolicy::new();
        let err = Error::Connection(crate::error::ConnectionError::Timeout { timeout_ms: 5000 });

        // After 3 retries (attempt 3), should send to DLQ
        match policy.should_retry(&err, 3) {
            RecoveryStrategy::SkipToDlq => {
                // Expected
            }
            other => assert!(
                matches!(other, RecoveryStrategy::SkipToDlq),
                "Expected SkipToDlq after max retries, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_fail_when_dlq_disabled_after_max_retries() {
        let config = RecoveryConfig::new().with_dlq(false);
        let policy = RetryPolicy::with_config(config);
        let err = Error::Connection(crate::error::ConnectionError::Timeout { timeout_ms: 5000 });

        // After 3 retries (attempt 3), should fail (no DLQ)
        match policy.should_retry(&err, 3) {
            RecoveryStrategy::Fail => {
                // Expected
            }
            other => assert!(
                matches!(other, RecoveryStrategy::Fail),
                "Expected Fail when DLQ disabled after max retries, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_skip_to_dlq_for_permanent_error_with_dlq_enabled() {
        let policy = RetryPolicy::new();
        let err = Error::InvalidEvent {
            reason: "corrupted data".to_string(),
        };

        // Permanent error with DLQ enabled should skip
        match policy.should_retry(&err, 0) {
            RecoveryStrategy::SkipToDlq => {
                // Expected
            }
            other => assert!(
                matches!(other, RecoveryStrategy::SkipToDlq),
                "Expected SkipToDlq for permanent error with DLQ enabled, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_fail_for_permanent_error_with_dlq_disabled() {
        let config = RecoveryConfig::new().with_dlq(false);
        let policy = RetryPolicy::with_config(config);
        let err = Error::EventNotFound {
            event_id: "evt-123".to_string(),
        };

        // Permanent error with DLQ disabled should fail
        match policy.should_retry(&err, 0) {
            RecoveryStrategy::Fail => {
                // Expected
            }
            other => assert!(
                matches!(other, RecoveryStrategy::Fail),
                "Expected Fail for permanent error with DLQ disabled, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_should_fail_for_permanent_error_immediately() {
        let policy = RetryPolicy::new();
        let err = Error::InvalidTransition {
            from: "open".to_string(),
            to: "completed".to_string(),
        };

        // Invalid transition is permanent, should fail immediately (even with DLQ)
        // Note: With DLQ enabled, it goes to DLQ instead of failing
        match policy.should_retry(&err, 0) {
            RecoveryStrategy::SkipToDlq => {
                // With DLQ, permanent errors go to DLQ
            }
            other => assert!(
                matches!(other, RecoveryStrategy::SkipToDlq),
                "Expected SkipToDlq for permanent error, got {:?}",
                other
            ),
        }
    }

    // ==========================================================================
    // RecoveryStrategy DISPLAY TESTS
    // ==========================================================================

    #[test]
    fn test_recovery_strategy_retry_display() {
        let strategy = RecoveryStrategy::Retry {
            attempt: 2,
            delay: Duration::from_millis(200),
        };
        // Just verify it can be created and compared
        assert_eq!(
            strategy,
            RecoveryStrategy::Retry {
                attempt: 2,
                delay: Duration::from_millis(200)
            }
        );
    }

    #[test]
    fn test_recovery_strategy_equality() {
        let retry1 = RecoveryStrategy::Retry {
            attempt: 1,
            delay: Duration::from_millis(100),
        };
        let retry2 = RecoveryStrategy::Retry {
            attempt: 1,
            delay: Duration::from_millis(100),
        };
        assert_eq!(retry1, retry2);

        let dlq1 = RecoveryStrategy::SkipToDlq;
        let dlq2 = RecoveryStrategy::SkipToDlq;
        assert_eq!(dlq1, dlq2);

        let fail1 = RecoveryStrategy::Fail;
        let fail2 = RecoveryStrategy::Fail;
        assert_eq!(fail1, fail2);

        assert_ne!(retry1, dlq1);
        assert_ne!(dlq1, fail1);
    }

    // ==========================================================================
    // retry_with_policy ASYNC PIPELINE TESTS
    // ==========================================================================

    #[tokio::test]
    async fn test_retry_with_policy_immediate_success() {
        let policy = RetryPolicy::new();
        let dlq = InMemoryDeadLetterQueue::new();

        let result = retry_with_policy(
            &|| Box::pin(async { Ok::<_, Error>(42) }),
            &policy,
            &dlq,
            "evt-1",
            None,
        )
        .await;

        assert_eq!(result.ok(), Some(42));
        assert_eq!(dlq.count().ok(), Some(0), "DLQ should be empty");
    }

    #[tokio::test]
    async fn test_retry_with_policy_transient_error_recovers() {
        let config = RecoveryConfig::new().with_max_retries(3);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let result = retry_with_policy(
            &move || {
                let counter = std::sync::Arc::clone(&attempt_count);
                Box::pin(async move {
                    let attempt = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if attempt == 0 {
                        Err(Error::Connection(ConnectionError::Timeout {
                            timeout_ms: 100,
                        }))
                    } else {
                        Ok::<_, Error>(99)
                    }
                })
            },
            &policy,
            &dlq,
            "evt-2",
            None,
        )
        .await;

        assert_eq!(result.ok(), Some(99), "Should succeed on second attempt");
        assert_eq!(dlq.count().ok(), Some(0), "DLQ should be empty");
    }

    #[tokio::test]
    async fn test_retry_with_policy_exhausted_retries_sends_to_dlq() {
        let config = RecoveryConfig::new().with_max_retries(2);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let result = retry_with_policy(
            &|| {
                Box::pin(async {
                    Err::<(), _>(Error::Connection(ConnectionError::Timeout {
                        timeout_ms: 100,
                    }))
                })
            },
            &policy,
            &dlq,
            "evt-3",
            Some(vec![1, 2, 3]),
        )
        .await;

        assert!(result.is_err(), "Should fail after max retries");
        assert_eq!(dlq.count().ok(), Some(1), "DLQ should have one event");

        let poison_events = dlq.get_poison_events().ok();
        let events = poison_events.as_deref();
        assert!(
            events.is_some_and(|e| !e.is_empty()),
            "DLQ should contain the event"
        );
        if let Some(events) = events {
            assert_eq!(events[0].event_id, "evt-3");
            assert_eq!(events[0].attempt_count, 2);
            assert_eq!(events[0].event_data, Some(vec![1, 2, 3]));
        }
    }

    #[tokio::test]
    async fn test_retry_with_policy_permanent_error_sends_to_dlq() {
        let policy = RetryPolicy::new();
        let dlq = InMemoryDeadLetterQueue::new();

        let result = retry_with_policy(
            &|| {
                Box::pin(async {
                    Err::<(), _>(Error::InvalidEvent {
                        reason: "corrupted".to_string(),
                    })
                })
            },
            &policy,
            &dlq,
            "evt-4",
            None,
        )
        .await;

        assert!(result.is_err(), "Should fail on permanent error");
        assert_eq!(dlq.count().ok(), Some(1), "DLQ should have one event");

        let poison_events = dlq.get_poison_events().ok();
        let events = poison_events.as_deref();
        assert!(
            events.is_some_and(|e| !e.is_empty()),
            "DLQ should contain the event"
        );
        if let Some(events) = events {
            assert_eq!(events[0].event_id, "evt-4");
            assert_eq!(
                events[0].attempt_count, 0,
                "Should not retry permanent errors"
            );
            assert!(events[0].error.contains("corrupted"));
        }
    }

    #[tokio::test]
    async fn test_retry_with_policy_dlq_disabled_fails_immediately() {
        let config = RecoveryConfig::new().with_dlq(false).with_max_retries(3);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let result = retry_with_policy(
            &|| {
                Box::pin(async {
                    Err::<(), _>(Error::Connection(ConnectionError::Timeout {
                        timeout_ms: 100,
                    }))
                })
            },
            &policy,
            &dlq,
            "evt-5",
            None,
        )
        .await;

        assert!(result.is_err(), "Should fail when DLQ disabled");
        assert_eq!(
            dlq.count().ok(),
            Some(0),
            "DLQ should be empty when disabled"
        );
    }

    #[tokio::test]
    async fn test_retry_with_policy_permanent_error_dlq_disabled_fails() {
        let config = RecoveryConfig::new().with_dlq(false);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let result = retry_with_policy(
            &|| {
                Box::pin(async {
                    Err::<(), _>(Error::EventNotFound {
                        event_id: "missing".to_string(),
                    })
                })
            },
            &policy,
            &dlq,
            "evt-6",
            None,
        )
        .await;

        assert!(result.is_err(), "Should fail on permanent error");
        assert_eq!(
            dlq.count().ok(),
            Some(0),
            "DLQ should be empty when disabled"
        );
    }

    #[tokio::test]
    async fn test_retry_with_policy_custom_backoff() {
        let config = RecoveryConfig::new()
            .with_base_backoff(10)
            .with_max_retries(3);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let start = std::time::Instant::now();
        let _result = retry_with_policy(
            &|| {
                Box::pin(async {
                    Err::<(), _>(Error::Connection(ConnectionError::Timeout {
                        timeout_ms: 100,
                    }))
                })
            },
            &policy,
            &dlq,
            "evt-7",
            None,
        )
        .await;

        let elapsed = start.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(10),
            "Should respect backoff delay"
        );
    }

    // ==========================================================================
    // log_poison_event TESTS
    // ==========================================================================

    #[test]
    fn test_log_poison_event_returns_unit() {
        let error = Error::InvalidEvent {
            reason: "corrupted".to_string(),
        };

        // Just ensure it compiles and doesn't panic
        log_poison_event("evt-1", 3, &error);
    }

    // ==========================================================================
    // log_failed_event TESTS
    // ==========================================================================

    #[test]
    fn test_log_failed_event_returns_unit() {
        let error = Error::Connection(ConnectionError::Timeout { timeout_ms: 100 });

        // Just ensure it compiles and doesn't panic
        log_failed_event("evt-2", 1, &error);
    }

    #[test]
    fn test_log_failed_event_with_transient_error() {
        let error = Error::StoreFailed {
            operation: "append".to_string(),
            reason: "lock contention".to_string(),
        };

        // Just ensure it compiles and doesn't panic
        log_failed_event("evt-3", 2, &error);
    }

    #[test]
    fn test_log_failed_event_with_permanent_error() {
        let error = Error::InvalidEvent {
            reason: "missing field".to_string(),
        };

        // Just ensure it compiles and doesn't panic
        log_failed_event("evt-4", 0, &error);
    }

    // ==========================================================================
    // Railway-Oriented Integration Tests
    // ==========================================================================

    #[tokio::test]
    async fn test_railway_retry_chain_all_success() {
        let policy = RetryPolicy::new();
        let dlq = InMemoryDeadLetterQueue::new();

        // Simulate a chain of operations
        let result1 = retry_with_policy(
            &|| Box::pin(async { Ok::<_, Error>(10) }),
            &policy,
            &dlq,
            "step-1",
            None,
        )
        .await;
        let result2 = retry_with_policy(
            &|| Box::pin(async { Ok::<_, Error>(20) }),
            &policy,
            &dlq,
            "step-2",
            None,
        )
        .await;
        let result3 = retry_with_policy(
            &|| Box::pin(async { Ok::<_, Error>(30) }),
            &policy,
            &dlq,
            "step-3",
            None,
        )
        .await;

        assert_eq!(result1.ok(), Some(10));
        assert_eq!(result2.ok(), Some(20));
        assert_eq!(result3.ok(), Some(30));
        assert_eq!(dlq.count().ok(), Some(0));
    }

    #[tokio::test]
    async fn test_railway_retry_chain_middle_fails_to_dlq() {
        let config = RecoveryConfig::new().with_max_retries(2);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let result1 = retry_with_policy(
            &|| Box::pin(async { Ok::<_, Error>(10) }),
            &policy,
            &dlq,
            "step-1",
            None,
        )
        .await;
        let result2 = retry_with_policy(
            &|| {
                Box::pin(async {
                    Err::<i32, Error>(Error::InvalidEvent {
                        reason: "bad data".to_string(),
                    })
                })
            },
            &policy,
            &dlq,
            "step-2",
            None,
        )
        .await;

        assert_eq!(result1.ok(), Some(10), "First step should succeed");
        assert!(result2.is_err(), "Second step should fail");
        assert_eq!(dlq.count().ok(), Some(1), "Step 2 should be in DLQ");

        // Step 3 should not execute (short-circuit)
    }

    #[tokio::test]
    async fn test_railway_retry_transient_recovers_continues_chain() {
        let config = RecoveryConfig::new().with_max_retries(3);
        let policy = RetryPolicy::with_config(config);
        let dlq = InMemoryDeadLetterQueue::new();

        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let result1 = retry_with_policy(
            &|| Box::pin(async { Ok::<_, Error>(10) }),
            &policy,
            &dlq,
            "step-1",
            None,
        )
        .await;
        let result2 = retry_with_policy(
            &move || {
                let counter = std::sync::Arc::clone(&attempt);
                Box::pin(async move {
                    let current = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if current == 0 {
                        Err::<_, Error>(Error::Connection(ConnectionError::Timeout {
                            timeout_ms: 100,
                        }))
                    } else {
                        Ok::<_, Error>(20)
                    }
                })
            },
            &policy,
            &dlq,
            "step-2",
            None,
        )
        .await;

        assert_eq!(result1.ok(), Some(10));
        assert_eq!(result2.ok(), Some(20), "Second step should recover");
        assert_eq!(dlq.count().ok(), Some(0), "DLQ should be empty");
    }
}
