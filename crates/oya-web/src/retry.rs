//! Retry logic with exponential backoff and jitter.
//!
//! Provides configurable retry strategies for transient HTTP errors.
//! Only retries transient failures (5xx, timeouts, network errors).
//! Permanent errors (4xx) are never retried.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::error_handler::{ErrorCategory, HttpError};
use rand::Rng;
use std::time::Duration;

/// Default retry configuration.
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_BASE_DELAY_MS: u64 = 100;
const DEFAULT_MAX_DELAY_MS: u64 = 10000;
const DEFAULT_JITTER_FACTOR: f64 = 0.1;

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay for exponential backoff (milliseconds)
    pub base_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Jitter factor to add randomness (0.0 - 1.0)
    pub jitter_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay_ms: DEFAULT_BASE_DELAY_MS,
            max_delay_ms: DEFAULT_MAX_DELAY_MS,
            jitter_factor: DEFAULT_JITTER_FACTOR,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with custom settings.
    pub fn new(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
            jitter_factor: DEFAULT_JITTER_FACTOR,
        }
    }

    /// Set jitter factor.
    pub fn with_jitter(mut self, jitter_factor: f64) -> Self {
        self.jitter_factor = jitter_factor;
        self
    }

    /// Calculate delay for the given attempt number using exponential backoff with jitter.
    ///
    /// Delay formula: min(base_delay * 2^attempt + jitter, max_delay)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let exponential_delay = self.base_delay_ms.saturating_mul(2_u64.pow(attempt));
        let capped_delay = exponential_delay.min(self.max_delay_ms);

        // Add jitter to prevent thundering herd
        let jitter_ms = if self.jitter_factor > 0.0 {
            let jitter_range = (capped_delay as f64) * self.jitter_factor;
            rand::thread_rng().gen_range(0.0..jitter_range).floor() as u64
        } else {
            0
        };

        Duration::from_millis(capped_delay.saturating_add(jitter_ms))
    }

    /// Determine if an error should be retried.
    pub fn should_retry(&self, error: &HttpError) -> bool {
        match error.category() {
            ErrorCategory::Network | ErrorCategory::Timeout | ErrorCategory::Server => true,
            ErrorCategory::Client | ErrorCategory::Validation | ErrorCategory::Auth => false,
            ErrorCategory::Unknown => false,
        }
    }

    /// Determine if an error category should be retried.
    pub fn should_retry_category(&self, category: ErrorCategory) -> bool {
        matches!(
            category,
            ErrorCategory::Network | ErrorCategory::Timeout | ErrorCategory::Server
        )
    }

    /// Create a retry state for tracking attempts.
    pub fn state(&self) -> RetryState {
        RetryState::new(self.clone())
    }
}

/// Retry state for tracking retry attempts.
#[derive(Debug, Clone)]
pub struct RetryState {
    policy: RetryPolicy,
    attempt: u32,
}

impl RetryState {
    /// Create a new retry state.
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy, attempt: 0 }
    }

    /// Get current attempt number (0-indexed).
    pub fn current_attempt(&self) -> u32 {
        self.attempt
    }

    /// Check if more retries are available.
    pub fn can_retry(&self) -> bool {
        self.attempt < self.policy.max_retries
    }

    /// Record a retry attempt and get the delay before next retry.
    ///
    /// Returns None if no more retries are available.
    pub fn next_retry(&mut self) -> Option<Duration> {
        if !self.can_retry() {
            return None;
        }

        let delay = self.policy.calculate_delay(self.attempt);
        self.attempt = self.attempt.saturating_add(1);
        Some(delay)
    }

    /// Check if error should be retried and get delay.
    pub fn should_retry_with_delay(&mut self, error: &HttpError) -> Option<Duration> {
        if self.policy.should_retry(error) {
            self.next_retry()
        } else {
            None
        }
    }

    /// Reset retry state to initial state.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

/// Retry decision with metadata.
#[derive(Debug, Clone)]
pub struct RetryDecision {
    /// Whether to retry the operation
    pub should_retry: bool,
    /// Delay before next retry (if applicable)
    pub delay: Option<Duration>,
    /// Current attempt number
    pub attempt: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
}

impl RetryDecision {
    /// Create a decision to not retry.
    pub fn no_retry(attempt: u32, max_retries: u32) -> Self {
        Self {
            should_retry: false,
            delay: None,
            attempt,
            max_retries,
        }
    }

    /// Create a decision to retry with delay.
    pub fn retry_with_delay(attempt: u32, max_retries: u32, delay: Duration) -> Self {
        Self {
            should_retry: true,
            delay: Some(delay),
            attempt,
            max_retries,
        }
    }

    /// Get remaining retry attempts.
    pub fn remaining_attempts(&self) -> u32 {
        self.max_retries.saturating_sub(self.attempt)
    }
}

/// Calculate exponential backoff delay.
///
/// # Arguments
///
/// * `attempt` - Current retry attempt (0-indexed)
/// * `base_delay_ms` - Base delay in milliseconds
/// * `max_delay_ms` - Maximum delay in milliseconds
///
/// # Returns
///
/// Duration to wait before next retry
#[allow(dead_code)]
pub fn exponential_backoff(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> Duration {
    let exponential_delay = base_delay_ms.saturating_mul(2_u64.pow(attempt));
    let capped_delay = exponential_delay.min(max_delay_ms);
    Duration::from_millis(capped_delay)
}

/// Calculate exponential backoff with jitter.
///
/// Adds randomness to prevent thundering herd problem.
#[allow(dead_code)]
pub fn exponential_backoff_with_jitter(
    attempt: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    jitter_factor: f64,
) -> Duration {
    let base_delay = exponential_backoff(attempt, base_delay_ms, max_delay_ms);

    if jitter_factor <= 0.0 {
        return base_delay;
    }

    let jitter_ms = (base_delay.as_millis() as f64 * jitter_factor).floor() as u64;
    let jitter = rand::thread_rng().gen_range(0..=jitter_ms);

    base_delay
        .checked_add(Duration::from_millis(jitter))
        .unwrap_or(base_delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.base_delay_ms, 100);
        assert_eq!(policy.max_delay_ms, 10000);
        assert_eq!(policy.jitter_factor, 0.1);
    }

    #[test]
    fn test_retry_policy_new() {
        let policy = RetryPolicy::new(5, 200, 20000);
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.base_delay_ms, 200);
        assert_eq!(policy.max_delay_ms, 20000);
    }

    #[test]
    fn test_retry_policy_with_jitter() {
        let policy = RetryPolicy::new(3, 100, 1000).with_jitter(0.2);
        assert_eq!(policy.jitter_factor, 0.2);
    }

    #[test]
    fn test_calculate_delay_no_capping() {
        let policy = RetryPolicy::new(3, 100, 10000);

        // Attempt 0: 100ms
        let delay0 = policy.calculate_delay(0);
        assert!(delay0.as_millis() >= 100);
        assert!(delay0.as_millis() < 200); // 100 + jitter

        // Attempt 1: 200ms
        let delay1 = policy.calculate_delay(1);
        assert!(delay1.as_millis() >= 200);
        assert!(delay1.as_millis() < 300); // 200 + jitter

        // Attempt 2: 400ms
        let delay2 = policy.calculate_delay(2);
        assert!(delay2.as_millis() >= 400);
        assert!(delay2.as_millis() < 500); // 400 + jitter
    }

    #[test]
    fn test_calculate_delay_with_capping() {
        let policy = RetryPolicy::new(10, 100, 500).with_jitter(0.0); // No jitter for this test

        // Attempt 0: 100ms (no cap)
        let delay0 = policy.calculate_delay(0);
        assert!(delay0.as_millis() >= 100);

        // Attempt 3: 800ms -> capped to 500ms
        let delay3 = policy.calculate_delay(3);
        assert_eq!(delay3.as_millis(), 500); // Exactly capped with no jitter
    }

    #[test]
    fn test_should_retry_network_error() {
        let policy = RetryPolicy::default();
        let error = HttpError::Network {
            message: "Connection refused".to_string(),
            source: None,
        };

        assert!(policy.should_retry(&error));
    }

    #[test]
    fn test_should_retry_timeout_error() {
        let policy = RetryPolicy::default();
        let error = HttpError::Timeout {
            duration_secs: 30,
            operation: "API call".to_string(),
        };

        assert!(policy.should_retry(&error));
    }

    #[test]
    fn test_should_retry_server_error() {
        let policy = RetryPolicy::default();
        let error = HttpError::Server {
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal error".to_string(),
        };

        assert!(policy.should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_client_error() {
        let policy = RetryPolicy::default();
        let error = HttpError::Client {
            status: axum::http::StatusCode::NOT_FOUND,
            message: "Not found".to_string(),
        };

        assert!(!policy.should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_validation_error() {
        let policy = RetryPolicy::default();
        let error = HttpError::Validation {
            field: "email".to_string(),
            message: "Invalid email".to_string(),
        };

        assert!(!policy.should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_auth_error() {
        let policy = RetryPolicy::default();
        let error = HttpError::Auth {
            message: "Unauthorized".to_string(),
        };

        assert!(!policy.should_retry(&error));
    }

    #[test]
    fn test_retry_state_new() {
        let policy = RetryPolicy::default();
        let state = policy.state();

        assert_eq!(state.current_attempt(), 0);
        assert!(state.can_retry());
    }

    #[test]
    fn test_retry_state_next_retry() {
        let policy = RetryPolicy::new(3, 100, 1000);
        let mut state = policy.state();

        // First retry
        let delay1 = state.next_retry();
        assert!(delay1.is_some());
        assert_eq!(state.current_attempt(), 1);

        // Second retry
        let delay2 = state.next_retry();
        assert!(delay2.is_some());
        assert_eq!(state.current_attempt(), 2);

        // Third retry
        let delay3 = state.next_retry();
        assert!(delay3.is_some());
        assert_eq!(state.current_attempt(), 3);

        // No more retries
        let delay4 = state.next_retry();
        assert!(delay4.is_none());
        assert_eq!(state.current_attempt(), 3);
    }

    #[test]
    fn test_retry_state_should_retry_with_delay() {
        let policy = RetryPolicy::new(3, 100, 1000);
        let mut state = policy.state();

        let error = HttpError::Network {
            message: "Connection refused".to_string(),
            source: None,
        };

        // Should retry network error
        let delay1 = state.should_retry_with_delay(&error);
        assert!(delay1.is_some());

        // Should not retry client error
        let client_error = HttpError::Client {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: "Bad request".to_string(),
        };
        let delay2 = state.should_retry_with_delay(&client_error);
        assert!(delay2.is_none());
    }

    #[test]
    fn test_retry_state_reset() {
        let policy = RetryPolicy::new(2, 100, 1000);
        let mut state = policy.state();

        // Use all retries
        let _ = state.next_retry();
        let _ = state.next_retry();
        assert!(!state.can_retry());

        // Reset
        state.reset();
        assert!(state.can_retry());
        assert_eq!(state.current_attempt(), 0);
    }

    #[test]
    fn test_exponential_backoff() {
        // Attempt 0: 100ms
        let delay0 = exponential_backoff(0, 100, 10000);
        assert_eq!(delay0, Duration::from_millis(100));

        // Attempt 1: 200ms
        let delay1 = exponential_backoff(1, 100, 10000);
        assert_eq!(delay1, Duration::from_millis(200));

        // Attempt 2: 400ms
        let delay2 = exponential_backoff(2, 100, 10000);
        assert_eq!(delay2, Duration::from_millis(400));

        // Attempt 3: 800ms
        let delay3 = exponential_backoff(3, 100, 10000);
        assert_eq!(delay3, Duration::from_millis(800));
    }

    #[test]
    fn test_exponential_backoff_with_capping() {
        // No cap needed
        let delay0 = exponential_backoff(0, 100, 500);
        assert_eq!(delay0, Duration::from_millis(100));

        // Cap at 500ms
        let delay3 = exponential_backoff(3, 100, 500);
        assert_eq!(delay3, Duration::from_millis(500));

        // Attempt 4 would be 1600ms, but capped to 500ms
        let delay4 = exponential_backoff(4, 100, 500);
        assert_eq!(delay4, Duration::from_millis(500));
    }

    #[test]
    fn test_exponential_backoff_with_jitter() {
        // With jitter, delay should be in range [base, base + jitter]
        let delay0 = exponential_backoff_with_jitter(0, 100, 10000, 0.1);
        assert!(delay0.as_millis() >= 100);
        assert!(delay0.as_millis() < 110);

        let delay1 = exponential_backoff_with_jitter(1, 100, 10000, 0.2);
        assert!(delay1.as_millis() >= 200);
        assert!(delay1.as_millis() < 240); // 200 + 20% jitter
    }

    #[test]
    fn test_retry_decision_no_retry() {
        let decision = RetryDecision::no_retry(2, 3);

        assert!(!decision.should_retry);
        assert!(decision.delay.is_none());
        assert_eq!(decision.attempt, 2);
        assert_eq!(decision.max_retries, 3);
        assert_eq!(decision.remaining_attempts(), 1);
    }

    #[test]
    fn test_retry_decision_retry() {
        let decision = RetryDecision::retry_with_delay(1, 3, Duration::from_millis(200));

        assert!(decision.should_retry);
        assert_eq!(decision.delay, Some(Duration::from_millis(200)));
        assert_eq!(decision.attempt, 1);
        assert_eq!(decision.max_retries, 3);
        assert_eq!(decision.remaining_attempts(), 2);
    }
}
