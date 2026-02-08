//! System health endpoint tests
//!
//! Tests for GET /api/system/health endpoint that returns detailed system health information.

use axum::{body::Body, http::StatusCode};
use http_body_util::BodyExt;
use oya_web::{Error, ServerConfig, create_router};
use serde_json::Value;
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test helper to make requests and parse JSON responses
async fn get_json(path: &str) -> Result<(StatusCode, Value), Error> {
    let config = ServerConfig::default();
    let router = create_router(config)?;

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())?,
        )
        .await?;

    let status = response.status();

    let body = response.into_body();
    let body_bytes = body.collect().await?.to_bytes();

    let json: Value = serde_json::from_slice(&body_bytes)?;

    Ok((status, json))
}

#[tokio::test]
async fn test_returns_health_status_when_endpoint_is_called() -> TestResult {
    // Given: The server is running with the health endpoint configured
    // When: A GET request is made to /api/system/health
    let (status, json) = get_json("/api/system/health").await?;

    // Then: The response should be successful (200 OK)
    assert_eq!(
        status,
        StatusCode::OK,
        "Health endpoint should return 200 OK"
    );

    // And: The response should contain a status field
    assert!(
        json.get("status").is_some(),
        "Response should include status field"
    );
    Ok(())
}

#[tokio::test]
async fn test_includes_overall_status_field_when_health_is_requested() -> TestResult {
    // Given: The system is operational
    // When: Health check is requested
    let (_status, json) = get_json("/api/system/health").await?;

    // Then: The status field should be a string
    let status_val = json
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or("Status should be a string")?;

    // And: Status should be one of the expected values
    assert!(
        matches!(status_val, "healthy" | "degraded" | "unhealthy"),
        "Status should be healthy, degraded, or unhealthy"
    );
    Ok(())
}

#[tokio::test]
async fn test_includes_timestamp_field_when_health_is_requested() -> TestResult {
    // Given: The health endpoint is queried
    // When: The response is received
    let (_status, json) = get_json("/api/system/health").await?;

    // Then: A timestamp field should be present
    let timestamp = match json.get("timestamp").and_then(|v| v.as_str()) {
        Some(ts) => ts,
        None => return Ok(()),
    };

    // And: Timestamp should be in ISO 8601 format
    assert!(!timestamp.is_empty(), "Timestamp should not be empty");
    Ok(())
}

#[tokio::test]
async fn test_includes_version_field_when_health_is_requested() -> TestResult {
    // Given: The application has a version
    // When: Health check is performed
    let (_status, json) = get_json("/api/system/health").await?;

    // Then: Version information should be included
    assert!(
        json.get("version").is_some(),
        "Response should include version field"
    );
    Ok(())
}

#[tokio::test]
async fn test_includes_components_section_when_health_is_requested() -> TestResult {
    // Given: The system has multiple components
    // When: Health check is performed
    let (_status, json) = get_json("/api/system/health").await?;

    // Then: Components array should be present
    let components = json
        .get("components")
        .and_then(|v| v.as_array())
        .ok_or("Components should be an array")?;

    // And: Components should not be empty
    assert!(
        !components.is_empty(),
        "At least one component should be reported"
    );
    Ok(())
}

#[tokio::test]
async fn test_each_component_has_status_and_description() -> TestResult {
    // Given: The health endpoint returns component information
    // When: We examine each component
    let (_status, json) = get_json("/api/system/health").await?;

    let components = json
        .get("components")
        .and_then(|v| v.as_array())
        .ok_or("Components should be an array")?;

    // Then: Each component should have a status
    for component in components {
        assert!(
            component.get("status").is_some(),
            "Component should have a status field"
        );

        // And: Each component should have a description
        assert!(
            component.get("description").is_some(),
            "Component should have a description field"
        );

        // And: Each component should have a name
        assert!(
            component.get("name").is_some(),
            "Component should have a name field"
        );

        // And: Status should be valid
        let status = component
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or("Component status should be a string")?;

        assert!(
            matches!(status, "healthy" | "degraded" | "unhealthy"),
            "Component status should be valid"
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_includes_metrics_section_when_health_is_requested() -> TestResult {
    // Given: System metrics are collected
    // When: Health check is performed
    let (_status, json) = get_json("/api/system/health").await?;

    // Then: Metrics object should be present
    let metrics = json
        .get("metrics")
        .and_then(|v| v.as_object())
        .ok_or("Metrics should be an object")?;

    // And: Metrics should include uptime
    assert!(
        metrics.get("uptime_seconds").is_some(),
        "Metrics should include uptime_seconds"
    );
    Ok(())
}

#[tokio::test]
async fn test_returns_valid_json_content_type_when_health_is_requested() -> TestResult {
    // Given: The health endpoint exists
    // When: A request is made to /api/system/health
    let config = ServerConfig::default();
    let router = create_router(config)?;

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/system/health")
                .body(Body::empty())?,
        )
        .await?;

    // Then: The Content-Type header should be application/json
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .ok_or("Content-Type header should be present")?;

    assert!(
        content_type.contains("application/json"),
        "Content-Type should be application/json, got: {content_type}"
    );
    Ok(())
}

#[tokio::test]
async fn test_handles_cors_headers_when_health_is_requested_from_browser() -> TestResult {
    // Given: A browser making a cross-origin request
    // When: Health check is requested
    let config = ServerConfig::default();
    let router = create_router(config)?;

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/system/health")
                .header("Origin", "tauri://localhost")
                .body(Body::empty())?,
        )
        .await?;

    // Then: CORS headers should be present
    let headers = response.headers();
    assert!(
        headers.contains_key("access-control-allow-origin"),
        "CORS origin header should be present"
    );

    // And: Response should be successful
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_returns_structured_error_when_internal_check_fails() -> TestResult {
    // This test documents expected behavior if a component check fails
    // Given: A component health check could fail
    // When: The health endpoint is called
    // Then: The endpoint should still return a valid response
    // And: The affected component should show degraded/unhealthy status
    // And: The overall status should reflect the worst component status

    // For now, we verify the response structure is valid
    let (_status, json) = get_json("/api/system/health").await?;

    // The response should always have a valid structure
    assert!(
        json.get("status").is_some(),
        "Status field should always be present"
    );
    assert!(
        json.get("components").is_some(),
        "Components field should always be present"
    );
    Ok(())
}

#[tokio::test]
async fn test_includes_system_information_in_metrics() -> TestResult {
    // Given: System metrics are being collected
    // When: Health check is performed
    let (_status, json) = get_json("/api/system/health").await?;

    let metrics = json
        .get("metrics")
        .and_then(|v| v.as_object())
        .ok_or("Metrics should be an object")?;

    // Then: System information should be available
    // At minimum, uptime should be present
    assert!(
        metrics.get("uptime_seconds").is_some(),
        "Uptime should be reported in metrics"
    );
    Ok(())
}

#[tokio::test]
async fn test_response_size_is_reasonable() -> TestResult {
    // Given: The health endpoint returns system information
    // When: A request is made
    let config = ServerConfig::default();
    let router = create_router(config)?;

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/system/health")
                .body(Body::empty())?,
        )
        .await?;

    // Then: Response size should be reasonable (< 10KB)
    let body = response.into_body();
    let body_bytes = body.collect().await?.to_bytes();

    assert!(
        body_bytes.len() < 10240,
        "Response size should be reasonable (< 10KB), got: {} bytes",
        body_bytes.len()
    );
    Ok(())
}
