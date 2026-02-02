//! Graph types for dependency visualization
//!
//! These types represent nodes and edges in the dependency graph,
//! used for rendering the graph visualization in the UI.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// Type-safe wrapper for node identifiers
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub struct NodeId(String);

impl NodeId {
    /// Creates a new `NodeId` with validation
    ///
    /// # Errors
    /// Returns an error if the ID is empty
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        if id.is_empty() {
            return Err("Node ID cannot be empty".to_string());
        }
        Ok(Self(id))
    }

    /// Returns the ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Position in 2D space
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    /// Creates a new Position with validation
    ///
    /// # Errors
    /// Returns an error if coordinates are not finite
    pub fn new(x: f32, y: f32) -> Result<Self, String> {
        if !x.is_finite() {
            return Err(format!("X coordinate must be finite, got: {x}"));
        }
        if !y.is_finite() {
            return Err(format!("Y coordinate must be finite, got: {y}"));
        }
        Ok(Self { x, y })
    }

    /// Creates a position at the origin
    #[must_use]
    pub const fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// State of a node in the graph
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub enum NodeState {
    #[default]
    Idle,
    Running,
    Blocked,
    Completed,
    Failed,
}

/// Shape for rendering the node
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub enum NodeShape {
    #[default]
    Circle,
    Square,
    Diamond,
}

/// A node in the dependency graph
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub struct Node {
    id: NodeId,
    label: String,
    position: Position,
    state: NodeState,
    shape: NodeShape,
}

impl Node {
    /// Creates a new Node with validation
    ///
    /// # Errors
    /// Returns an error if ID validation fails
    pub fn new(id: &str, label: &str) -> Result<Self, String> {
        let node_id = NodeId::new(id)?;
        Ok(Self {
            id: node_id,
            label: label.to_string(),
            position: Position::origin(),
            state: NodeState::default(),
            shape: NodeShape::default(),
        })
    }

    /// Creates a Node with position
    ///
    /// # Errors
    /// Returns an error if ID or position validation fails
    pub fn with_position(id: &str, label: &str, x: f32, y: f32) -> Result<Self, String> {
        let node_id = NodeId::new(id)?;
        let position = Position::new(x, y)?;
        Ok(Self {
            id: node_id,
            label: label.to_string(),
            position,
            state: NodeState::default(),
            shape: NodeShape::default(),
        })
    }

    /// Returns the node's ID
    #[must_use]
    pub const fn id(&self) -> &NodeId {
        &self.id
    }

    /// Returns the node's label
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the node's position
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the node's state
    #[must_use]
    pub const fn state(&self) -> NodeState {
        self.state
    }

    /// Returns the node's shape
    #[must_use]
    pub const fn shape(&self) -> NodeShape {
        self.shape
    }

    /// Sets the node's position
    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    /// Sets the node's state
    pub fn set_state(&mut self, state: NodeState) {
        self.state = state;
    }

    /// Sets the node's shape
    pub fn set_shape(&mut self, shape: NodeShape) {
        self.shape = shape;
    }
}

/// Type of edge relationship
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub enum EdgeType {
    /// Blocks/BlockedBy relationship
    Dependency,
    /// Data passing between nodes
    DataFlow,
    /// Event triggering
    Trigger,
}

/// Visual style for edge rendering
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub enum EdgeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// Visual state of the edge
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub enum EdgeState {
    #[default]
    Normal,
    Highlighted,
    Dimmed,
}

/// An edge in the dependency graph
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub struct Edge {
    source: NodeId,
    target: NodeId,
    edge_type: EdgeType,
    style: EdgeStyle,
    state: EdgeState,
    label: Option<String>,
}

impl Edge {
    /// Creates a new Edge with validation
    ///
    /// # Errors
    /// Returns an error if the source and target are the same (self-referencing edge)
    pub fn new(source: NodeId, target: NodeId, edge_type: EdgeType) -> Result<Self, String> {
        if source.as_str() == target.as_str() {
            return Err("Edge cannot reference itself".to_string());
        }

        Ok(Self {
            source,
            target,
            edge_type,
            style: EdgeStyle::default(),
            state: EdgeState::default(),
            label: None,
        })
    }

    /// Creates a new Edge with an optional label
    ///
    /// # Errors
    /// Returns an error if the source and target are the same
    pub fn with_label(
        source: NodeId,
        target: NodeId,
        edge_type: EdgeType,
        label: Option<String>,
    ) -> Result<Self, String> {
        if source.as_str() == target.as_str() {
            return Err("Edge cannot reference itself".to_string());
        }

        Ok(Self {
            source,
            target,
            edge_type,
            style: EdgeStyle::default(),
            state: EdgeState::default(),
            label,
        })
    }

    /// Returns the source node ID
    #[must_use]
    pub const fn source(&self) -> &NodeId {
        &self.source
    }

    /// Returns the target node ID
    #[must_use]
    pub const fn target(&self) -> &NodeId {
        &self.target
    }

    /// Returns the edge type
    #[must_use]
    pub const fn edge_type(&self) -> EdgeType {
        self.edge_type
    }

    /// Returns the edge style
    #[must_use]
    pub const fn style(&self) -> EdgeStyle {
        self.style
    }

    /// Returns the edge state
    #[must_use]
    pub const fn state(&self) -> EdgeState {
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

    /// Returns the edge label (if any)
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Sets the edge label
    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }
}

/// A complete dependency graph
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    /// Creates a new empty graph
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node to the graph
    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Adds an edge to the graph
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    /// Returns all nodes
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Returns all edges
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Returns node count
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns edge count
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Finds a node by ID
    #[must_use]
    pub fn find_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id().as_str() == id)
    }

    /// Finds a mutable node by ID
    #[must_use]
    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id().as_str() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() -> Result<(), String> {
        let node = Node::new("node1", "Test Node")?;
        assert_eq!(node.id().as_str(), "node1");
        assert_eq!(node.label(), "Test Node");
        assert_eq!(node.state(), NodeState::Idle);
        Ok(())
    }

    #[test]
    fn test_node_empty_id_rejected() {
        let result = Node::new("", "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_edge_creation() -> Result<(), String> {
        let source = NodeId::new("n1")?;
        let target = NodeId::new("n2")?;
        let edge = Edge::new(source, target, EdgeType::Dependency)?;

        assert_eq!(edge.source().as_str(), "n1");
        assert_eq!(edge.target().as_str(), "n2");
        assert_eq!(edge.edge_type(), EdgeType::Dependency);
        Ok(())
    }

    #[test]
    fn test_self_referencing_edge_rejected() -> Result<(), String> {
        let node = NodeId::new("node1")?;
        let result = Edge::new(node.clone(), node, EdgeType::Dependency);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_graph_operations() -> Result<(), String> {
        let mut graph = Graph::new();

        let node1 = Node::new("n1", "Node 1")?;
        let node2 = Node::new("n2", "Node 2")?;
        graph.add_node(node1);
        graph.add_node(node2);

        let edge = Edge::new(NodeId::new("n1")?, NodeId::new("n2")?, EdgeType::Dependency)?;
        graph.add_edge(edge);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        let found = graph.find_node("n1");
        assert!(found.is_some());
        assert_eq!(found.map(|n| n.label()), Some("Node 1"));
        Ok(())
    }

    #[test]
    fn test_rkyv_graph() -> Result<(), String> {
        let mut graph = Graph::new();
        graph.add_node(Node::new("n1", "Test")?);

        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&graph);
        assert!(bytes.is_ok());
        Ok(())
    }
}
