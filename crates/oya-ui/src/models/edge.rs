//! Edge data structure for graph visualization

use super::node::NodeId;
use serde::{Deserialize, Serialize};

/// Type of edge relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Blocks/BlockedBy relationship
    Dependency,
    /// Data passing between nodes
    DataFlow,
    /// Event triggering
    Trigger,
}

/// Visual style for edge rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeStyle {
    Solid,
    Dashed,
    Dotted,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self::Solid
    }
}

/// Visual state of the edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeState {
    Normal,
    Highlighted,
    Dimmed,
}

impl Default for EdgeState {
    fn default() -> Self {
        Self::Normal
    }
}

/// An edge in the dependency graph with type-safe node references
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    source: NodeId,
    target: NodeId,
    edge_type: EdgeType,
    style: EdgeStyle,
    state: EdgeState,
}

impl Edge {
    /// Creates a new Edge with validation
    ///
    /// # Errors
    /// Returns an error if the source and target are the same (self-referencing edge)
    pub fn new(source: NodeId, target: NodeId, edge_type: EdgeType) -> Result<Self, String> {
        // Validation: reject self-referencing edges
        if source.as_str() == target.as_str() {
            return Err("Edge cannot reference itself".to_string());
        }

        Ok(Self {
            source,
            target,
            edge_type,
            style: EdgeStyle::default(),
            state: EdgeState::default(),
        })
    }

    /// Returns the source node ID
    pub fn source(&self) -> &NodeId {
        &self.source
    }

    /// Returns the target node ID
    pub fn target(&self) -> &NodeId {
        &self.target
    }

    /// Returns the edge type
    pub fn edge_type(&self) -> EdgeType {
        self.edge_type
    }

    /// Returns the edge style
    pub fn style(&self) -> EdgeStyle {
        self.style
    }

    /// Returns the edge state
    pub fn state(&self) -> EdgeState {
        self.state
    }

    /// Sets the edge style
    pub fn set_style(&mut self, style: EdgeStyle) {
        self.style = style;
    }

    /// Sets the edge state
    pub fn set_state(&mut self, state: EdgeState) {
        self.state = state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_edge_creation() {
        let source = NodeId::new("node1").unwrap();
        let target = NodeId::new("node2").unwrap();
        let result = Edge::new(source, target, EdgeType::Dependency);

        assert!(result.is_ok());
        let edge = result.unwrap();
        assert_eq!(edge.source().as_str(), "node1");
        assert_eq!(edge.target().as_str(), "node2");
        assert_eq!(edge.edge_type(), EdgeType::Dependency);
        assert_eq!(edge.style(), EdgeStyle::Solid);
        assert_eq!(edge.state(), EdgeState::Normal);
    }

    #[test]
    fn test_self_referencing_edge_rejected() {
        let node = NodeId::new("node1").unwrap();
        let result = Edge::new(node.clone(), node, EdgeType::Dependency);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Edge cannot reference itself");
    }

    #[test]
    fn test_edge_type_variants() {
        let source = NodeId::new("n1").unwrap();
        let target = NodeId::new("n2").unwrap();

        let dep_edge = Edge::new(source.clone(), target.clone(), EdgeType::Dependency).unwrap();
        assert_eq!(dep_edge.edge_type(), EdgeType::Dependency);

        let flow_edge = Edge::new(source.clone(), target.clone(), EdgeType::DataFlow).unwrap();
        assert_eq!(flow_edge.edge_type(), EdgeType::DataFlow);

        let trigger_edge = Edge::new(source, target, EdgeType::Trigger).unwrap();
        assert_eq!(trigger_edge.edge_type(), EdgeType::Trigger);
    }

    #[test]
    fn test_edge_style_variants() {
        let source = NodeId::new("n1").unwrap();
        let target = NodeId::new("n2").unwrap();
        let mut edge = Edge::new(source, target, EdgeType::Dependency).unwrap();

        // Default style
        assert_eq!(edge.style(), EdgeStyle::Solid);

        // Test all style variants
        edge.set_style(EdgeStyle::Dashed);
        assert_eq!(edge.style(), EdgeStyle::Dashed);

        edge.set_style(EdgeStyle::Dotted);
        assert_eq!(edge.style(), EdgeStyle::Dotted);

        edge.set_style(EdgeStyle::Solid);
        assert_eq!(edge.style(), EdgeStyle::Solid);
    }

    #[test]
    fn test_edge_state_variants() {
        let source = NodeId::new("n1").unwrap();
        let target = NodeId::new("n2").unwrap();
        let mut edge = Edge::new(source, target, EdgeType::Dependency).unwrap();

        // Default state
        assert_eq!(edge.state(), EdgeState::Normal);

        // Test all state variants
        edge.set_state(EdgeState::Highlighted);
        assert_eq!(edge.state(), EdgeState::Highlighted);

        edge.set_state(EdgeState::Dimmed);
        assert_eq!(edge.state(), EdgeState::Dimmed);

        edge.set_state(EdgeState::Normal);
        assert_eq!(edge.state(), EdgeState::Normal);
    }

    #[test]
    fn test_edge_serialization() {
        let source = NodeId::new("source").unwrap();
        let target = NodeId::new("target").unwrap();
        let edge = Edge::new(source, target, EdgeType::DataFlow).unwrap();

        let json = serde_json::to_string(&edge);
        assert!(json.is_ok());

        let deserialized: Result<Edge, _> = serde_json::from_str(&json.unwrap());
        assert!(deserialized.is_ok());

        let deserialized_edge = deserialized.unwrap();
        assert_eq!(deserialized_edge.source().as_str(), "source");
        assert_eq!(deserialized_edge.target().as_str(), "target");
        assert_eq!(deserialized_edge.edge_type(), EdgeType::DataFlow);
    }

    #[test]
    fn test_edge_with_unicode_node_ids() {
        let source = NodeId::new("节点-1").unwrap();
        let target = NodeId::new("節點-2").unwrap();
        let result = Edge::new(source, target, EdgeType::Dependency);

        assert!(result.is_ok());
        let edge = result.unwrap();
        assert_eq!(edge.source().as_str(), "节点-1");
        assert_eq!(edge.target().as_str(), "節點-2");
    }

    #[test]
    fn test_edge_type_serialization() {
        let dep = EdgeType::Dependency;
        let json = serde_json::to_string(&dep).unwrap();
        let deserialized: EdgeType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeType::Dependency);

        let flow = EdgeType::DataFlow;
        let json = serde_json::to_string(&flow).unwrap();
        let deserialized: EdgeType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeType::DataFlow);

        let trigger = EdgeType::Trigger;
        let json = serde_json::to_string(&trigger).unwrap();
        let deserialized: EdgeType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeType::Trigger);
    }

    #[test]
    fn test_edge_style_serialization() {
        let solid = EdgeStyle::Solid;
        let json = serde_json::to_string(&solid).unwrap();
        let deserialized: EdgeStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeStyle::Solid);

        let dashed = EdgeStyle::Dashed;
        let json = serde_json::to_string(&dashed).unwrap();
        let deserialized: EdgeStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeStyle::Dashed);

        let dotted = EdgeStyle::Dotted;
        let json = serde_json::to_string(&dotted).unwrap();
        let deserialized: EdgeStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeStyle::Dotted);
    }

    #[test]
    fn test_edge_state_serialization() {
        let normal = EdgeState::Normal;
        let json = serde_json::to_string(&normal).unwrap();
        let deserialized: EdgeState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeState::Normal);

        let highlighted = EdgeState::Highlighted;
        let json = serde_json::to_string(&highlighted).unwrap();
        let deserialized: EdgeState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeState::Highlighted);

        let dimmed = EdgeState::Dimmed;
        let json = serde_json::to_string(&dimmed).unwrap();
        let deserialized: EdgeState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EdgeState::Dimmed);
    }

    #[test]
    fn test_edge_clone() {
        let source = NodeId::new("n1").unwrap();
        let target = NodeId::new("n2").unwrap();
        let edge1 = Edge::new(source, target, EdgeType::Dependency).unwrap();
        let edge2 = edge1.clone();

        assert_eq!(edge1, edge2);
        assert_eq!(edge1.source().as_str(), edge2.source().as_str());
        assert_eq!(edge1.target().as_str(), edge2.target().as_str());
    }
}
