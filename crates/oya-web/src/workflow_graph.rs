//! Workflow graph response types for DAG visualization.
//!
//! Provides serializable types for representing workflow graphs with:
//! - Nodes (beads/tasks) with positions for layout
//! - Edges (dependencies) with type information
//! - JSON serialization/deserialization for frontend consumption

#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Dependency relationship types between workflow nodes.
///
/// Represents whether an edge is a hard requirement or a soft preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphEdgeType {
    /// Blocking dependency - source must complete before target can start.
    ///
    /// This is a hard requirement in the workflow DAG.
    Blocking,
    /// Preferred order - influences scheduling but doesn't block.
    ///
    /// This is a soft dependency that may be used for optimization
    /// but is not required for correctness.
    Preferred,
}

/// A graph edge representing a dependency between two nodes.
///
/// Contains source and target node identifiers along with the dependency type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node identifier (the dependency that must complete first).
    pub source: String,
    /// Target node identifier (the dependent that waits on source).
    pub target: String,
    /// Type of dependency relationship.
    #[serde(rename = "edge_type")]
    pub edge_type: GraphEdgeType,
}

/// A graph node representing a workflow task/bead.
///
/// Contains identifier, display label, and position coordinates for visualization.
/// Note: Does not derive Eq because f64 (x, y coordinates) doesn't implement Eq.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier for the node (typically a bead ID).
    pub id: String,
    /// Human-readable label for display in the UI.
    pub label: String,
    /// X coordinate for graph layout/positioning.
    ///
    /// Can be negative for nodes positioned left of the origin.
    pub x: f64,
    /// Y coordinate for graph layout/positioning.
    ///
    /// Can be negative for nodes positioned above the origin.
    pub y: f64,
}

/// Complete workflow graph response for DAG visualization.
///
/// Contains all nodes and edges needed to render a workflow DAG in the frontend.
/// This structure serializes to JSON with lowercase field names for consistency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraphResponse {
    /// All nodes in the workflow graph.
    pub nodes: Vec<GraphNode>,
    /// All edges (dependencies) in the workflow graph.
    pub edges: Vec<GraphEdge>,
}

// Legacy type alias for compatibility with orchestrator crate
/// Dependency type for backward compatibility.
///
/// # Deprecated
/// Use `GraphEdgeType` instead.
pub type DependencyType = GraphEdgeType;

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_graph_edge_type_blocking_serializes_to_lowercase() -> TestResult {
        let edge_type = GraphEdgeType::Blocking;
        let json = serde_json::to_string(&edge_type)?;
        assert_eq!(json, "\"blocking\"");
        Ok(())
    }

    #[test]
    fn test_graph_edge_type_preferred_serializes_to_lowercase() -> TestResult {
        let edge_type = GraphEdgeType::Preferred;
        let json = serde_json::to_string(&edge_type)?;
        assert_eq!(json, "\"preferred\"");
        Ok(())
    }

    #[test]
    fn test_graph_edge_type_deserializes_from_lowercase() -> TestResult {
        let blocking: GraphEdgeType = serde_json::from_str("\"blocking\"")?;
        assert_eq!(blocking, GraphEdgeType::Blocking);

        let preferred: GraphEdgeType = serde_json::from_str("\"preferred\"")?;
        assert_eq!(preferred, GraphEdgeType::Preferred);
        Ok(())
    }

    #[test]
    fn test_graph_edge_serialization() -> TestResult {
        let edge = GraphEdge {
            source: "node-a".to_string(),
            target: "node-b".to_string(),
            edge_type: GraphEdgeType::Blocking,
        };

        let json = serde_json::to_string(&edge)?;

        assert!(json.contains("\"source\":\"node-a\""));
        assert!(json.contains("\"target\":\"node-b\""));
        assert!(json.contains("\"edge_type\":\"blocking\""));
        Ok(())
    }

    #[test]
    fn test_graph_edge_deserialization() -> TestResult {
        let json = r#"{
            "source": "from",
            "target": "to",
            "edge_type": "preferred"
        }"#;

        let edge: GraphEdge = serde_json::from_str(json)?;

        assert_eq!(edge.source, "from");
        assert_eq!(edge.target, "to");
        assert_eq!(edge.edge_type, GraphEdgeType::Preferred);
        Ok(())
    }

    #[test]
    fn test_graph_node_with_negative_coordinates() {
        let node = GraphNode {
            id: "test".to_string(),
            label: "Test".to_string(),
            x: -100.5,
            y: -200.7,
        };

        assert!(node.x < 0.0);
        assert!(node.y < 0.0);
    }

    #[test]
    fn test_workflow_graph_response_empty() -> TestResult {
        let response = WorkflowGraphResponse {
            nodes: vec![],
            edges: vec![],
        };

        let json = serde_json::to_string(&response)?;

        assert!(json.contains("\"nodes\":[]"));
        assert!(json.contains("\"edges\":[]"));
        Ok(())
    }
}
