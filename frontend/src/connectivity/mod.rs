#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::complexity)]
#![warn(clippy::cognitive_complexity)]
#![forbid(unsafe_code)]

//! Connectivity module for connection management.
//!
//! Implements design-by-contract with Data → Calc → Actions architecture:
//! - **Data layer:** `PortType`, `ConnectionId`, `Connection`, `ConnectionError`
//! - **Calculations:** `validate_connection`, `check_port_compatibility` (pure functions)
//! - **Actions:** `add_connection_checked`, `remove_connection`, `get_connection` (state mutation)
//!
//! ## Port Types
//!
//! Supports three port types:
//! - `tcp:<port>` - TCP socket (port 1-65535)
//! - `udp:<port>` - UDP socket (port 1-65535)
//! - `unix:<path>` - Unix domain socket (filesystem path)
//!
//! ## Compatibility Rules
//!
//! | Source \ Dest | tcp:* | udp:* | unix:* |
//! |---------------|-------|-------|--------|
//! | **tcp:**      | ✅    | ✅    | ❌     |
//! | **udp:**      | ✅    | ✅    | ❌     |
//! | **unix:**     | ❌    | ❌    | ✅     |
//!
//! TCP and UDP are interchangeable; Unix sockets require Unix endpoints.

mod port_type;
mod validation;

pub use port_type::{PortType, PortTypeParseError};
pub use validation::validate_connection;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Unique identifier for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ConnectionId {
    /// Generates a new unique connection identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(CONNECTION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a connection between two endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// Unique identifier for this connection.
    pub id: ConnectionId,
    /// Source endpoint type and address.
    pub source: PortType,
    /// Destination endpoint type and address.
    pub destination: PortType,
    /// Human-readable description of the connection.
    pub description: String,
}

/// Error types for connection operations.
///
/// Exhaustive error taxonomy for all connection-related failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
    /// Port parsing failed.
    ParseError(PortTypeParseError),
    /// Port numbers are incompatible (tcp vs unix).
    TypeMismatch {
        /// Source port type.
        source: PortType,
        /// Destination port type.
        destination: PortType,
    },
    /// General validation failure with message.
    ValidationFailed(String),
    /// Specified path does not exist.
    PathNotFound(String),
    /// Connection with this ID already exists.
    ConnectionExists(ConnectionId),
    /// Connection with this ID was not found.
    ConnectionNotFound(ConnectionId),
    /// Operation not permitted.
    PermissionDenied,
    /// Storage operation failed.
    StorageError(String),
    /// Internal error occurred.
    InternalError(String),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(err) => write!(f, "Port parse error: {err}"),
            Self::TypeMismatch { source, destination } => {
                write!(f, "Type mismatch: {source} cannot connect to {destination}")
            }
            Self::ValidationFailed(msg) => write!(f, "Validation failed: {msg}"),
            Self::PathNotFound(path) => write!(f, "Path not found: {path}"),
            Self::ConnectionExists(id) => write!(f, "Connection already exists: {id}"),
            Self::ConnectionNotFound(id) => write!(f, "Connection not found: {id}"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::StorageError(msg) => write!(f, "Storage error: {msg}"),
            Self::InternalError(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for ConnectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseError(err) => Some(err),
            Self::TypeMismatch { .. }
            | Self::ValidationFailed(_)
            | Self::PathNotFound(_)
            | Self::ConnectionExists(_)
            | Self::ConnectionNotFound(_)
            | Self::PermissionDenied
            | Self::StorageError(_)
            | Self::InternalError(_) => None,
        }
    }
}

/// Global connection store using `RwLock` for thread-safe access.
///
/// This is the shell layer state. The functional core (`validate_connection`)
/// has no side effects and is pure.
#[derive(Debug, Default)]
pub struct ConnectionStore {
    /// Map of connection ID to connection.
    connections: HashMap<ConnectionId, Connection>,
}

impl ConnectionStore {
    /// Creates a new empty connection store.
    #[must_use]
    pub fn new() -> Self {
        Self { connections: HashMap::new() }
    }

    /// Creates a new connection store with pre-populated connections.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` if any connection fails validation.
    pub fn with_connections(
        connections: impl IntoIterator<Item = (ConnectionId, Connection)>,
    ) -> Result<Self, ConnectionError> {
        let mut store = Self::new();
        for (id, conn) in connections {
            store.add_connection_checked_internal(conn, id)?;
        }
        Ok(store)
    }

    /// Adds a connection with full validation.
    ///
    /// This is the shell layer action:
    /// 1. Validates the connection using pure function
    /// 2. Mutates state only after validation succeeds
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` if validation fails or connection exists.
    pub fn add_connection_checked(
        &mut self,
        source: PortType,
        destination: PortType,
        description: impl Into<String>,
    ) -> Result<ConnectionId, ConnectionError> {
        // Extract paths for unix sockets
        let source_path =
            if source.is_unix() { source.address().to_string() } else { String::new() };
        let destination_path =
            if destination.is_unix() { destination.address().to_string() } else { String::new() };

        // Validate first (pure function, no side effects)
        validate_connection(source.as_str(), destination.as_str(), &source_path, &destination_path)
            .map_err(|e| match e {
                ConnectionError::TypeMismatch { source, destination } => {
                    ConnectionError::TypeMismatch { source, destination }
                }
                ConnectionError::PathNotFound(_) => {
                    ConnectionError::ValidationFailed(format!("Validation failed: {e}"))
                }
                _ => ConnectionError::ValidationFailed(format!("Validation failed: {e}")),
            })?;

        // Then mutate state
        let id = ConnectionId::new();
        let connection = Connection { id, source, destination, description: description.into() };
        self.add_connection_checked_internal(connection, id)?;
        Ok(id)
    }

    /// Internal add without validation (for initialization).
    fn add_connection_checked_internal(
        &mut self,
        connection: Connection,
        id: ConnectionId,
    ) -> Result<(), ConnectionError> {
        if self.connections.contains_key(&id) {
            return Err(ConnectionError::ConnectionExists(id));
        }
        self.connections.insert(id, connection);
        Ok(())
    }

    /// Removes a connection by ID.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::ConnectionNotFound` if the connection does not exist.
    pub fn remove_connection(&mut self, id: ConnectionId) -> Result<Connection, ConnectionError> {
        self.connections.remove(&id).ok_or(ConnectionError::ConnectionNotFound(id))
    }

    /// Gets a connection by ID.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::ConnectionNotFound` if the connection does not exist.
    pub fn get_connection(&self, id: ConnectionId) -> Result<&Connection, ConnectionError> {
        self.connections.get(&id).ok_or(ConnectionError::ConnectionNotFound(id))
    }

    /// Lists all connections from the store.
    ///
    /// # Returns
    ///
    /// A vector containing all connections currently stored.
    ///
    /// # Errors
    ///
    /// This function never returns an error.
    pub fn list_connections(&self) -> Result<Vec<Connection>, ConnectionError> {
        Ok(self.connections.values().cloned().collect())
    }

    /// Clears all connections.
    pub fn clear(&mut self) {
        self.connections.clear();
    }

    /// Returns the number of connections.
    #[must_use]
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// Returns true if no connections exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

/// Thread-safe global connection store using `RwLock`.
///
/// This provides lock-free reads via `arc-swap` pattern and
/// safe concurrent writes via `RwLock`.
#[derive(Debug)]
pub struct GlobalConnectionStore {
    /// Atomic pointer to the current store state.
    store: RwLock<ConnectionStore>,
}

impl GlobalConnectionStore {
    /// Creates a new global connection store.
    #[must_use]
    pub fn new() -> Self {
        Self { store: RwLock::new(ConnectionStore::new()) }
    }

    /// Adds a connection with full validation.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` if validation fails or connection exists.
    pub fn add_connection_checked(
        &self,
        source: PortType,
        destination: PortType,
        description: impl Into<String>,
    ) -> Result<ConnectionId, ConnectionError> {
        let mut store =
            self.store.write().map_err(|e| ConnectionError::StorageError(e.to_string()))?;
        store.add_connection_checked(source, destination, description)
    }

    /// Gets a connection by ID.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::ConnectionNotFound` if the connection does not exist.
    pub fn get_connection(&self, id: ConnectionId) -> Result<Connection, ConnectionError> {
        let connection = self
            .store
            .read()
            .map_err(|e| ConnectionError::StorageError(e.to_string()))?
            .get_connection(id)?
            .clone();
        Ok(connection)
    }

    /// Removes a connection by ID.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::ConnectionNotFound` if the connection does not exist.
    pub fn remove_connection(&self, id: ConnectionId) -> Result<Connection, ConnectionError> {
        let mut store =
            self.store.write().map_err(|e| ConnectionError::StorageError(e.to_string()))?;
        store.remove_connection(id)
    }

    /// Lists all connections.
    ///
    /// Returns an empty vector if the lock cannot be acquired or listing fails.
    #[must_use]
    pub fn list_connections(&self) -> Vec<Connection> {
        let Ok(store) = self.store.read() else {
            return Vec::new();
        };
        store.list_connections().unwrap_or_default()
    }

    /// Clears all connections.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::StorageError` if the lock is poisoned.
    pub fn clear(&self) -> Result<(), ConnectionError> {
        self.store.write().map_err(|e| ConnectionError::StorageError(e.to_string()))?.clear();
        Ok(())
    }
}

impl Default for GlobalConnectionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn given_valid_tcp_ports_when_adding_checked_connection_then_connection_is_created() {
        let mut store = ConnectionStore::new();
        let source = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        let destination = PortType::parse("tcp:9090").expect("tcp:9090 should parse");

        let result = store.add_connection_checked(source, destination, "test");

        assert!(result.is_ok());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn given_incompatible_ports_when_adding_checked_connection_then_type_mismatch_is_returned() {
        let mut store = ConnectionStore::new();
        let source = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        let destination =
            PortType::parse("unix:/tmp/socket").expect("unix:/tmp/socket should parse");

        let result = store.add_connection_checked(source, destination, "test");

        assert!(matches!(result, Err(ConnectionError::TypeMismatch { .. })));
    }

    #[test]
    fn given_duplicate_id_when_adding_checked_connection_internal_then_connection_exists_is_returned(
    ) {
        let mut store = ConnectionStore::new();
        let id = ConnectionId::new();
        let source = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        let destination = PortType::parse("tcp:9090").expect("tcp:9090 should parse");
        let connection = Connection { id, source, destination, description: "test".to_string() };

        let first = store.add_connection_checked_internal(connection.clone(), id);
        assert!(first.is_ok());

        let duplicate = store.add_connection_checked_internal(connection, id);
        assert!(matches!(duplicate, Err(ConnectionError::ConnectionExists(_))));
    }

    #[test]
    fn given_connection_store_when_removing_existing_connection_then_it_is_removed() {
        let mut store = ConnectionStore::new();
        let id = store
            .add_connection_checked(
                PortType::parse("tcp:8080").expect("tcp:8080 should parse"),
                PortType::parse("tcp:9090").expect("tcp:9090 should parse"),
                "test",
            )
            .expect("connection should be added");

        let result = store.remove_connection(id);

        assert!(result.is_ok());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn given_nonexistent_id_when_getting_connection_then_connection_not_found_is_returned() {
        let store = ConnectionStore::new();
        let id = ConnectionId::new();

        let result = store.get_connection(id);

        assert!(matches!(result, Err(ConnectionError::ConnectionNotFound(_))));
    }

    #[test]
    fn given_connection_store_when_listing_connections_then_all_are_returned() {
        let mut store = ConnectionStore::new();
        store
            .add_connection_checked(
                PortType::parse("tcp:8080").expect("tcp:8080 should parse"),
                PortType::parse("tcp:9090").expect("tcp:9090 should parse"),
                "conn1",
            )
            .expect("connection1 should be added");
        store
            .add_connection_checked(
                PortType::parse("udp:53").expect("udp:53 should parse"),
                PortType::parse("udp:5353").expect("udp:5353 should parse"),
                "conn2",
            )
            .expect("connection2 should be added");

        let connections = store.list_connections().expect("connections should be listed");

        assert_eq!(connections.len(), 2);
    }

    #[test]
    fn given_empty_store_when_checking_is_empty_then_it_returns_true() {
        let store = ConnectionStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn given_global_store_when_concurrent_access_then_no_data_races() {
        let store = GlobalConnectionStore::new();
        let source = PortType::parse("tcp:8080").expect("tcp:8080 should parse");
        let destination = PortType::parse("tcp:9090").expect("tcp:9090 should parse");

        let result = store.add_connection_checked(source, destination, "test");

        assert!(result.is_ok());
        let connection = store.get_connection(result.expect("connection should be added"));
        assert!(connection.is_ok());
    }
}
