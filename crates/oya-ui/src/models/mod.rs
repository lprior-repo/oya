//! Data models for graph visualization

pub mod colors;
pub mod edge;
pub mod node;
pub mod task;

use serde::{Deserialize, Serialize};

/// Represents a node in the dependency graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub color: Option<String>,
}

/// Represents an edge between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: Option<f64>,
}

/// Complete graph structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl Graph {
    /// Creates a new empty graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds a node to the graph
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    /// Adds an edge to the graph
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = Graph::new();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = Graph::new();
        let node = GraphNode {
            id: "node1".to_string(),
            label: "Node 1".to_string(),
            x: 0.0,
            y: 0.0,
            color: None,
        };
        graph.add_node(node);
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = Graph::new();
        let edge = GraphEdge {
            source: "node1".to_string(),
            target: "node2".to_string(),
            weight: Some(1.0),
        };
        graph.add_edge(edge);
        assert_eq!(graph.edges.len(), 1);
    }
}
