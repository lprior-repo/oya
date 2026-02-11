//! HTTP error handling with categorized errors for retry logic.
//!
//! Provides structured error types that can be categorized for retry decisions.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::expect_used)]

use axum::{http::StatusCode, Json, response::{IntoResponse, Response}};
use serde::{Deserialize, Serialize};

/// Error categories for retry logic.
///
/// Determines whether an error should be retried based on its category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Network errors (connection refused, DNS failure, etc.)
    Network,
    /// Timeout errors
    Timeout,
    /// Server errors (5xx responses)
    Server,
    /// Client errors (4xx responses - never retry)
    Client,
    /// Validation errors (bad input - never retry)
    Validation,
    /// Authentication/authorization errors (never retry)
    Auth,
    /// Unknown error type
    Unknown,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network => write!(f, "Network"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Server => write!(f, "Server"),
            Self::Client => write!(f, "Client"),
            Self::Validation => write!(f, "Validation"),
            Self::Auth => write!(f, "Auth"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// HTTP error with categorization for retry logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpError {
    /// Network error (connection refused, DNS failure, etc.)
    Network {
        /// Human-readable error message
        message: String,
        /// Underlying error source
        source: Option<String>,
    },

    /// Timeout error
    Timeout {
        /// Timeout duration in seconds
        duration_secs: u64,
        /// Operation that timed out
        operation: String,
    },

    /// Server error (5xx response)
    Server {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Client error (4xx response - should not retry)
    Client {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Validation error (bad input - should not retry)
    Validation {
        /// Field that failed validation
        field: String,
        /// Validation error message
        message: String,
    },

    /// Authentication/authorization error (should not retry)
    Auth {
        /// Error message
        message: String,
    },

    /// Unknown error type
    Unknown {
        /// Error message
        message: String,
    },
}

impl HttpError {
    /// Get the error category for retry logic.
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::Network { .. } => ErrorCategory::Network,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::Server { .. } => ErrorCategory::Server,
            Self::Client { .. } => ErrorCategory::Client,
            Self::Validation { .. } => ErrorCategory::Validation,
            Self::Auth { .. } => ErrorCategory::Auth,
            Self::Unknown { .. } => ErrorCategory::Unknown,
        }
    }

    /// Get the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Network { message, .. } => message,
            Self::Timeout { operation, .. } => operation,
            Self::Server { message, .. } => message,
            Self::Client { message, .. } => message,
            Self::Validation { message, .. } => message,
            Self::Auth { message, .. } => message,
            Self::Unknown { message, .. } => message,
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.category(), self.message())
    }
}

impl std::error::Error for HttpError {}

/// Standard error response format for HTTP APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error category for client-side handling
    pub category: String,
    /// Human-readable error message
    pub message: String,
    /// Optional field name for validation errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// HTTP status code
    pub status: u16,
}

impl ErrorResponse {
    /// Create a new error response.
    #[must_use]
    pub fn new(category: ErrorCategory, message: String, status: StatusCode) -> Self {
        Self {
            category: format!("{category}"),
            message,
            field: None,
            status: status.as_u16(),
        }
    }

    /// Add field name for validation errors.
    #[must_use]
    pub fn with_field(mut self, field: String) -> Self {
        self.field = Some(field);
        self
    }

    /// Convert to Axum response.
    pub fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

impl From<HttpError> for ErrorResponse {
    fn from(error: HttpError) -> Self {
        let (status, message, field) = match &error {
            HttpError::Network { message, .. } => {
                (StatusCode::SERVICE_UNAVAILABLE, message.clone(), None)
            }
            HttpError::Timeout { operation, .. } => {
                (StatusCode::REQUEST_TIMEOUT, format!("Operation timed out: {operation}"), None)
            }
            HttpError::Server { status, message, .. } => {
                (StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), message.clone(), None)
            }
            HttpError::Client { status, message, .. } => {
                (StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_REQUEST), message.clone(), None)
            }
            HttpError::Validation { field, message, .. } => {
                (StatusCode::BAD_REQUEST, message.clone(), Some(field.clone()))
            }
            HttpError::Auth { message, .. } => {
                (StatusCode::UNAUTHORIZED, message.clone(), None)
            }
            HttpError::Unknown { message, .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, message.clone(), None)
            }
        };

        Self {
            category: format!("{}", error.category()),
            message,
            field,
            status: status.as_u16(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_network() {
        let error = HttpError::Network {
            message: "Connection refused".to_string(),
            source: Some("tcp".to_string()),
        };
        assert_eq!(error.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_error_category_timeout() {
        let error = HttpError::Timeout {
            duration_secs: 30,
            operation: "API call".to_string(),
        };
        assert_eq!(error.category(), ErrorCategory::Timeout);
    }

    #[test]
    fn test_error_category_server() {
        let error = HttpError::Server {
            status: 500,
            message: "Internal error".to_string(),
        };
        assert_eq!(error.category(), ErrorCategory::Server);
    }

    #[test]
    fn test_error_category_client() {
        let error = HttpError::Client {
            status: 404,
            message: "Not found".to_string(),
        };
        assert_eq!(error.category(), ErrorCategory::Client);
    }

    #[test]
    fn test_error_category_validation() {
        let error = HttpError::Validation {
            field: "email".to_string(),
            message: "Invalid email".to_string(),
        };
        assert_eq!(error.category(), ErrorCategory::Validation);
    }

    #[test]
    fn test_error_category_auth() {
        let error = HttpError::Auth {
            message: "Unauthorized".to_string(),
        };
        assert_eq!(error.category(), ErrorCategory::Auth);
    }

    #[test]
    fn test_error_message() {
        let error = HttpError::Network {
            message: "Connection refused".to_string(),
            source: None,
        };
        assert_eq!(error.message(), "Connection refused");
    }

    #[test]
    fn test_error_response_new() {
        let response = ErrorResponse::new(
            ErrorCategory::Validation,
            "Invalid email".to_string(),
            StatusCode::BAD_REQUEST,
        );
        assert_eq!(response.category, "Validation");
        assert_eq!(response.message, "Invalid email");
        assert_eq!(response.status, 400);
        assert!(response.field.is_none());
    }

    #[test]
    fn test_error_response_with_field() {
        let response = ErrorResponse::new(
            ErrorCategory::Validation,
            "Invalid email".to_string(),
            StatusCode::BAD_REQUEST,
        )
        .with_field("email".to_string());
        assert_eq!(response.field, Some("email".to_string()));
    }

    #[test]
    fn test_error_response_from_http_error_network() {
        let http_error = HttpError::Network {
            message: "Connection refused".to_string(),
            source: None,
        };
        let response = ErrorResponse::from(http_error);
        assert_eq!(response.category, "Network");
        assert_eq!(response.message, "Connection refused");
        assert_eq!(response.status, 503);
    }

    #[test]
    fn test_error_response_from_http_error_validation() {
        let http_error = HttpError::Validation {
            field: "email".to_string(),
            message: "Invalid email".to_string(),
        };
        let response = ErrorResponse::from(http_error);
        assert_eq!(response.category, "Validation");
        assert_eq!(response.message, "Invalid email");
        assert_eq!(response.field, Some("email".to_string()));
        assert_eq!(response.status, 400);
    }

    #[test]
    fn test_error_response_from_http_error_auth() {
        let http_error = HttpError::Auth {
            message: "Unauthorized".to_string(),
        };
        let response = ErrorResponse::from(http_error);
        assert_eq!(response.category, "Auth");
        assert_eq!(response.message, "Unauthorized");
        assert_eq!(response.status, 401);
    }
}
