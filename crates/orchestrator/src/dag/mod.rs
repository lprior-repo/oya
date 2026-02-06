//! # Workflow DAG
//!
//! Directed Acyclic Graph for workflow dependencies using petgraph.
//!
//! This module provides a `WorkflowDAG` structure for managing bead dependencies
//! with support for:
//! - Query operations (dependencies, dependents, ancestors, descendants)
//! - Ready detection (which beads can execute given completed set)
//! - Topological ordering (DFS and Kahn's algorithm)
//! - Validation (cycle detection, connectivity)
//! - Mutation (remove nodes/edges)
//! - Subgraph extraction

use im::{HashMap, HashSet};
use petgraph::Direction;
use petgraph::algo::{is_cyclic_directed, tarjan_scc, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Bfs, Dfs, EdgeRef, Reversed};
use std::collections::VecDeque;
use std::time::Duration;

pub mod error;
pub use error::{DagError, DagResult};

/// Type alias for a Bead identifier
pub type BeadId = String;

/// Dependency relationship types between beads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Bead must complete before dependent can start
    BlockingDependency,
    /// Soft dependency - influences scheduling but doesn't block
    PreferredOrder,
}

/// Workflow DAG structure wrapping petgraph's DiGraph
#[derive(Debug, Clone)]
pub struct WorkflowDAG {
    /// The underlying directed graph
    graph: DiGraph<BeadId, DependencyType>,
    /// Map from BeadId to NodeIndex for O(1) lookups
    node_map: HashMap<BeadId, NodeIndex>,
}

impl WorkflowDAG {
    /// Create a new empty WorkflowDAG
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// let dag = WorkflowDAG::new();
    /// assert_eq!(dag.node_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a node to the DAG
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The unique identifier for the bead
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the node was added successfully
    /// * `Err(DagError)` if the bead_id already exists in the DAG
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// let mut dag = WorkflowDAG::new();
    /// let result = dag.add_node("bead-001".to_string());
    /// assert!(result.is_ok());
    /// ```
    pub fn add_node(&mut self, bead_id: BeadId) -> DagResult<()> {
        if self.node_map.contains_key(&bead_id) {
            return Err(DagError::node_already_exists(bead_id));
        }

        let node_index = self.graph.add_node(bead_id.clone());
        self.node_map.insert(bead_id, node_index);

        Ok(())
    }

    /// Add an edge between two nodes
    ///
    /// # Arguments
    ///
    /// * `from_bead` - The BeadId of the source node (dependency)
    /// * `to_bead` - The BeadId of the target node (dependent)
    /// * `dep_type` - The type of dependency relationship
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the edge was added successfully
    /// * `Err(DagError)` if either node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string())?;
    /// dag.add_node("bead-002".to_string())?;
    /// let result = dag.add_edge(
    ///     "bead-001".to_string(),
    ///     "bead-002".to_string(),
    ///     DependencyType::BlockingDependency
    /// );
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_edge(
        &mut self,
        from_bead: BeadId,
        to_bead: BeadId,
        dep_type: DependencyType,
    ) -> DagResult<()> {
        if from_bead == to_bead {
            return Err(DagError::self_loop(from_bead));
        }

        let from_index = self
            .node_map
            .get(&from_bead)
            .ok_or_else(|| DagError::node_not_found(from_bead.clone()))?;

        let to_index = self
            .node_map
            .get(&to_bead)
            .ok_or_else(|| DagError::node_not_found(to_bead.clone()))?;

        if self.graph.find_edge(*from_index, *to_index).is_some() {
            return Err(DagError::edge_already_exists(from_bead, to_bead));
        }

        self.graph.add_edge(*from_index, *to_index, dep_type);

        Ok(())
    }

    /// Add a dependency relationship between two beads.
    ///
    /// This is a semantic alias for `add_edge` that makes the API more expressive.
    /// The `from` bead is the dependency, and the `to` bead depends on it.
    ///
    /// # Arguments
    ///
    /// * `from` - The BeadId of the dependency (what `to` depends on)
    /// * `to` - The BeadId of the dependent (depends on `from`)
    /// * `dep_type` - The type of dependency relationship
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the dependency was added successfully
    /// * `Err(DagError::SelfLoopDetected)` if from and to are the same
    /// * `Err(DagError::NodeNotFound)` if either bead doesn't exist
    /// * `Err(DagError::EdgeAlreadyExists)` if the edge already exists
    ///
    /// # Validation
    ///
    /// - Prevents self-loops (a bead cannot depend on itself)
    /// - Validates both beads exist in the DAG
    /// - Prevents duplicate edges
    /// - Preserves dependency type
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("build".to_string())?;
    /// dag.add_node("test".to_string())?;
    ///
    /// // test depends on build
    /// dag.add_dependency(
    ///     "build".to_string(),
    ///     "test".to_string(),
    ///     DependencyType::BlockingDependency
    /// )?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_dependency(
        &mut self,
        from: BeadId,
        to: BeadId,
        dep_type: DependencyType,
    ) -> DagResult<()> {
        self.add_edge(from, to, dep_type)
    }

    /// Get an iterator over all nodes in the DAG
    ///
    /// # Returns
    ///
    /// An iterator yielding references to BeadIds
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string())?;
    /// dag.add_node("bead-002".to_string())?;
    /// let nodes: Vec<&String> = dag.nodes().collect();
    /// assert_eq!(nodes.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn nodes(&self) -> impl Iterator<Item = &BeadId> {
        self.graph.node_weights()
    }

    /// Get an iterator over all edges in the DAG
    ///
    /// # Returns
    ///
    /// An iterator yielding tuples of (from_bead, to_bead, dependency_type)
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string())?;
    /// dag.add_node("bead-002".to_string())?;
    /// dag.add_edge(
    ///     "bead-001".to_string(),
    ///     "bead-002".to_string(),
    ///     DependencyType::BlockingDependency
    /// )?;
    /// let edges: Vec<_> = dag.edges().collect();
    /// assert_eq!(edges.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn edges(&self) -> impl Iterator<Item = (&BeadId, &BeadId, &DependencyType)> {
        self.graph.edge_references().filter_map(move |edge| {
            let from = self.graph.node_weight(edge.source())?;
            let to = self.graph.node_weight(edge.target())?;
            let weight = edge.weight();
            Some((from, to, weight))
        })
    }

    /// Get the number of nodes in the DAG
    ///
    /// # Returns
    ///
    /// The count of nodes
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// assert_eq!(dag.node_count(), 0);
    /// dag.add_node("bead-001".to_string())?;
    /// assert_eq!(dag.node_count(), 1);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges in the DAG
    ///
    /// # Returns
    ///
    /// The count of edges
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string())?;
    /// dag.add_node("bead-002".to_string())?;
    /// assert_eq!(dag.edge_count(), 0);
    /// dag.add_edge(
    ///     "bead-001".to_string(),
    ///     "bead-002".to_string(),
    ///     DependencyType::BlockingDependency
    /// )?;
    /// assert_eq!(dag.edge_count(), 1);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Check if a node exists in the DAG
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to check
    ///
    /// # Returns
    ///
    /// `true` if the node exists, `false` otherwise
    #[must_use]
    pub fn contains_node(&self, bead_id: &BeadId) -> bool {
        self.node_map.contains_key(bead_id)
    }

    // ==================== Query Methods ====================

    /// Get direct dependencies of a node (incoming edges).
    ///
    /// Returns the BeadIds that this node directly depends on.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to query
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<BeadId>)` - List of direct dependencies
    /// * `Err(DagError)` - If the node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let deps = dag.get_dependencies(&"b".to_string())?;
    /// assert_eq!(deps, vec!["a".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_dependencies(&self, bead_id: &BeadId) -> DagResult<Vec<BeadId>> {
        let node_index = self.get_node_index(bead_id)?;

        let mut deps: Vec<BeadId> = self
            .graph
            .neighbors_directed(node_index, Direction::Incoming)
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect();

        // Sort deterministically by BeadId
        deps.sort();
        Ok(deps)
    }

    /// Get direct dependents of a node (outgoing edges).
    ///
    /// Returns the BeadIds that directly depend on this node.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to query
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<BeadId>)` - List of direct dependents
    /// * `Err(DagError)` - If the node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let dependents = dag.get_dependents(&"a".to_string())?;
    /// assert_eq!(dependents, vec!["b".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_dependents(&self, bead_id: &BeadId) -> DagResult<Vec<BeadId>> {
        let node_index = self.get_node_index(bead_id)?;

        let mut dependents: Vec<BeadId> = self
            .graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect();

        // Sort deterministically by BeadId
        dependents.sort();
        Ok(dependents)
    }

    /// Get all ancestors of a node (transitive closure of dependencies).
    ///
    /// Returns all BeadIds that this node transitively depends on.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to query
    ///
    /// # Returns
    ///
    /// * `Ok(HashSet<BeadId>)` - Set of all ancestors
    /// * `Err(DagError)` - If the node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_node("c".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let ancestors = dag.get_all_ancestors(&"c".to_string())?;
    /// assert!(ancestors.contains(&"a".to_string()));
    /// assert!(ancestors.contains(&"b".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_all_ancestors(&self, bead_id: &BeadId) -> DagResult<HashSet<BeadId>> {
        let node_index = self.get_node_index(bead_id)?;

        let mut ancestors = HashSet::new();
        let reversed = Reversed(&self.graph);
        let mut dfs = Dfs::new(reversed, node_index);

        // Skip the starting node
        let _ = dfs.next(reversed);

        while let Some(idx) = dfs.next(reversed) {
            if let Some(id) = self.graph.node_weight(idx) {
                ancestors.insert(id.clone());
            }
        }

        Ok(ancestors)
    }

    /// Get all descendants of a node (transitive closure of dependents).
    ///
    /// Returns all BeadIds that transitively depend on this node.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to query
    ///
    /// # Returns
    ///
    /// * `Ok(HashSet<BeadId>)` - Set of all descendants
    /// * `Err(DagError)` - If the node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_node("c".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let descendants = dag.get_all_descendants(&"a".to_string())?;
    /// assert!(descendants.contains(&"b".to_string()));
    /// assert!(descendants.contains(&"c".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_all_descendants(&self, bead_id: &BeadId) -> DagResult<HashSet<BeadId>> {
        let node_index = self.get_node_index(bead_id)?;

        let mut descendants = HashSet::new();
        let mut bfs = Bfs::new(&self.graph, node_index);

        // Skip the starting node
        let _ = bfs.next(&self.graph);

        while let Some(idx) = bfs.next(&self.graph) {
            if let Some(id) = self.graph.node_weight(idx) {
                descendants.insert(id.clone());
            }
        }

        Ok(descendants)
    }

    /// Get root nodes (nodes with no incoming edges).
    ///
    /// Root nodes have no dependencies and can start immediately.
    ///
    /// # Returns
    ///
    /// Vector of BeadIds that are roots
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let roots = dag.get_roots();
    /// assert_eq!(roots, vec!["a".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_roots(&self) -> Vec<BeadId> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                self.graph
                    .edges_directed(idx, Direction::Incoming)
                    .filter(|edge| *edge.weight() == DependencyType::BlockingDependency)
                    .count()
                    == 0
            })
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect()
    }

    /// Get leaf nodes (nodes with no outgoing edges).
    ///
    /// Leaf nodes have no dependents.
    ///
    /// # Returns
    ///
    /// Vector of BeadIds that are leaves
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let leaves = dag.get_leaves();
    /// assert_eq!(leaves, vec!["b".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_leaves(&self) -> Vec<BeadId> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Outgoing)
                    .count()
                    == 0
            })
            .filter_map(|idx| self.graph.node_weight(idx).cloned())
            .collect()
    }

    // ==================== Ready Detection ====================

    /// Get all nodes that are ready to execute.
    ///
    /// A node is ready if all of its BlockingDependency dependencies are in the
    /// completed set. PreferredOrder dependencies do not block execution.
    ///
    /// # Arguments
    ///
    /// * `completed` - Set of BeadIds that have already completed
    ///
    /// # Returns
    ///
    /// Vector of BeadIds that are ready to execute
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    /// use im::HashSet;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let completed = HashSet::new();
    /// let ready = dag.get_ready_nodes(&completed);
    /// assert_eq!(ready, vec!["a".to_string()]);
    ///
    /// let mut completed = HashSet::new();
    /// completed.insert("a".to_string());
    /// let ready = dag.get_ready_nodes(&completed);
    /// assert_eq!(ready, vec!["b".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_ready_nodes(&self, completed: &HashSet<BeadId>) -> Vec<BeadId> {
        let mut ready: Vec<BeadId> = self
            .graph
            .node_indices()
            .filter_map(|idx| {
                let bead_id = self.graph.node_weight(idx)?;

                // Skip already completed nodes
                if completed.contains(bead_id) {
                    return None;
                }

                // Check if all blocking dependencies are completed
                let all_blocking_deps_complete = self
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .filter(|edge| *edge.weight() == DependencyType::BlockingDependency)
                    .all(|edge| {
                        self.graph
                            .node_weight(edge.source())
                            .map(|dep_id| completed.contains(dep_id))
                            .unwrap_or(false)
                    });

                if all_blocking_deps_complete {
                    Some(bead_id.clone())
                } else {
                    None
                }
            })
            .collect();

        // Sort deterministically by BeadId
        ready.sort();
        ready
    }

    /// Get all beads that are ready to execute.
    ///
    /// Alias for `get_ready_nodes` for consistency with bead terminology.
    ///
    /// # Arguments
    ///
    /// * `completed` - Set of BeadIds that have already completed
    ///
    /// # Returns
    ///
    /// Vector of BeadIds that are ready to execute (deterministically sorted)
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    /// use im::HashSet;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let completed = HashSet::new();
    /// let ready = dag.get_ready_beads(&completed);
    /// assert_eq!(ready, vec!["a".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_ready_beads(&self, completed: &HashSet<BeadId>) -> Vec<BeadId> {
        self.get_ready_nodes(completed)
    }

    /// Get all nodes that are blocked (not ready to execute).
    ///
    /// A node is blocked if it has at least one BlockingDependency that is not
    /// in the completed set.
    ///
    /// # Arguments
    ///
    /// * `completed` - Set of BeadIds that have already completed
    ///
    /// # Returns
    ///
    /// Vector of BeadIds that are blocked
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    /// use im::HashSet;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let completed = HashSet::new();
    /// let blocked = dag.get_blocked_nodes(&completed);
    /// assert_eq!(blocked, vec!["b".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_blocked_nodes(&self, completed: &HashSet<BeadId>) -> Vec<BeadId> {
        self.graph
            .node_indices()
            .filter_map(|idx| {
                let bead_id = self.graph.node_weight(idx)?;

                // Skip already completed nodes
                if completed.contains(bead_id) {
                    return None;
                }

                // Check if any blocking dependency is not completed
                let has_incomplete_blocking_dep = self
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .filter(|edge| *edge.weight() == DependencyType::BlockingDependency)
                    .any(|edge| {
                        self.graph
                            .node_weight(edge.source())
                            .map(|dep_id| !completed.contains(dep_id))
                            .unwrap_or(true)
                    });

                if has_incomplete_blocking_dep {
                    Some(bead_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if a specific node is ready to execute.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to check
    /// * `completed` - Set of BeadIds that have already completed
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Node is ready (all blocking deps complete)
    /// * `Ok(false)` - Node is blocked
    /// * `Err(DagError)` - Node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    /// use im::HashSet;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let completed = HashSet::new();
    /// assert!(dag.is_ready(&"a".to_string(), &completed)?);
    /// assert!(!dag.is_ready(&"b".to_string(), &completed)?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_ready(&self, bead_id: &BeadId, completed: &HashSet<BeadId>) -> DagResult<bool> {
        let node_index = self.get_node_index(bead_id)?;

        let all_blocking_deps_complete = self
            .graph
            .edges_directed(node_index, Direction::Incoming)
            .filter(|edge| *edge.weight() == DependencyType::BlockingDependency)
            .all(|edge| {
                self.graph
                    .node_weight(edge.source())
                    .map(|dep_id| completed.contains(dep_id))
                    .unwrap_or(false)
            });

        Ok(all_blocking_deps_complete)
    }

    // ==================== Ordering ====================

    /// Perform a topological sort using DFS-based algorithm.
    ///
    /// Returns nodes in an order where all dependencies come before their dependents.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<BeadId>)` - Topologically sorted list of BeadIds
    /// * `Err(DagError)` - If the graph contains a cycle
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_node("c".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let sorted = dag.topological_sort()?;
    /// // a must come before b, b must come before c
    /// let pos_a = sorted.iter().position(|x| x == "a").ok_or("missing a")?;
    /// let pos_b = sorted.iter().position(|x| x == "b").ok_or("missing b")?;
    /// let pos_c = sorted.iter().position(|x| x == "c").ok_or("missing c")?;
    /// assert!(pos_a < pos_b);
    /// assert!(pos_b < pos_c);
    /// # Ok(())
    /// # }
    /// ```
    pub fn topological_sort(&self) -> DagResult<Vec<BeadId>> {
        toposort(&self.graph, None)
            .map(|indices| {
                indices
                    .into_iter()
                    .filter_map(|idx| self.graph.node_weight(idx).cloned())
                    .collect()
            })
            .map_err(|cycle| {
                let cycle_node = self
                    .graph
                    .node_weight(cycle.node_id())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                DagError::cycle_detected(vec![cycle_node])
            })
    }

    /// Perform a topological sort using Kahn's algorithm.
    ///
    /// This algorithm processes nodes with zero in-degree first, which can be
    /// more intuitive for understanding execution order.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<BeadId>)` - Topologically sorted list of BeadIds
    /// * `Err(DagError)` - If the graph contains a cycle
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let sorted = dag.topological_sort_kahn()?;
    /// assert_eq!(sorted[0], "a");
    /// assert_eq!(sorted[1], "b");
    /// # Ok(())
    /// # }
    /// ```
    pub fn topological_sort_kahn(&self) -> DagResult<Vec<BeadId>> {
        let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        // Initialize in-degrees
        for idx in self.graph.node_indices() {
            let degree = self
                .graph
                .neighbors_directed(idx, Direction::Incoming)
                .count();
            in_degree.insert(idx, degree);
            if degree == 0 {
                queue.push_back(idx);
            }
        }

        // Process nodes with zero in-degree
        while let Some(idx) = queue.pop_front() {
            if let Some(bead_id) = self.graph.node_weight(idx) {
                result.push(bead_id.clone());
            }

            for neighbor in self.graph.neighbors_directed(idx, Direction::Outgoing) {
                if let Some(degree) = in_degree.get_mut(&neighbor) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Check for cycle
        if result.len() != self.graph.node_count() {
            // Find nodes that weren't processed (part of cycle)
            let processed: HashSet<BeadId> = result.iter().cloned().collect();
            let cycle_nodes: Vec<BeadId> = self
                .graph
                .node_weights()
                .filter(|id| !processed.contains(*id))
                .cloned()
                .collect();
            return Err(DagError::cycle_detected(cycle_nodes));
        }

        Ok(result)
    }

    /// Compute the critical path through the DAG.
    ///
    /// The critical path is the longest path through the graph, considering
    /// node weights (durations). This represents the minimum time to complete
    /// all work.
    ///
    /// # Arguments
    ///
    /// * `weights` - Map from BeadId to estimated Duration
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<BeadId>)` - Nodes on the critical path, in order
    /// * `Err(DagError)` - If the graph contains a cycle
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    /// use im::HashMap;
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_node("c".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// dag.add_edge("a".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let mut weights = HashMap::new();
    /// weights.insert("a".to_string(), Duration::from_secs(1));
    /// weights.insert("b".to_string(), Duration::from_secs(5));
    /// weights.insert("c".to_string(), Duration::from_secs(2));
    ///
    /// let critical = dag.critical_path(&weights)?;
    /// // Critical path is a -> b (total 6s) not a -> c (total 3s)
    /// assert!(critical.contains(&"a".to_string()));
    /// assert!(critical.contains(&"b".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn critical_path(&self, weights: &HashMap<BeadId, Duration>) -> DagResult<Vec<BeadId>> {
        let topo_order = self.topological_sort()?;

        if topo_order.is_empty() {
            return Ok(Vec::new());
        }

        // dist[node] = (max distance to reach this node, predecessor on critical path)
        let mut dist: HashMap<BeadId, (Duration, Option<BeadId>)> = HashMap::new();

        // Initialize all distances
        for bead_id in &topo_order {
            let weight = weights.get(bead_id).copied().unwrap_or(Duration::ZERO);
            dist.insert(bead_id.clone(), (weight, None));
        }

        // Process in topological order
        for bead_id in &topo_order {
            let node_idx = match self.node_map.get(bead_id) {
                Some(idx) => *idx,
                None => continue,
            };

            let current_dist = dist.get(bead_id).map(|(d, _)| *d).unwrap_or(Duration::ZERO);

            // Update distances to all neighbors
            for edge in self.graph.edges_directed(node_idx, Direction::Outgoing) {
                if *edge.weight() != DependencyType::BlockingDependency {
                    continue;
                }

                let neighbor_idx = edge.target();
                if let Some(neighbor_id) = self.graph.node_weight(neighbor_idx) {
                    let neighbor_weight =
                        weights.get(neighbor_id).copied().unwrap_or(Duration::ZERO);
                    let new_dist = current_dist + neighbor_weight;

                    let should_update = dist
                        .get(neighbor_id)
                        .map(|(d, _)| new_dist > *d)
                        .unwrap_or(true);

                    if should_update {
                        dist.insert(neighbor_id.clone(), (new_dist, Some(bead_id.clone())));
                    }
                }
            }
        }

        // Find the node with maximum distance (end of critical path)
        let end_node = dist
            .iter()
            .max_by_key(|(_, (d, _))| *d)
            .map(|(id, _)| id.clone());

        let end_node = match end_node {
            Some(n) => n,
            None => return Ok(Vec::new()),
        };

        // Backtrack to find the critical path
        let mut path = Vec::new();
        let mut current = Some(end_node);

        while let Some(node) = current {
            path.push(node.clone());
            current = dist.get(&node).and_then(|(_, pred)| pred.clone());
        }

        path.reverse();
        Ok(path)
    }

    // ==================== Validation ====================

    /// Check if the graph contains any cycle.
    ///
    /// # Returns
    ///
    /// `true` if the graph has at least one cycle, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// assert!(!dag.has_cycle());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn has_cycle(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }

    /// Find all cycles in the graph.
    ///
    /// Uses Tarjan's strongly connected components algorithm to find cycles.
    ///
    /// # Returns
    ///
    /// Vector of cycles, where each cycle is a vector of BeadIds
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// let dag = WorkflowDAG::new();
    /// let cycles = dag.find_cycles();
    /// assert!(cycles.is_empty());
    /// ```
    #[must_use]
    pub fn find_cycles(&self) -> Vec<Vec<BeadId>> {
        let sccs = tarjan_scc(&self.graph);

        sccs.into_iter()
            .filter(|scc| {
                // A cycle exists if SCC has more than one node, or if a single
                // node has a self-loop
                if scc.len() > 1 {
                    true
                } else if scc.len() == 1 {
                    let node = scc[0];
                    self.graph
                        .neighbors_directed(node, Direction::Outgoing)
                        .any(|n| n == node)
                } else {
                    false
                }
            })
            .map(|scc| {
                scc.into_iter()
                    .filter_map(|idx| self.graph.node_weight(idx).cloned())
                    .collect()
            })
            .collect()
    }

    /// Validate that there are no self-loops in the graph.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - No self-loops found
    /// * `Err(DagError)` - Self-loop detected
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// let dag = WorkflowDAG::new();
    /// assert!(dag.validate_no_self_loops().is_ok());
    /// ```
    pub fn validate_no_self_loops(&self) -> DagResult<()> {
        for node_idx in self.graph.node_indices() {
            let has_self_loop = self
                .graph
                .neighbors_directed(node_idx, Direction::Outgoing)
                .any(|n| n == node_idx);

            if has_self_loop {
                let bead_id = self
                    .graph
                    .node_weight(node_idx)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(DagError::self_loop(bead_id));
            }
        }
        Ok(())
    }

    /// Check if the graph is connected (weakly connected for directed graphs).
    ///
    /// A graph is connected if there is a path between every pair of nodes
    /// when edge direction is ignored.
    ///
    /// # Returns
    ///
    /// `true` if the graph is connected or empty, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// assert!(dag.is_connected());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_connected(&self) -> bool {
        if self.graph.node_count() <= 1 {
            return true;
        }

        // Get the first node
        let start = match self.graph.node_indices().next() {
            Some(idx) => idx,
            None => return true,
        };

        // BFS ignoring edge direction
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(node) = queue.pop_front() {
            // Visit both incoming and outgoing neighbors
            for neighbor in self.graph.neighbors_directed(node, Direction::Outgoing) {
                if visited.insert(neighbor).is_none() {
                    queue.push_back(neighbor);
                }
            }
            for neighbor in self.graph.neighbors_directed(node, Direction::Incoming) {
                if visited.insert(neighbor).is_none() {
                    queue.push_back(neighbor);
                }
            }
        }

        visited.len() == self.graph.node_count()
    }

    // ==================== Mutation ====================

    /// Remove a node and all its edges from the DAG.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The BeadId to remove
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Node was removed
    /// * `Err(DagError)` - Node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::WorkflowDAG;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// assert_eq!(dag.node_count(), 1);
    ///
    /// dag.remove_node(&"a".to_string())?;
    /// assert_eq!(dag.node_count(), 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_node(&mut self, bead_id: &BeadId) -> DagResult<()> {
        let node_index = self.get_node_index(bead_id)?;

        // Remove from graph (this also removes all connected edges)
        self.graph.remove_node(node_index);

        // Remove from node_map
        self.node_map.remove(bead_id);

        // Rebuild node_map since indices may have changed after removal
        self.rebuild_node_map();

        Ok(())
    }

    /// Remove an edge between two nodes.
    ///
    /// # Arguments
    ///
    /// * `from` - Source BeadId
    /// * `to` - Target BeadId
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Edge was removed (or didn't exist)
    /// * `Err(DagError)` - One of the nodes doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// assert_eq!(dag.edge_count(), 1);
    ///
    /// dag.remove_edge(&"a".to_string(), &"b".to_string())?;
    /// assert_eq!(dag.edge_count(), 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_edge(&mut self, from: &BeadId, to: &BeadId) -> DagResult<()> {
        let from_index = self.get_node_index(from)?;
        let to_index = self.get_node_index(to)?;

        // Find and remove the edge
        if let Some(edge_idx) = self.graph.find_edge(from_index, to_index) {
            self.graph.remove_edge(edge_idx);
        }

        Ok(())
    }

    // ==================== Subgraph Operations ====================

    /// Create a subgraph containing only the specified nodes.
    ///
    /// Edges between included nodes are preserved.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The BeadIds to include in the subgraph
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowDAG)` - New DAG containing only the specified nodes
    /// * `Err(DagError)` - If any specified node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_node("c".to_string())?;
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let subgraph = dag.subgraph(&["a".to_string(), "b".to_string()])?;
    /// assert_eq!(subgraph.node_count(), 2);
    /// assert_eq!(subgraph.edge_count(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn subgraph(&self, nodes: &[BeadId]) -> DagResult<WorkflowDAG> {
        let node_set: HashSet<&BeadId> = nodes.iter().collect();

        // Verify all nodes exist
        for bead_id in nodes {
            if !self.node_map.contains_key(bead_id) {
                return Err(DagError::node_not_found(bead_id.clone()));
            }
        }

        let mut subgraph = WorkflowDAG::new();

        // Add nodes
        for bead_id in nodes {
            subgraph.add_node(bead_id.clone())?;
        }

        // Add edges between included nodes
        for edge in self.graph.edge_references() {
            let from = self.graph.node_weight(edge.source());
            let to = self.graph.node_weight(edge.target());

            if let (Some(from_id), Some(to_id)) = (from, to) {
                if node_set.contains(from_id) && node_set.contains(to_id) {
                    subgraph.add_edge(from_id.clone(), to_id.clone(), *edge.weight())?;
                }
            }
        }

        Ok(subgraph)
    }

    /// Create an induced subgraph containing a node and all its ancestors and descendants.
    ///
    /// This is useful for extracting the complete context around a specific bead.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The central BeadId
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowDAG)` - New DAG with all ancestors, the node, and all descendants
    /// * `Err(DagError)` - If the node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("a".to_string())?;
    /// dag.add_node("b".to_string())?;
    /// dag.add_node("c".to_string())?;
    /// dag.add_node("d".to_string())?; // disconnected
    /// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
    /// dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
    ///
    /// let induced = dag.induced_subgraph(&"b".to_string())?;
    /// assert_eq!(induced.node_count(), 3); // a, b, c (not d)
    /// # Ok(())
    /// # }
    /// ```
    pub fn induced_subgraph(&self, bead_id: &BeadId) -> DagResult<WorkflowDAG> {
        // Verify node exists
        let _ = self.get_node_index(bead_id)?;

        let mut nodes: HashSet<BeadId> = HashSet::new();

        // Add the central node
        nodes.insert(bead_id.clone());

        // Add all ancestors
        let ancestors = self.get_all_ancestors(bead_id)?;
        nodes.extend(ancestors);

        // Add all descendants
        let descendants = self.get_all_descendants(bead_id)?;
        nodes.extend(descendants);

        let node_vec: Vec<BeadId> = nodes.into_iter().collect();
        self.subgraph(&node_vec)
    }

    // ==================== Helper Methods ====================

    /// Get the NodeIndex for a BeadId, or return an error if not found.
    fn get_node_index(&self, bead_id: &BeadId) -> DagResult<NodeIndex> {
        self.node_map
            .get(bead_id)
            .copied()
            .ok_or_else(|| DagError::node_not_found(bead_id.clone()))
    }

    /// Rebuild the node_map after a node removal.
    ///
    /// This is necessary because petgraph reuses node indices.
    fn rebuild_node_map(&mut self) {
        self.node_map.clear();
        for idx in self.graph.node_indices() {
            if let Some(bead_id) = self.graph.node_weight(idx) {
                self.node_map.insert(bead_id.clone(), idx);
            }
        }
    }
}

impl Default for WorkflowDAG {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_to_owned)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dag_is_empty() {
        let dag = WorkflowDAG::new();
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn test_add_single_node() {
        let mut dag = WorkflowDAG::new();
        let result = dag.add_node("bead-001".to_string());
        assert!(result.is_ok());
        assert_eq!(dag.node_count(), 1);
    }

    #[test]
    fn test_add_duplicate_node_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())?;
        let result = dag.add_node("bead-001".to_string());
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_edge_between_two_nodes() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())?;
        dag.add_node("bead-002".to_string())?;

        let result = dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_ok());
        assert_eq!(dag.edge_count(), 1);
        Ok(())
    }

    #[test]
    fn test_add_edge_with_nonexistent_source_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-002".to_string())?;

        let result = dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_edge_with_nonexistent_target_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())?;

        let result = dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_nodes_iterator() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())?;
        dag.add_node("bead-002".to_string())?;

        let nodes: Vec<&String> = dag.nodes().collect();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&&"bead-001".to_string()));
        assert!(nodes.contains(&&"bead-002".to_string()));
        Ok(())
    }

    #[test]
    fn test_edges_iterator() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())?;
        dag.add_node("bead-002".to_string())?;
        dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let edges: Vec<_> = dag.edges().collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, &"bead-001".to_string());
        assert_eq!(edges[0].1, &"bead-002".to_string());
        assert_eq!(*edges[0].2, DependencyType::BlockingDependency);
        Ok(())
    }

    #[test]
    fn test_multiple_edges_different_types() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())?;
        dag.add_node("bead-002".to_string())?;
        dag.add_node("bead-003".to_string())?;

        dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        )?;

        dag.add_edge(
            "bead-001".to_string(),
            "bead-003".to_string(),
            DependencyType::PreferredOrder,
        )?;

        assert_eq!(dag.edge_count(), 2);
        Ok(())
    }

    // ==================== Query Method Tests ====================

    #[test]
    fn test_get_dependencies() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let deps = dag.get_dependencies(&"c".to_string())?;
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"a".to_string()));
        assert!(deps.contains(&"b".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_dependents() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "a".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let dependents = dag.get_dependents(&"a".to_string())?;
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"b".to_string()));
        assert!(dependents.contains(&"c".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_all_ancestors() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_node("d".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "c".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let ancestors = dag.get_all_ancestors(&"d".to_string())?;
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&"a".to_string()));
        assert!(ancestors.contains(&"b".to_string()));
        assert!(ancestors.contains(&"c".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_all_descendants() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let descendants = dag.get_all_descendants(&"a".to_string())?;
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&"b".to_string()));
        assert!(descendants.contains(&"c".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_roots() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let roots = dag.get_roots();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&"a".to_string()));
        assert!(roots.contains(&"b".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_leaves() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "a".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let leaves = dag.get_leaves();
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&"b".to_string()));
        assert!(leaves.contains(&"c".to_string()));
        Ok(())
    }

    // ==================== Ready Detection Tests ====================

    #[test]
    fn test_get_ready_nodes() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        // Initially only a is ready
        let completed = HashSet::new();
        let ready = dag.get_ready_nodes(&completed);
        assert_eq!(ready, vec!["a".to_string()]);

        // After a completes, b is ready
        let mut completed = HashSet::new();
        completed.insert("a".to_string());
        let ready = dag.get_ready_nodes(&completed);
        assert_eq!(ready, vec!["b".to_string()]);
        Ok(())
    }

    #[test]
    fn test_preferred_order_does_not_block() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::PreferredOrder,
        )?;

        // b should be ready even without a completing (PreferredOrder is non-blocking)
        let completed = HashSet::new();
        let ready = dag.get_ready_nodes(&completed);
        assert!(ready.contains(&"a".to_string()));
        assert!(ready.contains(&"b".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_blocked_nodes() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let completed = HashSet::new();
        let blocked = dag.get_blocked_nodes(&completed);
        assert_eq!(blocked, vec!["b".to_string()]);
        Ok(())
    }

    #[test]
    fn test_is_ready() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let completed = HashSet::new();
        assert!(dag.is_ready(&"a".to_string(), &completed)?);
        assert!(!dag.is_ready(&"b".to_string(), &completed)?);
        Ok(())
    }

    // ==================== Ordering Tests ====================

    #[test]
    fn test_topological_sort() -> Result<(), Box<dyn std::error::Error>> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let sorted = dag.topological_sort()?;
        let pos_a = sorted.iter().position(|x| x == "a").ok_or("find a")?;
        let pos_b = sorted.iter().position(|x| x == "b").ok_or("find b")?;
        let pos_c = sorted.iter().position(|x| x == "c").ok_or("find c")?;
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
        Ok(())
    }

    #[test]
    fn test_topological_sort_kahn() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let sorted = dag.topological_sort_kahn()?;
        assert_eq!(sorted[0], "a");
        assert_eq!(sorted[1], "b");
        assert_eq!(sorted[2], "c");
        Ok(())
    }

    #[test]
    fn test_critical_path() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "a".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let mut weights = HashMap::new();
        weights.insert("a".to_string(), Duration::from_secs(1));
        weights.insert("b".to_string(), Duration::from_secs(5));
        weights.insert("c".to_string(), Duration::from_secs(2));

        let critical = dag.critical_path(&weights)?;
        assert!(critical.contains(&"a".to_string()));
        assert!(critical.contains(&"b".to_string()));
        // c is not on critical path since a->b is longer
        Ok(())
    }

    // ==================== Validation Tests ====================

    #[test]
    fn test_has_cycle_false() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        assert!(!dag.has_cycle());
        Ok(())
    }

    #[test]
    fn test_find_cycles_empty() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let cycles = dag.find_cycles();
        assert!(cycles.is_empty());
        Ok(())
    }

    #[test]
    fn test_validate_no_self_loops() {
        let dag = WorkflowDAG::new();
        assert!(dag.validate_no_self_loops().is_ok());
    }

    #[test]
    fn test_is_connected_true() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        assert!(dag.is_connected());
        Ok(())
    }

    #[test]
    fn test_is_connected_false() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        // No edge between a and b

        assert!(!dag.is_connected());
        Ok(())
    }

    // ==================== Mutation Tests ====================

    #[test]
    fn test_remove_node() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);

        dag.remove_node(&"a".to_string())?;
        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
        Ok(())
    }

    #[test]
    fn test_remove_edge() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        assert_eq!(dag.edge_count(), 1);
        dag.remove_edge(&"a".to_string(), &"b".to_string())?;
        assert_eq!(dag.edge_count(), 0);
        Ok(())
    }

    // ==================== Subgraph Tests ====================

    #[test]
    fn test_subgraph() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let subgraph = dag.subgraph(&["a".to_string(), "b".to_string()])?;
        assert_eq!(subgraph.node_count(), 2);
        assert_eq!(subgraph.edge_count(), 1);
        assert!(subgraph.contains_node(&"a".to_string()));
        assert!(subgraph.contains_node(&"b".to_string()));
        assert!(!subgraph.contains_node(&"c".to_string()));
        Ok(())
    }

    #[test]
    fn test_induced_subgraph() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_node("d".to_string())?; // disconnected
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let induced = dag.induced_subgraph(&"b".to_string())?;
        assert_eq!(induced.node_count(), 3); // a, b, c (not d)
        assert!(induced.contains_node(&"a".to_string()));
        assert!(induced.contains_node(&"b".to_string()));
        assert!(induced.contains_node(&"c".to_string()));
        assert!(!induced.contains_node(&"d".to_string()));
        Ok(())
    }

    #[test]
    fn test_contains_node() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;

        assert!(dag.contains_node(&"a".to_string()));
        assert!(!dag.contains_node(&"b".to_string()));
        Ok(())
    }

    // ==================== Add Dependency Tests ====================

    #[test]
    fn test_add_dependency_blocking() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;
        dag.add_node("task-b".to_string())?;

        let result = dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_ok());
        assert_eq!(dag.edge_count(), 1);
        Ok(())
    }

    #[test]
    fn test_add_dependency_preferred() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;
        dag.add_node("task-b".to_string())?;

        let result = dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::PreferredOrder,
        );
        assert!(result.is_ok());
        assert_eq!(dag.edge_count(), 1);
        Ok(())
    }

    #[test]
    fn test_add_dependency_self_loop_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;

        let result = dag.add_dependency(
            "task-a".to_string(),
            "task-a".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(DagError::SelfLoopDetected(_))));
        Ok(())
    }

    #[test]
    fn test_add_dependency_source_not_found_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-b".to_string())?;

        let result = dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(DagError::NodeNotFound(_))));
        Ok(())
    }

    #[test]
    fn test_add_dependency_target_not_found_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;

        let result = dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(DagError::NodeNotFound(_))));
        Ok(())
    }

    #[test]
    fn test_add_dependency_duplicate_edge_fails() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;
        dag.add_node("task-b".to_string())?;

        // Add first dependency
        dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        // Try to add duplicate
        let result = dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(DagError::EdgeAlreadyExists(_, _))));
        Ok(())
    }

    #[test]
    fn test_add_dependency_preserves_dependency_type() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;
        dag.add_node("task-b".to_string())?;

        dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        // Verify the edge type was preserved
        let edges: Vec<_> = dag.edges().collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(*edges[0].2, DependencyType::BlockingDependency);
        Ok(())
    }

    #[test]
    fn test_add_dependency_multiple_edges_from_same_source() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("task-a".to_string())?;
        dag.add_node("task-b".to_string())?;
        dag.add_node("task-c".to_string())?;

        // Add multiple dependencies from the same source
        dag.add_dependency(
            "task-a".to_string(),
            "task-b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_dependency(
            "task-a".to_string(),
            "task-c".to_string(),
            DependencyType::PreferredOrder,
        )?;

        assert_eq!(dag.edge_count(), 2);
        Ok(())
    }
}

    // ==================== Bead src-3clb: Deterministic Query Tests ====================

    #[test]
    fn test_get_dependencies_deterministic_ordering() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("c".to_string())?;
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("d".to_string())?;

        // Add dependencies in non-alphabetical order
        dag.add_edge(
            "c".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "a".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let deps = dag.get_dependencies(&"d".to_string())?;

        // Must be deterministically sorted by BeadId
        assert_eq!(deps, vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        // No duplicates allowed
        let unique_deps: std::collections::HashSet<_> = deps.iter().collect();
        assert_eq!(unique_deps.len(), deps.len());

        Ok(())
    }

    #[test]
    fn test_get_dependents_deterministic_ordering() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("d".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_node("b".to_string())?;

        // Add dependents in non-alphabetical order
        dag.add_edge(
            "a".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "a".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;

        let dependents = dag.get_dependents(&"a".to_string())?;

        // Must be deterministically sorted by BeadId
        assert_eq!(
            dependents,
            vec!["b".to_string(), "c".to_string(), "d".to_string()]
        );

        // No duplicates allowed
        let unique_dependents: std::collections::HashSet<_> = dependents.iter().collect();
        assert_eq!(unique_dependents.len(), dependents.len());

        Ok(())
    }

    #[test]
    fn test_get_ready_beads_basic() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("c".to_string())?;
        dag.add_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "c".to_string(),
            DependencyType::BlockingDependency,
        )?;

        // Initially only a is ready (in-degree=0)
        let completed = HashSet::new();
        let ready = dag.get_ready_beads(&completed);

        // Must be deterministically sorted
        assert_eq!(ready, vec!["a".to_string()]);

        // After a completes, b is ready
        let mut completed = HashSet::new();
        completed.insert("a".to_string());
        let ready = dag.get_ready_beads(&completed);
        assert_eq!(ready, vec!["b".to_string()]);

        Ok(())
    }

    #[test]
    fn test_get_ready_nodes_multiple_roots_sorted() -> DagResult<()> {
        let mut dag = WorkflowDAG::new();
        dag.add_node("c".to_string())?;
        dag.add_node("a".to_string())?;
        dag.add_node("b".to_string())?;
        dag.add_node("d".to_string())?;

        // Add edges to create multiple roots
        dag.add_edge(
            "a".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "b".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;
        dag.add_edge(
            "c".to_string(),
            "d".to_string(),
            DependencyType::BlockingDependency,
        )?;

        // All three roots should be ready, sorted
        let completed = HashSet::new();
        let ready = dag.get_ready_nodes(&completed);

        // Must be deterministically sorted: a, b, c
        assert_eq!(
            ready,
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string()
            ]
        );

        Ok(())
    }

    #[test]
    fn test_query_performance_100_nodes() -> DagResult<()> {
        use std::time::Instant;

        let mut dag = WorkflowDAG::new();

        // Create 100 nodes
        for i in 0..100 {
            dag.add_node(format!("bead-{:03}", i))?;
        }

        // Create some dependencies
        for i in 0..90 {
            dag.add_edge(
                format!("bead-{:03}", i),
                format!("bead-{:03}", i + 1),
                DependencyType::BlockingDependency,
            )?;
        }

        // Test get_dependencies performance
        let start = Instant::now();
        let deps = dag.get_dependencies(&"bead-050".to_string())?;
        let duration = start.elapsed();

        assert_eq!(deps.len(), 1); // bead-049
        assert!(duration.as_millis() < 10, "Query took {:?}", duration);

        // Test get_dependents performance
        let start = Instant::now();
        let dependents = dag.get_dependents(&"bead-050".to_string())?;
        let duration = start.elapsed();

        assert_eq!(dependents.len(), 1); // bead-051
        assert!(duration.as_millis() < 10, "Query took {:?}", duration);

        // Test get_ready_beads performance
        let completed = HashSet::new();
        let start = Instant::now();
        let ready = dag.get_ready_beads(&completed);
        let duration = start.elapsed();

        // beads 000, 091-099 are all roots (no incoming edges)
        assert_eq!(ready.len(), 10);
        assert!(duration.as_millis() < 10, "Query took {:?}", duration);

        Ok(())
    }
