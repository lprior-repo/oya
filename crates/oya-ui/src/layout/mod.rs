//! Graph layout algorithms

pub mod gravity;
pub mod spring_force;

use crate::models::GraphNode;

/// Result type for layout operations
pub type LayoutResult<T> = Result<T, LayoutError>;

/// Errors that can occur during layout computation
#[derive(Debug, Clone)]
pub enum LayoutError {
    /// No nodes provided
    EmptyGraph,
    /// Invalid parameters
    InvalidParameters(String),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::EmptyGraph => write!(f, "Cannot compute layout for empty graph"),
            LayoutError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
        }
    }
}

impl std::error::Error for LayoutError {}

/// Force-directed layout module
pub mod force_directed {
    use super::*;

    /// Computes positions for nodes using force-directed layout
    pub fn compute_positions(nodes: &[GraphNode]) -> LayoutResult<Vec<GraphNode>> {
        if nodes.is_empty() {
            return Err(LayoutError::EmptyGraph);
        }

        // Simple initial positioning - just return a copy for now
        Ok(nodes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph_error() {
        let result = force_directed::compute_positions(&[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(LayoutError::EmptyGraph)));
    }

    #[test]
    fn test_simple_layout() {
        let nodes = vec![GraphNode {
            id: "node1".to_string(),
            label: "Node 1".to_string(),
            x: 0.0,
            y: 0.0,
            color: None,
        }];

        let result = force_directed::compute_positions(&nodes);
        assert!(result.is_ok());
        let positioned = result.ok().unwrap_or_default();
        assert_eq!(positioned.len(), 1);
    }
}
