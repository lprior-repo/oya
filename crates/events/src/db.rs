#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use futures::stream::{iter, StreamExt, TryStreamExt};
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use thiserror::Error;
use tokio::fs;
use tracing::{debug, info};

pub struct SurrealDbConfig {
    pub path: String,
}

impl SurrealDbConfig {
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self { path }
    }
}

pub struct SurrealDbClient {
    client: Surreal<Db>,
    namespace: String,
    database: String,
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("query execution failed: {0}")]
    QueryFailed(String),

    #[error("schema initialization failed: {0}")]
    SchemaInitFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("surrealdb error: {0}")]
    SurrealDb(String),

    #[error("database is locked by another process. Only one instance of oya can run at a time. If you're sure no other instance is running, delete the LOCK file at: {path}")]
    DatabaseLocked { path: String },
}

impl From<surrealdb::Error> for DbError {
    fn from(err: surrealdb::Error) -> Self {
        Self::SurrealDb(err.to_string())
    }
}

impl SurrealDbClient {
    /// Connects to `SurrealDB` with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database directory cannot be created
    /// - The database is already locked by another process
    /// - The `RocksDB` instance cannot be initialized
    /// - The namespace/database cannot be selected
    pub async fn connect(config: SurrealDbConfig) -> Result<Self, DbError> {
        let path = &config.path;
        debug!(path = %path, "Ensuring database directory exists");
        fs::create_dir_all(path).await?;

        // Check for existing RocksDB lock file
        let lock_path = std::path::PathBuf::from(path).join("LOCK");
        if lock_path.exists() {
            // Try to detect if this is a stale lock or an active one
            // by attempting to connect and handling the specific error
            debug!(lock_path = %lock_path.display(), "LOCK file exists, checking if database is in use");

            // Attempt connection - if it fails with lock error, provide clear message
            let connect_result = Surreal::new::<RocksDb>(path).await;
            let client = match connect_result {
                Ok(c) => {
                    // Connection succeeded despite LOCK file existing
                    // This means the lock was stale or we can connect
                    info!("Connected to database despite existing LOCK file (lock was stale)");
                    c
                }
                Err(e) => {
                    let error_msg = e.to_string().to_lowercase();
                    if error_msg.contains("lock")
                        || error_msg.contains("resource temporarily unavailable")
                        || error_msg.contains("permission denied")
                    {
                        let lock_path_display = lock_path.display().to_string();
                        return Err(DbError::DatabaseLocked {
                            path: lock_path_display,
                        });
                    }
                    return Err(DbError::ConnectionFailed(format!(
                        "Failed to create RocksDb instance: {e}"
                    )));
                }
            };

            // Continue with namespace/database setup
            let namespace = "oya";
            let database = "events";

            info!(
                namespace = %namespace,
                database = %database,
                "Using namespace and database"
            );

            client
                .use_ns(namespace)
                .use_db(database)
                .await
                .map_err(|e| {
                    DbError::ConnectionFailed(format!("Failed to select namespace/database: {e}"))
                })?;

            return Ok(Self {
                client,
                namespace: namespace.to_string(),
                database: database.to_string(),
            });
        }

        // No lock file, proceed with normal connection
        info!(path = %path, "Connecting to SurrealDB with kv-rocksdb backend");
        let client = match Surreal::new::<RocksDb>(path).await {
            Ok(c) => c,
            Err(e) => {
                return Err(DbError::ConnectionFailed(format!(
                    "Failed to create RocksDb instance: {e}"
                )))
            }
        };

        let namespace = "oya";
        let database = "events";

        info!(
            namespace = %namespace,
            database = %database,
            "Using namespace and database"
        );

        client
            .use_ns(namespace)
            .use_db(database)
            .await
            .map_err(|e| {
                DbError::ConnectionFailed(format!("Failed to select namespace/database: {e}"))
            })?;

        Ok(Self {
            client,
            namespace: namespace.to_string(),
            database: database.to_string(),
        })
    }

    /// Initializes the database schema with the given SQL statements.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the schema queries fail to execute.
    pub async fn init_schema(&self, schema_content: &str) -> Result<(), DbError> {
        info!("Initializing database schema");

        let queries: Vec<&str> = schema_content
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let total = queries.len();

        // Use functional iteration with early termination on error
        let succeeded = execute_schema_queries(&self.client, &queries).await?;

        info!(succeeded, total, "Schema initialization completed");
        Ok(())
    }

    #[must_use]
    pub const fn client(&self) -> &Surreal<Db> {
        &self.client
    }

    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    #[must_use]
    pub fn database(&self) -> &str {
        &self.database
    }

    /// Performs a health check on the database connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the health check query fails to execute.
    pub async fn health_check(&self) -> Result<(), DbError> {
        debug!("Performing health check");
        self.client
            .query("RETURN true")
            .await
            .map_err(|e| DbError::QueryFailed(format!("Health check failed: {e}")))?;
        Ok(())
    }
}

/// Executes schema queries sequentially using functional patterns.
///
/// This helper function processes a list of SQL queries, skipping comments
/// and short queries, while providing detailed logging for each executed query.
async fn execute_schema_queries(client: &Surreal<Db>, queries: &[&str]) -> Result<usize, DbError> {
    let total = queries.len();

    // Use functional fold with async processing via futures::stream
    iter(queries.iter().enumerate())
        .then(|(idx, query): (usize, &&str)| async move {
            let trimmed = query.trim();

            // Skip comments and empty queries using functional guards
            if trimmed.starts_with("--") || trimmed.len() < 10 {
                debug!("Skipping comment or short query at index {}", idx + 1);
                return Ok::<usize, DbError>(0);
            }

            debug!(
                query = %trimmed.chars().take(80).collect::<String>(),
                idx = idx + 1,
                total,
                "Executing schema query"
            );

            client
                .query(trimmed)
                .await
                .map(|_| {
                    debug!(idx = idx + 1, total, "Schema query succeeded");
                    1usize // Count successful query
                })
                .map_err(|e| DbError::SchemaInitFailed(format!("Query {} failed: {e}", idx + 1)))
        })
        .try_fold(0usize, |acc, count| async move { Ok(acc + count) })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_connect_and_health_check() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir
            .path()
            .join("test_db")
            .to_string_lossy()
            .to_string();

        let config = SurrealDbConfig::new(db_path);
        let client = SurrealDbClient::connect(config).await?;

        let health = client.health_check().await;
        assert!(health.is_ok(), "Health check should succeed");
        Ok(())
    }

    #[tokio::test]
    async fn test_schema_init() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir
            .path()
            .join("test_db")
            .to_string_lossy()
            .to_string();

        let config = SurrealDbConfig::new(db_path);
        let client = SurrealDbClient::connect(config).await?;

        let schema = "DEFINE TABLE test SCHEMAFULL;";
        let result = client.init_schema(schema).await;
        assert!(result.is_ok(), "Schema init should succeed");
        Ok(())
    }

    #[tokio::test]
    async fn test_schema_init_with_errors() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir
            .path()
            .join("test_db")
            .to_string_lossy()
            .to_string();

        let config = SurrealDbConfig::new(db_path);
        let client = SurrealDbClient::connect(config).await?;

        let schema = "INVALID SQL SYNTAX;";
        let result = client.init_schema(schema).await;
        assert!(result.is_err(), "Invalid schema should fail");
        Ok(())
    }

    #[tokio::test]
    async fn test_database_lock_contention() -> crate::Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir
            .path()
            .join("test_db")
            .to_string_lossy()
            .to_string();

        // First connection should succeed
        let config1 = SurrealDbConfig::new(db_path.clone());
        let _client1 = SurrealDbClient::connect(config1).await?;

        // Second connection to the same path should fail with DatabaseLocked
        let config2 = SurrealDbConfig::new(db_path);
        let result = SurrealDbClient::connect(config2).await;

        assert!(
            result.is_err(),
            "Second connection should fail due to lock contention"
        );

        match result {
            Err(DbError::DatabaseLocked { .. }) => {
                // Expected error type
            }
            Err(other) => {
                unreachable!("Expected DatabaseLocked error, got: {other}");
            }
            Ok(_) => {
                unreachable!("Expected database to be locked, but second connection succeeded");
            }
        }

        Ok(())
    }
}
