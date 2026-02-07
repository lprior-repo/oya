//! Core error types for OYA operations using Railway-Oriented Programming.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::path::PathBuf;

use thiserror::Error;

/// Core error type for OYA operations.
#[derive(Debug, Error)]
pub enum Error {
    // I/O errors
    #[error("failed to read file '{path}'")]
    FileReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write file '{path}'")]
    FileWriteFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create directory '{path}'")]
    DirectoryCreationFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("directory does not exist: {path}")]
    DirectoryNotFound { path: PathBuf },

    // Parsing errors
    #[error("JSON parse error")]
    JsonParseFailed {
        #[source]
        source: serde_json::Error,
    },

    #[error("YAML parse error")]
    YamlParseFailed {
        #[source]
        source: serde_yaml::Error,
    },

    #[error("TOML parse error")]
    TomlParseFailed {
        #[source]
        source: toml::de::Error,
    },

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
    pub fn file_read_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::FileReadFailed {
            path: path.into(),
            source,
        }
    }

    /// Create a file write error.
    pub fn file_write_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::FileWriteFailed {
            path: path.into(),
            source,
        }
    }

    /// Create a directory creation error.
    pub fn directory_creation_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::DirectoryCreationFailed {
            path: path.into(),
            source,
        }
    }

    /// Create a JSON parse error.
    pub fn json_parse_failed(source: serde_json::Error) -> Self {
        Self::JsonParseFailed { source }
    }

    /// Create a YAML parse error.
    pub fn yaml_parse_failed(source: serde_yaml::Error) -> Self {
        Self::YamlParseFailed { source }
    }

    /// Create a TOML parse error.
    pub fn toml_parse_failed(source: toml::de::Error) -> Self {
        Self::TomlParseFailed { source }
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
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let error = Error::file_read_failed(_path.clone(), io_error);
        assert!(matches!(error, Error::FileReadFailed { .. }));
    }

    #[test]
    fn test_file_write_failed_factory() {
        let _path = PathBuf::from("/test/path");
        let io_error = std::io::Error::new(std::io::ErrorKind::StorageFull, "disk full");
        let error = Error::file_write_failed(_path.clone(), io_error);
        assert!(matches!(error, Error::FileWriteFailed { .. }));
    }

    #[test]
    fn test_directory_creation_failed_factory() {
        let _path = PathBuf::from("/test/dir");
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "readonly");
        let error = Error::directory_creation_failed(_path.clone(), io_error);
        assert!(matches!(error, Error::DirectoryCreationFailed { .. }));
    }

    #[test]
    fn test_json_parse_failed_factory() {
        // Create a JSON error by parsing invalid JSON
        let json_error = match serde_json::from_str::<serde_json::Value>("{invalid}") {
            Err(e) => e,
            Ok(_) => panic!("Expected JSON parse error"),
        };
        let error = Error::json_parse_failed(json_error);
        assert!(matches!(error, Error::JsonParseFailed { .. }));
    }

    #[test]
    fn test_yaml_parse_failed_factory() {
        // Create a YAML error by parsing invalid YAML
        let yaml_error = match serde_yaml::from_str::<serde_yaml::Value>("*: invalid") {
            Err(e) => e,
            Ok(_) => panic!("Expected YAML parse error"),
        };
        let error = Error::yaml_parse_failed(yaml_error);
        assert!(matches!(error, Error::YamlParseFailed { .. }));
    }

    #[test]
    fn test_toml_parse_failed_factory() {
        // Create a TOML error by parsing invalid TOML
        let toml_error = match toml::from_str::<toml::Value>("invalid = [") {
            Err(e) => e,
            Ok(_) => panic!("Expected TOML parse error"),
        };
        let error = Error::toml_parse_failed(toml_error);
        assert!(matches!(error, Error::TomlParseFailed { .. }));
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
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let json_error = match serde_json::from_str::<serde_json::Value>("{invalid}") {
            Err(e) => e,
            Ok(_) => panic!("Expected JSON parse error"),
        };
        let errors = vec![
            Error::file_read_failed(PathBuf::from("/path"), io_error),
            Error::json_parse_failed(json_error),
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
