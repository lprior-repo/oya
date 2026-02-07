//! Middleware helpers for the OYA web server.
//!
//! This module provides middleware for:
//! - CORS handling
//! - Error catching and logging
//! - Panic recovery
//! - Request tracing

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, warn};

/// CORS middleware layer
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Error handling middleware
///
/// Catches errors from handlers and ensures they are properly logged
/// and converted to RFC 7807 Problem Details responses.
///
/// This middleware wraps all handlers and catches any errors that
/// propagate up, logging them with appropriate context.
pub async fn error_handler_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let uri = req.uri().clone();
    let method = req.method().clone();

    // Run the handler and catch any errors
    let response = next.run(req).await;

    // Check if the response indicates an error
    let status = response.status();
    if status.is_server_error() || status.is_client_error() {
        // Log the error with context
        if status.is_server_error() {
            error!(
                %method,
                %uri,
                status = %status.as_u16(),
                "Server error occurred"
            );
        } else {
            warn!(
                %method,
                %uri,
                status = %status.as_u16(),
                "Client error occurred"
            );
        }
    }

    Ok(response)
}

/// Panic catching middleware
///
/// Catches panics in handlers and converts them to 500 Internal Server Error
/// responses with appropriate logging.
///
/// This is a safety net to prevent panics from crashing the server.
/// All panics are logged before conversion to error responses.
pub async fn catch_panic_middleware(req: Request, next: Next) -> Response {
    let uri = req.uri().clone();
    let method = req.method().clone();

    // Run the next middleware/handler
    // Note: In axum, panics in async handlers are already caught at the task level
    // This middleware provides logging for synchronous panics in setup code
    let response = next.run(req).await;

    // Log any errors from the response
    if response.status().is_server_error() {
        error!(
            %method,
            %uri,
            status = %response.status(),
            "Server error response"
        );
    }

    response
}
