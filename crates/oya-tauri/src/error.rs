//! Error types for oya-tauri
//!
//! All errors are serializable for transmission to the frontend.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Application-level errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum AppError {
    /// File system error
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Bead not found
    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    /// Invalid bead data
    #[error("Invalid bead data: {0}")]
    InvalidBead(String),

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Stream error
    #[error("Stream error: {0}")]
    Stream(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::FileSystem(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

/// Result type alias for app operations
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AppError::BeadNotFound("bead-123".to_string());
        assert_eq!(err.to_string(), "Bead not found: bead-123");
    }

    #[test]
    fn test_error_serialization() -> Result<(), serde_json::Error> {
        let err = AppError::FileSystem("permission denied".to_string());
        let json = serde_json::to_string(&err)?;
        assert!(json.contains("FileSystem"));
        Ok(())
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::FileSystem(_)));
    }
}
