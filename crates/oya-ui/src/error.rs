//! Error types for Leptos UI components
//!
//! This module provides error handling types that follow the project's
//! zero-unwrap and functional programming patterns.

use std::fmt;

/// Errors that can occur in Leptos UI components
#[derive(Debug, Clone, PartialEq)]
pub enum LeptosError {
    /// Route not found
    RouteNotFound(String),
    /// Invalid route parameter
    InvalidRouteParam { param: String, value: String },
    /// Navigation failed
    NavigationFailed(String),
    /// Component initialization failed
    InitializationFailed(String),
    /// Canvas operation failed
    CanvasError(String),
    /// Generic error with context
    Generic(String),
}

impl fmt::Display for LeptosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RouteNotFound(route) => write!(f, "Route not found: {}", route),
            Self::InvalidRouteParam { param, value } => {
                write!(f, "Invalid route parameter '{}': {}", param, value)
            }
            Self::NavigationFailed(msg) => write!(f, "Navigation failed: {}", msg),
            Self::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::CanvasError(msg) => write!(f, "Canvas error: {}", msg),
            Self::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LeptosError {}

/// Result type alias for Leptos operations
pub type Result<T> = std::result::Result<T, LeptosError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = LeptosError::RouteNotFound("/invalid".to_string());
        assert_eq!(error.to_string(), "Route not found: /invalid");

        let error = LeptosError::InvalidRouteParam {
            param: "id".to_string(),
            value: "abc".to_string(),
        };
        assert_eq!(error.to_string(), "Invalid route parameter 'id': abc");

        let error = LeptosError::NavigationFailed("state error".to_string());
        assert_eq!(error.to_string(), "Navigation failed: state error");
    }

    #[test]
    fn test_error_clone() {
        let error = LeptosError::Generic("test".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_result_type() {
        let success: Result<i32> = Ok(42);
        assert!(success.is_ok());

        let failure: Result<i32> = Err(LeptosError::Generic("failed".to_string()));
        assert!(failure.is_err());
    }
}
