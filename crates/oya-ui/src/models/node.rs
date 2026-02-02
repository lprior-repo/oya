//! Node data structure with type-safe validation

use serde::{Deserialize, Serialize};

/// Type-safe wrapper for node identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Creates a new NodeId with validation
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        if id.is_empty() {
            return Err("Node ID cannot be empty".to_string());
        }
        Ok(Self(id))
    }

    /// Returns the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Position in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    /// Creates a new Position with validation
    pub fn new(x: f32, y: f32) -> Result<Self, String> {
        if !x.is_finite() {
            return Err(format!("X coordinate must be finite, got: {}", x));
        }
        if !y.is_finite() {
            return Err(format!("Y coordinate must be finite, got: {}", y));
        }
        Ok(Self { x, y })
    }

    /// Creates a position at the origin
    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// State of a node in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NodeState {
    #[default]
    Idle,
    Running,
    Blocked,
    Completed,
    Failed,
}

/// Shape for rendering the node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NodeShape {
    #[default]
    Circle,
    Square,
    Diamond,
}

/// A node in the dependency graph with type-safe validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    id: NodeId,
    label: String,
    position: Position,
    state: NodeState,
    shape: NodeShape,
}

impl Node {
    /// Creates a new Node with validation
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

    /// Creates a Node with all fields
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
    pub fn id(&self) -> &NodeId {
        &self.id
    }

    /// Returns the node's label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the node's position
    pub fn position(&self) -> Position {
        self.position
    }

    /// Returns the node's state
    pub fn state(&self) -> NodeState {
        self.state
    }

    /// Returns the node's shape
    pub fn shape(&self) -> NodeShape {
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

    /// Tests if a screen-space point is inside this node
    ///
    /// This is a convenience method that delegates to the interaction::hover module.
    /// It handles viewport transforms (pan/zoom) correctly.
    ///
    /// # Arguments
    ///
    /// * `screen_x` - X coordinate in screen space (pixels)
    /// * `screen_y` - Y coordinate in screen space (pixels)
    /// * `viewport` - Current viewport state
    ///
    /// # Errors
    ///
    /// Returns an error if coordinate transformation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use oya_ui::models::node::Node;
    /// # use oya_ui::components::canvas::coords::Viewport;
    /// # fn example() -> Result<(), String> {
    /// let node = Node::new("test", "Test")?;
    /// let viewport = Viewport::new(1200.0, 800.0)?;
    ///
    /// let is_hovering = node.contains_point(650.0, 425.0, &viewport)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn contains_point(
        &self,
        screen_x: f32,
        screen_y: f32,
        viewport: &crate::components::canvas::coords::Viewport,
    ) -> Result<bool, String> {
        crate::interaction::hover::contains_point(self, screen_x, screen_y, viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_node_creation() -> Result<(), String> {
        let node = Node::new("node1", "Test Node")?;
        assert_eq!(node.id().as_str(), "node1");
        assert_eq!(node.label(), "Test Node");
        assert_eq!(node.position(), Position::origin());
        assert_eq!(node.state(), NodeState::Idle);
        assert_eq!(node.shape(), NodeShape::Circle);
        Ok(())
    }

    #[test]
    fn test_invalid_node_id_empty() {
        let result = Node::new("", "Test Node");
        assert!(result.is_err());
        if let Err(msg) = result {
            assert_eq!(msg, "Node ID cannot be empty");
        }
    }

    #[test]
    fn test_node_with_position() -> Result<(), String> {
        let node = Node::with_position("node2", "Node 2", 10.5, 20.5)?;
        assert_eq!(node.position().x, 10.5);
        assert_eq!(node.position().y, 20.5);
        Ok(())
    }

    #[test]
    fn test_position_validation_nan() {
        let result = Position::new(f32::NAN, 0.0);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("must be finite"));
        }
    }

    #[test]
    fn test_position_validation_infinity() {
        let result = Position::new(f32::INFINITY, 0.0);
        assert!(result.is_err());

        let result = Position::new(0.0, f32::NEG_INFINITY);
        assert!(result.is_err());
    }

    #[test]
    fn test_position_validation_negative() {
        // Negative values are valid, just not infinite
        let result = Position::new(-10.0, -20.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_node_state_transitions() -> Result<(), String> {
        let mut node = Node::new("node3", "Node 3")?;

        assert_eq!(node.state(), NodeState::Idle);

        node.set_state(NodeState::Running);
        assert_eq!(node.state(), NodeState::Running);

        node.set_state(NodeState::Completed);
        assert_eq!(node.state(), NodeState::Completed);

        node.set_state(NodeState::Failed);
        assert_eq!(node.state(), NodeState::Failed);

        node.set_state(NodeState::Blocked);
        assert_eq!(node.state(), NodeState::Blocked);
        Ok(())
    }

    #[test]
    fn test_node_shape_variants() -> Result<(), String> {
        let mut node = Node::new("node4", "Node 4")?;

        assert_eq!(node.shape(), NodeShape::Circle);

        node.set_shape(NodeShape::Square);
        assert_eq!(node.shape(), NodeShape::Square);

        node.set_shape(NodeShape::Diamond);
        assert_eq!(node.shape(), NodeShape::Diamond);
        Ok(())
    }

    #[test]
    fn test_node_serialization() -> Result<(), String> {
        let node = Node::new("node5", "Node 5")?;

        let json = serde_json::to_string(&node).map_err(|e| e.to_string())?;
        let deserialized: Node = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(deserialized.id().as_str(), "node5");
        assert_eq!(deserialized.label(), "Node 5");
        Ok(())
    }

    #[test]
    fn test_nodeid_unicode() -> Result<(), String> {
        // Test Unicode support
        let node_id = NodeId::new("节点-1")?;
        assert_eq!(node_id.as_str(), "节点-1");
        Ok(())
    }

    #[test]
    fn test_nodeid_very_long() -> Result<(), String> {
        // Test very long IDs
        let long_id = "a".repeat(10000);
        let node_id = NodeId::new(long_id.clone())?;
        assert_eq!(node_id.as_str(), &long_id);
        Ok(())
    }

    #[test]
    fn test_position_origin() {
        let origin = Position::origin();
        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
    }

    #[test]
    fn test_node_position_update() -> Result<(), String> {
        let mut node = Node::new("node6", "Node 6")?;

        let new_pos = Position::new(42.0, 43.0)?;
        node.set_position(new_pos);

        assert_eq!(node.position().x, 42.0);
        assert_eq!(node.position().y, 43.0);
        Ok(())
    }
}
