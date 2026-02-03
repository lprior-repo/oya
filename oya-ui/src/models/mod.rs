//! Node data structures for DAG visualization.

pub mod node;

pub use node::{
    Node, NodeBuilder, NodeError, NodeId, NodeShape, NodeSize, NodeState, Point, RectSize,
};
