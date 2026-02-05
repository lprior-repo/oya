#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

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
    /// - The `RocksDB` instance cannot be initialized
    /// - The namespace/database cannot be selected
    pub async fn connect(config: SurrealDbConfig) -> Result<Self, DbError> {
        let path = &config.path;
        debug!(path = %path, "Ensuring database directory exists");
        fs::create_dir_all(path).await?;

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
        let mut succeeded = 0;

        for (idx, query) in queries.iter().enumerate() {
            let trimmed = query.trim();
            if trimmed.starts_with("--") {
                debug!("Skipping comment line");
                continue;
            }

            if trimmed.len() < 10 {
                debug!("Skipping empty/short query");
                continue;
            }

            debug!(
                query = %trimmed.chars().take(80).collect::<String>(),
                idx = idx + 1,
                total,
                "Executing schema query"
            );

            match self.client.query(trimmed).await {
                Ok(_) => {
                    succeeded += 1;
                    debug!(idx = idx + 1, total, "Schema query succeeded");
                }
                Err(e) => {
                    return Err(DbError::SchemaInitFailed(format!(
                        "Query {} failed: {e}",
                        idx + 1
                    )));
                }
            }
        }

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
}
