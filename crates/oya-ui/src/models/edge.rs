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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EdgeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// Visual state of the edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EdgeState {
    #[default]
    Normal,
    Highlighted,
    Dimmed,
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
    fn test_valid_edge_creation() -> Result<(), String> {
        let source = NodeId::new("node1")?;
        let target = NodeId::new("node2")?;
        let edge = Edge::new(source, target, EdgeType::Dependency)?;

        assert_eq!(edge.source().as_str(), "node1");
        assert_eq!(edge.target().as_str(), "node2");
        assert_eq!(edge.edge_type(), EdgeType::Dependency);
        assert_eq!(edge.style(), EdgeStyle::Solid);
        assert_eq!(edge.state(), EdgeState::Normal);
        Ok(())
    }

    #[test]
    fn test_self_referencing_edge_rejected() -> Result<(), String> {
        let node = NodeId::new("node1")?;
        let result = Edge::new(node.clone(), node, EdgeType::Dependency);

        assert!(result.is_err());
        let err = result.err().ok_or("Expected error but got Ok")?;
        assert_eq!(err, "Edge cannot reference itself");
        Ok(())
    }

    #[test]
    fn test_edge_type_variants() -> Result<(), String> {
        let source = NodeId::new("n1")?;
        let target = NodeId::new("n2")?;

        let dep_edge = Edge::new(source.clone(), target.clone(), EdgeType::Dependency)?;
        assert_eq!(dep_edge.edge_type(), EdgeType::Dependency);

        let flow_edge = Edge::new(source.clone(), target.clone(), EdgeType::DataFlow)?;
        assert_eq!(flow_edge.edge_type(), EdgeType::DataFlow);

        let trigger_edge = Edge::new(source, target, EdgeType::Trigger)?;
        assert_eq!(trigger_edge.edge_type(), EdgeType::Trigger);
        Ok(())
    }

    #[test]
    fn test_edge_style_variants() -> Result<(), String> {
        let source = NodeId::new("n1")?;
        let target = NodeId::new("n2")?;
        let mut edge = Edge::new(source, target, EdgeType::Dependency)?;

        // Default style
        assert_eq!(edge.style(), EdgeStyle::Solid);

        // Test all style variants
        edge.set_style(EdgeStyle::Dashed);
        assert_eq!(edge.style(), EdgeStyle::Dashed);

        edge.set_style(EdgeStyle::Dotted);
        assert_eq!(edge.style(), EdgeStyle::Dotted);

        edge.set_style(EdgeStyle::Solid);
        assert_eq!(edge.style(), EdgeStyle::Solid);
        Ok(())
    }

    #[test]
    fn test_edge_state_variants() -> Result<(), String> {
        let source = NodeId::new("n1")?;
        let target = NodeId::new("n2")?;
        let mut edge = Edge::new(source, target, EdgeType::Dependency)?;

        // Default state
        assert_eq!(edge.state(), EdgeState::Normal);

        // Test all state variants
        edge.set_state(EdgeState::Highlighted);
        assert_eq!(edge.state(), EdgeState::Highlighted);

        edge.set_state(EdgeState::Dimmed);
        assert_eq!(edge.state(), EdgeState::Dimmed);

        edge.set_state(EdgeState::Normal);
        assert_eq!(edge.state(), EdgeState::Normal);
        Ok(())
    }

    #[test]
    fn test_edge_serialization() -> Result<(), String> {
        let source = NodeId::new("source")?;
        let target = NodeId::new("target")?;
        let edge = Edge::new(source, target, EdgeType::DataFlow)?;

        let json = serde_json::to_string(&edge).map_err(|e| e.to_string())?;
        let deserialized: Edge = serde_json::from_str(&json).map_err(|e| e.to_string())?;

        assert_eq!(deserialized.source().as_str(), "source");
        assert_eq!(deserialized.target().as_str(), "target");
        assert_eq!(deserialized.edge_type(), EdgeType::DataFlow);
        Ok(())
    }

    #[test]
    fn test_edge_with_unicode_node_ids() -> Result<(), String> {
        let source = NodeId::new("节点-1")?;
        let target = NodeId::new("節點-2")?;
        let edge = Edge::new(source, target, EdgeType::Dependency)?;

        assert_eq!(edge.source().as_str(), "节点-1");
        assert_eq!(edge.target().as_str(), "節點-2");
        Ok(())
    }

    #[test]
    fn test_edge_type_serialization() -> Result<(), String> {
        let dep = EdgeType::Dependency;
        let json = serde_json::to_string(&dep).map_err(|e| e.to_string())?;
        let deserialized: EdgeType = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeType::Dependency);

        let flow = EdgeType::DataFlow;
        let json = serde_json::to_string(&flow).map_err(|e| e.to_string())?;
        let deserialized: EdgeType = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeType::DataFlow);

        let trigger = EdgeType::Trigger;
        let json = serde_json::to_string(&trigger).map_err(|e| e.to_string())?;
        let deserialized: EdgeType = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeType::Trigger);
        Ok(())
    }

    #[test]
    fn test_edge_style_serialization() -> Result<(), String> {
        let solid = EdgeStyle::Solid;
        let json = serde_json::to_string(&solid).map_err(|e| e.to_string())?;
        let deserialized: EdgeStyle = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeStyle::Solid);

        let dashed = EdgeStyle::Dashed;
        let json = serde_json::to_string(&dashed).map_err(|e| e.to_string())?;
        let deserialized: EdgeStyle = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeStyle::Dashed);

        let dotted = EdgeStyle::Dotted;
        let json = serde_json::to_string(&dotted).map_err(|e| e.to_string())?;
        let deserialized: EdgeStyle = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeStyle::Dotted);
        Ok(())
    }

    #[test]
    fn test_edge_state_serialization() -> Result<(), String> {
        let normal = EdgeState::Normal;
        let json = serde_json::to_string(&normal).map_err(|e| e.to_string())?;
        let deserialized: EdgeState = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeState::Normal);

        let highlighted = EdgeState::Highlighted;
        let json = serde_json::to_string(&highlighted).map_err(|e| e.to_string())?;
        let deserialized: EdgeState = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeState::Highlighted);

        let dimmed = EdgeState::Dimmed;
        let json = serde_json::to_string(&dimmed).map_err(|e| e.to_string())?;
        let deserialized: EdgeState = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized, EdgeState::Dimmed);
        Ok(())
    }

    #[test]
    fn test_edge_clone() -> Result<(), String> {
        let source = NodeId::new("n1")?;
        let target = NodeId::new("n2")?;
        let edge1 = Edge::new(source, target, EdgeType::Dependency)?;
        let edge2 = edge1.clone();

        assert_eq!(edge1, edge2);
        assert_eq!(edge1.source().as_str(), edge2.source().as_str());
        assert_eq!(edge1.target().as_str(), edge2.target().as_str());
        Ok(())
    }
}
