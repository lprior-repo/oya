use uuid::Uuid;

use crate::graph::{Connection, NodeId, PortName, Workflow};

use super::validators::{validate_connection, ConnectionRequest, ValidationState};
use super::{ConnectionError, ConnectionResult};

impl Workflow {
    /// Adds a connection between two nodes.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionError`] if the connection would be invalid:
    /// - Same source and target node ([`ConnectionError::SelfConnection`])
    /// - Source or target node does not exist
    /// - Connection would create a cycle
    /// - An identical connection already exists
    /// - Source and target port types are incompatible
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_frontend::graph::{Workflow, NodeId, PortName};
    /// let mut workflow = Workflow::new();
    /// let source = workflow.add_node("http-handler", 0.0, 0.0);
    /// let target = workflow.add_node("run", 100.0, 0.0);
    /// let main = PortName("main".to_string());
    /// assert!(workflow.add_connection(source, target, &main, &main).is_ok());
    /// ```
    pub fn add_connection(
        &mut self,
        source: NodeId,
        target: NodeId,
        source_port: &PortName,
        target_port: &PortName,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.add_connection_checked(source, target, source_port, target_port)
    }

    /// Adds a connection with full validation and type checking.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionError`] if:
    /// - `source` and `target` are the same node
    /// - Either endpoint does not exist in the workflow
    /// - The connection would create a cycle
    /// - An identical connection already exists
    /// - Source and target port types are incompatible
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_frontend::graph::{Workflow, NodeId, PortName, ConnectionResult};
    /// let mut workflow = Workflow::new();
    /// let source = workflow.add_node("http-handler", 0.0, 0.0);
    /// let target = workflow.add_node("run", 100.0, 0.0);
    /// let main = PortName("main".to_string());
    /// assert_eq!(
    ///     workflow.add_connection_checked(source, target, &main, &main),
    ///     Ok(ConnectionResult::Created)
    /// );
    /// ```
    pub fn add_connection_checked(
        &mut self,
        source: NodeId,
        target: NodeId,
        source_port: &PortName,
        target_port: &PortName,
    ) -> Result<ConnectionResult, ConnectionError> {
        let validation = validate_connection(
            &self.nodes,
            &self.connections,
            &ConnectionRequest { source, target, source_port, target_port },
        )?;
        commit_connection(&mut self.connections, validation);
        Ok(ConnectionResult::Created)
    }
}

/// Commits a validated connection to the graph.
///
/// # Safety
///
/// Only call this after `validate_connection` has succeeded.
fn commit_connection(connections: &mut Vec<Connection>, validation: ValidationState) {
    connections.push(Connection {
        id: Uuid::new_v4(),
        source: validation.source,
        target: validation.target,
        source_port: validation.source_port,
        target_port: validation.target_port,
    });
}
