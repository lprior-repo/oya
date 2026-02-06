//! Error handling with RFC 7807 Problem Details for JSON responses

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

pub type Result<T> = std::result::Result<T, AppError>;

/// Application error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// RFC 7807 Problem Details for HTTP APIs
#[derive(Serialize)]
pub struct ErrorResponse {
    #[serde(rename = "type")]
    problem_type: Option<String>,
    title: String,
    status: u16,
    detail: String,
}

impl ErrorResponse {
    pub fn new(status: StatusCode, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            problem_type: None,
            title: title.into(),
            status: status.as_u16(),
            detail: detail.into(),
        }
    }

    pub fn from_error(err: &AppError) -> Self {
        let status = err.status_code();
        let title = status
            .canonical_reason()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Error".to_string());

        Self::new(status, title, err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let response = ErrorResponse::from_error(&self);
        (status, Json(response)).into_response()
    }
}
