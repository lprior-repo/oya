//! Circuit breaker pattern implementation.
//!
//! Prevents cascading failures by automatically stopping requests to a failing service.
//! Tracks failures in a sliding window and transitions between states:
//! - Closed: Normal operation, requests pass through
//! - Open: Circuit is tripped, requests fail immediately
//! - Half-Open: Testing if service has recovered

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

/// Circuit breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed - normal operation
    Closed,
    /// Circuit is open - rejecting requests
    Open,
    /// Circuit is half-open - testing recovery
    HalfOpen,
}

/// Default circuit breaker configuration.
const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_WINDOW_SIZE_SECS: u64 = 60;

/// Circuit breaker configuration.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures to trip the circuit
    pub failure_threshold: u32,
    /// Number of successes to close the circuit (in half-open state)
    pub success_threshold: u32,
    /// How long to wait before transitioning from Open to Half-Open (seconds)
    pub timeout_secs: u64,
    /// Sliding window size for tracking failures (seconds)
    pub window_size_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            success_threshold: DEFAULT_SUCCESS_THRESHOLD,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            window_size_secs: DEFAULT_WINDOW_SIZE_SECS,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker config.
    pub fn new(failure_threshold: u32, timeout_secs: u64) -> Self {
        Self {
            failure_threshold,
            success_threshold: DEFAULT_SUCCESS_THRESHOLD,
            timeout_secs,
            window_size_secs: DEFAULT_WINDOW_SIZE_SECS,
        }
    }

    /// Set success threshold.
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set window size.
    pub fn with_window_size(mut self, window_size_secs: u64) -> Self {
        self.window_size_secs = window_size_secs;
        self
    }
}

/// Result of a request through the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestResult {
    /// Request succeeded
    Success,
    /// Request failed
    Failure,
}

/// A single failure or success record with timestamp.
#[derive(Debug, Clone)]
struct EventRecord {
    timestamp: Instant,
    result: RequestResult,
}

/// Circuit breaker with sliding window failure tracking.
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    events: Vec<EventRecord>,
    state_changed_at: Instant,
    consecutive_successes: u32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default config.
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom config.
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            events: Vec::new(),
            state_changed_at: Instant::now(),
            consecutive_successes: 0,
        }
    }

    /// Get current circuit state.
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Check if a request is allowed.
    ///
    /// Returns true if request should proceed, false if rejected.
    pub fn allow_request(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let elapsed = self.state_changed_at.elapsed();
                if elapsed >= Duration::from_secs(self.config.timeout_secs) {
                    // Transition to Half-Open
                    false // Will be transitioned in record_result
                } else {
                    false // Circuit is still open
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record the result of a request and update circuit state.
    pub fn record_result(&mut self, result: RequestResult) {
        // Add event to sliding window
        self.events.push(EventRecord {
            timestamp: Instant::now(),
            result,
        });

        // Clean old events outside the window
        self.clean_old_events();

        // Update state based on result and current state
        match self.state {
            CircuitState::Closed => {
                self.handle_closed_state(result);
            }
            CircuitState::Open => {
                // Check if we should transition to Half-Open
                let elapsed = self.state_changed_at.elapsed();
                if elapsed >= Duration::from_secs(self.config.timeout_secs) {
                    self.transition_to_half_open();
                }
            }
            CircuitState::HalfOpen => {
                self.handle_half_open_state(result);
            }
        }
    }

    /// Handle recording result in Closed state.
    fn handle_closed_state(&mut self, result: RequestResult) {
        if result == RequestResult::Failure {
            let failure_count = self.count_failures_in_window();
            if failure_count >= self.config.failure_threshold {
                self.transition_to_open();
            }
        }
    }

    /// Handle recording result in Half-Open state.
    fn handle_half_open_state(&mut self, result: RequestResult) {
        match result {
            RequestResult::Success => {
                self.consecutive_successes = self.consecutive_successes.saturating_add(1);
                if self.consecutive_successes >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            RequestResult::Failure => {
                // Any failure in Half-Open trips the circuit again
                self.transition_to_open();
            }
        }
    }

    /// Transition to Open state.
    fn transition_to_open(&mut self) {
        self.state = CircuitState::Open;
        self.state_changed_at = Instant::now();
        self.consecutive_successes = 0;
    }

    /// Transition to Closed state.
    fn transition_to_closed(&mut self) {
        self.state = CircuitState::Closed;
        self.state_changed_at = Instant::now();
        self.consecutive_successes = 0;
        self.events.clear(); // Clear history on successful recovery
    }

    /// Transition to Half-Open state.
    fn transition_to_half_open(&mut self) {
        self.state = CircuitState::HalfOpen;
        self.state_changed_at = Instant::now();
        self.consecutive_successes = 0;
    }

    /// Count failures in the sliding window.
    fn count_failures_in_window(&self) -> u32 {
        self.events
            .iter()
            .filter(|e| {
                e.result == RequestResult::Failure
                    && e.timestamp.elapsed() < Duration::from_secs(self.config.window_size_secs)
            })
            .count() as u32
    }

    /// Remove events outside the sliding window.
    fn clean_old_events(&mut self) {
        let window_duration = Duration::from_secs(self.config.window_size_secs);
        self.events
            .retain(|e| e.timestamp.elapsed() < window_duration);
    }

    /// Get failure count in current window.
    pub fn failure_count(&self) -> u32 {
        self.count_failures_in_window()
    }

    /// Get success count in current window.
    pub fn success_count(&self) -> u32 {
        self.events
            .iter()
            .filter(|e| {
                e.result == RequestResult::Success
                    && e.timestamp.elapsed() < Duration::from_secs(self.config.window_size_secs)
            })
            .count() as u32
    }

    /// Get time since state changed.
    pub fn time_since_state_change(&self) -> Duration {
        self.state_changed_at.elapsed()
    }

    /// Reset circuit breaker to initial state.
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.events.clear();
        self.state_changed_at = Instant::now();
        self.consecutive_successes = 0;
    }

    /// Execute a function with circuit breaker protection.
    ///
    /// Returns Err(CircuitBreakerError::Open) if circuit is open.
    /// Otherwise executes the function and records the result.
    pub async fn call<F, T, E>(&mut self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        if !self.allow_request() {
            return Err(CircuitBreakerError::Open);
        }

        match f.await {
            Ok(result) => {
                self.record_result(RequestResult::Success);
                Ok(result)
            }
            Err(err) => {
                self.record_result(RequestResult::Failure);
                Err(CircuitBreakerError::Inner(err))
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker error types.
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected
    Open,
    /// Inner error from the wrapped function
    Inner(E),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_default() {
        let breaker = CircuitBreaker::new();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_with_config() {
        let config = CircuitBreakerConfig::new(3, 30);
        let breaker = CircuitBreaker::with_config(config);

        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_allow_request_when_closed() {
        let mut breaker = CircuitBreaker::new();
        assert!(breaker.allow_request());
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig::new(3, 60);
        let mut breaker = CircuitBreaker::with_config(config);

        // Record failures
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Closed);

        // Third failure trips the circuit
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_allow_request_when_open() {
        let config = CircuitBreakerConfig::new(2, 1);
        let mut breaker = CircuitBreaker::with_config(config);

        // Trip the circuit
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);

        // Requests should be rejected
        assert!(!breaker.allow_request());
    }

    #[test]
    fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig::new(2, 1);
        let mut breaker = CircuitBreaker::with_config(config);

        // Trip the circuit
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_secs(2));

        // Check allow_request (will trigger transition in record_result)
        breaker.record_result(RequestResult::Failure); // Any event triggers state check
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_closes_after_successes() {
        let config = CircuitBreakerConfig::new(2, 1).with_success_threshold(2);
        let mut breaker = CircuitBreaker::with_config(config);

        // Trip the circuit
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait and transition to Half-Open
        std::thread::sleep(Duration::from_secs(2));
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record successes
        breaker.record_result(RequestResult::Success);
        assert_eq!(breaker.state(), CircuitState::HalfOpen); // Still half-open

        breaker.record_result(RequestResult::Success);
        assert_eq!(breaker.state(), CircuitState::Closed); // Now closed
    }

    #[test]
    fn test_failure_in_half_open_reopens() {
        let config = CircuitBreakerConfig::new(2, 1);
        let mut breaker = CircuitBreaker::with_config(config);

        // Trip the circuit
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait and transition to Half-Open
        std::thread::sleep(Duration::from_secs(2));
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Any failure trips the circuit again
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_failure_count() {
        let config = CircuitBreakerConfig::new(10, 60);
        let mut breaker = CircuitBreaker::with_config(config);

        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Success);

        assert_eq!(breaker.failure_count(), 2);
    }

    #[test]
    fn test_success_count() {
        let config = CircuitBreakerConfig::new(10, 60);
        let mut breaker = CircuitBreaker::with_config(config);

        breaker.record_result(RequestResult::Success);
        breaker.record_result(RequestResult::Success);
        breaker.record_result(RequestResult::Failure);

        assert_eq!(breaker.success_count(), 2);
    }

    #[test]
    fn test_reset() {
        let mut breaker = CircuitBreaker::new();

        // Trip the circuit
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);

        assert_eq!(breaker.state(), CircuitState::Open);

        // Reset
        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_count(), 0);
        assert!(breaker.allow_request());
    }

    #[tokio::test]
    async fn test_call_success() {
        let mut breaker = CircuitBreaker::new();

        async fn success_fn() -> Result<String, &'static str> {
            Ok("success".to_string())
        }

        let result = breaker.call(success_fn()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_call_failure() {
        let mut breaker = CircuitBreaker::new();

        async fn failure_fn() -> Result<String, &'static str> {
            Err("error")
        }

        let result = breaker.call(failure_fn()).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(CircuitBreakerError::Inner(_))));
    }

    #[tokio::test]
    async fn test_call_when_open() {
        let config = CircuitBreakerConfig::new(1, 60);
        let mut breaker = CircuitBreaker::with_config(config);

        // Trip the circuit
        breaker.record_result(RequestResult::Failure);
        assert_eq!(breaker.state(), CircuitState::Open);

        async fn test_fn() -> Result<String, &'static str> {
            Ok("success".to_string())
        }

        let result = breaker.call(test_fn()).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open)));
    }

    #[test]
    fn test_sliding_window_cleanup() {
        let config = CircuitBreakerConfig::new(10, 1).with_window_size(1);
        let mut breaker = CircuitBreaker::with_config(config);

        // Add events
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Success);

        assert_eq!(breaker.failure_count(), 2);

        // Wait for window to expire
        std::thread::sleep(Duration::from_secs(2));

        // Add new event to trigger cleanup
        breaker.record_result(RequestResult::Success);

        // Old events should be cleaned up
        assert_eq!(breaker.failure_count(), 0);
    }

    #[test]
    fn test_time_since_state_change() {
        let mut breaker = CircuitBreaker::new();

        let elapsed = breaker.time_since_state_change();
        assert!(elapsed.as_millis() < 100); // Should be very recent

        // Change state
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);
        breaker.record_result(RequestResult::Failure);

        // Add a small delay to ensure time has passed
        std::thread::sleep(Duration::from_millis(10));

        let elapsed2 = breaker.time_since_state_change();
        assert!(elapsed2.as_millis() < 100); // Should be very recent
        assert!(elapsed2.as_millis() >= 10); // At least 10ms should have passed
    }
}
