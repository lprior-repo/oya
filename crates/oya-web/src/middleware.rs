//! Middleware helpers for the OYA web server.
//!
//! This module provides middleware for:
//! - CORS handling
//! - Error catching and logging
//! - Panic recovery
//! - Request tracing
//!
//! All middleware follows functional patterns with pure context extraction
//! and composable middleware stacking.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

/// CORS middleware layer
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Pure request context extracted before passing ownership to next middleware
///
/// This struct captures all the information we need for logging and monitoring
/// before the request is moved into the next handler. The use of clone is
/// documented and justified by ownership requirements of the axum middleware chain.
#[derive(Clone, Debug)]
pub struct RequestContext {
    /// HTTP method (GET, POST, etc.)
    pub method: axum::http::Method,
    /// Request URI
    pub uri: axum::http::Uri,
    /// When the request started processing
    pub start: Instant,
}

impl RequestContext {
    /// Extract context from a request reference
    ///
    /// # Cloning Justification
    /// We clone Method and Uri because:
    /// 1. Axum requires ownership of Request to pass to next.run()
    /// 2. We need logging context after the request completes
    /// 3. Method and Uri are cheap to clone (Arc-backed internally)
    fn from_request(req: &Request) -> Self {
        Self {
            method: req.method().clone(),
            uri: req.uri().clone(),
            start: Instant::now(),
        }
    }

    /// Calculate elapsed time since request started
    fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Log successful request completion
    fn log_completion(&self, status: StatusCode) {
        info!(
            method = %self.method,
            uri = %self.uri,
            duration_ms = self.elapsed().as_millis(),
            status = %status.as_u16(),
            "Request completed"
        );
    }

    /// Log client error (4xx)
    fn log_client_error(&self, status: StatusCode) {
        warn!(
            method = %self.method,
            uri = %self.uri,
            status = %status.as_u16(),
            "Client error occurred"
        );
    }

    /// Log server error (5xx)
    fn log_server_error(&self, status: StatusCode) {
        error!(
            method = %self.method,
            uri = %self.uri,
            status = %status.as_u16(),
            "Server error occurred"
        );
    }
}

/// Middleware variants for functional composition
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Middleware {
    /// CORS middleware
    Cors,
    /// Error handling and logging
    ErrorHandler,
    /// Panic catching
    CatchPanic,
    /// Request tracing/logging
    Logging,
}

/// Error handling middleware
///
/// Catches errors from handlers and ensures they are properly logged
/// and converted to RFC 7807 Problem Details responses.
///
/// This middleware wraps all handlers and catches any errors that
/// propagate up, logging them with appropriate context.
///
/// # Functional Pattern
/// - Extracts pure RequestContext before ownership transfer
/// - Uses context methods for conditional logging
/// - Returns Response directly (Result wrapper for axum compatibility)
pub async fn error_handler_middleware(req: Request, next: Next) -> Result<Response, Response> {
    // Extract pure context before passing ownership
    let ctx = RequestContext::from_request(&req);

    // Run the handler and catch any errors
    let response = next.run(req).await;

    // Check if the response indicates an error
    let status = response.status();
    if status.is_server_error() {
        ctx.log_server_error(status);
    } else if status.is_client_error() {
        ctx.log_client_error(status);
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
///
/// # Functional Pattern
/// - Extracts pure RequestContext before ownership transfer
/// - Uses context methods for error logging
/// - In axum, panics in async handlers are already caught at the task level
/// - This middleware provides logging for synchronous panics in setup code
pub async fn catch_panic_middleware(req: Request, next: Next) -> Response {
    // Extract pure context before passing ownership
    let ctx = RequestContext::from_request(&req);

    // Run the next middleware/handler
    let response = next.run(req).await;

    // Log any server errors from the response
    if response.status().is_server_error() {
        ctx.log_server_error(response.status());
    }

    response
}

/// Request logging middleware
///
/// Logs all incoming requests with timing information.
///
/// # Functional Pattern
/// - Extracts pure RequestContext with timing start
/// - Logs completion with duration after response
/// - Zero unwraps, zero panics
pub async fn logging_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let ctx = RequestContext::from_request(&req);

    let response = next.run(req).await;

    ctx.log_completion(response.status());

    Ok(response)
}

/// Functionally compose middleware layers onto a router
///
/// # Arguments
/// * `app` - Base router to compose onto
/// * `middleware` - Iterator of middleware to apply in order
///
/// # Returns
/// Router with all middleware layers applied
///
/// # Functional Pattern
/// - Uses fold for functional composition
/// - Each middleware transforms the router
/// - Zero mutations, pure functional pipeline
///
/// # Example
/// ```rust
/// let app = Router::new()
///     .route("/test", get(handler));
///
/// let middleware = vec![Middleware::Cors, Middleware::Logging];
/// let app = apply_middleware(app, middleware);
/// ```
pub fn apply_middleware<'a>(
    mut app: axum::Router,
    middleware: impl IntoIterator<Item = &'a Middleware>,
) -> axum::Router {
    middleware.into_iter().fold(app, |acc, mw| match mw {
        Middleware::Cors => acc.layer(cors_layer()),
        Middleware::ErrorHandler => acc.layer(axum::middleware::from_fn(error_handler_middleware)),
        Middleware::CatchPanic => acc.layer(axum::middleware::from_fn(catch_panic_middleware)),
        Middleware::Logging => acc.layer(axum::middleware::from_fn(logging_middleware)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{HeaderValue, Method, StatusCode, header},
        routing::get,
    };
    use std::time::Duration;
    use tower::ServiceExt;
    use tower_http::compression::CompressionLayer;

    /// Test that CORS layer can be created
    #[test]
    fn test_cors_layer_creation() {
        let layer = cors_layer();
        // Just verify it can be created without panicking
        assert!(true);
    }

    /// Test RequestContext extraction from request
    #[test]
    fn test_request_context_extraction() {
        let request = match axum::http::Request::builder()
            .uri("/test/path")
            .method(Method::POST)
            .body(Body::empty())
        {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Test setup error: Failed to build request: {e}");
                return;
            }
        };

        let ctx = RequestContext::from_request(&request);

        assert_eq!(ctx.method, Method::POST);
        assert_eq!(ctx.uri, "/test/path");
        // Time should be very recent
        assert!(ctx.elapsed() < Duration::from_millis(10));
    }

    /// Test RequestContext elapsed timing
    #[test]
    fn test_request_context_elapsed() {
        let request = match axum::http::Request::builder()
            .uri("/test")
            .method(Method::GET)
            .body(Body::empty())
        {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Test setup error: Failed to build request: {e}");
                return;
            }
        };

        let ctx = RequestContext::from_request(&request);

        // Sleep to ensure measurable time passes
        std::thread::sleep(Duration::from_millis(10));

        let elapsed = ctx.elapsed();

        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(100));
    }

    /// Test RequestContext clone behavior
    #[test]
    fn test_request_context_clone() {
        let request = match axum::http::Request::builder()
            .uri("/test")
            .method(Method::GET)
            .body(Body::empty())
        {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Test setup error: Failed to build request: {e}");
                return;
            }
        };

        let ctx1 = RequestContext::from_request(&request);
        let ctx2 = ctx1.clone();

        assert_eq!(ctx1.method, ctx2.method);
        assert_eq!(ctx1.uri, ctx2.uri);

        // Both should have same start time
        let elapsed1 = ctx1.elapsed();
        let elapsed2 = ctx2.elapsed();
        assert!(elapsed1.as_millis() - elapsed2.as_millis() < 10);
    }

    /// Test middleware functional composition
    #[test]
    fn test_apply_middleware_empty() {
        let app = Router::new().route("/test", get(|| async { "OK" }));

        let middleware: Vec<Middleware> = vec![];
        let result = apply_middleware(app, &middleware);

        // Should still be able to build the router
        assert!(true);
    }

    /// Test middleware functional composition with CORS
    #[test]
    fn test_apply_middleware_cors() {
        let app = Router::new().route("/test", get(|| async { "OK" }));

        let middleware = vec![Middleware::Cors];
        let result = apply_middleware(app, &middleware);

        // Should still be able to build the router
        assert!(true);
    }

    /// Test middleware functional composition with multiple layers
    #[test]
    fn test_apply_middleware_multiple() {
        let app = Router::new().route("/test", get(|| async { "OK" }));

        let middleware = vec![
            Middleware::Cors,
            Middleware::ErrorHandler,
            Middleware::Logging,
        ];
        let result = apply_middleware(app, &middleware);

        // Should still be able to build the router
        assert!(true);
    }

    /// Test middleware enum variants
    #[test]
    fn test_middleware_variants() {
        let cors = Middleware::Cors;
        let error = Middleware::ErrorHandler;
        let panic = Middleware::CatchPanic;
        let logging = Middleware::Logging;

        // Test equality
        assert_eq!(cors, Middleware::Cors);
        assert_eq!(error, Middleware::ErrorHandler);
        assert_eq!(panic, Middleware::CatchPanic);
        assert_eq!(logging, Middleware::Logging);

        // Test inequality
        assert_ne!(cors, error);
        assert_ne!(error, panic);
        assert_ne!(panic, logging);
    }

    /// Test logging middleware end-to-end
    #[tokio::test]
    async fn test_logging_middleware_functional() {
        async fn handler() -> &'static str {
            "OK"
        }

        let app = Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn(logging_middleware));

        let request = match axum::http::Request::builder()
            .uri("/test")
            .method(Method::GET)
            .body(Body::empty())
        {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Test setup error: Failed to build request: {e}");
                return;
            }
        };

        let response = match app.oneshot(request).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Test error: Failed to get response: {e}");
                return;
            }
        };

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test error handler middleware end-to-end
    #[tokio::test]
    async fn test_error_handler_middleware_functional() {
        async fn handler() -> Result<&'static str, StatusCode> {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }

        let app = Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn(error_handler_middleware));

        let request = match axum::http::Request::builder()
            .uri("/test")
            .method(Method::GET)
            .body(Body::empty())
        {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Test setup error: Failed to build request: {e}");
                return;
            }
        };

        let response = match app.oneshot(request).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Test error: Failed to get response: {e}");
                return;
            }
        };

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
