//! # Workflow DAG
//!
//! Directed Acyclic Graph for workflow dependencies using petgraph.

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

pub use crate::{Error, Result};

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
    /// * `Err(Error)` if the bead_id already exists in the DAG
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
    pub fn add_node(&mut self, bead_id: BeadId) -> Result<()> {
        if self.node_map.contains_key(&bead_id) {
            return Err(Error::invalid_record(format!(
                "Node with BeadId '{}' already exists in the DAG",
                bead_id
            )));
        }

        let node_index = self.graph.add_node(bead_id.clone());
        self.node_map.insert(bead_id, node_index);

        Ok(())
    }

    /// Add an edge between two nodes
    ///
    /// # Arguments
    ///
    /// * `from_bead` - The BeadId of the source node
    /// * `to_bead` - The BeadId of the target node
    /// * `dep_type` - The type of dependency relationship
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the edge was added successfully
    /// * `Err(Error)` if either node doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use orchestrator::dag::{WorkflowDAG, DependencyType};
    ///
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string()).unwrap();
    /// dag.add_node("bead-002".to_string()).unwrap();
    /// let result = dag.add_edge(
    ///     "bead-001".to_string(),
    ///     "bead-002".to_string(),
    ///     DependencyType::BlockingDependency
    /// );
    /// assert!(result.is_ok());
    /// ```
    pub fn add_edge(
        &mut self,
        from_bead: BeadId,
        to_bead: BeadId,
        dep_type: DependencyType,
    ) -> Result<()> {
        let from_index = self.node_map.get(&from_bead).ok_or_else(|| {
            Error::invalid_record(format!(
                "Source node with BeadId '{}' not found in DAG",
                from_bead
            ))
        })?;

        let to_index = self.node_map.get(&to_bead).ok_or_else(|| {
            Error::invalid_record(format!(
                "Target node with BeadId '{}' not found in DAG",
                to_bead
            ))
        })?;

        self.graph.add_edge(*from_index, *to_index, dep_type);

        Ok(())
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
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string()).unwrap();
    /// dag.add_node("bead-002".to_string()).unwrap();
    /// let nodes: Vec<&String> = dag.nodes().collect();
    /// assert_eq!(nodes.len(), 2);
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
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string()).unwrap();
    /// dag.add_node("bead-002".to_string()).unwrap();
    /// dag.add_edge(
    ///     "bead-001".to_string(),
    ///     "bead-002".to_string(),
    ///     DependencyType::BlockingDependency
    /// ).unwrap();
    /// let edges: Vec<_> = dag.edges().collect();
    /// assert_eq!(edges.len(), 1);
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
    /// let mut dag = WorkflowDAG::new();
    /// assert_eq!(dag.node_count(), 0);
    /// dag.add_node("bead-001".to_string()).unwrap();
    /// assert_eq!(dag.node_count(), 1);
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
    /// let mut dag = WorkflowDAG::new();
    /// dag.add_node("bead-001".to_string()).unwrap();
    /// dag.add_node("bead-002".to_string()).unwrap();
    /// assert_eq!(dag.edge_count(), 0);
    /// dag.add_edge(
    ///     "bead-001".to_string(),
    ///     "bead-002".to_string(),
    ///     DependencyType::BlockingDependency
    /// ).unwrap();
    /// assert_eq!(dag.edge_count(), 1);
    /// ```
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

impl Default for WorkflowDAG {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
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
    fn test_add_duplicate_node_fails() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())
            .expect("First add should succeed");
        let result = dag.add_node("bead-001".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_add_edge_between_two_nodes() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())
            .expect("Should add first node");
        dag.add_node("bead-002".to_string())
            .expect("Should add second node");

        let result = dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_ok());
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_with_nonexistent_source_fails() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-002".to_string())
            .expect("Should add node");

        let result = dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_edge_with_nonexistent_target_fails() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())
            .expect("Should add node");

        let result = dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_nodes_iterator() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())
            .expect("Should add first node");
        dag.add_node("bead-002".to_string())
            .expect("Should add second node");

        let nodes: Vec<&String> = dag.nodes().collect();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&&"bead-001".to_string()));
        assert!(nodes.contains(&&"bead-002".to_string()));
    }

    #[test]
    fn test_edges_iterator() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())
            .expect("Should add first node");
        dag.add_node("bead-002".to_string())
            .expect("Should add second node");
        dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        )
        .expect("Should add edge");

        let edges: Vec<_> = dag.edges().collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, &"bead-001".to_string());
        assert_eq!(edges[0].1, &"bead-002".to_string());
        assert_eq!(*edges[0].2, DependencyType::BlockingDependency);
    }

    #[test]
    fn test_multiple_edges_different_types() {
        let mut dag = WorkflowDAG::new();
        dag.add_node("bead-001".to_string())
            .expect("Should add node 1");
        dag.add_node("bead-002".to_string())
            .expect("Should add node 2");
        dag.add_node("bead-003".to_string())
            .expect("Should add node 3");

        dag.add_edge(
            "bead-001".to_string(),
            "bead-002".to_string(),
            DependencyType::BlockingDependency,
        )
        .expect("Should add blocking edge");

        dag.add_edge(
            "bead-001".to_string(),
            "bead-003".to_string(),
            DependencyType::PreferredOrder,
        )
        .expect("Should add preferred order edge");

        assert_eq!(dag.edge_count(), 2);
    }
}
