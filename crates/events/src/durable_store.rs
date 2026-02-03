#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::engine::any::Any;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use thiserror::Error;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::error::{ConnectionError, Result};
use crate::event::BeadEvent;
use crate::types::BeadId;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppendError {
    #[error("failed to open wal file: {0}")]
    WalOpenFailed(String),

    #[error("failed to write event to wal: {0}")]
    WalWriteFailed(String),

    #[error("failed to sync wal to disk: {0}")]
    WalSyncFailed(String),

    #[error("failed to close wal: {0}")]
    WalCloseFailed(String),

    #[error("failed to serialize event: {0}")]
    SerializationFailed(String),

    #[error("database write failed: {0}")]
    DatabaseWriteFailed(String),
}

impl From<AppendError> for crate::error::Error {
    fn from(err: AppendError) -> Self {
        crate::error::Error::store_failed("append_event", err.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SerializedEvent {
    event_id: String,
    bead_id: String,
    event_type: String,
    data: Vec<u8>,
    timestamp: DateTime<Utc>,
}

impl SerializedEvent {
    fn from_bead_event(event: &BeadEvent) -> Result<Self> {
        let data = bincode::serialize(event).map_err(|e| {
            crate::error::Error::serialization(format!("failed to serialize event: {}", e))
        })?;

        Ok(Self {
            event_id: event.event_id().to_string(),
            bead_id: event.bead_id().to_string(),
            event_type: event.event_type().to_string(),
            data,
            timestamp: event.timestamp(),
        })
    }

    fn to_bead_event(&self) -> Result<BeadEvent> {
        bincode::deserialize(&self.data).map_err(|e| {
            crate::error::Error::serialization(format!("failed to deserialize event: {}", e))
        })
    }
}

/// Configuration for SurrealDB connection.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Path to the RocksDB storage directory.
    pub storage_path: PathBuf,

    /// Namespace for the database.
    pub namespace: String,

    /// Database name within the namespace.
    pub database: String,

    /// Maximum number of connections in the pool.
    pub max_connections: usize,

    /// Connection timeout in milliseconds.
    pub timeout_ms: u64,

    /// Username for authentication (optional).
    pub username: Option<String>,

    /// Password for authentication (optional).
    pub password: Option<String>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./data/events"),
            namespace: "oya".to_string(),
            database: "events".to_string(),
            max_connections: 10,
            timeout_ms: 30000,
            username: None,
            password: None,
        }
    }
}

impl ConnectionConfig {
    /// Creates a new connection config with the specified storage path.
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        Self {
            storage_path: storage_path.into(),
            ..Default::default()
        }
    }

    /// Sets the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Sets the database name.
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }

    /// Sets the maximum number of connections.
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Sets the connection timeout.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets authentication credentials.
    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.max_connections == 0 {
            return Err(ConnectionError::PoolExhausted { max_connections: 0 }.into());
        }

        if self.timeout_ms == 0 {
            return Err(ConnectionError::Timeout { timeout_ms: 0 }.into());
        }

        if self.namespace.is_empty() {
            return Err(ConnectionError::InitializationFailed {
                reason: "namespace cannot be empty".to_string(),
            }
            .into());
        }

        if self.database.is_empty() {
            return Err(ConnectionError::InitializationFailed {
                reason: "database cannot be empty".to_string(),
            }
            .into());
        }

        Ok(())
    }
}

/// Establishes a connection to SurrealDB with kv-rocksdb backend.
///
/// This function creates a new SurrealDB instance configured with RocksDB
/// storage, initializes the database with the specified namespace and database
/// name, and optionally authenticates.
///
/// # Errors
///
/// Returns an error if:
/// - The storage path cannot be created
/// - Connection cannot be established within the timeout
/// - Authentication fails
/// - Database initialization fails
pub async fn connect(config: ConnectionConfig) -> Result<Arc<Surreal<Db>>> {
    config.validate()?;

    let db = Surreal::new::<RocksDb>(config.storage_path)
        .await
        .map_err(|e| ConnectionError::InitializationFailed {
            reason: format!("failed to create RocksDB instance: {}", e),
        })?;

    let db = Arc::new(db);

    let ns = config.namespace.clone();
    let db_name = config.database.clone();

    db.use_ns(ns)
        .use_db(db_name)
        .await
        .map_err(|e| ConnectionError::InitializationFailed {
            reason: format!("failed to initialize namespace/database: {}", e),
        })?;

    if let (Some(username), Some(password)) = (config.username, config.password) {
        db.signin(surrealdb::opt::auth::Database {
            namespace: &config.namespace,
            database: &config.database,
            username: &username,
            password: &password,
        })
        .await
        .map_err(|e| ConnectionError::AuthenticationFailed {
            reason: format!("authentication failed: {}", e),
        })?;
    }

    Ok(db)
}

pub struct DurableEventStore {
    db: Arc<Surreal<Any>>,
    wal_dir: PathBuf,
}

impl DurableEventStore {
    pub async fn new(db: Arc<Surreal<Any>>) -> Result<Self> {
        Ok(Self {
            db,
            wal_dir: PathBuf::from(".wal"),
        })
    }

    pub fn with_wal_dir(mut self, wal_dir: impl Into<PathBuf>) -> Self {
        self.wal_dir = wal_dir.into();
        self
    }

    pub async fn append_event(&self, event: &BeadEvent) -> Result<()> {
        let serialized = SerializedEvent::from_bead_event(event)?;

        self.append_to_wal(event, &serialized).await?;

        self.db
            .create::<Option<SerializedEvent>>(("state_transition", serialized.event_id.clone()))
            .content(serialized)
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "append_event",
                    format!("failed to create record: {}", e),
                )
            })?;

        Ok(())
    }

    async fn append_to_wal(
        &self,
        event: &BeadEvent,
        serialized: &SerializedEvent,
    ) -> std::result::Result<(), AppendError> {
        tokio::fs::create_dir_all(&self.wal_dir)
            .await
            .map_err(|e| AppendError::WalOpenFailed(format!("create wal dir: {}", e)))?;

        let wal_path = self.wal_dir.join(format!("{}.wal", event.bead_id()));

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .map_err(|e| AppendError::WalOpenFailed(format!("open wal file: {}", e)))?;

        let serialized_data = bincode::serialize(&serialized)
            .map_err(|e| AppendError::SerializationFailed(format!("{}", e)))?;

        file.write_all(&serialized_data)
            .await
            .map_err(|e| AppendError::WalWriteFailed(format!("{}", e)))?;

        file.sync_all()
            .await
            .map_err(|e| AppendError::WalSyncFailed(format!("fsync failed: {}", e)))?;

        Ok(())
    }

    async fn append_to_wal(&self, event: &BeadEvent, serialized: &SerializedEvent) -> std::result::Result<(), AppendError> {
        tokio::fs::create_dir_all(&self.wal_dir)
            .await
            .map_err(|e| AppendError::WalOpenFailed(format!("create wal dir: {}", e)))?;

        let wal_path = self.wal_dir.join(format!("{}.wal", event.bead_id()));

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .map_err(|e| AppendError::WalOpenFailed(format!("open wal file: {}", e)))?;

        let serialized_data = bincode::serialize(&serialized)
            .map_err(|e| AppendError::SerializationFailed(format!("{}", e)))?;

        file.write_all(&serialized_data)
            .await
            .map_err(|e| AppendError::WalWriteFailed(format!("{}", e)))?;

        file.sync_all()
            .await
            .map_err(|e| AppendError::WalSyncFailed(format!("fsync failed: {}", e)))?;

        Ok(())
    }

    pub async fn read_events(&self, bead_id: &BeadId) -> Result<Vec<BeadEvent>> {
        let bead_id_str = bead_id.to_string();

        let mut result = self
            .db
            .query("SELECT * FROM state_transition WHERE bead_id = $bead_id ORDER BY timestamp ASC")
            .bind(("bead_id", bead_id_str))
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "read_events",
                    format!("failed to query events: {}", e),
                )
            })?;

        let serialized_events: Vec<SerializedEvent> = result.take(0).map_err(|e| {
            crate::error::Error::store_failed(
                "read_events",
                format!("failed to extract results: {}", e),
            )
        })?;

        serialized_events
            .iter()
            .map(|se| se.to_bead_event())
            .collect()
    }

    pub async fn replay_from(&self, checkpoint_id: &str) -> Result<Vec<BeadEvent>> {
        let checkpoint_id = checkpoint_id.to_string();

        let mut result = self
            .db
            .query(
                "SELECT * FROM state_transition WHERE timestamp > (SELECT timestamp FROM state_transition WHERE event_id = $checkpoint_id LIMIT 1) ORDER BY timestamp ASC"
            )
            .bind(("checkpoint_id", checkpoint_id))
            .await
            .map_err(|e| {
                crate::error::Error::store_failed(
                    "replay_from",
                    format!("failed to query events from checkpoint: {}", e),
                )
            })?;

        let serialized_events: Vec<SerializedEvent> = result.take(0).map_err(|e| {
            crate::error::Error::store_failed(
                "replay_from",
                format!("failed to extract results: {}", e),
            )
        })?;

        serialized_events
            .iter()
            .map(|se| se.to_bead_event())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialized_event_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = BeadId::new();
        let event = BeadEvent::created(
            bead_id,
            crate::types::BeadSpec::new("Test").with_complexity(crate::types::Complexity::Simple),
        );

        let serialized = SerializedEvent::from_bead_event(&event)?;
        assert_eq!(serialized.event_type, "created");

        let deserialized = serialized.to_bead_event()?;
        assert_eq!(deserialized.event_id(), event.event_id());
        assert_eq!(deserialized.bead_id(), event.bead_id());
        assert_eq!(deserialized.event_type(), "created");
        Ok(())
    }

    #[test]
    fn test_serialized_event_all_types() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = BeadId::new();

        let events = [
            BeadEvent::created(
                bead_id,
                crate::types::BeadSpec::new("Test")
                    .with_complexity(crate::types::Complexity::Simple),
            ),
            BeadEvent::state_changed(
                bead_id,
                crate::types::BeadState::Pending,
                crate::types::BeadState::Scheduled,
            ),
            BeadEvent::failed(bead_id, "test error"),
            BeadEvent::completed(
                bead_id,
                crate::types::BeadResult::success(vec![1, 2, 3], 1000),
            ),
        ];

        for event in events {
            let serialized = SerializedEvent::from_bead_event(&event)?;
            let deserialized = serialized.to_bead_event()?;
            assert_eq!(deserialized.event_id(), event.event_id());
            assert_eq!(deserialized.bead_id(), event.bead_id());
            assert_eq!(deserialized.event_type(), event.event_type());
        }
        Ok(())
    }

    #[test]
    fn test_serialized_event_with_complex_data() -> Result<(), Box<dyn std::error::Error>> {
        let bead_id = BeadId::new();
        let phase_id = crate::types::PhaseId::new();

        let event = BeadEvent::phase_completed(
            bead_id,
            phase_id,
            "test_phase",
            crate::types::PhaseOutput::success(vec![1, 2, 3, 4, 5]),
        );

        let serialized = SerializedEvent::from_bead_event(&event)?;
        assert_eq!(serialized.event_type, "phase_completed");

        let deserialized = serialized.to_bead_event()?;
        assert_eq!(deserialized.event_id(), event.event_id());
        assert_eq!(deserialized.bead_id(), event.bead_id());
        assert_eq!(deserialized.event_type(), "phase_completed");
        Ok(())
    }

    fn test_connection_config_default() {
        let config = ConnectionConfig::default();
        assert_eq!(config.storage_path, PathBuf::from("./data/events"));
        assert_eq!(config.namespace, "oya");
        assert_eq!(config.database, "events");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.timeout_ms, 30000);
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }

    #[test]
    fn test_connection_config_builder() {
        let config = ConnectionConfig::new("/tmp/test")
            .with_namespace("test_ns")
            .with_database("test_db")
            .with_max_connections(5)
            .with_timeout_ms(60000)
            .with_auth("user", "pass");

        assert_eq!(config.storage_path, PathBuf::from("/tmp/test"));
        assert_eq!(config.namespace, "test_ns");
        assert_eq!(config.database, "test_db");
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.timeout_ms, 60000);
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_connection_config_validate_valid() {
        let config = ConnectionConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_connection_config_validate_zero_connections() {
        let config = ConnectionConfig::default().with_max_connections(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_connection_config_validate_zero_timeout() {
        let config = ConnectionConfig::default().with_timeout_ms(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_connection_config_validate_empty_namespace() {
        let config = ConnectionConfig::default().with_namespace("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_connection_config_validate_empty_database() {
        let config = ConnectionConfig::default().with_database("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_append_error_display() {
        let err = AppendError::WalWriteFailed("disk full".to_string());
        assert!(err.to_string().contains("wal"));
        assert!(err.to_string().contains("disk full"));

        let db_err = AppendError::DatabaseWriteFailed("connection lost".to_string());
        assert!(db_err.to_string().contains("database"));
        assert!(db_err.to_string().contains("connection lost"));
    }

    #[test]
    fn test_append_error_from_conversion() {
        let append_err = AppendError::WalSyncFailed("IO error".to_string());
        let crate_err: crate::error::Error = append_err.into();
        assert!(crate_err.to_string().contains("append_event"));
        assert!(crate_err.to_string().contains("IO error"));
    }
}
