//! Workflow execution error types.
//!
//! This module defines all error types that can occur during workflow execution,
//! following the Design-by-Contract principles from the specification.

use crate::graph::{ExecutionState, NodeId};
use std::fmt::{Formatter, Result as FmtResult};

// ===========================================================================
// Workflow Execution Errors
// ===========================================================================

/// Errors that can occur during workflow execution.
///
/// This enum represents all possible failure modes during workflow execution,
/// following the contract specification for comprehensive error handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowExecutionError {
    /// A cycle was detected in the workflow graph.
    /// Contains the node IDs that form the cycle.
    CycleDetected {
        /// Node IDs that form the cycle.
        cycle_nodes: Vec<NodeId>,
    },

    /// Nodes have unresolved dependencies (missing nodes or circular).
    /// Distinguishes between missing nodes and circular dependencies.
    UnresolvedDependencies {
        /// Nodes that cannot be executed.
        nodes: Vec<NodeId>,
        /// For each node, which dependencies are unresolved.
        missing_deps: Vec<NodeId>,
    },

    /// Invalid workflow state (precondition violation).
    InvalidWorkflowState {
        /// Description of what is invalid.
        reason: String,
    },

    /// A node was not found during execution.
    ///
    /// This occurs when a connection references a node that doesn't exist.
    NodeNotFound {
        /// The connection ID that references the missing node.
        connection_id: uuid::Uuid,
        /// The ID of the node that was not found.
        referenced_node: NodeId,
    },

    /// A node execution failed.
    ExecutionFailed {
        /// The node ID that failed.
        node_id: NodeId,
        /// The error message describing the failure.
        error: String,
        /// Whether the failure is retryable.
        retryable: bool,
    },

    /// A node execution exceeded its timeout.
    Timeout {
        /// The node ID that timed out (if known).
        node_id: Option<NodeId>,
        /// The actual duration in milliseconds.
        duration_ms: u64,
        /// The timeout limit in milliseconds.
        limit_ms: u64,
    },

    /// A node execution exceeded the memory limit.
    MemoryLimitExceeded {
        /// The node ID that exceeded the limit (if known).
        node_id: Option<NodeId>,
        /// The memory used in bytes.
        bytes_used: u64,
        /// The memory limit in bytes.
        limit_bytes: u64,
    },

    /// An invalid configuration was detected.
    InvalidConfig {
        /// The node ID with the invalid configuration.
        node_id: NodeId,
        /// The error message describing the configuration issue.
        error: String,
    },

    /// The workflow has no entry nodes to start execution.
    NoEntryNodes,

    /// The workflow is empty (no nodes).
    EmptyWorkflow,

    /// An invalid state transition was attempted.
    InvalidStateTransition {
        /// The node ID where the transition failed.
        node_id: NodeId,
        /// The current state before the transition.
        from: ExecutionState,
        /// The requested state after the transition.
        to: ExecutionState,
    },
}

impl std::fmt::Display for WorkflowExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::CycleDetected { cycle_nodes } => fmt_cycle_detected(f, cycle_nodes),
            Self::UnresolvedDependencies { nodes, missing_deps } => {
                write!(f, "Unresolved dependencies: nodes {nodes:?} missing deps {missing_deps:?}")
            }
            Self::InvalidWorkflowState { reason } => {
                write!(f, "Invalid workflow state: {reason}")
            }
            Self::NodeNotFound { connection_id, referenced_node } => {
                write!(f, "Node not found: connection {connection_id} references non-existent node {referenced_node}")
            }
            Self::ExecutionFailed { node_id, error, retryable } => {
                fmt_execution_failed(f, *node_id, error, *retryable)
            }
            Self::Timeout { node_id, duration_ms, limit_ms } => {
                fmt_timeout(f, *node_id, *duration_ms, *limit_ms)
            }
            Self::MemoryLimitExceeded { node_id, bytes_used, limit_bytes } => {
                fmt_memory_limit(f, *node_id, *bytes_used, *limit_bytes)
            }
            Self::InvalidConfig { node_id, error } => {
                write!(f, "Node {node_id} has invalid configuration: {error}")
            }
            Self::NoEntryNodes => write!(f, "Workflow has no entry nodes to start execution"),
            Self::EmptyWorkflow => write!(f, "Workflow is empty (no nodes)"),
            Self::InvalidStateTransition { node_id, from, to } => {
                write!(f, "Invalid state transition for node {node_id}: {from:?} -> {to:?}")
            }
        }
    }
}

fn fmt_cycle_detected(f: &mut Formatter<'_>, cycle_nodes: &[NodeId]) -> FmtResult {
    let first =
        cycle_nodes.first().map_or_else(|| "<empty>".to_string(), std::string::ToString::to_string);
    let second =
        cycle_nodes.get(1).map_or_else(|| "...".to_string(), std::string::ToString::to_string);

    write!(f, "Cycle detected in workflow: {first} -> {second} -> ...")
}

fn fmt_execution_failed(
    f: &mut Formatter<'_>,
    node_id: NodeId,
    error: &str,
    retryable: bool,
) -> FmtResult {
    let retry_label = if retryable { "retryable" } else { "non-retryable" };

    write!(f, "Node {node_id} failed ({retry_label}): {error}")
}

fn fmt_timeout(
    f: &mut Formatter<'_>,
    node_id: Option<NodeId>,
    duration_ms: u64,
    limit_ms: u64,
) -> FmtResult {
    match node_id {
        Some(id) => write!(f, "Node {id} timed out: {duration_ms}ms > {limit_ms}ms limit"),
        None => write!(f, "Execution timed out: {duration_ms}ms > {limit_ms}ms limit"),
    }
}

fn fmt_memory_limit(
    f: &mut Formatter<'_>,
    node_id: Option<NodeId>,
    bytes_used: u64,
    limit_bytes: u64,
) -> FmtResult {
    match node_id {
        Some(id) => write!(f, "Node {id} exceeded memory: {bytes_used}B > {limit_bytes}B limit"),
        None => write!(f, "Execution exceeded memory: {bytes_used}B > {limit_bytes}B limit"),
    }
}

impl std::error::Error for WorkflowExecutionError {}
