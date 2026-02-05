//! Retry logic with exponential backoff.
//!
//! Provides configurable retry strategies for fallible operations,
//! following Railway-Oriented Programming principles.

use std::time::Duration;

use tracing::{debug, warn};

use crate::error::{Error, Result};

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries, just one attempt).
    pub max_attempts: u32,
    /// Initial delay between retries.
    pub initial_delay: Duration,
    /// Maximum delay between retries (caps exponential growth).
    pub max_delay: Duration,
    /// Multiplier for exponential backoff (typically 2.0).
    pub backoff_multiplier: f64,
    /// Whether to add jitter to delays (reduces thundering herd).
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with specified max attempts.
    #[must_use]
    pub const fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Create a new retry config with specified initial delay.
    #[must_use]
    pub const fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Create a new retry config with specified max delay.
    #[must_use]
    pub const fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Create a new retry config with specified backoff multiplier.
    #[must_use]
    pub const fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Create a new retry config with jitter enabled/disabled.
    #[must_use]
    pub const fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// No retries - execute only once.
    #[must_use]
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            ..Default::default()
        }
    }

    /// Quick retries for fast-failing operations.
    #[must_use]
    pub fn quick() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_millis(500),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Standard retries for typical operations.
    #[must_use]
    pub fn standard() -> Self {
        Self::default()
    }

    /// Patient retries for slow or flaky operations.
    #[must_use]
    pub fn patient() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Calculate delay for a given attempt (0-indexed).
    fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let base_delay_ms = self.initial_delay.as_millis() as f64
            * self
                .backoff_multiplier
                .powi(i32::try_from(attempt - 1).unwrap_or(i32::MAX));

        let capped_delay_ms = base_delay_ms.min(self.max_delay.as_millis() as f64);

        let final_delay_ms = if self.jitter {
            // Add up to 25% jitter
            let jitter_factor = 1.0 + (simple_random() * 0.25);
            capped_delay_ms * jitter_factor
        } else {
            capped_delay_ms
        };

        Duration::from_millis(final_delay_ms as u64)
    }
}

/// Outcome of a retry operation.
#[derive(Debug)]
pub struct RetryOutcome<T> {
    /// The successful value, if any.
    pub value: Option<T>,
    /// Number of attempts made.
    pub attempts: u32,
    /// Last error encountered, if operation failed.
    pub last_error: Option<Error>,
    /// Total time spent (including delays).
    pub total_duration: Duration,
}

impl<T> RetryOutcome<T> {
    /// Check if the operation succeeded.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.value.is_some()
    }

    /// Convert to Result, returning the value or the last error.
    pub fn into_result(self) -> Result<T> {
        match (self.value, self.last_error) {
            (Some(v), _) => Ok(v),
            (None, Some(e)) => Err(e),
            (None, None) => Err(Error::InvalidRecord {
                reason: "retry completed with no value and no error".into(),
            }),
        }
    }
}

/// Execute an operation with retry logic.
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation` - The fallible operation to execute
///
/// # Returns
/// `RetryOutcome` containing the result and retry metadata.
pub fn with_retry<T, F>(config: &RetryConfig, mut operation: F) -> RetryOutcome<T>
where
    F: FnMut() -> Result<T>,
{
    let start = std::time::Instant::now();
    let total_attempts = config.max_attempts + 1; // max_attempts is retries, +1 for initial try

    let mut last_error = None;

    for attempt in 0..total_attempts {
        // Calculate and apply delay (no delay for first attempt)
        let delay = config.calculate_delay(attempt);
        if !delay.is_zero() {
            debug!(attempt, delay_ms = ?delay.as_millis(), "Retrying after delay");
            std::thread::sleep(delay);
        }

        match operation() {
            Ok(value) => {
                if attempt > 0 {
                    debug!(attempt, "Operation succeeded after retry");
                }
                return RetryOutcome {
                    value: Some(value),
                    attempts: attempt + 1,
                    last_error: None,
                    total_duration: start.elapsed(),
                };
            }
            Err(e) => {
                warn!(
                    attempt,
                    error = %e,
                    remaining = total_attempts - attempt - 1,
                    "Operation failed"
                );
                last_error = Some(e);
            }
        }
    }

    RetryOutcome {
        value: None,
        attempts: total_attempts,
        last_error,
        total_duration: start.elapsed(),
    }
}

/// Execute an operation with retry, returning Result directly.
///
/// Convenience wrapper around `with_retry` that returns the Result.
pub fn retry<T, F>(config: &RetryConfig, operation: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    with_retry(config, operation).into_result()
}

/// Retry with default configuration.
pub fn retry_default<T, F>(operation: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    retry(&RetryConfig::default(), operation)
}

/// Simple pseudo-random number generator for jitter.
/// Returns a value between 0.0 and 1.0.
fn simple_random() -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);

    let hash = hasher.finish();
    (hash as f64) / (u64::MAX as f64)
}

/// Predicate for determining if an error is retryable.
pub trait Retryable {
    /// Check if this error should trigger a retry.
    fn is_retryable(&self) -> bool;
}

impl Retryable for Error {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::CommandFailed { .. }
                | Error::CommandTimeout { .. }
                | Error::Io(_)
                | Error::Core(_)
                | Error::StageFailed { .. }
        )
    }
}

/// Execute an operation with retry, but only retry on retryable errors.
pub fn retry_on_retryable<T, F>(config: &RetryConfig, mut operation: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let start = std::time::Instant::now();
    let total_attempts = config.max_attempts + 1;

    let mut last_error = None;

    for attempt in 0..total_attempts {
        let delay = config.calculate_delay(attempt);
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }

        match operation() {
            Ok(value) => return Ok(value),
            Err(e) => {
                if !e.is_retryable() {
                    // Non-retryable error, fail immediately
                    return Err(e);
                }
                warn!(
                    attempt,
                    error = %e,
                    elapsed_ms = ?start.elapsed().as_millis(),
                    "Retryable error, will retry"
                );
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| Error::InvalidRecord {
        reason: "retry exhausted with no error".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_retry_succeeds_first_try() {
        let config = RetryConfig::default();
        let result = retry(&config, || Ok(42));
        assert_eq!(result.ok(), Some(42));
    }

    #[test]
    fn test_retry_succeeds_after_failures() {
        let attempts = RefCell::new(0);
        let config = RetryConfig::no_retry().with_max_attempts(3);

        let result = retry(&config, || {
            let mut count = attempts.borrow_mut();
            *count += 1;
            if *count < 3 {
                Err(Error::InvalidRecord {
                    reason: "not yet".into(),
                })
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.ok(), Some(42));
        assert_eq!(*attempts.borrow(), 3);
    }

    #[test]
    fn test_retry_exhausts_attempts() {
        let attempts = RefCell::new(0);
        let config = RetryConfig::no_retry().with_max_attempts(2);

        let result: Result<i32> = retry(&config, || {
            *attempts.borrow_mut() += 1;
            Err(Error::InvalidRecord {
                reason: "always fails".into(),
            })
        });

        assert!(result.is_err());
        assert_eq!(*attempts.borrow(), 3); // initial + 2 retries
    }

    #[test]
    fn test_with_retry_outcome() {
        let config = RetryConfig::no_retry();
        let outcome = with_retry(&config, || Ok(42));

        assert!(outcome.is_success());
        assert_eq!(outcome.attempts, 1);
        assert!(outcome.last_error.is_none());
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(10),
            jitter: false,
            ..Default::default()
        };

        assert_eq!(config.calculate_delay(0), Duration::ZERO);
        assert_eq!(config.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(400));
    }

    #[test]
    fn test_calculate_delay_capped() {
        let config = RetryConfig {
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 10.0,
            max_delay: Duration::from_secs(5),
            jitter: false,
            ..Default::default()
        };

        // 1 * 10^3 = 1000 seconds, but capped at 5
        assert_eq!(config.calculate_delay(4), Duration::from_secs(5));
    }
}
