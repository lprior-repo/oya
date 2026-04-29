//! Execution plan types.

use std::collections::HashMap;

use crate::graph::NodeId;

// ===========================================================================
// Execution Plan
// ===========================================================================

/// Execution plan with topologically sorted nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    /// Topologically sorted execution order.
    pub execution_order: Vec<NodeId>,
    /// Entry nodes (in-degree 0).
    pub entry_nodes: Vec<NodeId>,
    /// Detected cycles (if any).
    pub cycles: Vec<Vec<NodeId>>,
    /// Map of node to input ports.
    pub input_map: HashMap<NodeId, Vec<NodeId>>,
}
