#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chaos tests for SurrealDB connection management.

use orchestrator::actors::storage::{
    surreal_integration::{
        ConnectionManagerConfig, RetryPolicy, SurrealConnectionManager, SurrealError,
    },
    DatabaseConfig,
};
use std::time::{Duration, Instant};
use tokio::time::timeout;

fn test_config(name: &str) -> ConnectionManagerConfig {
    let storage_path = format!("/tmp/test_surreal_chaos_{name}");
    ConnectionManagerConfig::new(DatabaseConfig {
        storage_path,
        namespace: "chaos_ns".to_string(),
        database: format!("chaos_db_{name}"),
    })
    .with_max_connections(3)
    .with_retry_policy(RetryPolicy::new(4, 50, 300).without_jitter())
    .with_query_timeout(Duration::from_millis(500))
}

#[tokio::test]
async fn chaos_test_basic_connection() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SurrealConnectionManager::new(test_config("basic")).await?;

    let conn = manager.get_connection().await?;
    assert!(conn
        .client()
        .use_ns("chaos_ns")
        .use_db("chaos_db_basic")
        .await
        .is_ok());
    drop(conn);
    Ok(())
}

#[tokio::test]
async fn chaos_test_pool_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
    let config = test_config("pool_exhaustion");
    let manager = SurrealConnectionManager::new(config).await?;

    let conn1 = manager.get_connection().await?;
    let _conn2 = manager.get_connection().await?;
    let _conn3 = manager.get_connection().await?;

    let start = Instant::now();
    let _result: Result<Result<_, SurrealError>, _> =
        timeout(Duration::from_millis(200), manager.get_connection()).await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(150));

    drop(conn1);
    let _conn4 = manager.get_connection().await?;
    Ok(())
}

#[tokio::test]
async fn chaos_test_retry_logic() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SurrealConnectionManager::new(test_config("retry")).await?;

    // Use a counter with Arc<Mutex<>> to track attempts across retries
    use std::sync::{Arc, Mutex};
    let attempt_count = Arc::new(Mutex::new(0u32));
    let result = manager
        .execute_with_retry(|_conn| {
            let counter = attempt_count.clone();
            async move {
                let mut count = counter
                    .lock()
                    .map_err(|e| SurrealError::QueryFailed(format!("mutex poisoned: {e}")))?;
                *count = count.saturating_add(1);
                if *count < 3 {
                    Err(SurrealError::QueryFailed("Simulated failure".to_string()))
                } else {
                    Ok("success".to_string())
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.map_err(|e| format!("{e:?}"))?, "success");

    // Verify retries occurred
    let final_count = *attempt_count
        .lock()
        .map_err(|e| format!("mutex poisoned: {e}"))?;
    assert_eq!(final_count, 3);
    Ok(())
}

#[tokio::test]
async fn chaos_test_connection_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let config = test_config("cleanup").with_max_connections(2);
    let manager = SurrealConnectionManager::new(config).await?;

    let conn1 = manager.get_connection().await?;
    let conn2 = manager.get_connection().await?;

    let start = Instant::now();
    let _result: Result<Result<_, SurrealError>, _> =
        timeout(Duration::from_millis(100), manager.get_connection()).await;
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(90));

    drop(conn1);
    drop(conn2);

    tokio::time::sleep(Duration::from_millis(50)).await;

    let _conn3 = manager.get_connection().await?;
    Ok(())
}

#[tokio::test]
async fn chaos_test_health_check() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SurrealConnectionManager::new(test_config("health_check")).await?;

    let result = manager.health_check().await;
    assert!(result.is_ok(), "Health check should succeed");
    Ok(())
}
