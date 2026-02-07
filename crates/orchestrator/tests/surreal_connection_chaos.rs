//! Chaos tests for SurrealDB connection management.
//!
//! These tests validate the robustness of the connection manager under failure conditions:
//! - Connection pool exhaustion
//! - Connection timeout handling
//! - Retry logic with exponential backoff
//! - Transaction failure recovery
//! - Concurrent access stress testing

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use oya_orchestrator::actors::storage::surreal_integration::{
    ConnectionManagerConfig, DatabaseConfig, RetryPolicy, SurrealConnectionManager, SurrealError,
};
use tokio::time::timeout;
use tracing::debug;

/// Helper to create a test database config.
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

/// Test connection pool exhaustion and recovery.
#[tokio::test]
async fn chaos_test_pool_exhaustion_and_recovery() {
    let manager = SurrealConnectionManager::new(test_config("pool_exhaustion"))
        .await
        .expect("Failed to create manager");

    // Acquire all connections
    let mut connections = Vec::new();
    for i in 0..3 {
        let conn = manager
            .get_connection()
            .await
            .expect("Failed to get connection");
        connections.push(conn);
        debug!("Acquired connection {}", i);
    }

    // Try to acquire one more (should block or fail)
    let start = Instant::now();
    let result = timeout(Duration::from_millis(200), manager.get_connection()).await;

    let elapsed = start.elapsed();

    // Should timeout or take at least 200ms
    assert!(
        result.is_err() || elapsed >= Duration::from_millis(150),
        "Pool should be exhausted, elapsed: {:?}, result: {:?}",
        elapsed,
        result
    );

    // Drop one connection
    connections.pop();
    debug!("Dropped one connection");

    // Now we should be able to acquire a new connection
    let conn = manager
        .get_connection()
        .await
        .expect("Should be able to acquire connection after dropping one");

    debug!("Successfully acquired connection after recovery");

    drop(conn);

    // Cleanup
    connections.clear();
}

/// Test retry logic with exponential backoff.
#[tokio::test]
async fn chaos_test_retry_with_exponential_backoff() {
    let manager = SurrealConnectionManager::new(
        test_config("retry_backoff")
            .with_retry_policy(RetryPolicy::new(5, 50, 500).without_jitter()),
    )
    .await
    .expect("Failed to create manager");

    let mut attempt_count = 0;

    let result = manager
        .execute_with_retry(|_conn| async {
            attempt_count = attempt_count.saturating_add(1);

            // Fail first 3 attempts
            if attempt_count < 4 {
                Err(SurrealError::QueryFailed(format!(
                    "Simulated failure {}",
                    attempt_count
                )))
            } else {
                Ok("success".to_string())
            }
        })
        .await;

    assert!(result.is_ok(), "Should succeed after retries");
    assert_eq!(result.expect("Result should be Ok"), "success");
    assert_eq!(attempt_count, 4, "Should have made 4 attempts");
}

/// Test query timeout handling.
#[tokio::test]
async fn chaos_test_query_timeout() {
    let manager = SurrealConnectionManager::new(
        test_config("query_timeout").with_query_timeout(Duration::from_millis(200)),
    )
    .await
    .expect("Failed to create manager");

    let result = manager
        .execute_with_retry(|_conn| async {
            // Simulate a slow operation
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok::<(), SurrealError>(())
        })
        .await;

    // Should fail with timeout
    assert!(result.is_err(), "Should fail due to timeout");

    match result {
        Err(SurrealError::RetryLimitExceeded(_)) => {
            // Expected - timeout should trigger retry limit
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
        Ok(_) => {
            panic!("Should have failed due to timeout");
        }
    }
}

/// Test concurrent access stress.
#[tokio::test]
async fn chaos_test_concurrent_access() {
    let manager = Arc::new(
        SurrealConnectionManager::new(
            test_config("concurrent_access")
                .with_max_connections(5)
                .with_query_timeout(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to create manager"),
    );

    let mut handles = Vec::new();

    // Spawn 20 concurrent tasks
    for i in 0..20 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let result = manager_clone
                .execute_with_retry(|conn| async move {
                    // Simulate some work
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Verify connection is working
                    conn.client()
                        .query("RETURN 1")
                        .await
                        .map_err(|e| SurrealError::QueryFailed(e.to_string()))?;

                    Ok::<i32, SurrealError>(i)
                })
                .await;

            result
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut success_count = 0;
    let mut failure_count = 0;

    for handle in handles {
        let result = handle.await.expect("Task should complete");

        match result {
            Ok(_) => success_count = success_count.saturating_add(1),
            Err(_) => failure_count = failure_count.saturating_add(1),
        }
    }

    debug!(
        "Concurrent access test: {} successes, {} failures",
        success_count, failure_count
    );

    // Most should succeed despite pool contention
    assert!(
        success_count >= 15,
        "Expected at least 15 successes, got {}",
        success_count
    );
}

/// Test connection resilience during operation.
#[tokio::test]
async fn chaos_test_connection_resilience() {
    let manager = SurrealConnectionManager::new(
        test_config("resilience").with_retry_policy(RetryPolicy::new(3, 50, 200).without_jitter()),
    )
    .await
    .expect("Failed to create manager");

    let mut operations_completed = 0;

    // Execute multiple operations with simulated failures
    for i in 0..10 {
        let result = manager
            .execute_with_retry(|_conn| async move {
                // Simulate intermittent failures
                if i % 3 == 0 {
                    Err(SurrealError::QueryFailed(
                        "Simulated connection instability".to_string(),
                    ))
                } else {
                    operations_completed = operations_completed.saturating_add(1);
                    Ok::<String, SurrealError>(format!("op_{}", i))
                }
            })
            .await;

        // Every operation should eventually succeed or exhaust retries
        // We don't assert here because some may legitimately fail after retry limit
        debug!("Operation {} result: {:?}", i, result.is_ok());
    }

    debug!(
        "Resilience test: {} operations completed successfully",
        operations_completed
    );

    // At least some operations should succeed
    assert!(
        operations_completed >= 5,
        "Expected at least 5 successful operations"
    );
}

/// Test health check under stress.
#[tokio::test]
async fn chaos_test_health_check_under_stress() {
    let manager = Arc::new(
        SurrealConnectionManager::new(test_config("health_check_stress").with_max_connections(3))
            .await
            .expect("Failed to create manager"),
    );

    let mut handles = Vec::new();

    // Spawn concurrent health checks
    for _ in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move { manager_clone.health_check().await });

        handles.push(handle);
    }

    let mut success_count = 0;

    for handle in handles {
        let result = handle.await.expect("Task should complete");

        match result {
            Ok(()) => success_count = success_count.saturating_add(1),
            Err(e) => {
                debug!("Health check failed: {:?}", e);
            }
        }
    }

    debug!("Health check stress test: {} / 10 passed", success_count);

    // Most health checks should succeed
    assert!(
        success_count >= 7,
        "Expected at least 7 successful health checks"
    );
}

/// Test connection cleanup on drop.
#[tokio::test]
async fn chaos_test_connection_cleanup() {
    let manager = SurrealConnectionManager::new(test_config("cleanup").with_max_connections(2))
        .await
        .expect("Failed to create manager");

    // Acquire both connections
    let conn1 = manager.get_connection().await.expect("Failed to get conn1");
    let conn2 = manager.get_connection().await.expect("Failed to get conn2");

    // Verify pool is exhausted
    let start = Instant::now();
    let result = timeout(Duration::from_millis(100), manager.get_connection()).await;
    assert!(result.is_err(), "Pool should be exhausted");

    // Drop both connections
    drop(conn1);
    drop(conn2);

    // Give a moment for cleanup
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Now we should be able to acquire connections again
    let _conn3 = manager
        .get_connection()
        .await
        .expect("Should be able to acquire after cleanup");
    let _conn4 = manager
        .get_connection()
        .await
        .expect("Should be able to acquire second connection");
}

/// Test retry policy limits.
#[tokio::test]
async fn chaos_test_retry_limits() {
    let manager = SurrealConnectionManager::new(
        test_config("retry_limits")
            .with_retry_policy(RetryPolicy::new(3, 50, 200).without_jitter()),
    )
    .await
    .expect("Failed to create manager");

    let mut attempt_count = 0;

    let start = Instant::now();

    let result = manager
        .execute_with_retry(|_conn| async {
            attempt_count = attempt_count.saturating_add(1);

            // Always fail
            Err::<(), SurrealError>(SurrealError::QueryFailed(format!(
                "Always fails, attempt {}",
                attempt_count
            )))
        })
        .await;

    let elapsed = start.elapsed();

    // Should fail after exceeding retry limit
    assert!(result.is_err(), "Should fail after retry limit");

    match result {
        Err(SurrealError::RetryLimitExceeded(_)) => {
            // Expected
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
        Ok(_) => {
            panic!("Should have failed after retry limit");
        }
    }

    // Should have made exactly max_attempts attempts
    assert_eq!(attempt_count, 3, "Should have made exactly 3 attempts");

    // With backoff of 50ms, 100ms, the total time should be at least 150ms
    assert!(
        elapsed >= Duration::from_millis(140),
        "Should have taken at least 140ms for retries, took {:?}",
        elapsed
    );
}

/// Test rapid connection acquisition and release.
#[tokio::test]
async fn chaos_test_rapid_connection_cycles() {
    let manager =
        SurrealConnectionManager::new(test_config("rapid_cycles").with_max_connections(3))
            .await
            .expect("Failed to create manager");

    // Perform many rapid connection cycles
    for i in 0..50 {
        let conn = manager
            .get_connection()
            .await
            .expect("Failed to get connection");

        // Do a quick operation
        let _ = conn.client().query("RETURN 1").await;

        drop(conn);

        if i % 10 == 0 {
            debug!("Completed {} cycles", i);
        }
    }

    // Final connection should still work
    let conn = manager
        .get_connection()
        .await
        .expect("Should still be able to get connection");

    assert!(
        conn.client()
            .use_ns("chaos_ns")
            .use_db("chaos_db_rapid_cycles")
            .await
            .is_ok()
    );
}

/// Test behavior with zero max_connections (edge case).
#[tokio::test]
async fn chaos_test_zero_max_connections() {
    let storage_path = "/tmp/test_surreal_chaos_zero_conn".to_string();

    let config = ConnectionManagerConfig::new(DatabaseConfig {
        storage_path,
        namespace: "chaos_ns".to_string(),
        database: "chaos_db_zero".to_string(),
    })
    .with_max_connections(0);

    // Manager should still initialize
    let manager = SurrealConnectionManager::new(config)
        .await
        .expect("Manager should initialize even with zero max connections");

    // But getting a connection should fail immediately
    let result = manager.get_connection().await;

    assert!(
        result.is_err(),
        "Should fail to get connection with pool size 0"
    );

    match result {
        Err(SurrealError::PoolExhausted(0)) => {
            // Expected
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
        Ok(_) => {
            panic!("Should not succeed with zero pool size");
        }
    }
}
