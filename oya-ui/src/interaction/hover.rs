#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

//! Hover state detection for interactive DAG visualization.
//!
//! Provides efficient hit testing, viewport transform handling,
//! and reactive hover state management for graph nodes.

use crate::models::node::{Node, Point};
use std::collections::HashMap;
use thiserror::Error;

/// Viewport transformation (pan and zoom) for canvas coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportTransform {
    /// Horizontal offset (pan).
    pub offset_x: f64,
    /// Vertical offset (pan).
    pub offset_y: f64,
    /// Zoom scale factor.
    pub scale: f64,
}

impl ViewportTransform {
    /// Creates a new viewport transform with default values.
    pub fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            scale: 1.0,
        }
    }

    /// Sets the pan offset.
    pub fn with_offset(self, x: f64, y: f64) -> Self {
        Self {
            offset_x: x,
            offset_y: y,
            ..self
        }
    }

    /// Sets the zoom scale.
    ///
    /// # Errors
    /// Returns an error if scale is non-positive.
    pub fn with_scale(self, scale: f64) -> Result<Self, HoverError> {
        if scale <= 0.0 {
            Err(HoverError::InvalidScale(
                "Scale must be positive".to_string(),
            ))
        } else {
            Ok(Self { scale, ..self })
        }
    }

    /// Transforms screen coordinates to world coordinates.
    ///
    /// # Arguments
    /// * `screen_x` - Screen x coordinate
    /// * `screen_y` - Screen y coordinate
    ///
    /// # Returns
    /// World coordinates after applying inverse transform.
    pub fn screen_to_world(&self, screen_x: f64, screen_y: f64) -> Point {
        Point::new(
            (screen_x - self.offset_x) / self.scale,
            (screen_y - self.offset_y) / self.scale,
        )
    }

    /// Transforms world coordinates to screen coordinates.
    ///
    /// # Arguments
    /// * `world_x` - World x coordinate
    /// * `world_y` - World y coordinate
    ///
    /// # Returns
    /// Screen coordinates after applying transform.
    pub fn world_to_screen(&self, world_x: f64, world_y: f64) -> Point {
        Point::new(
            world_x * self.scale + self.offset_x,
            world_y * self.scale + self.offset_y,
        )
    }
}

impl Default for ViewportTransform {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a hit test operation.
#[derive(Debug, Clone, PartialEq)]
pub struct HitTestResult {
    /// The ID of the hit node, if any.
    pub node_id: Option<String>,
    /// Whether the cursor changed.
    pub cursor_changed: bool,
}

impl HitTestResult {
    /// Creates a new hit test result.
    pub fn new(node_id: Option<String>, cursor_changed: bool) -> Self {
        Self {
            node_id,
            cursor_changed,
        }
    }

    /// Creates a result with no hit and no cursor change.
    pub fn none() -> Self {
        Self::new(None, false)
    }

    /// Creates a result with a hit and cursor change.
    pub fn hit(node_id: String) -> Self {
        Self::new(Some(node_id), true)
    }
}

/// Hover state manager for interactive nodes.
///
/// Handles hit testing, hover state updates, and cursor management
/// with viewport transform support.
#[derive(Debug, Clone)]
pub struct HoverManager {
    /// Map of node IDs to node instances.
    nodes: HashMap<String, Node>,
    /// Current viewport transform.
    viewport: ViewportTransform,
    /// Currently hovered node ID, if any.
    hovered_node_id: Option<String>,
}

impl HoverManager {
    /// Creates a new hover manager.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            viewport: ViewportTransform::new(),
            hovered_node_id: None,
        }
    }

    /// Sets the viewport transform.
    pub fn set_viewport(&mut self, viewport: ViewportTransform) {
        self.viewport = viewport;
    }

    /// Gets the current viewport transform.
    pub fn viewport(&self) -> ViewportTransform {
        self.viewport
    }

    /// Adds or updates a node.
    pub fn set_node(&mut self, node: Node) {
        let id = node.id.as_str().to_string();
        self.nodes.insert(id.clone(), node);
    }

    /// Adds multiple nodes.
    pub fn set_nodes(&mut self, nodes: Vec<Node>) {
        for node in nodes {
            self.set_node(node);
        }
    }

    /// Removes a node.
    pub fn remove_node(&mut self, node_id: &str) -> Option<Node> {
        self.nodes.remove(node_id)
    }

    /// Gets a node by ID.
    pub fn get_node(&self, node_id: &str) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    /// Gets the currently hovered node.
    pub fn hovered_node(&self) -> Option<&Node> {
        self.hovered_node_id
            .as_ref()
            .and_then(|id| self.nodes.get(id))
    }

    /// Performs hit testing at screen coordinates.
    ///
    /// Transforms screen coordinates to world coordinates and checks
    /// each node in reverse order (top-most first for overlapping nodes).
    ///
    /// # Arguments
    /// * `screen_x` - Screen x coordinate
    /// * `screen_y` - Screen y coordinate
    ///
    /// # Returns
    /// Hit test result containing the hit node ID and cursor change flag.
    pub fn hit_test(&self, screen_x: f64, screen_y: f64) -> HitTestResult {
        // Transform screen to world coordinates
        let world_point = self.viewport.screen_to_world(screen_x, screen_y);

        // Check nodes in reverse order (top-most first)
        let node_ids: Vec<_> = self.nodes.keys().cloned().collect();
        let hit_id = node_ids.into_iter().rev().find(|id| {
            self.nodes
                .get(id)
                .map(|node| node.contains_point(world_point.x, world_point.y))
                .unwrap_or(false)
        });

        // Determine if cursor should change
        let cursor_changed = hit_id != self.hovered_node_id;

        HitTestResult::new(hit_id, cursor_changed)
    }

    /// Updates hover state based on screen coordinates.
    ///
    /// Performs hit testing and updates the hovered state of all nodes.
    /// Only the hovered node's `hovered` field is set to true; all others
    /// are set to false.
    ///
    /// # Arguments
    /// * `screen_x` - Screen x coordinate
    /// * `screen_y` - Screen y coordinate
    ///
    /// # Returns
    /// Hit test result with cursor change flag.
    pub fn update_hover(&mut self, screen_x: f64, screen_y: f64) -> HitTestResult {
        let hit_test_result = self.hit_test(screen_x, screen_y);

        // Update hovered state for all nodes
        let new_hovered_id = hit_test_result.node_id.clone();

        for (id, node) in &mut self.nodes {
            let is_hovered = new_hovered_id
                .as_ref()
                .map(|new_id| id == new_id)
                .unwrap_or(false);
            node.hovered = is_hovered;
        }

        self.hovered_node_id = new_hovered_id;

        hit_test_result
    }

    /// Clears hover state for all nodes.
    pub fn clear_hover(&mut self) {
        for node in self.nodes.values_mut() {
            node.hovered = false;
        }
        self.hovered_node_id = None;
    }

    /// Gets all nodes with their current hover states.
    pub fn all_nodes(&self) -> Vec<&Node> {
        self.nodes.values().collect()
    }

    /// Gets the number of nodes being managed.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for HoverManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Hover-related errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum HoverError {
    #[error("invalid scale: {0}")]
    InvalidScale(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(id: &str, x: f64, y: f64, radius: f64) -> Node {
        Node::builder(id.to_string(), id.to_string())
            .position(x, y)
            .circle_radius(radius)
            .build()
            .unwrap()
    }

    #[test]
    fn test_viewport_transform_new() {
        let viewport = ViewportTransform::new();
        assert_eq!(viewport.offset_x, 0.0);
        assert_eq!(viewport.offset_y, 0.0);
        assert_eq!(viewport.scale, 1.0);
    }

    #[test]
    fn test_viewport_with_offset() {
        let viewport = ViewportTransform::new().with_offset(100.0, 200.0);
        assert_eq!(viewport.offset_x, 100.0);
        assert_eq!(viewport.offset_y, 200.0);
    }

    #[test]
    fn test_viewport_with_scale_valid() {
        let viewport = ViewportTransform::new().with_scale(2.0).unwrap();
        assert_eq!(viewport.scale, 2.0);
    }

    #[test]
    fn test_viewport_with_scale_invalid() {
        assert!(ViewportTransform::new().with_scale(0.0).is_err());
        assert!(ViewportTransform::new().with_scale(-1.0).is_err());
    }

    #[test]
    fn test_screen_to_world_identity() {
        let viewport = ViewportTransform::new();
        let world = viewport.screen_to_world(100.0, 200.0);
        assert_eq!(world.x, 100.0);
        assert_eq!(world.y, 200.0);
    }

    #[test]
    fn test_screen_to_world_with_transform() {
        let viewport = ViewportTransform::new()
            .with_offset(10.0, 20.0)
            .with_scale(2.0)
            .unwrap();
        let world = viewport.screen_to_world(50.0, 60.0);
        assert_eq!(world.x, 20.0); // (50 - 10) / 2
        assert_eq!(world.y, 20.0); // (60 - 20) / 2
    }

    #[test]
    fn test_world_to_screen_identity() {
        let viewport = ViewportTransform::new();
        let screen = viewport.world_to_screen(100.0, 200.0);
        assert_eq!(screen.x, 100.0);
        assert_eq!(screen.y, 200.0);
    }

    #[test]
    fn test_world_to_screen_with_transform() {
        let viewport = ViewportTransform::new()
            .with_offset(10.0, 20.0)
            .with_scale(2.0)
            .unwrap();
        let screen = viewport.world_to_screen(20.0, 20.0);
        assert_eq!(screen.x, 50.0); // 20 * 2 + 10
        assert_eq!(screen.y, 60.0); // 20 * 2 + 20
    }

    #[test]
    fn test_hit_test_result_none() {
        let result = HitTestResult::none();
        assert_eq!(result.node_id, None);
        assert!(!result.cursor_changed);
    }

    #[test]
    fn test_hit_test_result_hit() {
        let result = HitTestResult::hit("node-1".to_string());
        assert_eq!(result.node_id, Some("node-1".to_string()));
        assert!(result.cursor_changed);
    }

    #[test]
    fn test_hover_manager_new() {
        let manager = HoverManager::new();
        assert_eq!(manager.node_count(), 0);
        assert!(manager.hovered_node().is_none());
    }

    #[test]
    fn test_hover_manager_set_node() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 0.0, 0.0, 10.0);
        manager.set_node(node);
        assert_eq!(manager.node_count(), 1);
    }

    #[test]
    fn test_hover_manager_get_node() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 0.0, 0.0, 10.0);
        manager.set_node(node.clone());

        let retrieved = manager.get_node("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id.as_str(), "test");
    }

    #[test]
    fn test_hover_manager_remove_node() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 0.0, 0.0, 10.0);
        manager.set_node(node);

        let removed = manager.remove_node("test");
        assert!(removed.is_some());
        assert_eq!(manager.node_count(), 0);
    }

    #[test]
    fn test_hit_test_no_hit() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 100.0, 100.0, 10.0);
        manager.set_node(node);

        let result = manager.hit_test(0.0, 0.0);
        assert_eq!(result.node_id, None);
    }

    #[test]
    fn test_hit_test_circle_hit() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 100.0, 100.0, 10.0);
        manager.set_node(node);

        let result = manager.hit_test(100.0, 100.0);
        assert_eq!(result.node_id, Some("test".to_string()));
    }

    #[test]
    fn test_hit_test_with_viewport_transform() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 100.0, 100.0, 10.0);
        manager.set_node(node);

        // Apply pan and zoom
        let viewport = ViewportTransform::new()
            .with_offset(50.0, 50.0)
            .with_scale(2.0)
            .unwrap();
        manager.set_viewport(viewport);

        // World (100, 100) becomes Screen (250, 250)
        let result = manager.hit_test(250.0, 250.0);
        assert_eq!(result.node_id, Some("test".to_string()));
    }

    #[test]
    fn test_update_hover_sets_hovered_state() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 100.0, 100.0, 10.0);
        manager.set_node(node);

        manager.update_hover(100.0, 100.0);

        assert!(manager.get_node("test").unwrap().hovered);
        assert_eq!(manager.hovered_node_id, Some("test".to_string()));
    }

    #[test]
    fn test_update_hover_clears_previous_hover() {
        let mut manager = HoverManager::new();
        let node1 = create_test_node("node1", 100.0, 100.0, 10.0);
        let node2 = create_test_node("node2", 200.0, 200.0, 10.0);
        manager.set_node(node1);
        manager.set_node(node2);

        // Hover over node1
        manager.update_hover(100.0, 100.0);
        assert!(manager.get_node("node1").unwrap().hovered);
        assert!(!manager.get_node("node2").unwrap().hovered);

        // Hover over node2
        manager.update_hover(200.0, 200.0);
        assert!(!manager.get_node("node1").unwrap().hovered);
        assert!(manager.get_node("node2").unwrap().hovered);
    }

    #[test]
    fn test_clear_hover() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 100.0, 100.0, 10.0);
        manager.set_node(node);

        manager.update_hover(100.0, 100.0);
        assert!(manager.get_node("test").unwrap().hovered);

        manager.clear_hover();
        assert!(!manager.get_node("test").unwrap().hovered);
        assert!(manager.hovered_node_id.is_none());
    }

    #[test]
    fn test_cursor_changed_flag() {
        let mut manager = HoverManager::new();
        let node = create_test_node("test", 100.0, 100.0, 10.0);
        manager.set_node(node);

        // First hover - cursor should change
        let result = manager.update_hover(100.0, 100.0);
        assert!(result.cursor_changed);

        // Same position - cursor should not change
        let result = manager.update_hover(100.0, 100.0);
        assert!(!result.cursor_changed);

        // Move away - cursor should change
        let result = manager.update_hover(0.0, 0.0);
        assert!(result.cursor_changed);
    }
}
