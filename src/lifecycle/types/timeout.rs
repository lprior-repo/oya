use serde::{Deserialize, Deserializer, Serialize};
use std::time::Duration;
use thiserror::Error;

const MIN_TIMEOUT_SECS: u64 = 1;
const MAX_TIMEOUT_SECS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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
    /// Creates a validated timeout value in seconds.
    ///
    /// # Errors
    /// Returns `TimeoutError::TooSmall` if value is less than 1 second.
    /// Returns `TimeoutError::TooLarge` if value exceeds 3600 seconds.
    pub fn new(secs: u64) -> Result<Self, TimeoutError> {
        Self::parse(secs)
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

impl<'de> Deserialize<'de> for TimeoutSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        TimeoutSeconds::parse(secs).map_err(serde::de::Error::custom)
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
        let timeout = TimeoutSeconds::new(60).expect("timeout should be valid");
        assert_eq!(timeout.duration(), Duration::from_secs(60));
    }

    #[test]
    fn test_timeout_seconds_default() {
        let timeout = TimeoutSeconds::default();
        assert_eq!(timeout.secs(), 120);
    }

    #[test]
    fn test_timeout_seconds_into_duration() {
        let timeout = TimeoutSeconds::new(30).expect("timeout should be valid");
        let duration: Duration = timeout.into();
        assert_eq!(duration, Duration::from_secs(30));
    }

    #[test]
    fn test_timeout_seconds_into_u64() {
        let timeout = TimeoutSeconds::new(45).expect("timeout should be valid");
        let value: u64 = timeout.into();
        assert_eq!(value, 45);
    }

    #[test]
    fn test_timeout_seconds_new_rejects_out_of_range() {
        assert!(matches!(TimeoutSeconds::new(0), Err(TimeoutError::TooSmall { .. })));
        assert!(matches!(TimeoutSeconds::new(3601), Err(TimeoutError::TooLarge { .. })));
    }

    #[test]
    fn test_timeout_seconds_deserialize_rejects_out_of_range() {
        let too_small: Result<TimeoutSeconds, _> = serde_json::from_str("0");
        let too_large: Result<TimeoutSeconds, _> = serde_json::from_str("3601");
        assert!(too_small.is_err());
        assert!(too_large.is_err());
    }

    #[test]
    fn test_timeout_seconds_deserialize_accepts_valid() {
        let parsed: TimeoutSeconds =
            serde_json::from_str("120").expect("timeout should deserialize");
        assert_eq!(parsed.secs(), 120);
    }
}
