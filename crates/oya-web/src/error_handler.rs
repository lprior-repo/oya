//! HTTP error handling middleware.
//!
//! Provides comprehensive error categorization, structured error responses,
//! and recovery strategies for HTTP failures. All error handling follows
//! Railway-Oriented Programming with zero unwraps/panics.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// HTTP error categories for classification and handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-level errors (connection refused, timeout, DNS failure)
    Network,
    /// Client errors (4xx) - permanent, should not retry
    Client,
    /// Server errors (5xx) - transient, may retry
    Server,
    /// Timeout errors - transient, may retry
    Timeout,
    /// Request validation errors
    Validation,
    /// Authentication/authorization errors
    Auth,
    /// Unknown or uncategorized errors
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network => write!(f, "network"),
            Self::Client => write!(f, "client"),
            Self::Server => write!(f, "server"),
            Self::Timeout => write!(f, "timeout"),
            Self::Validation => write!(f, "validation"),
            Self::Auth => write!(f, "auth"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Recovery strategy for error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Safe to retry with exponential backoff
    RetryWithBackoff,
    /// Safe to retry immediately
    RetryImmediately,
    /// Do not retry - permanent failure
    NoRetry,
    /// Degraded service - return cached/partial data
    GracefulDegradation,
}

/// Structured error response for HTTP clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// HTTP status code
    pub status: u16,
    /// Error category for client-side handling
    pub category: String,
    /// Human-readable error message
    pub message: String,
    /// Machine-readable error code
    pub code: String,
    /// Whether the error is retryable
    pub retryable: bool,
    /// ISO 8601 timestamp when error occurred
    pub timestamp: String,
    /// Request ID for tracing (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ErrorResponse {
    /// Create a new error response.
    pub fn new(
        status: u16,
        category: ErrorCategory,
        message: String,
        code: String,
        retryable: bool,
    ) -> Self {
        let timestamp = Self::current_timestamp();
        Self {
            status,
            category: category.to_string(),
            message,
            code,
            retryable,
            timestamp,
            request_id: None,
        }
    }

    /// Add request ID for tracing.
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Get current timestamp as ISO 8601 string.
    fn current_timestamp() -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_or_else(
                |_| "unknown".to_string(),
                |d| {
                    format!("{}", d.as_secs()) // Simplified - in production use chrono or time crate
                },
            )
    }

    /// Categorize HTTP status code into error category.
    pub fn categorize_status_code(status: StatusCode) -> ErrorCategory {
        match status {
            s if s.is_client_error() => match s {
                StatusCode::BAD_REQUEST
                | StatusCode::UNPROCESSABLE_ENTITY
                | StatusCode::TOO_MANY_REQUESTS => ErrorCategory::Validation,
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ErrorCategory::Auth,
                _ => ErrorCategory::Client,
            },
            s if s.is_server_error() => ErrorCategory::Server,
            _ => ErrorCategory::Unknown,
        }
    }

    /// Determine recovery strategy for error category.
    pub fn recovery_strategy(category: ErrorCategory) -> RecoveryStrategy {
        match category {
            ErrorCategory::Network | ErrorCategory::Timeout | ErrorCategory::Server => {
                RecoveryStrategy::RetryWithBackoff
            }
            ErrorCategory::Validation | ErrorCategory::Auth | ErrorCategory::Client => {
                RecoveryStrategy::NoRetry
            }
            ErrorCategory::Unknown => RecoveryStrategy::NoRetry,
        }
    }

    /// Check if error is retryable based on category.
    pub fn is_retryable(category: ErrorCategory) -> bool {
        matches!(
            category,
            ErrorCategory::Network | ErrorCategory::Timeout | ErrorCategory::Server
        )
    }
}

/// Comprehensive HTTP error for middleware.
#[derive(Debug)]
pub enum HttpError {
    /// Network error (connection refused, DNS failure, etc.)
    Network {
        message: String,
        source: Option<String>,
    },
    /// Timeout error
    Timeout {
        duration_secs: u64,
        operation: String,
    },
    /// Client error (4xx)
    Client { status: StatusCode, message: String },
    /// Server error (5xx)
    Server { status: StatusCode, message: String },
    /// Validation error
    Validation { field: String, message: String },
    /// Authentication error
    Auth { message: String },
    /// Unknown error
    Unknown { message: String },
}

impl HttpError {
    /// Get error category.
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Network { .. } => ErrorCategory::Network,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::Client { .. } => ErrorCategory::Client,
            Self::Server { .. } => ErrorCategory::Server,
            Self::Validation { .. } => ErrorCategory::Validation,
            Self::Auth { .. } => ErrorCategory::Auth,
            Self::Unknown { .. } => ErrorCategory::Unknown,
        }
    }

    /// Get HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Network { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            Self::Client { status, .. } => *status,
            Self::Server { status, .. } => *status,
            Self::Validation { .. } => StatusCode::BAD_REQUEST,
            Self::Auth { .. } => StatusCode::UNAUTHORIZED,
            Self::Unknown { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get error message.
    pub fn message(&self) -> String {
        match self {
            Self::Network { message, .. } => message.clone(),
            Self::Timeout {
                duration_secs,
                operation,
            } => format!("Operation '{operation}' timed out after {duration_secs}s"),
            Self::Client { message, .. } => message.clone(),
            Self::Server { message, .. } => message.clone(),
            Self::Validation { message, .. } => message.clone(),
            Self::Auth { message } => message.clone(),
            Self::Unknown { message } => message.clone(),
        }
    }

    /// Get error code for machine readability.
    pub fn error_code(&self) -> String {
        match self {
            Self::Network { .. } => "NETWORK_ERROR".to_string(),
            Self::Timeout { .. } => "TIMEOUT_ERROR".to_string(),
            Self::Client { .. } => "CLIENT_ERROR".to_string(),
            Self::Server { .. } => "SERVER_ERROR".to_string(),
            Self::Validation { .. } => "VALIDATION_ERROR".to_string(),
            Self::Auth { .. } => "AUTH_ERROR".to_string(),
            Self::Unknown { .. } => "UNKNOWN_ERROR".to_string(),
        }
    }

    /// Convert to structured error response.
    pub fn to_response(&self) -> ErrorResponse {
        let category = self.category();
        let status = self.status_code();
        let message = self.message();
        let code = self.error_code();
        let retryable = ErrorResponse::is_retryable(category);

        ErrorResponse::new(status.as_u16(), category, message, code, retryable)
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.category(), self.message())
    }
}

impl std::error::Error for HttpError {}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let response = self.to_response();
        let status = self.status_code();
        (status, Json(response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_display() {
        assert_eq!(ErrorCategory::Network.to_string(), "network");
        assert_eq!(ErrorCategory::Client.to_string(), "client");
        assert_eq!(ErrorCategory::Server.to_string(), "server");
        assert_eq!(ErrorCategory::Timeout.to_string(), "timeout");
    }

    #[test]
    fn test_categorize_status_code() {
        // Client errors
        assert_eq!(
            ErrorResponse::categorize_status_code(StatusCode::BAD_REQUEST),
            ErrorCategory::Validation
        );
        assert_eq!(
            ErrorResponse::categorize_status_code(StatusCode::UNAUTHORIZED),
            ErrorCategory::Auth
        );
        assert_eq!(
            ErrorResponse::categorize_status_code(StatusCode::NOT_FOUND),
            ErrorCategory::Client
        );

        // Server errors
        assert_eq!(
            ErrorResponse::categorize_status_code(StatusCode::INTERNAL_SERVER_ERROR),
            ErrorCategory::Server
        );
        assert_eq!(
            ErrorResponse::categorize_status_code(StatusCode::BAD_GATEWAY),
            ErrorCategory::Server
        );
    }

    #[test]
    fn test_is_retryable() {
        assert!(ErrorResponse::is_retryable(ErrorCategory::Network));
        assert!(ErrorResponse::is_retryable(ErrorCategory::Timeout));
        assert!(ErrorResponse::is_retryable(ErrorCategory::Server));

        assert!(!ErrorResponse::is_retryable(ErrorCategory::Client));
        assert!(!ErrorResponse::is_retryable(ErrorCategory::Validation));
        assert!(!ErrorResponse::is_retryable(ErrorCategory::Auth));
    }

    #[test]
    fn test_recovery_strategy() {
        assert_eq!(
            ErrorResponse::recovery_strategy(ErrorCategory::Network),
            RecoveryStrategy::RetryWithBackoff
        );
        assert_eq!(
            ErrorResponse::recovery_strategy(ErrorCategory::Timeout),
            RecoveryStrategy::RetryWithBackoff
        );
        assert_eq!(
            ErrorResponse::recovery_strategy(ErrorCategory::Server),
            RecoveryStrategy::RetryWithBackoff
        );
        assert_eq!(
            ErrorResponse::recovery_strategy(ErrorCategory::Client),
            RecoveryStrategy::NoRetry
        );
        assert_eq!(
            ErrorResponse::recovery_strategy(ErrorCategory::Validation),
            RecoveryStrategy::NoRetry
        );
    }

    #[test]
    fn test_error_response_new() {
        let response = ErrorResponse::new(
            500,
            ErrorCategory::Server,
            "Internal error".to_string(),
            "INTERNAL_ERROR".to_string(),
            true,
        );

        assert_eq!(response.status, 500);
        assert_eq!(response.category, "server");
        assert_eq!(response.message, "Internal error");
        assert_eq!(response.code, "INTERNAL_ERROR");
        assert!(response.retryable);
        assert!(!response.timestamp.is_empty());
    }

    #[test]
    fn test_error_response_with_request_id() {
        let response = ErrorResponse::new(
            400,
            ErrorCategory::Validation,
            "Invalid input".to_string(),
            "VALIDATION_ERROR".to_string(),
            false,
        )
        .with_request_id("req-123".to_string());

        assert_eq!(response.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_http_error_network() {
        let error = HttpError::Network {
            message: "Connection refused".to_string(),
            source: Some("backend-service".to_string()),
        };

        assert_eq!(error.category(), ErrorCategory::Network);
        assert_eq!(error.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.message(), "Connection refused");
        assert_eq!(error.error_code(), "NETWORK_ERROR");
    }

    #[test]
    fn test_http_error_timeout() {
        let error = HttpError::Timeout {
            duration_secs: 30,
            operation: "database query".to_string(),
        };

        assert_eq!(error.category(), ErrorCategory::Timeout);
        assert_eq!(error.status_code(), StatusCode::GATEWAY_TIMEOUT);
        assert!(error.message().contains("timed out after 30s"));
        assert_eq!(error.error_code(), "TIMEOUT_ERROR");
    }

    #[test]
    fn test_http_error_client() {
        let error = HttpError::Client {
            status: StatusCode::NOT_FOUND,
            message: "Resource not found".to_string(),
        };

        assert_eq!(error.category(), ErrorCategory::Client);
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(error.message(), "Resource not found");
        assert_eq!(error.error_code(), "CLIENT_ERROR");
    }

    #[test]
    fn test_http_error_server() {
        let error = HttpError::Server {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Database connection failed".to_string(),
        };

        assert_eq!(error.category(), ErrorCategory::Server);
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.message(), "Database connection failed");
        assert_eq!(error.error_code(), "SERVER_ERROR");
    }

    #[test]
    fn test_http_error_to_response() {
        let error = HttpError::Timeout {
            duration_secs: 10,
            operation: "API call".to_string(),
        };

        let response = error.to_response();
        assert_eq!(response.status, 504); // GATEWAY_TIMEOUT
        assert_eq!(response.category, "timeout");
        assert!(response.retryable);
    }

    #[test]
    fn test_http_error_display() {
        let error = HttpError::Validation {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("validation"));
        assert!(display.contains("Invalid email format"));
    }
}
