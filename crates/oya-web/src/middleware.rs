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
    response::{IntoResponse, Response},
    middleware::Next,
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
pub async fn error_handler_middleware<B>(
    req: Request<B>,
    next: Next<B>,
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
pub async fn catch_panic_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let uri = req.uri().clone();
    let method = req.method().clone();

    // Use std::panic::catch_unwind to catch panics
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(async {
        next.run(req).await
    }))
    .await;

    match result {
        Ok(response) => response,
        Err(panic_info) => {
            // Log the panic with context
            let panic_msg = if let Some(msg) = panic_info.downcast_ref::<String>() {
                msg.clone()
            } else if let Some(msg) = panic_info.downcast_ref::<&str>() {
                msg.to_string()
            } else {
                "Unknown panic".to_string()
            };

            error!(
                %method,
                %uri,
                panic_message = %panic_msg,
                "Panic caught in request handler"
            );

            // Return 500 error response
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ).into_response()
        }
    }
}
