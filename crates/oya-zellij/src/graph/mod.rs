//! Graph data structures for DAG visualization
//!
//! Functional, immutable graph implementation using persistent data structures.

mod node;

pub use node::{GraphNode, NodeMetadata, TraversalResult, TraversalVisitor};
