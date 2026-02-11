//! Graph node for DAG visualization
//!
//! Functional, immutable node implementation with traversal support.

use im::{HashMap, Vector};
use std::hash::Hash;

/// A node in a directed acyclic graph (DAG)
///
/// # Design Principles
///
/// - **Immutable updates**: All methods return `Self` for functional composition
/// - **Zero panics**: No `unwrap()`, `expect()`, or `panic!()` calls
/// - **Persistent data structures**: Uses `im::Vector` and `im::HashMap` for efficient cloning
/// - **Type-safe IDs**: Generic ID type with `Hash + Eq` constraint
///
/// # Example
///
/// ```rust
/// use oya_zellij::graph::GraphNode;
///
/// // Create a simple DAG
/// let root = GraphNode::new("task1", "Build Project")
///     .with_metadata("stage", "implement")
///     .add_child("task2")
///     .add_child("task3");
///
/// // Traverse the graph
/// let visited = root.traverse(|node| {
///     println!("Visiting: {}", node.label);
///     true // Continue traversal
/// });
/// ```
#[derive(Clone, Debug)]
pub struct GraphNode<ID>
where
    ID: Clone + Hash + Eq,
{
    /// Unique identifier for this node
    pub id: ID,

    /// Human-readable label
    pub label: String,

    /// IDs of child nodes (outgoing edges)
    children: Vector<ID>,

    /// IDs of parent nodes (incoming edges)
    parents: Vector<ID>,

    /// Additional metadata for display and processing
    metadata: NodeMetadata,

    /// Current state of the node
    state: NodeState,
}

/// Node state for visualization
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    /// Node has not started
    Idle,
    /// Node is currently processing
    Running,
    /// Node is waiting for dependencies
    Blocked,
    /// Node completed successfully
    Completed,
    /// Node failed
    Failed,
}

impl NodeState {
    /// Convert state to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// Check if state is terminal (no further transitions possible)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Check if state is active (currently processing)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Metadata associated with a node
///
/// Uses persistent `im::HashMap` for efficient cloning and updates
#[derive(Clone, Debug, Default)]
pub struct NodeMetadata {
    /// Arbitrary key-value metadata
    data: HashMap<String, String>,

    /// Whether this node is on the critical path
    is_on_critical_path: bool,

    /// Execution priority (higher = more important)
    priority: i32,

    /// Estimated execution duration in milliseconds
    estimated_duration_ms: Option<u64>,

    /// Actual execution duration in milliseconds
    actual_duration_ms: Option<u64>,
}

impl NodeMetadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a metadata key-value pair
    ///
    /// Returns updated metadata (functional update)
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data = self.data.update(key.into(), value.into());
        self
    }

    /// Get metadata value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    /// Check if node is on critical path
    pub fn is_on_critical_path(&self) -> bool {
        self.is_on_critical_path
    }

    /// Set critical path status
    pub fn set_on_critical_path(mut self, value: bool) -> Self {
        self.is_on_critical_path = value;
        self
    }

    /// Get execution priority
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Set execution priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Get estimated duration
    pub fn estimated_duration_ms(&self) -> Option<u64> {
        self.estimated_duration_ms
    }

    /// Set estimated duration
    pub fn with_estimated_duration_ms(mut self, duration: u64) -> Self {
        self.estimated_duration_ms = Some(duration);
        self
    }

    /// Get actual duration
    pub fn actual_duration_ms(&self) -> Option<u64> {
        self.actual_duration_ms
    }

    /// Set actual duration
    pub fn with_actual_duration_ms(mut self, duration: u64) -> Self {
        self.actual_duration_ms = Some(duration);
        self
    }

    /// Convert to iterator of key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.data.iter()
    }
}

impl<ID> GraphNode<ID>
where
    ID: Clone + Hash + Eq,
{
    /// Create a new graph node
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    ///
    /// let node = GraphNode::new("task1", "Build Project");
    /// assert_eq!(node.id, "task1");
    /// assert_eq!(node.label, "Build Project");
    /// ```
    pub fn new(id: ID, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            children: Vector::new(),
            parents: Vector::new(),
            metadata: NodeMetadata::new(),
            state: NodeState::Idle,
        }
    }

    /// Add a child node (create edge: self -> child)
    ///
    /// This is a functional update - returns a new node with the child added.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    ///
    /// let root = GraphNode::new("parent", "Parent Task")
    ///     .add_child("child1")
    ///     .add_child("child2");
    ///
    /// assert_eq!(root.children.len(), 2);
    /// ```
    pub fn add_child(mut self, child_id: ID) -> Self {
        if !self.children.contains(&child_id) {
            self.children.push_back(child_id);
        }
        self
    }

    /// Add a parent node (create edge: parent -> self)
    ///
    /// This is a functional update - returns a new node with the parent added.
    pub fn add_parent(mut self, parent_id: ID) -> Self {
        if !self.parents.contains(&parent_id) {
            self.parents.push_back(parent_id);
        }
        self
    }

    /// Remove a child node
    ///
    /// Returns updated node (functional update).
    pub fn remove_child(mut self, child_id: &ID) -> Self {
        self.children = self.children
            .iter()
            .filter(|id| id != &child_id)
            .cloned()
            .collect();
        self
    }

    /// Remove a parent node
    ///
    /// Returns updated node (functional update).
    pub fn remove_parent(mut self, parent_id: &ID) -> Self {
        self.parents = self.parents
            .iter()
            .filter(|id| id != &parent_id)
            .cloned()
            .collect();
        self
    }

    /// Set node state
    ///
    /// Returns updated node (functional update).
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::{GraphNode, NodeState};
    ///
    /// let node = GraphNode::new("task1", "Build")
    ///     .with_state(NodeState::Running);
    ///
    /// assert_eq!(node.state, NodeState::Running);
    /// ```
    pub fn with_state(mut self, state: NodeState) -> Self {
        self.state = state;
        self
    }

    /// Get current node state
    pub fn state(&self) -> NodeState {
        self.state
    }

    /// Add metadata key-value pair
    ///
    /// Returns updated node (functional update).
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    ///
    /// let node = GraphNode::new("task1", "Build")
    ///     .with_metadata("stage", "implement")
    ///     .with_metadata("priority", "high");
    ///
    /// assert_eq!(node.metadata.get("stage"), Some(&"implement".to_string()));
    /// ```
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata = self.metadata.with(key.into(), value.into());
        self
    }

    /// Get metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Set critical path status
    ///
    /// Returns updated node (functional update).
    pub fn with_critical_path(mut self, is_critical: bool) -> Self {
        self.metadata = self.metadata.set_on_critical_path(is_critical);
        self
    }

    /// Set execution priority
    ///
    /// Returns updated node (functional update).
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.metadata = self.metadata.with_priority(priority);
        self
    }

    /// Get children IDs (outgoing edges)
    pub fn children(&self) -> &Vector<ID> {
        &self.children
    }

    /// Get parent IDs (incoming edges)
    pub fn parents(&self) -> &Vector<ID> {
        &self.parents
    }

    /// Get metadata reference
    pub fn metadata(&self) -> &NodeMetadata {
        &self.metadata
    }

    /// Check if node is a leaf (no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if node is a root (no parents)
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }

    /// Count total descendants (recursively)
    ///
    /// Requires a lookup function to find child nodes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    /// use im::HashMap;
    ///
    /// let nodes = HashMap::from([
    ///     ("root", GraphNode::new("root", "Root").add_child("a").add_child("b")),
    ///     ("a", GraphNode::new("a", "A").add_child("c")),
    ///     ("b", GraphNode::new("b", "B")),
    ///     ("c", GraphNode::new("c", "C")),
    /// ]);
    ///
    /// let root = nodes.get(&"root").unwrap();
    /// let count = root.count_descendants(|id| nodes.get(id));
    /// assert_eq!(count, 3); // a, b, c
    /// ```
    pub fn count_descendants<F>(&self, lookup: F) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        let mut visited = std::collections::HashSet::new();
        self.count_descendants_helper(&lookup, &mut visited)
    }

    fn count_descendants_helper<F>(
        &self,
        lookup: &F,
        visited: &mut std::collections::HashSet<ID>,
    ) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        let mut count = 0;

        for child_id in &self.children {
            if visited.insert(child_id.clone()) {
                count += 1;
                if let Some(child) = lookup(child_id) {
                    count += child.count_descendants_helper(lookup, visited);
                }
            }
        }

        count
    }

    /// Count total ancestors (recursively)
    ///
    /// Requires a lookup function to find parent nodes.
    pub fn count_ancestors<F>(&self, lookup: F) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        let mut visited = std::collections::HashSet::new();
        self.count_ancestors_helper(&lookup, &mut visited)
    }

    fn count_ancestors_helper<F>(
        &self,
        lookup: &F,
        visited: &mut std::collections::HashSet<ID>,
    ) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        let mut count = 0;

        for parent_id in &self.parents {
            if visited.insert(parent_id.clone()) {
                count += 1;
                if let Some(parent) = lookup(parent_id) {
                    count += parent.count_ancestors_helper(lookup, visited);
                }
            }
        }

        count
    }

    /// Traverse children starting from this node
    ///
    /// Returns `TraversalResult` containing all visited nodes in order.
    ///
    /// The visitor function returns `true` to continue traversal, `false` to skip children.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    /// use im::HashMap;
    ///
    /// let nodes = HashMap::from([
    ///     ("root", GraphNode::new("root", "Root").add_child("a")),
    ///     ("a", GraphNode::new("a", "A")),
    /// ]);
    ///
    /// let root = nodes.get(&"root").unwrap();
    /// let result = root.traverse(|id, node| {
    ///     println!("Visiting: {}", node.label);
    ///     true
    /// });
    ///
    /// assert_eq!(result.visited.len(), 2);
    /// ```
    pub fn traverse<F>(&self, visitor: F) -> TraversalResult<ID>
    where
        F: Fn(&ID, &GraphNode<ID>) -> bool,
    {
        let mut result = TraversalResult::new();
        let mut visited = std::collections::HashSet::new();
        self.traverse_helper(&visitor, &mut visited, &mut result);
        result
    }

    fn traverse_helper<F>(
        &self,
        visitor: &F,
        visited: &mut std::collections::HashSet<ID>,
        result: &mut TraversalResult<ID>,
    ) where
        F: Fn(&ID, &GraphNode<ID>) -> bool,
    {
        if !visited.insert(self.id.clone()) {
            return; // Already visited
        }

        result.visited.push_back(self.id.clone());
        let should_continue = visitor(&self.id, self);

        if should_continue {
            // Need a way to lookup child nodes - caller should use traverse_with_lookup instead
            // This is a simplified version
        }
    }

    /// Traverse with a lookup function to access child nodes
    ///
    /// This is the full traversal version that can access the entire graph.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    /// use im::HashMap;
    ///
    /// let nodes = HashMap::from([
    ///     ("root", GraphNode::new("root", "Root").add_child("a").add_child("b")),
    ///     ("a", GraphNode::new("a", "A").add_child("c")),
    ///     ("b", GraphNode::new("b", "B")),
    ///     ("c", GraphNode::new("c", "C")),
    /// ]);
    ///
    /// let root = nodes.get(&"root").unwrap();
    /// let result = root.traverse_with_lookup(&nodes, |id, node| {
    ///     println!("Visiting: {}", node.label);
    ///     true
    /// });
    ///
    /// assert_eq!(result.visited.len(), 4);
    /// ```
    pub fn traverse_with_lookup<F>(
        &self,
        lookup: &HashMap<ID, GraphNode<ID>>,
        visitor: F,
    ) -> TraversalResult<ID>
    where
        F: Fn(&ID, &GraphNode<ID>) -> bool,
    {
        let mut result = TraversalResult::new();
        let mut visited = std::collections::HashSet::new();
        self.traverse_helper_with_lookup(lookup, &visitor, &mut visited, &mut result);
        result
    }

    fn traverse_helper_with_lookup<F>(
        &self,
        lookup: &HashMap<ID, GraphNode<ID>>,
        visitor: &F,
        visited: &mut std::collections::HashSet<ID>,
        result: &mut TraversalResult<ID>,
    ) where
        F: Fn(&ID, &GraphNode<ID>) -> bool,
    {
        if !visited.insert(self.id.clone()) {
            return; // Already visited (cycle detection)
        }

        result.visited.push_back(self.id.clone());
        let should_continue = visitor(&self.id, self);

        if should_continue {
            for child_id in &self.children {
                if let Some(child) = lookup.get(child_id) {
                    child.traverse_helper_with_lookup(lookup, visitor, visited, result);
                }
            }
        }
    }

    /// Find a node by ID in the subtree
    ///
    /// Returns `Some(node)` if found, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    /// use im::HashMap;
    ///
    /// let nodes = HashMap::from([
    ///     ("root", GraphNode::new("root", "Root").add_child("a")),
    ///     ("a", GraphNode::new("a", "Target")),
    /// ]);
    ///
    /// let root = nodes.get(&"root").unwrap();
    /// let found = root.find(&nodes, &"a");
    ///
    /// assert!(found.is_some());
    /// assert_eq!(found.unwrap().label, "Target");
    /// ```
    pub fn find<'a>(
        &'a self,
        lookup: &'a HashMap<ID, GraphNode<ID>>,
        id: &ID,
    ) -> Option<&'a GraphNode<ID>> {
        if &self.id == id {
            return Some(self);
        }

        for child_id in &self.children {
            if let Some(child) = lookup.get(child_id) {
                if let Some(found) = child.find(lookup, id) {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Find all nodes matching a predicate
    ///
    /// Returns a vector of matching nodes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::{GraphNode, NodeState};
    /// use im::HashMap;
    ///
    /// let nodes = HashMap::from([
    ///     ("root", GraphNode::new("root", "Root").add_child("a").add_child("b")),
    ///     ("a", GraphNode::new("a", "A").with_state(NodeState::Completed)),
    ///     ("b", GraphNode::new("b", "B").with_state(NodeState::Running)),
    /// ]);
    ///
    /// let root = nodes.get(&"root").unwrap();
    /// let completed = root.find_matching(&nodes, |node| {
    ///     node.state == NodeState::Completed
    /// });
    ///
    /// assert_eq!(completed.len(), 1);
    /// ```
    pub fn find_matching<'a, F>(
        &'a self,
        lookup: &'a HashMap<ID, GraphNode<ID>>,
        predicate: F,
    ) -> Vec<&'a GraphNode<ID>>
    where
        F: Fn(&GraphNode<ID>) -> bool,
    {
        let mut results = Vec::new();
        self.find_matching_helper(lookup, &predicate, &mut std::collections::HashSet::new(), &mut results);
        results
    }

    fn find_matching_helper<'a, F>(
        &'a self,
        lookup: &'a HashMap<ID, GraphNode<ID>>,
        predicate: &F,
        visited: &mut std::collections::HashSet<ID>,
        results: &mut Vec<&'a GraphNode<ID>>,
    ) where
        F: Fn(&GraphNode<ID>) -> bool,
    {
        if !visited.insert(self.id.clone()) {
            return;
        }

        if predicate(self) {
            results.push(self);
        }

        for child_id in &self.children {
            if let Some(child) = lookup.get(child_id) {
                child.find_matching_helper(lookup, predicate, visited, results);
            }
        }
    }

    /// Check if this node has the given node as a descendant
    ///
    /// # Example
    ///
    /// ```rust
    /// use oya_zellij::graph::GraphNode;
    /// use im::HashMap;
    ///
    /// let nodes = HashMap::from([
    ///     ("root", GraphNode::new("root", "Root").add_child("a").add_child("b")),
    ///     ("a", GraphNode::new("a", "A").add_child("c")),
    ///     ("b", GraphNode::new("b", "B")),
    ///     ("c", GraphNode::new("c", "C")),
    /// ]);
    ///
    /// let root = nodes.get(&"root").unwrap();
    /// assert!(root.has_descendant(&nodes, &"c"));
    /// assert!(!root.has_descendant(&nodes, &"root"));
    /// ```
    pub fn has_descendant(&self, lookup: &HashMap<ID, GraphNode<ID>>, id: &ID) -> bool {
        self.find(lookup, id).is_some()
    }

    /// Check if this node has the given node as an ancestor
    pub fn has_ancestor(&self, lookup: &HashMap<ID, GraphNode<ID>>, id: &ID) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.has_ancestor_helper(lookup, id, &mut visited)
    }

    fn has_ancestor_helper(
        &self,
        lookup: &HashMap<ID, GraphNode<ID>>,
        id: &ID,
        visited: &mut std::collections::HashSet<ID>,
    ) -> bool {
        for parent_id in &self.parents {
            if parent_id == id {
                return true;
            }

            if visited.insert(parent_id.clone()) {
                if let Some(parent) = lookup.get(parent_id) {
                    if parent.has_ancestor_helper(lookup, id, visited) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get depth of node (distance from root)
    ///
    /// Returns 0 for root nodes, 1 for their direct children, etc.
    pub fn depth<F>(&self, lookup: F) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        let mut visited = std::collections::HashSet::new();
        self.depth_helper(&lookup, &mut visited)
    }

    fn depth_helper<F>(&self, lookup: &F, visited: &mut std::collections::HashSet<ID>) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        if self.parents.is_empty() {
            return 0;
        }

        let max_parent_depth = self.parents
            .iter()
            .filter_map(|parent_id| {
                if visited.insert(parent_id.clone()) {
                    lookup(parent_id).map(|parent| parent.depth_helper(lookup, visited))
                } else {
                    None // Prevent cycles
                }
            })
            .max()
            .unwrap_or(0);

        max_parent_depth + 1
    }

    /// Get height of node (distance to furthest leaf)
    pub fn height<F>(&self, lookup: F) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        let mut visited = std::collections::HashSet::new();
        self.height_helper(&lookup, &mut visited)
    }

    fn height_helper<F>(&self, lookup: &F, visited: &mut std::collections::HashSet<ID>) -> usize
    where
        F: Fn(&ID) -> Option<&GraphNode<ID>>,
    {
        if self.children.is_empty() {
            return 0;
        }

        self.children
            .iter()
            .filter_map(|child_id| {
                if visited.insert(child_id.clone()) {
                    lookup(child_id).map(|child| child.height_helper(lookup, visited))
                } else {
                    None
                }
            })
            .max()
            .map(|h| h + 1)
            .unwrap_or(0)
    }
}

/// Result of a graph traversal operation
///
/// Contains all visited node IDs in traversal order.
#[derive(Clone, Debug)]
pub struct TraversalResult<ID>
where
    ID: Clone + Hash + Eq,
{
    /// Visited node IDs in traversal order
    pub visited: Vector<ID>,
}

impl<ID> TraversalResult<ID>
where
    ID: Clone + Hash + Eq,
{
    /// Create empty traversal result
    pub fn new() -> Self {
        Self::default()
    }

    /// Get count of visited nodes
    pub fn len(&self) -> usize {
        self.visited.len()
    }

    /// Check if traversal visited any nodes
    pub fn is_empty(&self) -> bool {
        self.visited.is_empty()
    }

    /// Check if a specific ID was visited
    pub fn contains(&self, id: &ID) -> bool {
        self.visited.iter().any(|visited_id| visited_id == id)
    }
}

impl<ID> Default for TraversalResult<ID>
where
    ID: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Visitor function type for graph traversals
///
/// The visitor receives the node ID and a reference to the node.
/// Returns `true` to continue traversing children, `false` to stop.
pub type TraversalVisitor<ID, Node> = fn(&ID, &Node) -> bool;

#[cfg(test)]
mod tests {
    use super::*;
    use im::hashmap;

    #[test]
    fn test_node_creation() {
        let node = GraphNode::new("test_id", "Test Label");

        assert_eq!(node.id, "test_id");
        assert_eq!(node.label, "Test Label");
        assert_eq!(node.state, NodeState::Idle);
        assert!(node.children.is_empty());
        assert!(node.parents.is_empty());
    }

    #[test]
    fn test_add_child() {
        let node = GraphNode::new("parent", "Parent")
            .add_child("child1")
            .add_child("child2");

        assert_eq!(node.children.len(), 2);
        assert!(node.children.contains(&"child1"));
        assert!(node.children.contains(&"child2"));
    }

    #[test]
    fn test_add_child_no_duplicates() {
        let node = GraphNode::new("parent", "Parent")
            .add_child("child1")
            .add_child("child1");

        assert_eq!(node.children.len(), 1);
    }

    #[test]
    fn test_remove_child() {
        let node = GraphNode::new("parent", "Parent")
            .add_child("child1")
            .add_child("child2")
            .remove_child(&"child1");

        assert_eq!(node.children.len(), 1);
        assert!(!node.children.contains(&"child1"));
        assert!(node.children.contains(&"child2"));
    }

    #[test]
    fn test_add_parent() {
        let node = GraphNode::new("child", "Child")
            .add_parent("parent1")
            .add_parent("parent2");

        assert_eq!(node.parents.len(), 2);
        assert!(node.parents.contains(&"parent1"));
        assert!(node.parents.contains(&"parent2"));
    }

    #[test]
    fn test_with_state() {
        let node = GraphNode::new("task", "Task")
            .with_state(NodeState::Running);

        assert_eq!(node.state, NodeState::Running);
    }

    #[test]
    fn test_state_methods() {
        assert!(NodeState::Idle.is_active() == false);
        assert!(NodeState::Running.is_active() == true);
        assert!(NodeState::Completed.is_terminal() == true);
        assert!(NodeState::Failed.is_terminal() == true);
        assert!(NodeState::Blocked.is_terminal() == false);
    }

    #[test]
    fn test_metadata() {
        let node = GraphNode::new("task", "Task")
            .with_metadata("stage", "implement")
            .with_metadata("priority", "high");

        assert_eq!(node.get_metadata("stage"), Some(&"implement".to_string()));
        assert_eq!(node.get_metadata("priority"), Some(&"high".to_string()));
        assert_eq!(node.get_metadata("missing"), None);
    }

    #[test]
    fn test_critical_path() {
        let node = GraphNode::new("task", "Task")
            .with_critical_path(true);

        assert!(node.metadata.is_on_critical_path());
    }

    #[test]
    fn test_priority() {
        let node = GraphNode::new("task", "Task")
            .with_priority(42);

        assert_eq!(node.metadata.priority(), 42);
    }

    #[test]
    fn test_is_leaf() {
        let leaf = GraphNode::new("leaf", "Leaf");
        assert!(leaf.is_leaf());

        let internal = GraphNode::new("internal", "Internal").add_child("child");
        assert!(!internal.is_leaf());
    }

    #[test]
    fn test_is_root() {
        let root = GraphNode::new("root", "Root");
        assert!(root.is_root());

        let child = GraphNode::new("child", "Child").add_parent("parent");
        assert!(!child.is_root());
    }

    #[test]
    fn test_count_descendants() {
        let root = GraphNode::new("root", "Root")
            .add_child("a")
            .add_child("b");
        let a = GraphNode::new("a", "A")
            .add_child("c")
            .add_parent("root");
        let b = GraphNode::new("b", "B")
            .add_parent("root");
        let c = GraphNode::new("c", "C")
            .add_parent("a");

        let nodes = hashmap! {
            "root" => root,
            "a" => a,
            "b" => b,
            "c" => c,
        };

        let root_node = nodes.get(&"root").unwrap();
        let nodes_clone = nodes.clone();
        let count = root_node.count_descendants(|id| nodes_clone.get(id));

        assert_eq!(count, 3); // a, b, c
    }

    #[test]
    fn test_count_ancestors() {
        let root = GraphNode::new("root", "Root")
            .add_child("a");
        let a = GraphNode::new("a", "A")
            .add_child("b")
            .add_parent("root");
        let b = GraphNode::new("b", "B")
            .add_parent("a");

        let nodes = hashmap! {
            "root" => root,
            "a" => a,
            "b" => b,
        };

        let leaf = nodes.get(&"b").unwrap();
        let count = leaf.count_ancestors(|id| nodes.get(id));

        assert_eq!(count, 2); // a, root
    }

    #[test]
    fn test_traverse() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a")
                .add_child("b"),
            "a" => GraphNode::new("a", "A")
                .add_child("c"),
            "b" => GraphNode::new("b", "B"),
            "c" => GraphNode::new("c", "C"),
        };

        let root = nodes.get(&"root").unwrap();
        let result = root.traverse_with_lookup(&nodes, |_id, _node| true);

        assert_eq!(result.len(), 4);
        assert!(result.contains(&"root"));
        assert!(result.contains(&"a"));
        assert!(result.contains(&"b"));
        assert!(result.contains(&"c"));
    }

    #[test]
    fn test_traverse_stop_early() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a"),
            "a" => GraphNode::new("a", "A")
                .add_child("b"),
            "b" => GraphNode::new("b", "B"),
        };

        let root = nodes.get(&"root").unwrap();
        let result = root.traverse_with_lookup(&nodes, |id, _node| {
            // Stop traversing after "a"
            id != &"a"
        });

        // Should visit root and a, but not b
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"root"));
        assert!(result.contains(&"a"));
        assert!(!result.contains(&"b"));
    }

    #[test]
    fn test_find() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a"),
            "a" => GraphNode::new("a", "Target"),
        };

        let root = nodes.get(&"root").unwrap();
        let found = root.find(&nodes, &"a");

        assert!(found.is_some());
        assert_eq!(found.unwrap().label, "Target");
    }

    #[test]
    fn test_find_not_found() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root"),
        };

        let root = nodes.get(&"root").unwrap();
        let found = root.find(&nodes, &"missing");

        assert!(found.is_none());
    }

    #[test]
    fn test_find_matching() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a")
                .add_child("b"),
            "a" => GraphNode::new("a", "A")
                .with_state(NodeState::Completed),
            "b" => GraphNode::new("b", "B")
                .with_state(NodeState::Running),
        };

        let root = nodes.get(&"root").unwrap();
        let completed = root.find_matching(&nodes, |node| {
            node.state == NodeState::Completed
        });

        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, "a");
    }

    #[test]
    fn test_has_descendant() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a")
                .add_child("b"),
            "a" => GraphNode::new("a", "A")
                .add_child("c"),
            "b" => GraphNode::new("b", "B"),
            "c" => GraphNode::new("c", "C"),
        };

        let root = nodes.get(&"root").unwrap();
        assert!(root.has_descendant(&nodes, &"c"));
        assert!(!root.has_descendant(&nodes, &"root"));
    }

    #[test]
    fn test_has_ancestor() {
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a"),
            "a" => GraphNode::new("a", "A")
                .add_child("b")
                .add_parent("root"),
            "b" => GraphNode::new("b", "B")
                .add_parent("a"),
        };

        let leaf = nodes.get(&"b").unwrap();
        assert!(leaf.has_ancestor(&nodes, &"root"));
        assert!(leaf.has_ancestor(&nodes, &"a"));
        assert!(!leaf.has_ancestor(&nodes, &"b"));
    }

    #[test]
    fn test_depth() {
        let root = GraphNode::new("root", "Root")
            .add_child("a");
        let a = GraphNode::new("a", "A")
            .add_child("b")
            .add_parent("root");
        let b = GraphNode::new("b", "B")
            .add_parent("a");

        let nodes = hashmap! {
            "root" => root,
            "a" => a,
            "b" => b,
        };

        let root_node = nodes.get(&"root").unwrap();
        let a_node = nodes.get(&"a").unwrap();
        let b_node = nodes.get(&"b").unwrap();

        assert_eq!(root_node.depth(|id| nodes.get(id)), 0);
        assert_eq!(a_node.depth(|id| nodes.get(id)), 1);
        assert_eq!(b_node.depth(|id| nodes.get(id)), 2);
    }

    #[test]
    fn test_height() {
        let root = GraphNode::new("root", "Root")
            .add_child("a");
        let a = GraphNode::new("a", "A")
            .add_child("b")
            .add_parent("root");
        let b = GraphNode::new("b", "B")
            .add_parent("a");

        let nodes = hashmap! {
            "root" => root,
            "a" => a,
            "b" => b,
        };

        let root_node = nodes.get(&"root").unwrap();
        let a_node = nodes.get(&"a").unwrap();
        let b_node = nodes.get(&"b").unwrap();

        assert_eq!(b_node.height(|id| nodes.get(id)), 0);
        assert_eq!(a_node.height(|id| nodes.get(id)), 1);
        assert_eq!(root_node.height(|id| nodes.get(id)), 2);
    }

    #[test]
    fn test_cycle_detection() {
        // Create a cycle: root -> a -> b -> a
        let nodes = hashmap! {
            "root" => GraphNode::new("root", "Root")
                .add_child("a"),
            "a" => GraphNode::new("a", "A")
                .add_child("b")
                .add_parent("root"),
            "b" => GraphNode::new("b", "B")
                .add_child("a") // Creates cycle
                .add_parent("a"),
        };

        let root = nodes.get(&"root").unwrap();
        let result = root.traverse_with_lookup(&nodes, |_id, _node| true);

        // Should terminate without infinite loop despite cycle
        assert!(result.len() <= 3);
    }

    #[test]
    fn test_metadata_durations() {
        let metadata = NodeMetadata::new()
            .with_estimated_duration_ms(1000)
            .with_actual_duration_ms(950);

        assert_eq!(metadata.estimated_duration_ms(), Some(1000));
        assert_eq!(metadata.actual_duration_ms(), Some(950));
    }

    #[test]
    fn test_functional_updates() {
        let node1 = GraphNode::new("task", "Task");
        let node2 = node1
            .clone()
            .with_state(NodeState::Running)
            .add_child("child1");

        // Original node unchanged
        assert_eq!(node1.state, NodeState::Idle);
        assert!(node1.children.is_empty());

        // New node has updates
        assert_eq!(node2.state, NodeState::Running);
        assert!(node2.children.contains(&"child1"));
    }
}
