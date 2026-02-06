//! SurrealDB client for orchestrator persistence.
//!
//! Provides connection management and basic database operations.

use std::sync::Arc;

use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;

use super::error::{PersistenceResult, from_surrealdb_error};

/// Configuration for the orchestrator store.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Connection URL (e.g., "mem://", "ws://localhost:8000")
    pub url: String,
    /// Namespace to use
    pub namespace: String,
    /// Database to use
    pub database: String,
    /// Optional root credentials
    pub credentials: Option<Credentials>,
}

/// Root credentials for authentication.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
}

impl StoreConfig {
    /// Create an in-memory configuration for testing.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            url: "mem://".to_string(),
            namespace: "orchestrator".to_string(),
            database: "test".to_string(),
            credentials: None,
        }
    }

    /// Create a WebSocket configuration.
    #[must_use]
    pub fn websocket(host: &str, port: u16) -> Self {
        Self {
            url: format!("ws://{}:{}", host, port),
            namespace: "orchestrator".to_string(),
            database: "production".to_string(),
            credentials: None,
        }
    }

    /// Set credentials for authentication.
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.credentials = Some(Credentials {
            username: username.into(),
            password: password.into(),
        });
        self
    }

    /// Set the namespace.
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set the database.
    #[must_use]
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self::in_memory()
    }
}

/// Connection to the orchestrator database.
///
/// This is a thin wrapper around the SurrealDB client that provides
/// orchestrator-specific error handling.
#[derive(Debug, Clone)]
pub struct OrchestratorStore {
    db: Arc<Surreal<Any>>,
    config: StoreConfig,
}

impl OrchestratorStore {
    /// Connect to the database with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails or authentication fails.
    pub async fn connect(config: StoreConfig) -> PersistenceResult<Self> {
        let db = Surreal::<Any>::init();

        // Connect to the database
        db.connect(&config.url)
            .await
            .map_err(from_surrealdb_error)?;

        // Authenticate if credentials are provided
        if let Some(creds) = &config.credentials {
            db.signin(Root {
                username: &creds.username,
                password: &creds.password,
            })
            .await
            .map_err(from_surrealdb_error)?;
        }

        // Use the namespace and database
        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .map_err(from_surrealdb_error)?;

        Ok(Self {
            db: Arc::new(db),
            config,
        })
    }

    /// Get a reference to the underlying database client.
    #[must_use]
    pub fn db(&self) -> &Surreal<Any> {
        &self.db
    }

    /// Get the store configuration.
    #[must_use]
    pub fn config(&self) -> &StoreConfig {
        &self.config
    }

    /// Initialize the database schema.
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    pub async fn initialize_schema(&self) -> PersistenceResult<()> {
        let schema = include_str!("schema.surql");

        self.db.query(schema).await.map_err(from_surrealdb_error)?;

        Ok(())
    }

    /// Check if the database is healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> PersistenceResult<()> {
        // Simple query to verify connectivity using INFO statement
        self.db
            .query("INFO FOR DB")
            .await
            .map_err(from_surrealdb_error)?;

        Ok(())
    }

    /// Close the database connection.
    ///
    /// Note: SurrealDB handles connection cleanup automatically when dropped,
    /// but this method can be used for explicit cleanup.
    pub fn close(&self) {
        // SurrealDB handles cleanup on drop
        // This is a no-op but provides explicit interface
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_config_in_memory() {
        let config = StoreConfig::in_memory();
        assert_eq!(config.url, "mem://");
        assert_eq!(config.namespace, "orchestrator");
        assert_eq!(config.database, "test");
        assert!(config.credentials.is_none());
    }

    #[tokio::test]
    async fn test_store_config_websocket() {
        let config = StoreConfig::websocket("localhost", 8000);
        assert_eq!(config.url, "ws://localhost:8000");
    }

    #[tokio::test]
    async fn test_store_config_with_credentials() {
        let config = StoreConfig::in_memory().with_credentials("root", "secret");

        assert!(config.credentials.is_some());
        if let Some(creds) = config.credentials {
            assert_eq!(creds.username, "root");
            assert_eq!(creds.password, "secret");
        }
    }

    #[tokio::test]
    async fn test_connect_in_memory() {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await;

        assert!(store.is_ok(), "should connect to in-memory database");
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config)
            .await
            .ok()
            .filter(|_| true);

        if let Some(store) = store {
            let health = store.health_check().await;
            assert!(
                health.is_ok(),
                "health check should pass: {:?}",
                health.err()
            );
        }
    }
}
