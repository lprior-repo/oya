//! Tests for WorkflowGraphResponse struct
//!
//! Tests follow Martin Fowler's Given-When-Then pattern:
//! - test_{verb}_{outcome}_when_{condition}
//!
//! Each test validates one specific behavior with clear naming.

use oya_web::workflow_graph::{GraphEdge, GraphEdgeType, GraphNode, WorkflowGraphResponse};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Given a WorkflowGraphResponse with nodes and edges,
/// When serializing to JSON,
/// Then the JSON should contain all node and edge data
#[test]
fn test_serializes_graph_data_when_response_has_nodes_and_edges() -> TestResult {
    // Given: A response with nodes and edges
    let nodes = vec![
        GraphNode {
            id: "bead-1".to_string(),
            label: "First Task".to_string(),
            x: 100.0,
            y: 200.0,
        },
        GraphNode {
            id: "bead-2".to_string(),
            label: "Second Task".to_string(),
            x: 300.0,
            y: 400.0,
        },
    ];

    let edges = vec![GraphEdge {
        source: "bead-1".to_string(),
        target: "bead-2".to_string(),
        edge_type: GraphEdgeType::Blocking,
    }];

    let response = WorkflowGraphResponse { nodes, edges };

    // When: Serializing to JSON
    let json = serde_json::to_string(&response)?;

    // Then: JSON should contain expected data
    assert!(json.contains("bead-1"), "JSON should contain first node id");
    assert!(
        json.contains("bead-2"),
        "JSON should contain second node id"
    );
    assert!(
        json.contains("First Task"),
        "JSON should contain node label"
    );
    assert!(json.contains("blocking"), "JSON should contain edge type");
    Ok(())
}

/// Given a WorkflowGraphResponse JSON,
/// When deserializing from JSON,
/// Then the response should reconstruct nodes and edges correctly
#[test]
fn test_deserializes_from_json_when_json_is_valid() -> TestResult {
    // Given: Valid JSON representation
    let json = r#"{
        "nodes": [
            {"id": "task-a", "label": "Task A", "x": 10.0, "y": 20.0},
            {"id": "task-b", "label": "Task B", "x": 30.0, "y": 40.0}
        ],
        "edges": [
            {"source": "task-a", "target": "task-b", "edge_type": "blocking"}
        ]
    }"#;

    // When: Deserializing
    let response: WorkflowGraphResponse = serde_json::from_str(json)?;

    // Then: Should succeed with correct data
    assert_eq!(response.nodes.len(), 2, "Should have 2 nodes");
    assert_eq!(response.nodes[0].id, "task-a", "First node id should match");
    assert_eq!(response.edges.len(), 1, "Should have 1 edge");
    assert_eq!(
        response.edges[0].source, "task-a",
        "Edge source should match"
    );
    Ok(())
}

/// Given an empty WorkflowGraphResponse,
/// When serializing to JSON,
/// Then should produce valid JSON with empty arrays
#[test]
fn test_serializes_empty_graph_when_response_has_no_data() -> TestResult {
    // Given: Empty response
    let response = WorkflowGraphResponse {
        nodes: vec![],
        edges: vec![],
    };

    // When: Serializing
    let json = serde_json::to_string(&response)?;

    // Then: Should have empty arrays
    assert!(
        json.contains("\"nodes\":[]"),
        "Should have empty nodes array"
    );
    assert!(
        json.contains("\"edges\":[]"),
        "Should have empty edges array"
    );
    Ok(())
}

/// Given a GraphNode with position data,
/// When accessing the x and y coordinates,
/// Then the values should be accessible and correct
#[test]
fn test_graph_node_has_position_when_created() {
    // Given: A node with specific position
    let node = GraphNode {
        id: "node-1".to_string(),
        label: "Test Node".to_string(),
        x: 123.45,
        y: 678.90,
    };

    // When: Accessing coordinates
    // Then: Values should match
    assert_eq!(node.x, 123.45, "X coordinate should match");
    assert_eq!(node.y, 678.90, "Y coordinate should match");
}

/// Given a GraphEdge with blocking dependency type,
/// When serializing the edge,
/// Then the edge_type should be "blocking"
#[test]
fn test_edge_type_serializes_to_lowercase_when_edge_is_blocking() -> TestResult {
    // Given: Edge with blocking type
    let edge = GraphEdge {
        source: "a".to_string(),
        target: "b".to_string(),
        edge_type: GraphEdgeType::Blocking,
    };

    // When: Serializing
    let json = serde_json::to_string(&edge)?;

    // Then: Should be lowercase "blocking"
    assert!(
        json.contains("\"edge_type\":\"blocking\""),
        "Edge type should serialize to lowercase"
    );
    Ok(())
}

/// Given a GraphEdge with preferred dependency type,
/// When serializing the edge,
/// Then the edge_type should be "preferred"
#[test]
fn test_edge_type_serializes_to_lowercase_when_edge_is_preferred() -> TestResult {
    // Given: Edge with preferred type
    let edge = GraphEdge {
        source: "a".to_string(),
        target: "b".to_string(),
        edge_type: GraphEdgeType::Preferred,
    };

    // When: Serializing
    let json = serde_json::to_string(&edge)?;

    // Then: Should be lowercase "preferred"
    assert!(
        json.contains("\"edge_type\":\"preferred\""),
        "Edge type should serialize to lowercase"
    );
    Ok(())
}

/// Given a JSON with "blocking" edge type,
/// When deserializing to GraphEdge,
/// Then should create GraphEdgeType::Blocking
#[test]
fn test_edge_type_deserializes_from_lowercase_when_json_has_blocking() -> TestResult {
    // Given: JSON with lowercase edge type
    let json = r#"{
        "source": "from-node",
        "target": "to-node",
        "edge_type": "blocking"
    }"#;

    // When: Deserializing
    let edge: GraphEdge = serde_json::from_str(json)?;

    // Then: Should be Blocking variant
    assert_eq!(
        edge.edge_type,
        GraphEdgeType::Blocking,
        "Should deserialize to Blocking"
    );
    Ok(())
}

/// Given a JSON with "preferred" edge type,
/// When deserializing to GraphEdge,
/// Then should create GraphEdgeType::Preferred
#[test]
fn test_edge_type_deserializes_from_lowercase_when_json_has_preferred() -> TestResult {
    // Given: JSON with lowercase edge type
    let json = r#"{
        "source": "from-node",
        "target": "to-node",
        "edge_type": "preferred"
    }"#;

    // When: Deserializing
    let edge: GraphEdge = serde_json::from_str(json)?;

    // Then: Should be Preferred variant
    assert_eq!(
        edge.edge_type,
        GraphEdgeType::Preferred,
        "Should deserialize to Preferred"
    );
    Ok(())
}

/// Given a WorkflowGraphResponse,
/// When calculating size metrics,
/// Then should return correct node and edge counts
#[test]
fn test_calculates_graph_metrics_when_response_has_data() {
    // Given: Response with 3 nodes and 2 edges
    let nodes = (0..3)
        .map(|i| GraphNode {
            id: format!("node-{}", i),
            label: format!("Node {}", i),
            x: i as f64 * 100.0,
            y: 0.0,
        })
        .collect();

    let edges = vec![
        GraphEdge {
            source: "node-0".to_string(),
            target: "node-1".to_string(),
            edge_type: GraphEdgeType::Blocking,
        },
        GraphEdge {
            source: "node-1".to_string(),
            target: "node-2".to_string(),
            edge_type: GraphEdgeType::Preferred,
        },
    ];

    let response = WorkflowGraphResponse { nodes, edges };

    // When: Checking sizes
    let node_count = response.nodes.len();
    let edge_count = response.edges.len();

    // Then: Should match expected counts
    assert_eq!(node_count, 3, "Should have 3 nodes");
    assert_eq!(edge_count, 2, "Should have 2 edges");
}

/// Given GraphNodes with various labels,
/// When accessing the labels,
/// Then all labels should be preserved correctly
#[test]
fn test_preserves_node_labels_when_labels_have_special_characters() -> TestResult {
    // Given: Nodes with special characters in labels
    let nodes = vec![
        GraphNode {
            id: "1".to_string(),
            label: "Task: Fix Bug #123".to_string(),
            x: 0.0,
            y: 0.0,
        },
        GraphNode {
            id: "2".to_string(),
            label: "UTF-8: 你好世界".to_string(),
            x: 0.0,
            y: 0.0,
        },
    ];

    // When: Serializing and deserializing
    let json = serde_json::to_string(&nodes)?;
    let deserialized: Vec<GraphNode> = serde_json::from_str(&json)?;

    // Then: Labels should match
    assert_eq!(deserialized[0].label, "Task: Fix Bug #123");
    assert_eq!(deserialized[1].label, "UTF-8: 你好世界");
    Ok(())
}

/// Given edges with different types,
/// When filtering by edge type,
/// Then should correctly distinguish blocking from preferred
#[test]
fn test_distinguishes_edge_types_when_edges_are_mixed() {
    // Given: Mixed edge types
    let blocking_edge = GraphEdge {
        source: "a".to_string(),
        target: "b".to_string(),
        edge_type: GraphEdgeType::Blocking,
    };

    let preferred_edge = GraphEdge {
        source: "b".to_string(),
        target: "c".to_string(),
        edge_type: GraphEdgeType::Preferred,
    };

    // When: Checking types
    let is_blocking_1 = matches!(blocking_edge.edge_type, GraphEdgeType::Blocking);
    let is_preferred_1 = matches!(blocking_edge.edge_type, GraphEdgeType::Preferred);
    let is_blocking_2 = matches!(preferred_edge.edge_type, GraphEdgeType::Blocking);
    let is_preferred_2 = matches!(preferred_edge.edge_type, GraphEdgeType::Preferred);

    // Then: Types should match
    assert!(is_blocking_1, "First edge should be blocking");
    assert!(!is_preferred_1, "First edge should not be preferred");
    assert!(!is_blocking_2, "Second edge should not be blocking");
    assert!(is_preferred_2, "Second edge should be preferred");
}

/// Given negative coordinates,
/// When creating a GraphNode,
/// Then should accept negative values (positions can be negative)
#[test]
fn test_accepts_negative_coordinates_when_positions_are_below_origin() {
    // Given: Node with negative coordinates
    let node = GraphNode {
        id: "negative".to_string(),
        label: "Negative Position".to_string(),
        x: -100.5,
        y: -200.7,
    };

    // When: Checking coordinates
    // Then: Should preserve negative values
    assert!(node.x < 0.0, "X should be negative");
    assert!(node.y < 0.0, "Y should be negative");
}

/// Given a complex workflow graph,
/// When serializing to JSON,
/// Then should produce valid parseable JSON
#[test]
fn test_produces_valid_json_when_graph_is_complex() -> TestResult {
    // Given: Complex graph with many nodes and edges
    let nodes: Vec<GraphNode> = (0..10)
        .map(|i| GraphNode {
            id: format!("node-{}", i),
            label: format!("Node {}", i),
            x: (i % 3) as f64 * 150.0,
            y: (i / 3) as f64 * 150.0,
        })
        .collect();

    let edges: Vec<GraphEdge> = (0..9)
        .map(|i| GraphEdge {
            source: format!("node-{}", i),
            target: format!("node-{}", i + 1),
            edge_type: if i % 2 == 0 {
                GraphEdgeType::Blocking
            } else {
                GraphEdgeType::Preferred
            },
        })
        .collect();

    let response = WorkflowGraphResponse { nodes, edges };

    // When: Serializing
    let json = serde_json::to_string(&response)?;

    // When: Deserializing back
    let deserialized: WorkflowGraphResponse = serde_json::from_str(&json)?;

    // Then: Should round-trip successfully
    assert_eq!(deserialized.nodes.len(), 10, "Should preserve all nodes");
    assert_eq!(deserialized.edges.len(), 9, "Should preserve all edges");
    Ok(())
}
