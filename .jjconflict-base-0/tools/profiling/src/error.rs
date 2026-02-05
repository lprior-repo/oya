#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Error types for memory profiling harness

use thiserror::Error;

/// Result type alias for profiling operations
pub type Result<T> = std::result::Result<T, ProfilingError>;

/// Errors that can occur during memory profiling
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProfilingError {
    /// Heaptrack binary not found in PATH
    #[error("heaptrack binary not found - install heaptrack and ensure it's in PATH")]
    HeaptrackNotFound,

    /// Failed to spawn heaptrack process
    #[error("failed to spawn heaptrack process: {0}")]
    ProcessSpawnFailed(String),

    /// Process terminated unexpectedly
    #[error("profiling process terminated unexpectedly: {0}")]
    ProcessTerminated(String),

    /// Failed to read memory metrics
    #[error("failed to read memory metrics from /proc/{0}/status: {1}")]
    MetricsReadFailed(u32, String),

    /// Failed to parse memory value
    #[error("failed to parse memory value from /proc: {0}")]
    MetricsParseError(String),

    /// Failed to write metrics log
    #[error("failed to write metrics to log file: {0}")]
    LogWriteFailed(String),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Duration exceeded maximum allowed
    #[error("profiling duration {0}s exceeds maximum {1}s")]
    DurationTooLong(u64, u64),

    /// Sampling interval too short (would cause >10% overhead)
    #[error("sampling interval {0}s is too short (minimum {1}s for <10% overhead)")]
    SamplingIntervalTooShort(u64, u64),

    /// IO error wrapper
    #[error("IO error: {0}")]
    IoError(String),

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    SerializationError(String),
}

impl From<std::io::Error> for ProfilingError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ProfilingError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}
