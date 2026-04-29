#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod topo_tests;
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod validation_tests;

pub(super) use super::super::{NodeId, Workflow, WorkflowExecutionError};
pub(super) use crate::graph::{Connection, PortName};
pub(super) use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(super) fn main_port() -> PortName {
    PortName::from("main")
}

pub(super) fn add_connection(workflow: &mut Workflow, source: NodeId, target: NodeId) {
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source,
        target,
        source_port: main_port(),
        target_port: main_port(),
    });
}

/// Calls `build_execution_queue` through `prepare_run`, which resets
/// dirty state so it can be called repeatedly. Returns the queue or
/// the first error encountered.
pub(super) fn prepare_and_get_queue(
    workflow: &mut Workflow,
) -> Result<Vec<NodeId>, WorkflowExecutionError> {
    workflow.prepare_run()?;
    Ok(workflow.execution_queue.clone())
}
