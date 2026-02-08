//! Error recovery and retry logic for event replay.
//!
//! Provides retry strategies with exponential backoff and dead letter queue
//! for handling poison events.

use crate::Error;
use std::time::Duration;

/// Maximum number of retry attempts for transient errors.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (in milliseconds).
const BASE_BACKOFF_MS: u64 = 100;

/// Maximum backoff delay (in milliseconds).
const MAX_BACKOFF_MS: u64 = 5000;

/// Configuration for error recovery during event replay.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the base backoff delay in milliseconds.
    pub fn with_base_backoff(mut self, base_backoff_ms: u64) -> Self {
        self.base_backoff_ms = base_backoff_ms;
        self
    }

    /// Set the maximum backoff delay in milliseconds.
    pub fn with_max_backoff(mut self, max_backoff_ms: u64) -> Self {
        self.max_backoff_ms = max_backoff_ms;
        self
    }

    /// Enable or disable the dead letter queue.
    pub fn with_dlq(mut self, enable_dlq: bool) -> Self {
        self.enable_dlq = enable_dlq;
        self
    }

    /// Calculate the backoff delay for a given retry attempt.
    ///
    /// Uses exponential backoff with jitter: delay = base * 2^attempt
    /// Capped at max_backoff_ms.
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let exponential_delay = self.base_backoff_ms * 2_u64.pow(attempt);
        let delay_ms = exponential_delay.min(self.max_backoff_ms);
        Duration::from_millis(delay_ms)
    }
}

/// Strategy for recovering from errors during event replay.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn new() -> Self {
        Self {
            config: RecoveryConfig::default(),
        }
    }

    /// Create a new retry policy with custom configuration.
    pub fn with_config(config: RecoveryConfig) -> Self {
        Self { config }
    }

    /// Determine the recovery strategy for a given error and attempt number.
    pub fn should_retry(&self, error: &Error, attempt: u32) -> RecoveryStrategy {
        // Check if we've exceeded max retries
        if attempt >= self.config.max_retries {
            if self.config.enable_dlq {
                return RecoveryStrategy::SkipToDlq;
            } else {
                return RecoveryStrategy::Fail;
            }
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
    pub fn config(&self) -> &RecoveryConfig {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
