//! JSON output structures for AI-first CLI design
//!
//! This module provides consistent JSON output formats across all commands.

use serde::{Deserialize, Serialize};

/// Standard JSON success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSuccess<T> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T> JsonSuccess<T> {
    /// Create a new success response
    pub const fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Standard JSON error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub success: bool,
    pub error: ErrorDetail,
}

impl Default for JsonError {
    fn default() -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "UNKNOWN".to_string(),
                message: "An unknown error occurred".to_string(),
                exit_code: 4,
                details: None,
                suggestion: None,
            },
        }
    }
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code (`SCREAMING_SNAKE_CASE`)
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Semantic exit code (1-4)
    pub exit_code: i32,
    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggestion for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl JsonError {
    /// Create a new JSON error with just a code and message
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                exit_code: 4, // Default to unknown/external error
                details: None,
                suggestion: None,
            },
        }
    }

    /// Add details to the error
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }

    /// Add a suggestion to the error
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.error.suggestion = Some(suggestion.into());
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize error: {e}")))
    }
}

/// Error codes for machine-readable errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Session errors
    SessionNotFound,
    SessionAlreadyExists,
    SessionNameInvalid,

    // Workspace errors
    WorkspaceCreationFailed,
    WorkspaceNotFound,

    // JJ errors
    JjNotInstalled,
    JjCommandFailed,
    NotJjRepository,

    // Zellij errors
    ZellijNotRunning,
    ZellijCommandFailed,

    // Config errors
    ConfigNotFound,
    ConfigParseError,
    ConfigKeyNotFound,

    // Hook errors
    HookFailed,
    HookExecutionError,

    // State errors
    StateDbCorrupted,
    StateDbLocked,

    // Generic errors
    InvalidArgument,
    Unknown,
}

impl ErrorCode {
    /// Get the string representation of the error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyExists => "SESSION_ALREADY_EXISTS",
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::JjNotInstalled => "JJ_NOT_INSTALLED",
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::NotJjRepository => "NOT_JJ_REPOSITORY",
            Self::ZellijNotRunning => "ZELLIJ_NOT_RUNNING",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigKeyNotFound => "CONFIG_KEY_NOT_FOUND",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::StateDbCorrupted => "STATE_DB_CORRUPTED",
            Self::StateDbLocked => "STATE_DB_LOCKED",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}

/// Classify an error into a semantic exit code.
///
/// Exit codes follow this semantic mapping:
/// - 1: Validation errors (user input issues)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors
const fn classify_exit_code(error: &crate::Error) -> i32 {
    use crate::Error;
    match error {
        // Validation errors: exit code 1
        Error::InvalidConfig(_) | Error::ValidationError(_) | Error::ParseError(_) => 1,
        // Not found errors: exit code 2
        Error::NotFound(_) => 2,
        // System errors: exit code 3
        Error::IoError(_) | Error::DatabaseError(_) => 3,
        // External command errors: exit code 4
        Error::Command(_)
        | Error::JjCommandError { .. }
        | Error::HookFailed { .. }
        | Error::HookExecutionFailed { .. }
        | Error::Unknown(_) => 4,
    }
}

impl ErrorDetail {
    /// Construct an `ErrorDetail` from an Error.
    ///
    /// This is the standard way to convert errors to JSON-serializable format.
    #[must_use]
    pub fn from_error(error: &crate::Error) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            exit_code: classify_exit_code(error),
            details: error.context_map(),
            suggestion: error.suggestion(),
        }
    }
}

impl From<&crate::Error> for JsonError {
    fn from(err: &crate::Error) -> Self {
        use crate::Error;

        let (code, message, suggestion) = match err {
            Error::InvalidConfig(msg) => (
                ErrorCode::ConfigParseError,
                format!("Invalid configuration: {msg}"),
                Some("Check your configuration file for errors".to_string()),
            ),
            Error::IoError(msg) => (
                ErrorCode::Unknown,
                format!("IO error: {msg}"),
                None,
            ),
            Error::ParseError(msg) => (
                ErrorCode::ConfigParseError,
                format!("Parse error: {msg}"),
                None,
            ),
            Error::ValidationError(msg) => (
                ErrorCode::InvalidArgument,
                format!("Validation error: {msg}"),
                None,
            ),
            Error::NotFound(msg) => (
                ErrorCode::SessionNotFound,
                format!("Not found: {msg}"),
                Some("Use 'jjz list' to see available sessions".to_string()),
            ),
            Error::DatabaseError(msg) => (
                ErrorCode::StateDbCorrupted,
                format!("Database error: {msg}"),
                Some("Try running 'jjz doctor --fix' to repair the database".to_string()),
            ),
            Error::Command(msg) => (
                ErrorCode::Unknown,
                format!("Command error: {msg}"),
                None,
            ),
            Error::HookFailed {
                hook_type,
                command,
                exit_code,
                stdout: _,
                stderr,
            } => (
                ErrorCode::HookFailed,
                format!(
                    "Hook '{hook_type}' failed: {command}\nExit code: {exit_code:?}\nStderr: {stderr}"
                ),
                Some("Check your hook configuration and ensure the command is correct".to_string()),
            ),
            Error::HookExecutionFailed { command, source } => (
                ErrorCode::HookExecutionError,
                format!("Failed to execute hook '{command}': {source}"),
                Some("Ensure the hook command exists and is executable".to_string()),
            ),
            Error::JjCommandError {
                operation,
                source,
                is_not_found,
            } => {
                if *is_not_found {
                    (
                        ErrorCode::JjNotInstalled,
                        format!("Failed to {operation}: JJ is not installed or not in PATH"),
                        Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
                    )
                } else {
                    (
                        ErrorCode::JjCommandFailed,
                        format!("Failed to {operation}: {source}"),
                        None,
                    )
                }
            }
            Error::Unknown(msg) => (
                ErrorCode::Unknown,
                format!("Unknown error: {msg}"),
                None,
            ),
        };

        let mut json_error = Self::new(code, message);
        if let Some(sugg) = suggestion {
            json_error = json_error.with_suggestion(sugg);
        }
        json_error
    }
}

impl From<crate::Error> for JsonError {
    fn from(err: crate::Error) -> Self {
        Self::from(&err)
    }
}

// Note: from_anyhow method removed as zjj-core doesn't depend on anyhow
// If needed, implement this in the zjj crate instead

/// Trait for types that can be serialized to JSON
pub trait JsonSerializable: Serialize {
    /// Convert to pretty-printed JSON string
    fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize to JSON: {e}")))
    }
}

// Implement for all Serialize types
impl<T: Serialize> JsonSerializable for T {}

/// Helper to create error details with available sessions
pub fn error_with_available_sessions(
    code: ErrorCode,
    message: impl Into<String>,
    session_name: impl Into<String>,
    available: &[String],
) -> JsonError {
    let details = serde_json::json!({
        "session_name": session_name.into(),
        "available_sessions": available,
    });

    JsonError::new(code, message)
        .with_details(details)
        .with_suggestion("Use 'jjz list' to see available sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error_basic() {
        let err = JsonError::new("TEST_ERROR", "Test error message");
        assert_eq!(err.error.code, "TEST_ERROR");
        assert_eq!(err.error.message, "Test error message");
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_with_details() {
        let details = serde_json::json!({"key": "value"});
        let err = JsonError::new("TEST_ERROR", "Test").with_details(details.clone());

        assert!(err.error.details.is_some());
        assert_eq!(err.error.details, Some(details));
    }

    #[test]
    fn test_json_error_with_suggestion() {
        let err = JsonError::new("TEST_ERROR", "Test").with_suggestion("Try this instead");

        assert_eq!(err.error.suggestion, Some("Try this instead".to_string()));
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(ErrorCode::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
        assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
    }

    #[test]
    fn test_error_code_to_string() {
        let code: String = ErrorCode::SessionNotFound.into();
        assert_eq!(code, "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_json_error_serialization() -> crate::Result<()> {
        let err = JsonError::new("TEST_ERROR", "Test message");
        let json = err.to_json()?;

        assert!(json.contains("\"code\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));

        Ok(())
    }

    #[test]
    fn test_error_with_available_sessions() {
        let available = vec!["session1".to_string(), "session2".to_string()];
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "Session 'foo' not found",
            "foo",
            &available,
        );

        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert!(err.error.details.is_some());
        assert!(err.error.suggestion.is_some());
    }

    #[test]
    fn test_json_serializable_trait() -> crate::Result<()> {
        #[derive(Serialize)]
        struct TestStruct {
            field: String,
        }

        let test = TestStruct {
            field: "value".to_string(),
        };

        let json = test.to_json()?;
        assert!(json.contains("\"field\""));
        assert!(json.contains("\"value\""));

        Ok(())
    }

    #[test]
    fn test_json_success_wrapper() -> crate::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
            count: usize,
        }

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        let success = JsonSuccess {
            success: true,
            data,
        };
        let json = success.to_json()?;

        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test\""));
        assert!(json.contains("\"count\""));
        assert!(json.contains("42"));

        Ok(())
    }

    #[test]
    fn test_error_detail_skip_none() -> crate::Result<()> {
        let err = JsonError::new("TEST", "message");
        let json = err.to_json()?;

        // Should not contain "details" or "suggestion" fields when they're None
        assert!(!json.contains("\"details\""));
        assert!(!json.contains("\"suggestion\""));

        Ok(())
    }

    // Tests for ErrorDetail::from_error() constructor (zjj-lgkf Phase 4 - RED)
    #[test]
    fn test_error_detail_from_validation_error() {
        let err = crate::Error::ValidationError("invalid session name".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "VALIDATION_ERROR");
        assert!(detail.message.contains("Validation error"));
        assert_eq!(detail.exit_code, 1);
    }

    #[test]
    fn test_error_detail_from_io_error() {
        let err = crate::Error::IoError("file not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "IO_ERROR");
        assert!(detail.message.contains("IO error"));
        assert_eq!(detail.exit_code, 3);
    }

    #[test]
    fn test_error_detail_from_not_found_error() {
        let err = crate::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "NOT_FOUND");
        assert!(detail.message.contains("Not found"));
        assert_eq!(detail.exit_code, 2);
    }

    #[test]
    fn test_error_detail_preserves_context() {
        let err = crate::Error::ValidationError("invalid input".into());
        let detail = ErrorDetail::from_error(&err);

        // Should have context map populated
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_includes_suggestion() {
        let err = crate::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        // Should have suggestion populated
        assert!(detail.suggestion.is_some());
        if let Some(sugg) = detail.suggestion {
            assert!(sugg.contains("list"));
        }
    }
}
