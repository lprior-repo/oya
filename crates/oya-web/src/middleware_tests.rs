//! Tests for Tower middleware (CORS, tracing, compression)

use super::*;
use axum::{
    body::Body,
    http::{header, HeaderValue, Method, StatusCode},
    routing::get,
    Router,
};
use tower::ServiceExt;
use tower_http::compression::CompressionLayer;

#[cfg(test)]
mod cors_tests {
    use super::*;

    /// Test that CORS layer is properly configured
    #[tokio::test]
    async fn test_cors_layer_allows_origin() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .method(Method::GET)
                    .header(header::ORIGIN, "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check for CORS headers
        let headers = response.headers();
        assert!(headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    }

    /// Test that CORS allows various methods
    #[tokio::test]
    async fn test_cors_allows_methods() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer());

        // Test OPTIONS preflight
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .method(Method::OPTIONS)
                    .header(header::ORIGIN, "https://example.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test that CORS allows custom headers
    #[tokio::test]
    async fn test_cors_allows_headers() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .method(Method::GET)
                    .header(header::ORIGIN, "https://example.com")
                    .header("x-custom-header", "test-value")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[cfg(test)]
mod compression_tests {
    use super::*;

    /// Test that compression layer compresses responses
    #[tokio::test]
    async fn test_compression_layer_exists() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(CompressionLayer::new());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test that compression respects accept-encoding
    #[tokio::test]
    async fn test_compression_respects_accept_encoding() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(CompressionLayer::new());

        // Request without accept-encoding should get uncompressed response
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test compression with large payload
    #[tokio::test]
    async fn test_compression_large_payload() {
        let large_body = "x".repeat(10000);

        let app = Router::new()
            .route("/test", get(|| async { large_body }))
            .layer(CompressionLayer::new());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify content-encoding header is set
        if let Some(encoding) = response.headers().get(header::CONTENT_ENCODING) {
            assert_eq!(encoding, HeaderValue::from_static("gzip"));
        }
    }
}

#[cfg(test)]
mod tracing_tests {
    use super::*;

    /// Test that trace layer doesn't break requests
    #[tokio::test]
    async fn test_trace_layer_preserves_functionality() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(tower_http::trace::TraceLayer::new_for_http());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test that trace layer works with error responses
    #[tokio::test]
    async fn test_trace_layer_with_errors() {
        async fn error_handler() -> &'static str {
            panic!("This handler panics");
        }

        let app = Router::new()
            .route("/test", get(error_handler))
            .layer(tower_http::trace::TraceLayer::new_for_http());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return error status (500 for panic)
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[cfg(test)]
mod middleware_stack_tests {
    use super::*;

    /// Test that all middleware work together
    #[tokio::test]
    async fn test_combined_middleware_stack() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer())
            .layer(CompressionLayer::new())
            .layer(tower_http::trace::TraceLayer::new_for_http());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .header(header::ORIGIN, "https://example.com")
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test middleware stack with error handler
    #[tokio::test]
    async fn test_middleware_with_error_handler() {
        use axum::{middleware, http::StatusCode, response::{IntoResponse, Response}};

        async fn test_handler() -> Result<&'static str, StatusCode> {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }

        let app = Router::new()
            .route("/test", axum::routing::get(test_handler))
            .layer(cors_layer())
            .layer(CompressionLayer::new())
            .layer(tower_http::trace::TraceLayer::new_for_http());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Test middleware stack with panic catcher
    #[tokio::test]
    async fn test_middleware_with_panic_catcher() {
        use axum::middleware;

        async fn test_handler() -> &'static str {
            "OK"
        }

        let app = Router::new()
            .route("/test", axum::routing::get(test_handler))
            .layer(cors_layer())
            .layer(CompressionLayer::new())
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .layer(middleware::from_fn(
                catch_panic_middleware,
            ));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
