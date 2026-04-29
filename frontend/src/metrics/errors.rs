use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("Failed to acquire lock on metrics data")]
    LockAcquisition,
    #[error("Failed to read metrics file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to write metrics file: {0}")]
    WriteError(#[source] std::io::Error),
    #[error("Failed to parse metrics data: {0}")]
    ParseError(#[source] serde_json::Error),
    #[error("Invalid session id: {0}")]
    InvalidSessionId(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Invalid feedback level: {0}. Must be 1-5")]
    InvalidFeedbackLevel(u8),
    #[error("Unsupported export format: {0}")]
    UnsupportedExportFormat(String),
}
