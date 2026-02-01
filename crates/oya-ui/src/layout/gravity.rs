//! Center gravity force for force-directed layout
//!
//! Implements a weak gravity force that pulls nodes toward the viewport center.
//! This prevents the graph from drifting off-screen during simulation.

use crate::models::node::{Node, Position};

/// Configuration for center gravity force
#[derive(Debug, Clone, Copy)]
pub struct GravityConfig {
    /// X coordinate of the gravity center
    pub center_x: f32,
    /// Y coordinate of the gravity center
    pub center_y: f32,
    /// Strength of the gravity force (0.0 - 1.0, typical: 0.01 - 0.05)
    pub strength: f32,
}

impl GravityConfig {
    /// Creates a new GravityConfig with validation
    pub fn new(center_x: f32, center_y: f32, strength: f32) -> Result<Self, String> {
        if !center_x.is_finite() {
            return Err(format!("Center X must be finite, got: {}", center_x));
        }
        if !center_y.is_finite() {
            return Err(format!("Center Y must be finite, got: {}", center_y));
        }
        if !strength.is_finite() {
            return Err(format!("Strength must be finite, got: {}", strength));
        }
        if strength < 0.0 {
            return Err(format!("Strength must be non-negative, got: {}", strength));
        }
        if strength > 1.0 {
            return Err(format!("Strength must be <= 1.0, got: {}", strength));
        }

        Ok(Self {
            center_x,
            center_y,
            strength,
        })
    }
}

impl Default for GravityConfig {
    /// Default configuration for a 1200x800 canvas
    fn default() -> Self {
        Self {
            center_x: 600.0,
            center_y: 400.0,
            strength: 0.03,
        }
    }
}

/// Calculate gravity force pulling node toward center
///
/// Pure function - no side effects, returns new position.
/// Force is linear and proportional to distance from center.
///
/// # Formula
/// `F = strength * (center - position)`
///
/// # Arguments
/// * `node` - The node to apply gravity to
/// * `config` - Gravity configuration
///
/// # Returns
/// New position after applying gravity force
pub fn apply_center_gravity(node: &Node, config: &GravityConfig) -> Result<Position, String> {
    let pos = node.position();

    // Calculate distance vector from node to center
    let dx = config.center_x - pos.x;
    let dy = config.center_y - pos.y;

    // Apply gravity force (scaled by strength)
    let new_x = pos.x + dx * config.strength;
    let new_y = pos.y + dy * config.strength;

    // Validate result (no NaN/infinity)
    Position::new(new_x, new_y)
}

/// Apply center gravity to all nodes
///
/// Returns new node list with updated positions.
/// Immutable - original nodes unchanged.
///
/// # Arguments
/// * `nodes` - Slice of nodes to process
/// * `config` - Gravity configuration
///
/// # Returns
/// New vector of nodes with updated positions
pub fn apply_gravity_to_all(nodes: &[Node], config: &GravityConfig) -> Result<Vec<Node>, String> {
    nodes
        .iter()
        .map(|node| {
            let new_pos = apply_center_gravity(node, config)?;
            let mut updated = node.clone();
            updated.set_position(new_pos);
            Ok(updated)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gravity_pulls_toward_center() -> Result<(), String> {
        // Node far from center should move closer
        let node = Node::with_position("n1", "Node 1", 800.0, 600.0)?;

        let config = GravityConfig::default(); // center at (600, 400)
        let new_pos = apply_center_gravity(&node, &config)?;

        // Should move closer to center (600, 400)
        assert!(new_pos.x < 800.0, "X should move toward 600");
        assert!(new_pos.y < 600.0, "Y should move toward 400");
        assert!(new_pos.x > 600.0, "X should not overshoot");
        assert!(new_pos.y > 400.0, "Y should not overshoot");
        Ok(())
    }

    #[test]
    fn test_gravity_is_deterministic() -> Result<(), String> {
        // Same input should produce same output
        let node = Node::with_position("n1", "Node 1", 100.0, 100.0)?;

        let config = GravityConfig::default();
        let pos1 = apply_center_gravity(&node, &config)?;
        let pos2 = apply_center_gravity(&node, &config)?;

        assert_eq!(pos1.x, pos2.x);
        assert_eq!(pos1.y, pos2.y);
        Ok(())
    }

    #[test]
    fn test_gravity_at_center_no_movement() -> Result<(), String> {
        // Node at center should not move
        let node = Node::with_position("n1", "Node 1", 600.0, 400.0)?;

        let config = GravityConfig::default();
        let new_pos = apply_center_gravity(&node, &config)?;

        // Should stay at center (within floating point precision)
        assert!((new_pos.x - 600.0).abs() < 1e-6);
        assert!((new_pos.y - 400.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_gravity_strength_zero_no_movement() -> Result<(), String> {
        // Zero strength should produce no movement
        let original_pos = Position::new(800.0, 600.0)?;
        let mut node = Node::new("n1", "Node 1")?;
        node.set_position(original_pos);

        let config = GravityConfig::new(600.0, 400.0, 0.0)?;
        let new_pos = apply_center_gravity(&node, &config)?;

        assert_eq!(new_pos.x, original_pos.x);
        assert_eq!(new_pos.y, original_pos.y);
        Ok(())
    }

    #[test]
    fn test_gravity_strength_validation() -> Result<(), String> {
        // Negative strength should error
        let result = GravityConfig::new(600.0, 400.0, -0.1);
        assert!(result.is_err());
        let err = result.err().ok_or("Expected error but got Ok")?;
        assert!(err.contains("non-negative"));

        // Strength > 1.0 should error
        let result = GravityConfig::new(600.0, 400.0, 1.5);
        assert!(result.is_err());
        let err = result.err().ok_or("Expected error but got Ok")?;
        assert!(err.contains("<= 1.0"));

        // Valid strengths
        assert!(GravityConfig::new(600.0, 400.0, 0.0).is_ok());
        assert!(GravityConfig::new(600.0, 400.0, 0.5).is_ok());
        assert!(GravityConfig::new(600.0, 400.0, 1.0).is_ok());
        Ok(())
    }

    #[test]
    fn test_gravity_nan_center_validation() {
        // NaN center coordinates should error
        let result = GravityConfig::new(f32::NAN, 400.0, 0.03);
        assert!(result.is_err());

        let result = GravityConfig::new(600.0, f32::NAN, 0.03);
        assert!(result.is_err());
    }

    #[test]
    fn test_gravity_infinity_center_validation() {
        // Infinite center coordinates should error
        let result = GravityConfig::new(f32::INFINITY, 400.0, 0.03);
        assert!(result.is_err());

        let result = GravityConfig::new(600.0, f32::NEG_INFINITY, 0.03);
        assert!(result.is_err());
    }

    #[test]
    fn test_gravity_extreme_position() -> Result<(), String> {
        // Node very far from center
        let node = Node::with_position("n1", "Node 1", 10000.0, 10000.0)?;

        let config = GravityConfig::default();
        let new_pos = apply_center_gravity(&node, &config)?;

        // Should move toward center but not reach it in one step
        assert!(new_pos.x < 10000.0);
        assert!(new_pos.y < 10000.0);
        assert!(new_pos.x > 600.0);
        assert!(new_pos.y > 400.0);

        // Result should be finite
        assert!(new_pos.x.is_finite());
        assert!(new_pos.y.is_finite());
        Ok(())
    }

    #[test]
    fn test_apply_gravity_to_all_immutability() -> Result<(), String> {
        // Original nodes should be unchanged
        let node1 = Node::with_position("n1", "Node 1", 100.0, 100.0)?;
        let node2 = Node::with_position("n2", "Node 2", 800.0, 600.0)?;

        let original_nodes = vec![node1.clone(), node2.clone()];
        let config = GravityConfig::default();

        let updated_nodes = apply_gravity_to_all(&original_nodes, &config)?;

        // Original nodes unchanged
        assert_eq!(original_nodes[0].position().x, 100.0);
        assert_eq!(original_nodes[0].position().y, 100.0);
        assert_eq!(original_nodes[1].position().x, 800.0);
        assert_eq!(original_nodes[1].position().y, 600.0);

        // Updated nodes have new positions
        assert_ne!(updated_nodes[0].position().x, 100.0);
        assert_ne!(updated_nodes[1].position().x, 800.0);
        Ok(())
    }

    #[test]
    fn test_apply_gravity_to_all_count() -> Result<(), String> {
        // Should return same number of nodes
        let mut nodes = Vec::new();
        for i in 0..10 {
            let node = Node::with_position(
                &format!("n{}", i),
                &format!("Node {}", i),
                i as f32 * 100.0,
                i as f32 * 50.0,
            )?;
            nodes.push(node);
        }

        let config = GravityConfig::default();
        let updated = apply_gravity_to_all(&nodes, &config)?;

        assert_eq!(updated.len(), nodes.len());
        Ok(())
    }

    #[test]
    fn test_apply_gravity_to_empty_list() -> Result<(), String> {
        // Empty list should return empty list
        let nodes: Vec<Node> = vec![];
        let config = GravityConfig::default();

        let result = apply_gravity_to_all(&nodes, &config)?;
        assert_eq!(result.len(), 0);
        Ok(())
    }

    #[test]
    fn test_gravity_convergence() -> Result<(), String> {
        // Multiple applications should converge toward center
        let node = Node::with_position("n1", "Node 1", 1000.0, 1000.0)?;

        let config = GravityConfig::default();

        let mut current = node;
        let initial_distance = {
            let pos = current.position();
            ((pos.x - 600.0).powi(2) + (pos.y - 400.0).powi(2)).sqrt()
        };

        // Apply gravity multiple times
        for _ in 0..10 {
            let new_pos = apply_center_gravity(&current, &config)?;
            current.set_position(new_pos);
        }

        let final_distance = {
            let pos = current.position();
            ((pos.x - 600.0).powi(2) + (pos.y - 400.0).powi(2)).sqrt()
        };

        // Distance to center should decrease
        assert!(final_distance < initial_distance);
        Ok(())
    }

    #[test]
    fn test_gravity_config_default() {
        let config = GravityConfig::default();
        assert_eq!(config.center_x, 600.0);
        assert_eq!(config.center_y, 400.0);
        assert_eq!(config.strength, 0.03);
    }

    #[test]
    fn test_gravity_negative_coordinates() -> Result<(), String> {
        // Negative coordinates should work fine
        let node = Node::with_position("n1", "Node 1", -100.0, -200.0)?;

        let config = GravityConfig::new(0.0, 0.0, 0.05)?;
        let new_pos = apply_center_gravity(&node, &config)?;

        // Should move toward (0, 0)
        assert!(new_pos.x > -100.0);
        assert!(new_pos.y > -200.0);
        assert!(new_pos.x.is_finite());
        assert!(new_pos.y.is_finite());
        Ok(())
    }

    #[test]
    fn test_gravity_max_strength_single_step() -> Result<(), String> {
        // Strength of 1.0 should move node directly to center in one step
        let node = Node::with_position("n1", "Node 1", 800.0, 600.0)?;

        let config = GravityConfig::new(600.0, 400.0, 1.0)?;
        let new_pos = apply_center_gravity(&node, &config)?;

        // With strength 1.0, should reach center exactly
        assert_eq!(new_pos.x, 600.0);
        assert_eq!(new_pos.y, 400.0);
        Ok(())
    }

    #[test]
    fn test_gravity_weak_strength_slow_convergence() -> Result<(), String> {
        // Very weak strength should converge slowly
        let node = Node::with_position("n1", "Node 1", 1000.0, 1000.0)?;

        let config = GravityConfig::new(600.0, 400.0, 0.001)?;

        let mut current = node;
        let initial_pos = current.position();

        // Apply once
        let new_pos = apply_center_gravity(&current, &config)?;
        current.set_position(new_pos);

        // Should move only slightly
        let distance_moved =
            ((new_pos.x - initial_pos.x).powi(2) + (new_pos.y - initial_pos.y).powi(2)).sqrt();

        // With 0.001 strength and distance ~700, movement should be small
        assert!(distance_moved < 10.0);
        assert!(distance_moved > 0.0);
        Ok(())
    }

    #[test]
    fn test_apply_gravity_preserves_node_identity() -> Result<(), String> {
        // Node IDs and labels should be preserved
        let node1 = Node::with_position("n1", "Node One", 100.0, 100.0)?;
        let node2 = Node::with_position("n2", "Node Two", 800.0, 600.0)?;

        let nodes = vec![node1, node2];
        let config = GravityConfig::default();

        let updated = apply_gravity_to_all(&nodes, &config)?;

        // IDs and labels preserved
        assert_eq!(updated[0].id().as_str(), "n1");
        assert_eq!(updated[0].label(), "Node One");
        assert_eq!(updated[1].id().as_str(), "n2");
        assert_eq!(updated[1].label(), "Node Two");
        Ok(())
    }
}
