//! Spring force for force-directed layout
//!
//! Implements Hooke's law spring forces along edges to maintain ideal edge length.
//! Connected nodes attract/repel each other toward the rest length distance.

use crate::models::edge::Edge;
use crate::models::node::{Node, NodeId, Position};
use std::collections::HashMap;

/// Configuration for spring force
#[derive(Debug, Clone, Copy)]
pub struct SpringConfig {
    /// Spring stiffness constant (0.0 - 1.0, typical: 0.01 - 0.1)
    pub stiffness: f32,
    /// Ideal rest length for edges in pixels (typical: 50 - 100)
    pub rest_length: f32,
}

impl SpringConfig {
    /// Creates a new SpringConfig with validation
    pub fn new(stiffness: f32, rest_length: f32) -> Result<Self, String> {
        if !stiffness.is_finite() {
            return Err(format!("Stiffness must be finite, got: {}", stiffness));
        }
        if stiffness < 0.0 {
            return Err(format!(
                "Stiffness must be non-negative, got: {}",
                stiffness
            ));
        }
        if stiffness > 1.0 {
            return Err(format!("Stiffness must be <= 1.0, got: {}", stiffness));
        }

        if !rest_length.is_finite() {
            return Err(format!("Rest length must be finite, got: {}", rest_length));
        }
        if rest_length <= 0.0 {
            return Err(format!("Rest length must be positive, got: {}", rest_length));
        }

        Ok(Self {
            stiffness,
            rest_length,
        })
    }
}

impl Default for SpringConfig {
    /// Default configuration: moderate stiffness, 75px rest length
    fn default() -> Self {
        Self {
            stiffness: 0.05,
            rest_length: 75.0,
        }
    }
}

/// Force vector in 2D space
#[derive(Debug, Clone, Copy, PartialEq)]
struct Force {
    dx: f32,
    dy: f32,
}

impl Force {
    fn zero() -> Self {
        Self { dx: 0.0, dy: 0.0 }
    }

    fn add(&mut self, other: Force) {
        self.dx += other.dx;
        self.dy += other.dy;
    }
}

/// Calculate spring force between two nodes connected by an edge
///
/// Uses Hooke's law: F = -k * x, where x is displacement from rest length.
/// Returns force to apply to target node (apply equal/opposite to source).
///
/// # Formula
/// ```text
/// distance = sqrt((x2-x1)^2 + (y2-y1)^2)
/// displacement = distance - rest_length
/// force_magnitude = stiffness * displacement
/// direction = (target - source) / distance
/// force = force_magnitude * direction
/// ```
///
/// # Arguments
/// * `source` - The source node
/// * `target` - The target node
/// * `config` - Spring configuration
///
/// # Returns
/// Force vector to apply to target (negate for source)
fn calculate_spring_force(
    source: &Node,
    target: &Node,
    config: &SpringConfig,
) -> Result<Force, String> {
    let source_pos = source.position();
    let target_pos = target.position();

    // Calculate distance between nodes
    let dx = target_pos.x - source_pos.x;
    let dy = target_pos.y - source_pos.y;
    let distance = (dx * dx + dy * dy).sqrt();

    // Handle zero-length edges gracefully (no division by zero)
    if distance < 1e-6 {
        return Ok(Force::zero());
    }

    // Calculate displacement from rest length
    let displacement = distance - config.rest_length;

    // Calculate force magnitude using Hooke's law
    let force_magnitude = config.stiffness * displacement;

    // Normalize direction vector
    let direction_x = dx / distance;
    let direction_y = dy / distance;

    // Apply force in direction of displacement
    let force = Force {
        dx: force_magnitude * direction_x,
        dy: force_magnitude * direction_y,
    };

    // Validate result
    if !force.dx.is_finite() || !force.dy.is_finite() {
        return Err(format!(
            "Force calculation resulted in non-finite values: dx={}, dy={}",
            force.dx, force.dy
        ));
    }

    Ok(force)
}

/// Apply spring forces to all nodes based on edges
///
/// Returns new node list with updated positions.
/// Immutable - original nodes unchanged.
///
/// # Arguments
/// * `nodes` - Slice of nodes to process
/// * `edges` - Slice of edges connecting nodes
/// * `config` - Spring configuration
///
/// # Returns
/// New vector of nodes with updated positions
pub fn apply_spring_forces(
    nodes: &[Node],
    edges: &[Edge],
    config: &SpringConfig,
) -> Result<Vec<Node>, String> {
    // Build node lookup by ID for efficient access
    let node_map: HashMap<&str, &Node> = nodes
        .iter()
        .map(|node| (node.id().as_str(), node))
        .collect();

    // Accumulate forces for each node
    let mut force_map: HashMap<&str, Force> = nodes
        .iter()
        .map(|node| (node.id().as_str(), Force::zero()))
        .collect();

    // Calculate spring forces for each edge
    for edge in edges {
        let source_id = edge.source().as_str();
        let target_id = edge.target().as_str();

        // Look up nodes (skip if either is missing)
        let source_node = match node_map.get(source_id) {
            Some(node) => node,
            None => continue,
        };
        let target_node = match node_map.get(target_id) {
            Some(node) => node,
            None => continue,
        };

        // Calculate spring force
        let force = calculate_spring_force(source_node, target_node, config)?;

        // Apply force to target (attractive/repulsive)
        if let Some(target_force) = force_map.get_mut(target_id) {
            target_force.add(force);
        }

        // Apply equal and opposite force to source (Newton's third law)
        if let Some(source_force) = force_map.get_mut(source_id) {
            source_force.add(Force {
                dx: -force.dx,
                dy: -force.dy,
            });
        }
    }

    // Apply accumulated forces to nodes
    nodes
        .iter()
        .map(|node| {
            let node_id = node.id().as_str();
            let force = force_map
                .get(node_id)
                .copied()
                .unwrap_or_else(Force::zero);

            let old_pos = node.position();
            let new_pos = Position::new(old_pos.x + force.dx, old_pos.y + force.dy)?;

            let mut updated = node.clone();
            updated.set_position(new_pos);
            Ok(updated)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edge::EdgeType;

    #[test]
    fn test_spring_config_validation() {
        // Valid configs
        assert!(SpringConfig::new(0.05, 75.0).is_ok());
        assert!(SpringConfig::new(0.0, 50.0).is_ok());
        assert!(SpringConfig::new(1.0, 100.0).is_ok());

        // Invalid stiffness
        assert!(SpringConfig::new(-0.1, 75.0).is_err());
        assert!(SpringConfig::new(1.5, 75.0).is_err());
        assert!(SpringConfig::new(f32::NAN, 75.0).is_err());
        assert!(SpringConfig::new(f32::INFINITY, 75.0).is_err());

        // Invalid rest_length
        assert!(SpringConfig::new(0.05, 0.0).is_err());
        assert!(SpringConfig::new(0.05, -10.0).is_err());
        assert!(SpringConfig::new(0.05, f32::NAN).is_err());
        assert!(SpringConfig::new(0.05, f32::INFINITY).is_err());
    }

    #[test]
    fn test_spring_config_default() {
        let config = SpringConfig::default();
        assert_eq!(config.stiffness, 0.05);
        assert_eq!(config.rest_length, 75.0);
    }

    #[test]
    fn test_spring_force_attracts_when_too_far() -> Result<(), String> {
        // Nodes farther than rest_length should attract
        let source = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let target = Node::with_position("n2", "Node 2", 200.0, 0.0)?; // 200px apart

        let config = SpringConfig::new(0.1, 75.0)?; // rest_length = 75px
        let force = calculate_spring_force(&source, &target, &config)?;

        // Force should pull target toward source (negative dx)
        assert!(force.dx < 0.0, "Force should be negative (attractive)");
        assert!(force.dy.abs() < 1e-6, "No vertical force expected");
        Ok(())
    }

    #[test]
    fn test_spring_force_repels_when_too_close() -> Result<(), String> {
        // Nodes closer than rest_length should repel
        let source = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let target = Node::with_position("n2", "Node 2", 25.0, 0.0)?; // 25px apart

        let config = SpringConfig::new(0.1, 75.0)?; // rest_length = 75px
        let force = calculate_spring_force(&source, &target, &config)?;

        // Force should push target away from source (positive dx)
        assert!(force.dx > 0.0, "Force should be positive (repulsive)");
        assert!(force.dy.abs() < 1e-6, "No vertical force expected");
        Ok(())
    }

    #[test]
    fn test_spring_force_zero_at_rest_length() -> Result<(), String> {
        // Nodes at rest_length should have zero force
        let source = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let target = Node::with_position("n2", "Node 2", 75.0, 0.0)?; // exactly 75px

        let config = SpringConfig::new(0.1, 75.0)?;
        let force = calculate_spring_force(&source, &target, &config)?;

        // Force should be near zero
        assert!(force.dx.abs() < 1e-6, "Force should be zero at rest length");
        assert!(force.dy.abs() < 1e-6, "Force should be zero at rest length");
        Ok(())
    }

    #[test]
    fn test_spring_force_handles_zero_distance() -> Result<(), String> {
        // Nodes at same position should produce zero force (no division by zero)
        let source = Node::with_position("n1", "Node 1", 100.0, 100.0)?;
        let target = Node::with_position("n2", "Node 2", 100.0, 100.0)?;

        let config = SpringConfig::default();
        let force = calculate_spring_force(&source, &target, &config)?;

        // Should return zero force gracefully
        assert_eq!(force.dx, 0.0);
        assert_eq!(force.dy, 0.0);
        Ok(())
    }

    #[test]
    fn test_spring_force_proportional_to_stiffness() -> Result<(), String> {
        let source = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let target = Node::with_position("n2", "Node 2", 200.0, 0.0)?;

        let config1 = SpringConfig::new(0.05, 75.0)?;
        let force1 = calculate_spring_force(&source, &target, &config1)?;

        let config2 = SpringConfig::new(0.1, 75.0)?; // Double stiffness
        let force2 = calculate_spring_force(&source, &target, &config2)?;

        // Force should scale with stiffness
        assert!((force2.dx / force1.dx - 2.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_spring_force_proportional_to_displacement() -> Result<(), String> {
        let source = Node::with_position("n1", "Node 1", 0.0, 0.0)?;

        let config = SpringConfig::new(0.1, 75.0)?;

        // Displacement of 25px
        let target1 = Node::with_position("n2", "Node 2", 100.0, 0.0)?;
        let force1 = calculate_spring_force(&source, &target1, &config)?;

        // Displacement of 50px
        let target2 = Node::with_position("n3", "Node 3", 125.0, 0.0)?;
        let force2 = calculate_spring_force(&source, &target2, &config)?;

        // Force should scale with displacement
        assert!((force2.dx / force1.dx - 2.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_spring_force_diagonal() -> Result<(), String> {
        // Test spring force at 45 degrees
        let source = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let target = Node::with_position("n2", "Node 2", 100.0, 100.0)?; // sqrt(2)*100 ≈ 141px

        let config = SpringConfig::new(0.1, 75.0)?;
        let force = calculate_spring_force(&source, &target, &config)?;

        // Force should be diagonal (equal x and y components)
        assert!((force.dx - force.dy).abs() < 1e-4);
        assert!(force.dx < 0.0); // Attractive (too far)
        assert!(force.dy < 0.0); // Attractive (too far)
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_empty_graph() -> Result<(), String> {
        let nodes: Vec<Node> = vec![];
        let edges: Vec<Edge> = vec![];
        let config = SpringConfig::default();

        let result = apply_spring_forces(&nodes, &edges, &config)?;
        assert_eq!(result.len(), 0);
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_no_edges() -> Result<(), String> {
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node 2", 100.0, 0.0)?;

        let nodes = vec![node1, node2];
        let edges: Vec<Edge> = vec![];
        let config = SpringConfig::default();

        let result = apply_spring_forces(&nodes, &edges, &config)?;

        // No edges = no forces, positions unchanged
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].position().x, 0.0);
        assert_eq!(result[1].position().x, 100.0);
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_single_edge() -> Result<(), String> {
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node 2", 200.0, 0.0)?;

        let nodes = vec![node1, node2];
        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n2")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];

        let config = SpringConfig::new(0.1, 75.0)?;
        let result = apply_spring_forces(&nodes, &edges, &config)?;

        // Nodes should move toward each other
        assert!(result[0].position().x > 0.0, "Source should move right");
        assert!(result[1].position().x < 200.0, "Target should move left");
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_newtons_third_law() -> Result<(), String> {
        // Equal and opposite forces
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node 2", 200.0, 0.0)?;

        let nodes = vec![node1.clone(), node2.clone()];
        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n2")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];

        let config = SpringConfig::new(0.1, 75.0)?;
        let result = apply_spring_forces(&nodes, &edges, &config)?;

        let delta1 = result[0].position().x - node1.position().x;
        let delta2 = result[1].position().x - node2.position().x;

        // Movements should be equal and opposite
        assert!((delta1 + delta2).abs() < 1e-4, "Forces should be equal/opposite");
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_multiple_edges() -> Result<(), String> {
        // Triangle: n1 -- n2 -- n3 -- n1
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node 2", 200.0, 0.0)?;
        let node3 = Node::with_position("n3", "Node 3", 100.0, 100.0)?;

        let nodes = vec![node1, node2, node3];
        let edges = vec![
            Edge::new(
                NodeId::new("n1")?,
                NodeId::new("n2")?,
                EdgeType::Dependency,
            )?,
            Edge::new(
                NodeId::new("n2")?,
                NodeId::new("n3")?,
                EdgeType::Dependency,
            )?,
            Edge::new(
                NodeId::new("n3")?,
                NodeId::new("n1")?,
                EdgeType::Dependency,
            )?,
        ];

        let config = SpringConfig::default();
        let result = apply_spring_forces(&nodes, &edges, &config)?;

        // All nodes should move (forces accumulate)
        assert_eq!(result.len(), 3);
        for node in &result {
            assert!(node.position().x.is_finite());
            assert!(node.position().y.is_finite());
        }
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_missing_node() -> Result<(), String> {
        // Edge references non-existent node - should skip gracefully
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;

        let nodes = vec![node1.clone()];
        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n_missing")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];

        let config = SpringConfig::default();
        let result = apply_spring_forces(&nodes, &edges, &config)?;

        // Should complete without error, n1 unchanged (no valid edge)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].position().x, node1.position().x);
        assert_eq!(result[0].position().y, node1.position().y);
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_preserves_identity() -> Result<(), String> {
        let node1 = Node::with_position("n1", "Node One", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node Two", 100.0, 0.0)?;

        let nodes = vec![node1, node2];
        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n2")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];

        let config = SpringConfig::default();
        let result = apply_spring_forces(&nodes, &edges, &config)?;

        // IDs and labels preserved
        assert_eq!(result[0].id().as_str(), "n1");
        assert_eq!(result[0].label(), "Node One");
        assert_eq!(result[1].id().as_str(), "n2");
        assert_eq!(result[1].label(), "Node Two");
        Ok(())
    }

    #[test]
    fn test_apply_spring_forces_immutability() -> Result<(), String> {
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node 2", 200.0, 0.0)?;

        let original_nodes = vec![node1.clone(), node2.clone()];
        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n2")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];

        let config = SpringConfig::default();
        let _result = apply_spring_forces(&original_nodes, &edges, &config)?;

        // Original nodes unchanged
        assert_eq!(original_nodes[0].position().x, 0.0);
        assert_eq!(original_nodes[1].position().x, 200.0);
        Ok(())
    }

    #[test]
    fn test_spring_force_convergence() -> Result<(), String> {
        // Multiple iterations should converge toward rest_length
        let mut node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let mut node2 = Node::with_position("n2", "Node 2", 200.0, 0.0)?;

        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n2")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];
        let config = SpringConfig::new(0.1, 75.0)?;

        let initial_distance = 200.0;

        // Run simulation for several iterations
        for _ in 0..20 {
            let nodes = vec![node1.clone(), node2.clone()];
            let result = apply_spring_forces(&nodes, &edges, &config)?;
            node1 = result[0].clone();
            node2 = result[1].clone();
        }

        // Calculate final distance
        let dx = node2.position().x - node1.position().x;
        let dy = node2.position().y - node1.position().y;
        let final_distance = (dx * dx + dy * dy).sqrt();

        // Should converge toward rest_length (75px)
        assert!(
            (final_distance - initial_distance).abs() > 10.0,
            "Distance should have changed significantly"
        );
        assert!(
            (final_distance - config.rest_length).abs() < 50.0,
            "Should converge toward rest_length"
        );
        Ok(())
    }

    #[test]
    fn test_spring_force_zero_stiffness() -> Result<(), String> {
        // Zero stiffness = no force
        let node1 = Node::with_position("n1", "Node 1", 0.0, 0.0)?;
        let node2 = Node::with_position("n2", "Node 2", 200.0, 0.0)?;

        let nodes = vec![node1.clone(), node2.clone()];
        let edge = Edge::new(
            NodeId::new("n1")?,
            NodeId::new("n2")?,
            EdgeType::Dependency,
        )?;
        let edges = vec![edge];

        let config = SpringConfig::new(0.0, 75.0)?;
        let result = apply_spring_forces(&nodes, &edges, &config)?;

        // Positions unchanged
        assert_eq!(result[0].position().x, 0.0);
        assert_eq!(result[1].position().x, 200.0);
        Ok(())
    }
}
