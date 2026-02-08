//! Workflow graph endpoint tests
//!
//! Tests for GET /api/graph endpoint that returns DAG workflow graph data.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![allow(clippy::expect_used)] // Tests are allowed to use expect

use axum::{body::Body, http::StatusCode};
use http_body_util::BodyExt;
use oya_web::{ServerConfig, create_router};
use serde_json::Value;
use tower::ServiceExt;

/// Test helper to make requests and parse JSON responses
async fn get_json(path: &str) -> (StatusCode, Value) {
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    let status = response.status();

    let body = response.into_body();
    let body_bytes = body
        .collect()
        .await
        .expect("Failed to collect body")
        .to_bytes();

    let json: Value = serde_json::from_slice(&body_bytes).expect("Failed to parse JSON");

    (status, json)
}

#[tokio::test]
async fn test_returns_graph_data_when_endpoint_is_called() {
    // Given: The server is running with the graph endpoint configured
    // When: A GET request is made to /api/graph
    let (status, json) = get_json("/api/graph").await;

    // Then: The response should be successful (200 OK)
    assert_eq!(
        status,
        StatusCode::OK,
        "Graph endpoint should return 200 OK"
    );

    // And: The response should contain nodes array
    assert!(
        json.get("nodes").is_some(),
        "Response should include nodes array"
    );
}

#[tokio::test]
async fn test_includes_nodes_array_when_graph_is_requested() {
    // Given: The graph endpoint exists
    // When: Graph data is requested
    let (_status, json) = get_json("/api/graph").await;

    // Then: Nodes field should be an array
    let nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("Nodes should be an array");

    // And: Should be valid (even if empty)
    let _node_count = nodes.len();
}

#[tokio::test]
async fn test_includes_edges_array_when_graph_is_requested() {
    // Given: The graph endpoint exists
    // When: Graph data is requested
    let (_status, json) = get_json("/api/graph").await;

    // Then: Edges field should be an array
    let edges = json
        .get("edges")
        .and_then(|v| v.as_array())
        .expect("Edges should be an array");

    // And: Should be valid (even if empty)
    let _edge_count = edges.len();
}

#[tokio::test]
async fn test_nodes_have_required_fields_when_graph_is_returned() {
    // Given: The graph endpoint returns node data
    // When: We examine the nodes
    let (_status, json) = get_json("/api/graph").await;

    let nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("Nodes should be an array");

    // Then: Each node should have required fields (if nodes exist)
    for node in nodes.iter().take(10) {
        // Only check first 10 nodes to avoid excessive output
        assert!(node.get("id").is_some(), "Node should have an id field");
        assert!(
            node.get("label").is_some(),
            "Node should have a label field"
        );
        assert!(
            node.get("x").is_some(),
            "Node should have an x coordinate field"
        );
        assert!(
            node.get("y").is_some(),
            "Node should have a y coordinate field"
        );
    }
}

#[tokio::test]
async fn test_edges_have_required_fields_when_graph_is_returned() {
    // Given: The graph endpoint returns edge data
    // When: We examine the edges
    let (_status, json) = get_json("/api/graph").await;

    let edges = json
        .get("edges")
        .and_then(|v| v.as_array())
        .expect("Edges should be an array");

    // Then: Each edge should have required fields (if edges exist)
    for edge in edges.iter().take(10) {
        // Only check first 10 edges to avoid excessive output
        assert!(
            edge.get("source").is_some(),
            "Edge should have a source field"
        );
        assert!(
            edge.get("target").is_some(),
            "Edge should have a target field"
        );
        assert!(
            edge.get("edge_type").is_some(),
            "Edge should have an edge_type field"
        );
    }
}

#[tokio::test]
async fn test_returns_valid_json_content_type_when_graph_is_requested() {
    // Given: The graph endpoint exists
    // When: A request is made to /api/graph
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/graph")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    // Then: The Content-Type header should be application/json
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .expect("Content-Type header should be present");

    assert!(
        content_type.contains("application/json"),
        "Content-Type should be application/json, got: {content_type}"
    );
}

#[tokio::test]
async fn test_handles_cors_headers_when_graph_is_requested_from_browser() {
    // Given: A browser making a cross-origin request
    // When: Graph data is requested
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/graph")
                .header("Origin", "tauri://localhost")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    // Then: CORS headers should be present
    let headers = response.headers();
    assert!(
        headers.contains_key("access-control-allow-origin"),
        "CORS origin header should be present"
    );

    // And: Response should be successful
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_response_size_is_reasonable() {
    // Given: The graph endpoint returns workflow information
    // When: A request is made
    let config = ServerConfig::default();
    let router = create_router(config).expect("Router creation failed");

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/graph")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Request failed");

    // Then: Response size should be reasonable (< 100KB for graph data)
    let body = response.into_body();
    let body_bytes = body
        .collect()
        .await
        .expect("Failed to collect body")
        .to_bytes();

    assert!(
        body_bytes.len() < 102400,
        "Response size should be reasonable (< 100KB), got: {} bytes",
        body_bytes.len()
    );
}

#[tokio::test]
async fn test_empty_graph_returns_valid_structure() {
    // Given: The graph endpoint might return empty data
    // When: Graph data is requested
    let (_status, json) = get_json("/api/graph").await;

    // Then: The response should still have valid structure
    let nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("Nodes should be an array");

    let edges = json
        .get("edges")
        .and_then(|v| v.as_array())
        .expect("Edges should be an array");

    // And: Empty arrays are valid
    let _ = nodes.is_empty();
    let _ = edges.is_empty();
}

#[tokio::test]
async fn test_graph_response_matches_schema() {
    // Given: A WorkflowGraphResponse struct
    // When: It's serialized to JSON
    use oya_web::workflow_graph::WorkflowGraphResponse;

    let response = WorkflowGraphResponse {
        nodes: vec![],
        edges: vec![],
    };

    let json = serde_json::to_string(&response).expect("Serialization should succeed");

    // Then: All top-level fields should be present
    assert!(json.contains("nodes"), "Should contain nodes field");
    assert!(json.contains("edges"), "Should contain edges field");
}
