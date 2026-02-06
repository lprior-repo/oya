//! Core error types for OYA operations using Railway-Oriented Programming.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::path::PathBuf;

use thiserror::Error;

/// Core error type for OYA operations.
#[derive(Debug, Error)]
pub enum Error {
    // I/O errors
    #[error("failed to read file '{path}': {reason}")]
    FileReadFailed { path: PathBuf, reason: String },

    #[error("failed to write file '{path}': {reason}")]
    FileWriteFailed { path: PathBuf, reason: String },

    #[error("failed to create directory '{path}': {reason}")]
    DirectoryCreationFailed { path: PathBuf, reason: String },

    #[error("directory does not exist: {path}")]
    DirectoryNotFound { path: PathBuf },

    // Parsing errors
    #[error("JSON parse error: {reason}")]
    JsonParseFailed { reason: String },

    #[error("YAML parse error: {reason}")]
    YamlParseFailed { reason: String },

    #[error("TOML parse error: {reason}")]
    TomlParseFailed { reason: String },

    // Generic errors
    #[error("invalid record: {reason}")]
    InvalidRecord { reason: String },

    #[error("unknown error: {0}")]
    Unknown(String),

    // Generic I/O error wrapper
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create a file read error.
    pub fn file_read_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::FileReadFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a file write error.
    pub fn file_write_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::FileWriteFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a directory creation error.
    pub fn directory_creation_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::DirectoryCreationFailed {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a JSON parse error.
    pub fn json_parse_failed(reason: impl Into<String>) -> Self {
        Self::JsonParseFailed {
            reason: reason.into(),
        }
    }

    /// Create an invalid record error.
    pub fn invalid_record(reason: impl Into<String>) -> Self {
        Self::InvalidRecord {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_read_failed_factory() {
        let _path = PathBuf::from("/test/path");
        let error = Error::file_read_failed(_path.clone(), "permission denied");
        assert!(matches!(error, Error::FileReadFailed { .. }));
    }

    #[test]
    fn test_file_write_failed_factory() {
        let _path = PathBuf::from("/test/path");
        let error = Error::file_write_failed(_path.clone(), "disk full");
        assert!(matches!(error, Error::FileWriteFailed { .. }));
    }

    #[test]
    fn test_directory_creation_failed_factory() {
        let _path = PathBuf::from("/test/dir");
        let error = Error::directory_creation_failed(_path.clone(), "readonly");
        assert!(matches!(error, Error::DirectoryCreationFailed { .. }));
    }

    #[test]
    fn test_json_parse_failed_factory() {
        let error = Error::json_parse_failed("bad comma");
        assert!(matches!(error, Error::JsonParseFailed { .. }));
    }

    #[test]
    fn test_invalid_record_factory() {
        let error = Error::invalid_record("missing field");
        assert!(matches!(error, Error::InvalidRecord { .. }));
    }

    #[test]
    fn test_unknown_factory() {
        let error = Error::Unknown(String::from("something went wrong"));
        assert!(matches!(error, Error::Unknown(_)));
    }

    #[test]
    fn test_io_error_from_std() {
        let std_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = Error::Io(std_error);
        assert!(matches!(error, Error::Io(_)));
    }

    #[test]
    fn test_directory_not_found() {
        let _path = PathBuf::from("/nonexistent");
        let error = Error::DirectoryNotFound {
            path: PathBuf::from("/nonexistent"),
        };
        assert!(matches!(error, Error::DirectoryNotFound { .. }));
    }

    #[test]
    fn test_error_display() {
        let errors = vec![
            Error::file_read_failed(PathBuf::from("/path"), "reason"),
            Error::json_parse_failed("bad json"),
            Error::Unknown(String::from("generic")),
        ];

        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_error_debug() {
        let error = Error::DirectoryNotFound {
            path: PathBuf::from("/test"),
        };
        let debug = format!("{:?}", error);
        assert!(debug.contains("DirectoryNotFound"));
    }

    #[test]
    fn test_error_into_string() {
        let error = Error::invalid_record("bad record");
        let error_string = error.to_string();
        assert!(error_string.contains("bad record"));
    }
}
