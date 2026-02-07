//! Chaos tests for SurrealDB connection management.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::time::{Duration, Instant};
use oya_orchestrator::actors::storage::surreal_integration::{
    ConnectionManagerConfig, DatabaseConfig, RetryPolicy, SurrealConnectionManager, SurrealError,
};
use tokio::time::timeout;
use tracing::debug;

fn test_config(name: &str) -> ConnectionManagerConfig {
    let storage_path = format!("/tmp/test_surreal_chaos_{}", name);
    ConnectionManagerConfig::new(DatabaseConfig {
        storage_path,
        namespace: "chaos_ns".to_string(),
        database: format!("chaos_db_{}", name),
    })
    .with_max_connections(3)
    .with_retry_policy(RetryPolicy::new(4, 50, 300).without_jitter())
    .with_query_timeout(Duration::from_millis(500))
}

#[tokio::test]
async fn chaos_test_basic_connection() {
    let manager = SurrealConnectionManager::new(test_config("basic"))
        .await
        .expect("Failed to create manager");

    let conn = manager.get_connection().await.expect("Failed to get connection");
    assert!(conn.client().use_ns("chaos_ns").use_db("chaos_db_basic").await.is_ok());
    drop(conn);
}

#[tokio::test]
async fn chaos_test_pool_exhaustion() {
    let manager = SurrealConnectionManager::new(test_config("pool_exhaustion"))
        .await
        .expect("Failed to create manager");

    let conn1 = manager.get_connection().await.expect("Failed to get conn1");
    let conn2 = manager.get_connection().await.expect("Failed to get conn2");
    let conn3 = manager.get_connection().await.expect("Failed to get conn3");

    let start = Instant::now();
    let result = timeout(Duration::from_millis(200), manager.get_connection()).await;
    let elapsed = start.elapsed();

    assert!(result.is_err() || elapsed >= Duration::from_millis(150));

    drop(conn1);
    let _conn4 = manager.get_connection().await.expect("Should get connection after drop");
}

#[tokio::test]
async fn chaos_test_retry_logic() {
    let manager = SurrealConnectionManager::new(test_config("retry"))
        .await
        .expect("Failed to create manager");

    let mut attempt_count = 0;
    let result = manager
        .execute_with_retry(|_conn| async {
            attempt_count = attempt_count.saturating_add(1);
            if attempt_count < 3 {
                Err(SurrealError::QueryFailed("Simulated failure".to_string()))
            } else {
                Ok("success".to_string())
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should be Ok"), "success");
}

#[tokio::test]
async fn chaos_test_connection_cleanup() {
    let manager = SurrealConnectionManager::new(test_config("cleanup").with_max_connections(2))
        .await
        .expect("Failed to create manager");

    let conn1 = manager.get_connection().await.expect("Failed to get conn1");
    let conn2 = manager.get_connection().await.expect("Failed to get conn2");

    let start = Instant::now();
    let result = timeout(Duration::from_millis(100), manager.get_connection()).await;
    assert!(result.is_err());

    drop(conn1);
    drop(conn2);

    tokio::time::sleep(Duration::from_millis(50)).await;

    let _conn3 = manager.get_connection().await.expect("Should acquire after cleanup");
}

#[tokio::test]
async fn chaos_test_health_check() {
    let manager = SurrealConnectionManager::new(test_config("health_check"))
        .await
        .expect("Failed to create manager");

    let result = manager.health_check().await;
    assert!(result.is_ok(), "Health check should succeed");
}
