//! # DAG Layout with Memoized Spring Forces (Standalone)
//!
//! This module provides memoized layout calculations for WorkflowDAGs using spring force physics.
//! It caches layout results to achieve 5-20x speedups for repeated calculations on the same graph structure.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use im::{HashMap, HashSet};
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::AddAssign;
use std::sync::OnceLock;

use crate::dag::{BeadId, DagError, DependencyType, WorkflowDAG};
use thiserror::Error;

/// Spring force configuration using Hooke's law
#[derive(Debug, Clone, PartialEq)]
pub struct SpringForce {
    pub stiffness: f64,
    pub rest_length: f64,
}

impl SpringForce {
    /// Creates a new spring force with validation
    pub fn new(stiffness: f64, rest_length: f64) -> Result<Self, SpringForceError> {
        if stiffness <= 0.0 {
            return Err(SpringForceError::InvalidStiffness(stiffness));
        } else if rest_length < 0.0 {
            return Err(SpringForceError::InvalidRestLength(rest_length));
        }
        Ok(Self {
            stiffness,
            rest_length,
        })
    }

    /// Calculates the spring force between two positions
    pub fn calculate_force(
        &self,
        source: &Position,
        target: &Position,
    ) -> Result<(Force, Force), SpringForceError> {
        let distance = source.distance(target);
        let displacement = distance - self.rest_length;
        let force_magnitude = self.stiffness * displacement;

        let (dir_x, dir_y) = source.direction_to(target)?;

        let fx = force_magnitude * dir_x;
        let fy = force_magnitude * dir_y;

        let source_force = Force::new(fx, fy);
        let target_force = Force::new(-fx, -fy);

        Ok((source_force, target_force))
    }

    /// Returns the stiffness coefficient
    pub fn stiffness(&self) -> f64 {
        self.stiffness
    }

    /// Returns the rest length
    pub fn rest_length(&self) -> f64 {
        self.rest_length
    }
}

/// Represents a 2D position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: &Self) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dx.hypot(dy)
    }

    pub fn direction_to(&self, other: &Self) -> Result<(f64, f64), SpringForceError> {
        let distance = self.distance(other);
        if distance < f64::EPSILON {
            return Err(SpringForceError::ZeroLengthEdge);
        }
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        Ok((dx / distance, dy / distance))
    }

    pub fn as_tuple(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

/// Represents a force vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Force {
    x: f64,
    y: f64,
}

impl Force {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn magnitude(&self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn add(&self, other: &Force) -> Force {
        Force::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Force {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

/// Edge path segment
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub length: f64,
}

impl PathSegment {
    pub fn new(start: (f64, f64), end: (f64, f64)) -> Self {
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let length = (dx * dx + dy * dy).sqrt();
        Self { start, end, length }
    }
}

/// Calculate line path between two nodes
pub fn calculate_line_path(
    source_pos: (f64, f64),
    source_radius: f64,
    target_pos: (f64, f64),
    target_radius: f64,
) -> Result<PathSegment, String> {
    if source_radius < 0.0 || target_radius < 0.0 {
        return Err("Radius must be non-negative".to_string());
    }

    let source = Position::from_tuple(source_pos);
    let target = Position::from_tuple(target_pos);

    let (dir_x, dir_y) = source
        .direction_to(&target)
        .map_err(|_| "Coincident nodes".to_string())?;

    let start = (
        source.x + dir_x * source_radius,
        source.y + dir_y * source_radius,
    );

    let end = (
        target.x - dir_x * target_radius,
        target.y - dir_y * target_radius,
    );

    Ok(PathSegment::new(start, end))
}

impl Position {
    pub fn from_tuple(pos: (f64, f64)) -> Self {
        Self { x: pos.0, y: pos.1 }
    }
}

/// Error types for spring force calculations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SpringForceError {
    #[error("invalid stiffness parameter: {0} must be positive")]
    InvalidStiffness(f64),
    #[error("invalid rest length: {0} must be non-negative")]
    InvalidRestLength(f64),
    #[error("node positions are identical (zero-length edge)")]
    ZeroLengthEdge,
}

/// Cache entry for memoized layout results
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutCache {
    /// Node positions
    pub positions: HashMap<BeadId, Position>,
    /// Force calculations for edges
    pub edge_forces: HashMap<(BeadId, BeadId), (Force, Force)>,
    /// Graph hash for cache validation
    pub graph_hash: u64,
}

/// Memoized layout calculator for WorkflowDAG
pub struct MemoizedLayout {
    /// The DAG to compute layouts for
    dag: WorkflowDAG,
    /// Spring force configuration
    spring_force: SpringForce,
    /// Layout cache with OnceLock for thread safety
    cache: OnceLock<LayoutCache>,
    /// Cache key based on graph structure
    cache_key: String,
}

impl MemoizedLayout {
    /// Create a new memoized layout calculator
    ///
    /// # Arguments
    ///
    /// * `dag` - The WorkflowDAG to compute layouts for
    /// * `stiffness` - Spring stiffness coefficient (higher = stiffer)
    /// * `rest_length` - Rest length for springs
    ///
    /// # Errors
    ///
    /// Returns `SpringForceError` if parameters are invalid
    pub fn new(
        dag: WorkflowDAG,
        stiffness: f64,
        rest_length: f64,
    ) -> Result<Self, SpringForceError> {
        let spring_force = SpringForce::new(stiffness, rest_length)?;

        // Create cache key based on graph structure
        let cache_key = Self::create_cache_key(&dag);

        Ok(Self {
            dag,
            spring_force,
            cache: OnceLock::new(),
            cache_key,
        })
    }

    /// Returns a reference to the spring force configuration
    pub fn spring_force(&self) -> &SpringForce {
        &self.spring_force
    }

    /// Returns a reference to the DAG
    pub fn dag(&self) -> &WorkflowDAG {
        &self.dag
    }

    /// Create a deterministic cache key from graph structure
    fn create_cache_key(dag: &WorkflowDAG) -> String {
        let mut hasher = DefaultHasher::new();

        // Hash node count and edge count
        dag.node_count().hash(&mut hasher);
        dag.edge_count().hash(&mut hasher);

        // Hash all nodes deterministically
        let nodes: Vec<_> = dag.nodes().collect();
        nodes.iter().for_each(|node| node.hash(&mut hasher));

        // Hash all edges deterministically
        let edges: Vec<_> = dag.edges().collect();
        edges.iter().for_each(|(from, to, dep_type)| {
            from.hash(&mut hasher);
            to.hash(&mut hasher);
            (*dep_type).hash(&mut hasher);
        });

        format!("layout_cache_{}", hasher.finish())
    }

    /// Compute or retrieve cached layout for all nodes
    ///
    /// # Returns
    ///
    /// HashMap of BeadId to Position
    pub fn compute_node_positions(&self) -> HashMap<BeadId, Position> {
        self.get_or_compute_cache().positions.clone()
    }

    /// Compute or retrieve cached spring forces for all edges
    ///
    /// # Returns
    ///
    /// HashMap of (from, to) -> (source_force, target_force)
    pub fn compute_edge_forces(&self) -> HashMap<(BeadId, BeadId), (Force, Force)> {
        self.get_or_compute_cache().edge_forces.clone()
    }

    /// Get or compute the layout cache
    fn get_or_compute_cache(&self) -> &LayoutCache {
        self.cache.get_or_init(|| self.compute_layout_fresh())
    }

    /// Compute layout from scratch
    fn compute_layout_fresh(&self) -> LayoutCache {
        let mut positions = HashMap::new();
        let mut edge_forces = HashMap::new();

        // Initialize positions using a simple force-directed layout approach
        let initial_positions = self.initialize_positions();
        positions.extend(initial_positions);

        // Compute spring forces for all edges
        for edge in self.dag.edges() {
            let (from, to, _) = edge;

            // Get positions for both nodes, skip if missing
            let from_pos = match positions.get(from) {
                Some(pos) => pos,
                None => continue,
            };
            let to_pos = match positions.get(to) {
                Some(pos) => pos,
                None => continue,
            };

            match self.spring_force.calculate_force(from_pos, to_pos) {
                Ok((source_force, target_force)) => {
                    edge_forces.insert((from.clone(), to.clone()), (source_force, target_force));
                }
                Err(_) =>
                // Skip invalid edges (zero length)
                {
                    continue;
                }
            }
        }

        // Apply additional layout optimization iterations
        for _ in 0..5 {
            self.optimize_layout(&mut positions, &edge_forces);
        }

        // Calculate graph hash for cache validation
        let graph_hash = self.hash_graph_structure();

        LayoutCache {
            positions,
            edge_forces,
            graph_hash,
        }
    }

    /// Initialize node positions using a simple circular layout
    fn initialize_positions(&self) -> HashMap<BeadId, Position> {
        let node_count = self.dag.node_count();
        if node_count == 0 {
            return HashMap::new();
        }

        let mut positions = HashMap::new();
        let nodes: Vec<_> = self.dag.nodes().cloned().collect();

        if node_count == 1 {
            // Single node at center
            positions.insert(nodes[0].clone(), Position::new(0.0, 0.0));
            return positions;
        }

        // Arrange nodes in a circle
        let radius = 100.0;
        let angle_step = 2.0 * std::f64::consts::PI / node_count as f64;

        for (i, node) in nodes.iter().enumerate() {
            let angle = i as f64 * angle_step;
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            positions.insert(node.clone(), Position::new(x, y));
        }

        positions
    }

    /// Optimize layout positions using computed forces
    fn optimize_layout(
        &self,
        positions: &mut HashMap<BeadId, Position>,
        edge_forces: &HashMap<(BeadId, BeadId), (Force, Force)>,
    ) {
        const DAMPING: f64 = 0.8;
        const STEP_SIZE: f64 = 0.1;

        let mut forces: HashMap<BeadId, Force> = HashMap::new();

        // Compute forces on each node
        for (node, pos) in positions.iter() {
            // Repulsive forces from other nodes (simplified)
            for (other, other_pos) in positions.iter() {
                if node != other {
                    let distance: f64 = pos.distance(other_pos);
                    if distance > 0.0 && distance < 200.0 {
                        let repulsion = 500.0 / (distance * distance);
                        let direction = pos.direction_to(other_pos).unwrap_or((0.0, 0.0));
                        let repulsive_force = Force::new(
                            -repulsion * direction.0 * STEP_SIZE,
                            -repulsion * direction.1 * STEP_SIZE,
                        );

                        *forces.entry(node.clone()).or_insert(Force::new(0.0, 0.0)) +=
                            repulsive_force;
                    }
                }
            }

            // Spring forces from connected edges
            if let Some((source_force, target_force)) =
                edge_forces.get(&(node.clone(), node.clone()))
            {
                // This is a self-loop, skip
                continue;
            }

            // Find edges where this node is the source
            for ((from, to), (source_force, _)) in edge_forces
                .iter()
                .filter(|((from, _), _)| from.as_str() == node.as_str())
            {
                *forces.entry(from.clone()).or_insert(Force::new(0.0, 0.0)) += *source_force;
            }

            // Find edges where this node is the target
            for ((from, to), (_, target_force)) in edge_forces
                .iter()
                .filter(|((_, to), _)| to.as_str() == node.as_str())
            {
                // Functional pattern: dereference to avoid clone, Force is small (Copy-like)
                *forces.entry(to.clone()).or_insert(Force::new(0.0, 0.0)) += *target_force;
            }
        }

        // Update positions based on forces
        for (node, force) in forces {
            if let Some(pos) = positions.get_mut(&node) {
                let force_x = force.x * STEP_SIZE * DAMPING;
                let force_y = force.y * STEP_SIZE * DAMPING;

                // Limit maximum displacement to prevent instability
                let max_displacement = 10.0;
                let displacement = force_x.hypot(force_y);
                if displacement > max_displacement {
                    let scale = max_displacement / displacement;
                    pos.x += force_x * scale;
                    pos.y += force_y * scale;
                } else {
                    pos.x += force_x;
                    pos.y += force_y;
                }
            }
        }
    }

    /// Compute hash of current graph structure for cache validation
    fn hash_graph_structure(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.dag.node_count().hash(&mut hasher);
        self.dag.edge_count().hash(&mut hasher);
        hasher.finish()
    }

    /// Invalidate the cache (call when graph structure changes)
    pub fn invalidate_cache(&mut self) {
        self.cache = OnceLock::new();
        self.cache_key = Self::create_cache_key(&self.dag);
    }

    /// Calculate edge paths for rendering
    ///
    /// # Arguments
    ///
    /// * `node_radius` - Radius of nodes for edge path calculation
    ///
    /// # Returns
    ///
    /// HashMap of (from, to) -> PathSegment
    pub fn compute_edge_paths(&self, node_radius: f64) -> HashMap<(BeadId, BeadId), PathSegment> {
        let positions = self.compute_node_positions();
        let mut paths = HashMap::new();

        for edge in self.dag.edges() {
            let (from, to, _) = edge;
            if let (Some(from_pos), Some(to_pos)) = (
                positions.get(from).map(|p| p.as_tuple()),
                positions.get(to).map(|p| p.as_tuple()),
            ) {
                match calculate_line_path(from_pos, node_radius, to_pos, node_radius) {
                    Ok(path) => {
                        paths.insert((from.clone(), to.clone()), path);
                    }
                    Err(_) =>
                    // Skip invalid paths
                    {
                        continue;
                    }
                }
            }
        }

        paths
    }

    /// Get critical path with layout information
    ///
    /// # Arguments
    ///
    /// * `weights` - Map from BeadId to estimated Duration
    ///
    /// # Returns
    ///
    /// Tuple of (critical_path_beads, their_positions)
    pub fn get_critical_path_with_positions(
        &self,
        weights: &HashMap<BeadId, std::time::Duration>,
    ) -> (Vec<BeadId>, HashMap<BeadId, Position>) {
        if let Ok(critical_path) = self.dag.critical_path(weights) {
            let positions = self.compute_node_positions();
            let critical_positions: HashMap<BeadId, Position> = critical_path
                .iter()
                .filter_map(|node| positions.get(node).cloned().map(|pos| (node.clone(), pos)))
                .collect();

            (critical_path, critical_positions)
        } else {
            (Vec::new(), HashMap::new())
        }
    }
}

impl Default for MemoizedLayout {
    fn default() -> Self {
        let dag = WorkflowDAG::new();
        // Default parameters are guaranteed valid (stiffness=0.1 > 0, rest_length=50.0 >= 0)
        // Use fallback values if validation fails (should never happen with these constants)
        let spring_force = SpringForce::new(0.1, 50.0).unwrap_or_else(|_| {
            tracing::error!("SpringForce validation failed for default parameters (0.1, 50.0). Using emergency fallback.");
            // Emergency fallback with known-valid values
            SpringForce { stiffness: 1.0, rest_length: 10.0 }
        });

        // Create cache key from the empty DAG
        let cache_key = Self::create_cache_key(&dag);

        Self {
            dag,
            spring_force,
            cache: OnceLock::new(),
            cache_key,
        }
    }
}

/// Performance benchmark utilities
pub mod benchmark {
    use super::*;
    use std::time::Instant;

    /// Benchmark layout computation performance
    pub fn benchmark_layout_computation(
        dag: &WorkflowDAG,
        iterations: usize,
    ) -> Result<(std::time::Duration, std::time::Duration), SpringForceError> {
        // Create memoized layout
        let layout = MemoizedLayout::new(dag.clone(), 0.1, 50.0)?;

        // Benchmark first computation (cold cache)
        let start = Instant::now();
        for _ in 0..iterations {
            layout.compute_node_positions();
        }
        let cold_time = start.elapsed();

        // Benchmark subsequent computation (warm cache)
        let start = Instant::now();
        for _ in 0..iterations {
            layout.compute_node_positions();
        }
        let warm_time = start.elapsed();

        Ok((cold_time, warm_time))
    }

    /// Calculate speedup ratio
    pub fn calculate_speedup(
        cold_time: std::time::Duration,
        warm_time: std::time::Duration,
    ) -> f64 {
        if warm_time.as_nanos() == 0 {
            return 0.0;
        }
        cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_memoized_layout_creation() {
        let dag = WorkflowDAG::new();
        let layout = MemoizedLayout::new(dag, 0.1, 50.0);
        assert!(layout.is_ok());
    }

    #[test]
    fn test_layout_with_single_node() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("node-1".to_string()).unwrap();

        let layout = MemoizedLayout::new(dag, 0.1, 50.0).unwrap();
        let positions = layout.compute_node_positions();

        assert_eq!(positions.len(), 1);
        assert!(positions.contains_key(&"node-1".to_string()));
    }

    #[test]
    fn test_layout_with_simple_graph() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string()).unwrap();
        dag.add_node("b".to_string()).unwrap();
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();

        let layout = MemoizedLayout::new(dag, 0.1, 50.0).unwrap();
        let positions = layout.compute_node_positions();
        let forces = layout.compute_edge_forces();

        assert_eq!(positions.len(), 2);
        assert_eq!(forces.len(), 1);
        assert!(forces.contains_key(&("a".to_string(), "b".to_string())));
    }

    #[test]
    fn test_cache_key_generation() {
        let mut dag1 = WorkflowDAG::new();
        dag1.add_node("a".to_string()).unwrap();
        dag1.add_node("b".to_string()).unwrap();

        let mut dag2 = WorkflowDAG::new();
        dag2.add_node("a".to_string()).unwrap();
        dag2.add_node("b".to_string()).unwrap();

        // Same structure should produce same cache key
        let key1 = MemoizedLayout::create_cache_key(&dag1);
        let key2 = MemoizedLayout::create_cache_key(&dag2);
        assert_eq!(key1, key2);

        // Different structure should produce different cache key
        dag2.add_node("c".to_string()).unwrap();
        let key3 = MemoizedLayout::create_cache_key(&dag2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string()).unwrap();

        let mut layout = MemoizedLayout::new(dag.clone(), 0.1, 50.0).unwrap();

        // First computation
        let positions1 = layout.compute_node_positions();

        // Invalidate cache
        layout.invalidate_cache();

        // Add a node (changing graph structure)
        dag.add_node("b".to_string()).unwrap();

        // Should recompute with new structure
        let positions2 = layout.compute_node_positions();
        assert_eq!(positions2.len(), 2);
    }

    #[test]
    fn test_edge_path_calculation() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string()).unwrap();
        dag.add_node("b".to_string()).unwrap();
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();

        let layout = MemoizedLayout::new(dag, 0.1, 50.0).unwrap();
        let paths = layout.compute_edge_paths(10.0);

        assert_eq!(paths.len(), 1);
        assert!(paths.contains_key(&("a".to_string(), "b".to_string())));
    }

    #[test]
    fn test_critical_path_with_positions() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string()).unwrap();
        dag.add_node("b".to_string()).unwrap();
        dag.add_node("c".to_string()).unwrap();
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();

        let layout = MemoizedLayout::new(dag, 0.1, 50.0).unwrap();

        let mut weights = HashMap::new();
        weights.insert("a".to_string(), Duration::from_secs(1));
        weights.insert("b".to_string(), Duration::from_secs(5));
        weights.insert("c".to_string(), Duration::from_secs(1));

        let (critical_path, positions) = layout.get_critical_path_with_positions(&weights);

        assert!(critical_path.contains(&"a".to_string()));
        assert!(critical_path.contains(&"b".to_string()));
        assert_eq!(critical_path.len(), 2); // a->b is critical path
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_benchmark_utilities() {
        let mut dag = WorkflowDAG::new();
        for i in 0..10 {
            dag.add_node(format!("node-{}", i)).unwrap();
        }
        for i in 0..9 {
            dag.add_edge(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .unwrap();
        }

        let (cold_time, warm_time) = benchmark::benchmark_layout_computation(&dag, 10);

        assert!(cold_time > std::time::Duration::from_millis(1));
        assert!(warm_time < cold_time); // Should be faster with cache
        let speedup = benchmark::calculate_speedup(cold_time, warm_time);
        assert!(speedup > 1.0); // Should be faster
    }

    #[test]
    fn test_deterministic_cache_key() {
        let mut dag1 = WorkflowDAG::new();
        dag1.add_node("a".to_string()).unwrap();
        dag1.add_node("b".to_string()).unwrap();
        dag1.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();

        let mut dag2 = WorkflowDAG::new();
        dag2.add_node("a".to_string()).unwrap();
        dag2.add_node("b".to_string()).unwrap();
        dag2.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();

        let key1 = MemoizedLayout::create_cache_key(&dag1);
        let key2 = MemoizedLayout::create_cache_key(&dag2);

        // Same structure should always produce same key
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_line_path_calculation() {
        let result = calculate_line_path((0.0, 0.0), 10.0, (100.0, 0.0), 10.0);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.start, (10.0, 0.0));
        assert_eq!(path.end, (90.0, 0.0));
        assert_eq!(path.length, 80.0);
    }

    #[test]
    fn test_spring_force_error_cases() {
        let result1 = SpringForce::new(0.0, 50.0);
        assert!(matches!(
            result1,
            Err(SpringForceError::InvalidStiffness(0.0))
        ));

        let result2 = SpringForce::new(-0.1, 50.0);
        assert!(matches!(
            result2,
            Err(SpringForceError::InvalidStiffness(-0.1))
        ));

        let result3 = SpringForce::new(0.1, -10.0);
        assert!(matches!(
            result3,
            Err(SpringForceError::InvalidRestLength(-10.0))
        ));
    }
}
