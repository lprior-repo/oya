use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const MIN_TIMEOUT_SECS: u64 = 1;
const MAX_TIMEOUT_SECS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimeoutSeconds(u64);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimeoutError {
    #[error("timeout must be at least {min} seconds, got {value}")]
    TooSmall { min: u64, value: u64 },
    #[error("timeout must be at most {max} seconds, got {value}")]
    TooLarge { max: u64, value: u64 },
}

impl TimeoutSeconds {
    #[must_use]
    pub const fn new(secs: u64) -> Self {
        Self(secs)
    }

    #[must_use]
    pub const fn secs(&self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }

    /// Parses a timeout value in seconds.
    ///
    /// # Errors
    /// Returns `TimeoutError::TooSmall` if value is less than 1 second.
    /// Returns `TimeoutError::TooLarge` if value exceeds 3600 seconds.
    pub fn parse(value: u64) -> Result<Self, TimeoutError> {
        if value < MIN_TIMEOUT_SECS {
            Err(TimeoutError::TooSmall { min: MIN_TIMEOUT_SECS, value })
        } else if value > MAX_TIMEOUT_SECS {
            Err(TimeoutError::TooLarge { max: MAX_TIMEOUT_SECS, value })
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn min() -> Self {
        Self(MIN_TIMEOUT_SECS)
    }

    #[must_use]
    pub const fn max() -> Self {
        Self(MAX_TIMEOUT_SECS)
    }
}

impl Default for TimeoutSeconds {
    fn default() -> Self {
        Self(120)
    }
}

impl From<TimeoutSeconds> for Duration {
    fn from(timeout: TimeoutSeconds) -> Self {
        timeout.duration()
    }
}

impl From<TimeoutSeconds> for u64 {
    fn from(timeout: TimeoutSeconds) -> Self {
        timeout.secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_seconds_parse_valid() {
        assert!(TimeoutSeconds::parse(60).is_ok());
        assert!(TimeoutSeconds::parse(120).is_ok());
        assert!(TimeoutSeconds::parse(3600).is_ok());
    }

    #[test]
    fn test_timeout_seconds_parse_too_small() {
        assert!(matches!(TimeoutSeconds::parse(0), Err(TimeoutError::TooSmall { .. })));
    }

    #[test]
    fn test_timeout_seconds_parse_too_large() {
        assert!(matches!(TimeoutSeconds::parse(3601), Err(TimeoutError::TooLarge { .. })));
    }

    #[test]
    fn test_timeout_seconds_duration() {
        let timeout = TimeoutSeconds::new(60);
        assert_eq!(timeout.duration(), Duration::from_secs(60));
    }

    #[test]
    fn test_timeout_seconds_default() {
        let timeout = TimeoutSeconds::default();
        assert_eq!(timeout.secs(), 120);
    }

    #[test]
    fn test_timeout_seconds_into_duration() {
        let timeout = TimeoutSeconds::new(30);
        let duration: Duration = timeout.into();
        assert_eq!(duration, Duration::from_secs(30));
    }

    #[test]
    fn test_timeout_seconds_into_u64() {
        let timeout = TimeoutSeconds::new(45);
        let value: u64 = timeout.into();
        assert_eq!(value, 45);
    }
}
