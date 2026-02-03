#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

//! Node data structures for DAG visualization.
//!
//! Provides type-safe node representations with state tracking,
//! geometry support for hit testing, and hover state management.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for a node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Creates a new NodeId from a string.
    ///
    /// # Errors
    /// Returns an error if the input string is empty.
    pub fn new(id: String) -> Result<Self, NodeError> {
        if id.is_empty() {
            Err(NodeError::InvalidId("Node ID cannot be empty".to_string()))
        } else {
            Ok(Self(id))
        }
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Represents the current state of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is pending execution.
    Pending,
    /// Node has been scheduled for execution.
    Scheduled,
    /// Node is currently running.
    Running,
    /// Node has completed successfully.
    Completed,
    /// Node has failed.
    Failed,
    /// Node execution was cancelled.
    Cancelled,
}

/// Geometric shape for rendering a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeShape {
    /// Circular shape.
    Circle,
    /// Rectangular shape.
    Rectangle,
}

/// 2D point in canvas coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Creates a new point from coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculates squared distance to another point.
    ///
    /// Returns the squared Euclidean distance, which is more
    /// efficient than `distance` when only comparison is needed.
    pub fn distance_squared(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Calculates distance to another point.
    pub fn distance(&self, other: &Self) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

/// Dimensions for rectangular nodes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RectSize {
    pub width: f64,
    pub height: f64,
}

impl RectSize {
    /// Creates a new rectangle size.
    ///
    /// # Errors
    /// Returns an error if either dimension is non-positive.
    pub fn new(width: f64, height: f64) -> Result<Self, NodeError> {
        if width <= 0.0 || height <= 0.0 {
            Err(NodeError::InvalidDimensions(
                "Width and height must be positive".to_string(),
            ))
        } else {
            Ok(Self { width, height })
        }
    }
}

/// Geometric size for a node (circle radius or rect dimensions).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeSize {
    /// Circle radius.
    Circle(f64),
    /// Rectangle dimensions.
    Rectangle(RectSize),
}

impl NodeSize {
    /// Creates a new circle size.
    ///
    /// # Errors
    /// Returns an error if radius is non-positive.
    pub fn circle(radius: f64) -> Result<Self, NodeError> {
        if radius <= 0.0 {
            Err(NodeError::InvalidDimensions(
                "Radius must be positive".to_string(),
            ))
        } else {
            Ok(Self::Circle(radius))
        }
    }

    /// Creates a new rectangle size.
    pub fn rectangle(width: f64, height: f64) -> Result<Self, NodeError> {
        RectSize::new(width, height).map(Self::Rectangle)
    }
}

/// A DAG node with state, geometry, and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier.
    pub id: NodeId,
    /// Display label.
    pub label: String,
    /// Current execution state.
    pub state: NodeState,
    /// Geometric shape for rendering.
    pub shape: NodeShape,
    /// Position in canvas coordinates.
    pub position: Point,
    /// Size (radius for circle, width/height for rectangle).
    pub size: NodeSize,
    /// List of parent node IDs (dependencies).
    pub dependencies: Vec<String>,
    /// Whether the node is currently hovered.
    pub hovered: bool,
}

impl Node {
    /// Creates a new node.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Node ID is invalid
    /// - Label is empty
    /// - Size is invalid
    pub fn new(
        id: String,
        label: String,
        state: NodeState,
        shape: NodeShape,
        position: Point,
        size: NodeSize,
        dependencies: Vec<String>,
    ) -> Result<Self, NodeError> {
        NodeId::new(id.clone())
            .map_err(|_| NodeError::InvalidId(format!("Invalid node ID: {}", id)))?;

        if label.trim().is_empty() {
            return Err(NodeError::InvalidLabel("Label cannot be empty".to_string()));
        }

        Ok(Self {
            id: NodeId::new(id)
                .map_err(|e| NodeError::InvalidId(format!("Failed to create NodeId: {}", e)))?,
            label,
            state,
            shape,
            position,
            size,
            dependencies,
            hovered: false,
        })
    }

    /// Checks if a point is within the node's bounds.
    ///
    /// Performs hit testing based on the node's shape:
    /// - Circle: distance from center <= radius
    /// - Rectangle: point within axis-aligned bounding box
    ///
    /// # Arguments
    /// * `x` - Canvas x coordinate
    /// * `y` - Canvas y coordinate
    ///
    /// # Returns
    /// `true` if the point is within the node, `false` otherwise.
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        let point = Point::new(x, y);
        match &self.shape {
            NodeShape::Circle => self.contains_circle(&point),
            NodeShape::Rectangle => self.contains_rectangle(&point),
        }
    }

    /// Checks if a point is within a circular node.
    ///
    /// Uses squared distance comparison to avoid sqrt overhead.
    fn contains_circle(&self, point: &Point) -> bool {
        match self.size {
            NodeSize::Circle(radius) => self.position.distance_squared(point) <= radius * radius,
            NodeSize::Rectangle(_) => false,
        }
    }

    /// Checks if a point is within a rectangular node.
    ///
    /// Uses AABB (Axis-Aligned Bounding Box) test.
    fn contains_rectangle(&self, point: &Point) -> bool {
        match self.size {
            NodeSize::Rectangle(dimensions) => {
                let half_w = dimensions.width / 2.0;
                let half_h = dimensions.height / 2.0;

                point.x >= self.position.x - half_w
                    && point.x <= self.position.x + half_w
                    && point.y >= self.position.y - half_h
                    && point.y <= self.position.y + half_h
            }
            NodeSize::Circle(_) => false,
        }
    }

    /// Creates a builder pattern for constructing nodes.
    pub fn builder(id: String, label: String) -> NodeBuilder {
        NodeBuilder::new(id, label)
    }
}

/// Builder for creating Node instances.
#[derive(Debug, Clone)]
pub struct NodeBuilder {
    id: String,
    label: String,
    state: NodeState,
    shape: NodeShape,
    position: Point,
    size: Option<NodeSize>,
    dependencies: Vec<String>,
}

impl NodeBuilder {
    /// Creates a new builder with required fields.
    pub fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            state: NodeState::Pending,
            shape: NodeShape::Circle,
            position: Point::new(0.0, 0.0),
            size: None,
            dependencies: Vec::new(),
        }
    }

    /// Sets the node state.
    pub fn state(mut self, state: NodeState) -> Self {
        self.state = state;
        self
    }

    /// Sets the node shape.
    pub fn shape(mut self, shape: NodeShape) -> Self {
        self.shape = shape;
        self
    }

    /// Sets the node position.
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.position = Point::new(x, y);
        self
    }

    /// Sets the node size (circle radius).
    pub fn circle_radius(mut self, radius: f64) -> Self {
        self.size = Some(NodeSize::circle(radius).expect("Invalid radius"));
        self
    }

    /// Sets the node size (rectangle dimensions).
    pub fn rectangle_size(mut self, width: f64, height: f64) -> Self {
        self.size = Some(NodeSize::rectangle(width, height).expect("Invalid dimensions"));
        self
    }

    /// Adds a dependency node ID.
    pub fn add_dependency(mut self, dep_id: String) -> Self {
        self.dependencies.push(dep_id);
        self
    }

    /// Sets all dependencies at once.
    pub fn dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Builds the Node.
    ///
    /// # Errors
    /// Returns an error if required fields are invalid.
    pub fn build(self) -> Result<Node, NodeError> {
        let size = self
            .size
            .ok_or_else(|| NodeError::InvalidDimensions("Size must be specified".to_string()))?;

        Node::new(
            self.id,
            self.label,
            self.state,
            self.shape,
            self.position,
            size,
            self.dependencies,
        )
    }
}

/// Node-related errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum NodeError {
    #[error("invalid node ID: {0}")]
    InvalidId(String),

    #[error("invalid label: {0}")]
    InvalidLabel(String),

    #[error("invalid dimensions: {0}")]
    InvalidDimensions(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_new_valid() {
        assert!(NodeId::new("test-123".to_string()).is_ok());
    }

    #[test]
    fn test_node_id_new_empty() {
        assert_eq!(
            NodeId::new(String::new()),
            Err(NodeError::InvalidId("Node ID cannot be empty".to_string()))
        );
    }

    #[test]
    fn test_point_distance_squared() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance_squared(&p2), 25.0);
    }

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance(&p2), 5.0);
    }

    #[test]
    fn test_rect_size_new_valid() {
        assert!(RectSize::new(10.0, 20.0).is_ok());
    }

    #[test]
    fn test_rect_size_new_negative() {
        assert!(RectSize::new(-1.0, 20.0).is_err());
        assert!(RectSize::new(10.0, -1.0).is_err());
    }

    #[test]
    fn test_node_size_circle_valid() {
        assert!(NodeSize::circle(10.0).is_ok());
    }

    #[test]
    fn test_node_size_circle_zero() {
        assert!(NodeSize::circle(0.0).is_err());
    }

    #[test]
    fn test_node_size_rectangle_valid() {
        assert!(NodeSize::rectangle(10.0, 20.0).is_ok());
    }

    #[test]
    fn test_node_new_empty_label() {
        let result = Node::new(
            "test".to_string(),
            String::new(),
            NodeState::Pending,
            NodeShape::Circle,
            Point::new(0.0, 0.0),
            NodeSize::circle(10.0).unwrap(),
            Vec::new(),
        );
        assert!(matches!(result, Err(NodeError::InvalidLabel(_))));
    }

    #[test]
    fn test_contains_circle_center() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .circle_radius(50.0)
            .build()
            .unwrap();

        assert!(node.contains_point(100.0, 100.0));
    }

    #[test]
    fn test_contains_circle_inside() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .circle_radius(50.0)
            .build()
            .unwrap();

        assert!(node.contains_point(125.0, 100.0));
        assert!(node.contains_point(100.0, 125.0));
    }

    #[test]
    fn test_contains_circle_edge() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .circle_radius(50.0)
            .build()
            .unwrap();

        assert!(node.contains_point(150.0, 100.0));
        assert!(node.contains_point(100.0, 150.0));
    }

    #[test]
    fn test_contains_circle_outside() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .circle_radius(50.0)
            .build()
            .unwrap();

        assert!(!node.contains_point(151.0, 100.0));
        assert!(!node.contains_point(100.0, 151.0));
    }

    #[test]
    fn test_contains_rectangle_center() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .shape(NodeShape::Rectangle)
            .rectangle_size(100.0, 60.0)
            .build()
            .unwrap();

        assert!(node.contains_point(100.0, 100.0));
    }

    #[test]
    fn test_contains_rectangle_inside() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .shape(NodeShape::Rectangle)
            .rectangle_size(100.0, 60.0)
            .build()
            .unwrap();

        assert!(node.contains_point(125.0, 100.0));
        assert!(node.contains_point(100.0, 120.0));
    }

    #[test]
    fn test_contains_rectangle_edge() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .shape(NodeShape::Rectangle)
            .rectangle_size(100.0, 60.0)
            .build()
            .unwrap();

        assert!(node.contains_point(150.0, 100.0));
        assert!(node.contains_point(100.0, 130.0));
    }

    #[test]
    fn test_contains_rectangle_outside() {
        let node = Node::builder("test".to_string(), "Test".to_string())
            .position(100.0, 100.0)
            .shape(NodeShape::Rectangle)
            .rectangle_size(100.0, 60.0)
            .build()
            .unwrap();

        assert!(!node.contains_point(151.0, 100.0));
        assert!(!node.contains_point(100.0, 131.0));
    }

    #[test]
    fn test_builder_pattern() {
        let node = Node::builder("test-id".to_string(), "Test Node".to_string())
            .state(NodeState::Completed)
            .position(50.0, 75.0)
            .circle_radius(25.0)
            .add_dependency("dep-1".to_string())
            .add_dependency("dep-2".to_string())
            .build()
            .unwrap();

        assert_eq!(node.id.as_str(), "test-id");
        assert_eq!(node.label, "Test Node");
        assert_eq!(node.state, NodeState::Completed);
        assert_eq!(node.position.x, 50.0);
        assert_eq!(node.position.y, 75.0);
        assert_eq!(node.dependencies.len(), 2);
    }
}
