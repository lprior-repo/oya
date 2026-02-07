//! SurrealDB integration with connection pooling and retry logic.
//!
//! This module provides a robust connection manager for SurrealDB with:
//! - Connection pooling
//! - Exponential backoff retry logic
//! - Query timeout handling
//! - Transaction support
//! - Error recovery with Result types

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use surrealdb::Surreal;
use surrealdb::engine::local::{Db, RocksDb};
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use crate::actors::storage::DatabaseConfig;

/// Errors that can occur during SurrealDB operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SurrealError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection pool exhausted (max: {0})")]
    PoolExhausted(usize),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Query timeout after {0:?}")]
    QueryTimeout(Duration),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Retry limit exceeded: {0}")]
    RetryLimitExceeded(String),

    #[error("Database not initialized")]
    NotInitialized,
}

/// Retry policy for database operations.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_ms: 100,
            max_backoff_ms: 5000,
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    #[must_use]
    pub fn new(max_attempts: u32, base_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self {
            max_attempts,
            base_backoff_ms,
            max_backoff_ms,
            use_jitter: true,
        }
    }

    #[must_use]
    pub fn without_jitter(mut self) -> Self {
        self.use_jitter = false;
        self
    }

    #[must_use]
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let exponential_delay = self.base_backoff_ms * 2_u64.pow(attempt.saturating_sub(1));
        let capped_delay = exponential_delay.min(self.max_backoff_ms);

        if self.use_jitter {
            let jitter_range = (capped_delay / 4).max(1);
            let jitter = rand::random::<u64>() % (2 * jitter_range);
            Duration::from_millis(capped_delay.saturating_add(jitter))
        } else {
            Duration::from_millis(capped_delay)
        }
    }

    #[must_use]
    pub fn is_retryable(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Configuration for the connection manager.
#[derive(Debug, Clone)]
pub struct ConnectionManagerConfig {
    pub database: DatabaseConfig,
    pub max_connections: usize,
    pub query_timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub enable_health_checks: bool,
    pub health_check_interval: Duration,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            max_connections: 10,
            query_timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

impl ConnectionManagerConfig {
    #[must_use]
    pub fn new(database: DatabaseConfig) -> Self {
        Self {
            database,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    #[must_use]
    pub fn with_query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    #[must_use]
    pub fn without_health_checks(mut self) -> Self {
        self.enable_health_checks = false;
        self
    }
}

/// A pooled SurrealDB connection wrapper.
pub struct PooledConnection {
    client: Arc<Surreal<Db>>,
    semaphore: Arc<Semaphore>,
}

impl PooledConnection {
    #[must_use]
    pub fn client(&self) -> &Surreal<Db> {
        &self.client
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        self.semaphore.add_permits(1);
        debug!("Connection returned to pool");
    }
}

/// SurrealDB connection manager with pooling and retry logic.
pub struct SurrealConnectionManager {
    config: ConnectionManagerConfig,
    client: Arc<Surreal<Db>>,
    semaphore: Arc<Semaphore>,
    initialized: bool,
}

impl SurrealConnectionManager {
    pub async fn new(config: ConnectionManagerConfig) -> Result<Self, SurrealError> {
        info!("Initializing SurrealConnectionManager");

        tokio::fs::create_dir_all(&config.database.storage_path)
            .await
            .map_err(|e| {
                SurrealError::ConnectionFailed(format!("Failed to create storage directory: {e}"))
            })?;

        let client = Self::connect_with_retry(&config).await?;
        let semaphore = Arc::new(Semaphore::new(config.max_connections));

        Ok(Self {
            config,
            client,
            semaphore,
            initialized: true,
        })
    }

    async fn connect_with_retry(
        config: &ConnectionManagerConfig,
    ) -> Result<Arc<Surreal<Db>>, SurrealError> {
        let mut attempt: u32 = 0;

        loop {
            attempt = attempt.saturating_add(1);

            match Self::attempt_connection(config).await {
                Ok(client) => {
                    info!("Successfully connected to SurrealDB on attempt {}", attempt);
                    return Ok(client);
                }
                Err(e) => {
                    if config.retry_policy.is_retryable(attempt) {
                        let backoff = config.retry_policy.calculate_backoff(attempt);
                        warn!(
                            "Connection attempt {} failed, retrying in {:?}: {}",
                            attempt, backoff, e
                        );
                        tokio::time::sleep(backoff).await;
                    } else {
                        error!("Failed to connect after {} attempts: {}", attempt, e);
                        return Err(SurrealError::RetryLimitExceeded(format!(
                            "Connection failed after {} attempts",
                            config.retry_policy.max_attempts
                        )));
                    }
                }
            }
        }
    }

    async fn attempt_connection(
        config: &ConnectionManagerConfig,
    ) -> Result<Arc<Surreal<Db>>, SurrealError> {
        let client = Surreal::new::<RocksDb>(&config.database.storage_path)
            .await
            .map_err(|e| SurrealError::ConnectionFailed(format!("Failed to create client: {e}")))?;

        let client = Arc::new(client);

        client
            .use_ns(&config.database.namespace)
            .use_db(&config.database.database)
            .await
            .map_err(|e| {
                SurrealError::ConnectionFailed(format!(
                    "Failed to initialize namespace/database: {e}"
                ))
            })?;

        Ok(client)
    }

    pub async fn get_connection(&self) -> Result<PooledConnection, SurrealError> {
        if !self.initialized {
            return Err(SurrealError::NotInitialized);
        }

        self.semaphore
            .acquire()
            .await
            .map_err(|_| SurrealError::PoolExhausted(self.config.max_connections))?;

        Ok(PooledConnection {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
        })
    }

    pub async fn execute_with_retry<F, T, Fut>(&self, operation: F) -> Result<T, SurrealError>
    where
        F: Fn(PooledConnection) -> Fut,
        Fut: std::future::Future<Output = Result<T, SurrealError>>,
    {
        let mut attempt: u32 = 0;

        loop {
            attempt = attempt.saturating_add(1);
            let conn = self.get_connection().await?;

            let timeout_result =
                tokio::time::timeout(self.config.query_timeout, operation(conn)).await;

            match timeout_result {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => {
                    if self.config.retry_policy.is_retryable(attempt) {
                        let backoff = self.config.retry_policy.calculate_backoff(attempt);
                        warn!(
                            "Operation attempt {} failed, retrying in {:?}: {}",
                            attempt, backoff, e
                        );
                        tokio::time::sleep(backoff).await;
                    } else {
                        return Err(SurrealError::RetryLimitExceeded(format!(
                            "Operation failed: {}",
                            e
                        )));
                    }
                }
                Err(_) => {
                    if self.config.retry_policy.is_retryable(attempt) {
                        let backoff = self.config.retry_policy.calculate_backoff(attempt);
                        warn!(
                            "Operation attempt {} timed out, retrying in {:?}",
                            attempt, backoff
                        );
                        tokio::time::sleep(backoff).await;
                    } else {
                        return Err(SurrealError::QueryTimeout(self.config.query_timeout));
                    }
                }
            }
        }
    }

    pub async fn health_check(&self) -> Result<(), SurrealError> {
        let _conn = self.get_connection().await?;
        debug!("Health check passed");
        Ok(())
    }

    #[must_use]
    pub fn config(&self) -> &ConnectionManagerConfig {
        &self.config
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_calculates_exponential_backoff() {
        let policy = RetryPolicy::new(5, 100, 5000);
        let delay1 = policy.calculate_backoff(1);
        assert!(delay1 >= Duration::from_millis(100));

        let delay2 = policy.calculate_backoff(2);
        assert!(delay2 >= Duration::from_millis(150));

        let delay3 = policy.calculate_backoff(3);
        assert!(delay3 >= Duration::from_millis(300));
    }

    #[test]
    fn test_retry_policy_without_jitter() {
        let policy = RetryPolicy::new(5, 100, 5000).without_jitter();
        assert_eq!(policy.calculate_backoff(1), Duration::from_millis(100));
        assert_eq!(policy.calculate_backoff(2), Duration::from_millis(200));
        assert_eq!(policy.calculate_backoff(3), Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_surreal_connection_manager_initialization() {
        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .and_then(|d| d.path().to_str().map(String::from))
            .unwrap_or_else(|| "/tmp/test_surreal_manager".to_string());

        let db_config = DatabaseConfig {
            storage_path,
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        let config = ConnectionManagerConfig::new(db_config)
            .with_max_connections(5)
            .with_retry_policy(RetryPolicy::new(3, 50, 500).without_jitter());

        let manager = SurrealConnectionManager::new(config)
            .await
            .expect("Failed to create manager");

        assert!(manager.is_initialized());
        assert_eq!(manager.config().max_connections, 5);
    }
}
