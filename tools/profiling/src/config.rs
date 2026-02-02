#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Configuration for memory profiling harness

use crate::error::{ProfilingError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Maximum allowed profiling duration (safety limit)
const MAX_DURATION_SECS: u64 = 4 * 3600; // 4 hours

/// Minimum sampling interval to ensure <10% overhead
const MIN_SAMPLING_INTERVAL_SECS: u64 = 5;

/// Configuration for memory profiling runs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Duration of the profiling run
    duration: Duration,

    /// Interval between RSS measurements
    sampling_interval: Duration,

    /// Path to output log file for RSS data
    output_path: PathBuf,

    /// Command to profile
    command: String,

    /// Arguments for the command
    args: Vec<String>,

    /// Working directory for the command
    working_dir: Option<PathBuf>,
}

impl ProfilingConfig {
    /// Create a new profiling configuration with validation
    ///
    /// # Arguments
    ///
    /// * `duration` - How long to run the profiling session
    /// * `sampling_interval` - How often to sample RSS (minimum 5s for <10% overhead)
    /// * `output_path` - Where to write the metrics log (JSON lines)
    /// * `command` - The command to profile
    /// * `args` - Arguments for the command
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Duration exceeds maximum allowed (4 hours)
    /// - Sampling interval is too short (<5s, would cause >10% overhead)
    /// - Output path is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_profiling::ProfilingConfig;
    /// # use std::time::Duration;
    /// # use std::path::PathBuf;
    /// let config = ProfilingConfig::new(
    ///     Duration::from_secs(3600),  // 1 hour
    ///     Duration::from_secs(10),     // Sample every 10s
    ///     PathBuf::from("metrics.jsonl"),
    ///     "my-app".to_string(),
    ///     vec!["--load-test".to_string()],
    /// );
    /// assert!(config.is_ok());
    /// ```
    pub fn new(
        duration: Duration,
        sampling_interval: Duration,
        output_path: PathBuf,
        command: String,
        args: Vec<String>,
    ) -> Result<Self> {
        Self::validate_duration(&duration)?;
        Self::validate_sampling_interval(&sampling_interval)?;
        Self::validate_output_path(&output_path)?;

        Ok(Self {
            duration,
            sampling_interval,
            output_path,
            command,
            args,
            working_dir: None,
        })
    }

    /// Create a default 1-hour profiling configuration
    ///
    /// Uses:
    /// - Duration: 3600s (1 hour)
    /// - Sampling interval: 10s
    /// - Output: "./memory-profile.jsonl"
    ///
    /// # Errors
    ///
    /// Returns error if configuration validation fails
    pub fn one_hour_default(command: String, args: Vec<String>) -> Result<Self> {
        Self::new(
            Duration::from_secs(3600),
            Duration::from_secs(10),
            PathBuf::from("memory-profile.jsonl"),
            command,
            args,
        )
    }

    /// Set the working directory for the profiled command
    #[must_use]
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Get the profiling duration
    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    /// Get the sampling interval
    #[must_use]
    pub const fn sampling_interval(&self) -> Duration {
        self.sampling_interval
    }

    /// Get the output path
    #[must_use]
    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }

    /// Get the command to profile
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Get the command arguments
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Get the working directory (if set)
    #[must_use]
    pub fn working_dir(&self) -> Option<&PathBuf> {
        self.working_dir.as_ref()
    }

    /// Validate profiling duration
    fn validate_duration(duration: &Duration) -> Result<()> {
        let secs = duration.as_secs();
        if secs > MAX_DURATION_SECS {
            Err(ProfilingError::DurationTooLong(secs, MAX_DURATION_SECS))
        } else if secs == 0 {
            Err(ProfilingError::InvalidConfig(
                "duration must be greater than 0".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate sampling interval (must be >=5s to ensure <10% overhead)
    fn validate_sampling_interval(interval: &Duration) -> Result<()> {
        let secs = interval.as_secs();
        if secs < MIN_SAMPLING_INTERVAL_SECS {
            Err(ProfilingError::SamplingIntervalTooShort(
                secs,
                MIN_SAMPLING_INTERVAL_SECS,
            ))
        } else {
            Ok(())
        }
    }

    /// Validate output path
    fn validate_output_path(path: &PathBuf) -> Result<()> {
        if path.as_os_str().is_empty() {
            Err(ProfilingError::InvalidConfig(
                "output path cannot be empty".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let config = ProfilingConfig::new(
            Duration::from_secs(3600),
            Duration::from_secs(10),
            PathBuf::from("test.jsonl"),
            "test-cmd".to_string(),
            vec!["arg1".to_string()],
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_duration_too_long() {
        let config = ProfilingConfig::new(
            Duration::from_secs(5 * 3600), // 5 hours > max
            Duration::from_secs(10),
            PathBuf::from("test.jsonl"),
            "test-cmd".to_string(),
            vec![],
        );
        assert!(matches!(config, Err(ProfilingError::DurationTooLong(_, _))));
    }

    #[test]
    fn test_sampling_interval_too_short() {
        let config = ProfilingConfig::new(
            Duration::from_secs(3600),
            Duration::from_secs(2), // <5s
            PathBuf::from("test.jsonl"),
            "test-cmd".to_string(),
            vec![],
        );
        assert!(matches!(
            config,
            Err(ProfilingError::SamplingIntervalTooShort(_, _))
        ));
    }

    #[test]
    fn test_one_hour_default() {
        let config =
            ProfilingConfig::one_hour_default("test-cmd".to_string(), vec!["arg".to_string()]);
        assert!(config.is_ok());
        let config = config.ok().filter(|c| c.duration().as_secs() == 3600);
        assert!(config.is_some());
    }

    #[test]
    fn test_with_working_dir() {
        let config = ProfilingConfig::one_hour_default("test".to_string(), vec![])
            .map(|c| c.with_working_dir(PathBuf::from("/tmp")));

        assert!(config.is_ok());
        assert!(config.as_ref().ok().and_then(|c| c.working_dir()).is_some());
    }
}
