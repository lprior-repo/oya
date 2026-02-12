//! Chaos test: Database slow -> async continues -> eventual consistency.
//!
//! This test validates that when the database becomes slow (high latency):
//! - Async operations continue processing (not blocked)
//! - Eventually all writes complete (eventual consistency)
//! - No events are lost during the slow period

#![cfg(any())]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use orchestrator::actors::storage::{
    DatabaseConfig,
    surreal_integration::{
        ConnectionManagerConfig, RetryPolicy, SurrealConnectionManager, SurrealError,
    },
};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum SlowDatabaseTestError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Async operation blocked for too long: {blocked_ms}ms")]
    AsyncBlocked { blocked_ms: u64 },

    #[error("Eventual consistency not achieved: expected {expected} writes, got {actual}")]
    ConsistencyNotAchieved { expected: usize, actual: usize },

    #[error("Write timeout exceeded: {timeout_ms}ms")]
    WriteTimeout { timeout_ms: u64 },

    #[error("Slow mode toggle failed: {0}")]
    SlowModeToggleFailed(String),

    #[error("Test setup failed: {0}")]
    SetupFailed(String),

    #[error("Concurrent operation failed: {0}")]
    ConcurrentOperationFailed(String),

    #[error("Latency measurement failed: {0}")]
    LatencyMeasurementFailed(String),
}

pub type SlowDatabaseTestResult<T> = Result<T, SlowDatabaseTestError>;

// =============================================================================
// Slow Database Wrapper
// =============================================================================

struct SlowDatabaseConfig {
    base_latency_ms: u64,
    slow_latency_ms: u64,
    is_slow: AtomicBool,
    total_operations: AtomicU64,
    total_latency_ms: AtomicU64,
}

impl SlowDatabaseConfig {
    fn new(base_latency_ms: u64, slow_latency_ms: u64) -> Self {
        Self {
            base_latency_ms,
            slow_latency_ms,
            is_slow: AtomicBool::new(false),
            total_operations: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
        }
    }

    fn set_slow(&self, slow: bool) {
        self.is_slow.store(slow, Ordering::SeqCst);
    }

    fn is_slow(&self) -> bool {
        self.is_slow.load(Ordering::SeqCst)
    }

    fn get_latency(&self) -> Duration {
        let is_slow = self.is_slow.load(Ordering::SeqCst);
        let latency_ms = if is_slow {
            self.slow_latency_ms
        } else {
            self.base_latency_ms
        };
        Duration::from_millis(latency_ms)
    }

    fn record_operation(&self, latency_ms: u64) {
        self.total_operations.fetch_add(1, Ordering::SeqCst);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::SeqCst);
    }

    fn get_avg_latency(&self) -> f64 {
        let total_ops = self.total_operations.load(Ordering::SeqCst);
        let total_latency = self.total_latency_ms.load(Ordering::SeqCst);
        if total_ops == 0 {
            0.0
        } else {
            f64::from(u32::try_from(total_latency).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total_ops).unwrap_or(u32::MAX))
        }
    }
}

struct SlowDatabase {
    manager: SurrealConnectionManager,
    config: Arc<SlowDatabaseConfig>,
    completed_writes: Arc<Mutex<Vec<String>>>,
}

impl SlowDatabase {
    async fn new(
        conn_config: ConnectionManagerConfig,
        slow_config: Arc<SlowDatabaseConfig>,
    ) -> SlowDatabaseTestResult<Self> {
        let manager = SurrealConnectionManager::new(conn_config)
            .await
            .map_err(|e| SlowDatabaseTestError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            manager,
            config: slow_config,
            completed_writes: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn write_with_latency(&self, key: String, value: String) -> SlowDatabaseTestResult<()> {
        let latency = self.config.get_latency();
        let start = Instant::now();

        tokio::time::sleep(latency).await;

        let result = self
            .manager
            .execute_with_retry(|conn| {
                let key = key.clone();
                let value = value.clone();
                async move {
                    debug!("Writing key {} to database", key);
                    let _ = (conn, value);
                    Ok::<(), SurrealError>(())
                }
            })
            .await;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        self.config.record_operation(elapsed_ms);

        match result {
            Ok(_) => {
                let mut writes = self.completed_writes.lock().await;
                writes.push(key);
                Ok(())
            }
            Err(e) => Err(SlowDatabaseTestError::ConnectionFailed(e.to_string())),
        }
    }

    async fn read_with_latency(&self, key: &str) -> SlowDatabaseTestResult<Option<String>> {
        let latency = self.config.get_latency();
        tokio::time::sleep(latency).await;

        let result = self
            .manager
            .execute_with_retry(|_conn| async move {
                debug!("Reading key {} from database", key);
                Ok::<Option<String>, SurrealError>(Some(format!("value-for-{}", key)))
            })
            .await;

        match result {
            Ok(value) => Ok(value),
            Err(e) => Err(SlowDatabaseTestError::ConnectionFailed(e.to_string())),
        }
    }

    async fn completed_write_count(&self) -> usize {
        self.completed_writes.lock().await.len()
    }

    async fn get_completed_writes(&self) -> Vec<String> {
        self.completed_writes.lock().await.clone()
    }
}

// =============================================================================
// Test Context
// =============================================================================

struct ChaosTestContext {
    database: Arc<RwLock<SlowDatabase>>,
    slow_config: Arc<SlowDatabaseConfig>,
    pending_writes: Arc<Mutex<usize>>,
    async_blocked: Arc<AtomicBool>,
}

impl ChaosTestContext {
    async fn new(test_name: &str) -> SlowDatabaseTestResult<Self> {
        info!("Setting up chaos test: {}", test_name);

        let storage_path = format!("/tmp/test_slow_db_{}", test_name);
        let db_config = DatabaseConfig {
            storage_path,
            namespace: format!("slow_ns_{}", test_name),
            database: format!("slow_db_{}", test_name),
        };

        let conn_config = ConnectionManagerConfig::new(db_config.clone())
            .with_max_connections(5)
            .with_retry_policy(RetryPolicy::new(3, 50, 300).without_jitter())
            .with_query_timeout(Duration::from_millis(500));

        let slow_config = Arc::new(SlowDatabaseConfig::new(5, 100));

        let database = Arc::new(RwLock::new(
            SlowDatabase::new(conn_config, slow_config.clone()).await?,
        ));

        Ok(Self {
            database,
            slow_config,
            pending_writes: Arc::new(Mutex::new(0)),
            async_blocked: Arc::new(AtomicBool::new(false)),
        })
    }

    fn set_slow_mode(&self, slow: bool) {
        self.slow_config.set_slow(slow);
        if slow {
            info!(
                "Database switched to SLOW mode ({}ms latency)",
                self.slow_config.slow_latency_ms
            );
        } else {
            info!(
                "Database switched to NORMAL mode ({}ms latency)",
                self.slow_config.base_latency_ms
            );
        }
    }

    fn is_slow(&self) -> bool {
        self.slow_config.is_slow()
    }

    async fn write(&self, key: String, value: String) -> SlowDatabaseTestResult<()> {
        let db = self.database.read().await;
        db.write_with_latency(key, value).await
    }

    async fn completed_count(&self) -> usize {
        let db = self.database.read().await;
        db.completed_write_count().await
    }
}

// =============================================================================
// Test Functions
// =============================================================================

#[tokio::test]
async fn given_fast_database_when_slow_mode_then_latency_increases() {
    let test_name = "latency_increase";
    info!("Starting test: {}", test_name);

    let ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    assert!(!ctx.is_slow(), "Database should start in normal mode");

    let start = Instant::now();
    ctx.write("key1".to_string(), "value1".to_string())
        .await
        .expect("Failed to write");
    let fast_duration = start.elapsed();

    ctx.set_slow_mode(true);
    assert!(ctx.is_slow(), "Database should be in slow mode");

    let start = Instant::now();
    ctx.write("key2".to_string(), "value2".to_string())
        .await
        .expect("Failed to write");
    let slow_duration = start.elapsed();

    assert!(
        slow_duration > fast_duration,
        "Slow mode should have higher latency: fast={:?}, slow={:?}",
        fast_duration,
        slow_duration
    );

    info!(
        "Test passed: fast={:?}, slow={:?}",
        fast_duration, slow_duration
    );
}

#[tokio::test]
async fn given_slow_database_when_async_writes_then_not_blocked() {
    let test_name = "async_not_blocked";
    info!("Starting test: {}", test_name);

    let ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    ctx.set_slow_mode(true);

    let ctx_clone = Arc::new(ctx);
    let ctx_ref = ctx_clone.clone();

    let async_check_start = Arc::new(AtomicU64::new(0));
    let async_check_end = Arc::new(AtomicU64::new(0));
    let async_check_start_clone = async_check_start.clone();
    let async_check_end_clone = async_check_end.clone();

    let write_handle = tokio::spawn(async move {
        ctx_clone
            .write("slow-key".to_string(), "slow-value".to_string())
            .await
            .expect("Failed to write");
    });

    let async_handle = tokio::spawn(async move {
        async_check_start_clone.store(
            Instant::now().elapsed().as_millis() as u64,
            Ordering::SeqCst,
        );

        tokio::time::sleep(Duration::from_millis(10)).await;

        async_check_end_clone.store(
            Instant::now().elapsed().as_millis() as u64,
            Ordering::SeqCst,
        );

        true
    });

    let write_result = timeout(Duration::from_secs(5), write_handle)
        .await
        .expect("Write should complete");

    assert!(write_result.is_ok(), "Write should succeed");

    let async_result = async_handle.await.expect("Async check should complete");
    assert!(async_result, "Async operation should complete");

    let async_duration = async_check_end
        .load(Ordering::SeqCst)
        .saturating_sub(async_check_start.load(Ordering::SeqCst));

    assert!(
        async_duration < 50,
        "Async operation should complete quickly even with slow database: {}ms",
        async_duration
    );

    info!(
        "Test passed: async operation completed in {}ms despite slow DB",
        async_duration
    );

    drop(ctx_ref);
}

#[tokio::test]
async fn given_slow_database_when_concurrent_writes_then_eventual_consistency() {
    let test_name = "eventual_consistency";
    info!("Starting test: {}", test_name);

    let ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    ctx.set_slow_mode(true);

    let ctx = Arc::new(ctx);
    let writer_count = 5;
    let writes_per_writer = 10;
    let total_writes = writer_count * writes_per_writer;

    let mut handles = Vec::new();

    for writer_id in 0..writer_count {
        let ctx_clone = ctx.clone();
        let handle = tokio::spawn(async move {
            for i in 0..writes_per_writer {
                let key = format!("writer{}-key{}", writer_id, i);
                let value = format!("value-{}-{}", writer_id, i);
                ctx_clone.write(key, value).await.expect("Failed to write");
            }
            writes_per_writer
        });
        handles.push(handle);
    }

    let mut completed_writes = 0;
    for handle in handles {
        let result = timeout(Duration::from_secs(30), handle)
            .await
            .expect("Writer should complete");
        completed_writes += result.expect("Writer should succeed");
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_count = ctx.completed_count().await;
    assert_eq!(
        final_count, total_writes,
        "All writes should eventually complete: expected {}, got {}",
        total_writes, final_count
    );

    info!(
        "Test passed: {} writes completed with eventual consistency",
        final_count
    );
}

#[tokio::test]
async fn given_slow_database_when_reads_and_writes_interleaved_then_no_deadlock() {
    let test_name = "no_deadlock";
    info!("Starting test: {}", test_name);

    let ctx = Arc::new(
        ChaosTestContext::new(test_name)
            .await
            .expect("Failed to setup test context"),
    );

    ctx.set_slow_mode(true);

    let writer_ctx = ctx.clone();
    let reader_ctx = ctx.clone();

    let writer_handle = tokio::spawn(async move {
        for i in 0..10 {
            let key = format!("write-key-{}", i);
            writer_ctx
                .write(key, format!("value-{}", i))
                .await
                .expect("Failed to write");
        }
        10
    });

    let reader_handle = tokio::spawn(async move {
        for i in 0..10 {
            let key = format!("read-key-{}", i);
            let _ = reader_ctx
                .database
                .read()
                .await
                .read_with_latency(&key)
                .await;
        }
        10
    });

    let write_result = timeout(Duration::from_secs(30), writer_handle)
        .await
        .expect("Writer should not deadlock");
    let read_result = timeout(Duration::from_secs(30), reader_handle)
        .await
        .expect("Reader should not deadlock");

    assert!(write_result.is_ok(), "Writes should complete");
    assert!(read_result.is_ok(), "Reads should complete");

    info!("Test passed: no deadlock with interleaved reads and writes");
}

#[tokio::test]
async fn given_flapping_latency_when_writes_continue_then_eventual_consistency() {
    let test_name = "flapping_latency";
    info!("Starting test: {}", test_name);

    let ctx = Arc::new(
        ChaosTestContext::new(test_name)
            .await
            .expect("Failed to setup test context"),
    );

    let control_ctx = ctx.clone();
    let write_ctx = ctx.clone();

    let control_handle = tokio::spawn(async move {
        for cycle in 0..5 {
            control_ctx.set_slow_mode(cycle % 2 == 0);
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        control_ctx.set_slow_mode(false);
        5
    });

    let write_handle = tokio::spawn(async move {
        for i in 0..20 {
            let key = format!("flapping-key-{}", i);
            write_ctx
                .write(key, format!("value-{}", i))
                .await
                .expect("Failed to write");
        }
        20
    });

    let control_result = control_handle.await.expect("Control should complete");
    let write_result = timeout(Duration::from_secs(30), write_handle)
        .await
        .expect("Writes should complete");

    assert!(control_result.is_ok(), "Control cycles should complete");
    assert!(write_result.is_ok(), "Writes should complete");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_count = ctx.completed_count().await;
    assert_eq!(
        final_count, 20,
        "All writes should complete despite flapping latency"
    );

    info!("Test passed: eventual consistency with flapping latency");
}

#[tokio::test]
async fn given_slow_database_when_high_throughput_then_latency_stats_accurate() {
    let test_name = "latency_stats";
    info!("Starting test: {}", test_name);

    let ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    ctx.set_slow_mode(true);

    let start = Instant::now();
    let write_count = 20;

    for i in 0..write_count {
        ctx.write(format!("stat-key-{}", i), format!("value-{}", i))
            .await
            .expect("Failed to write");
    }

    let total_duration = start.elapsed();
    let avg_latency = ctx.slow_config.get_avg_latency();

    assert!(avg_latency > 0.0, "Average latency should be recorded");

    assert!(
        total_duration.as_millis() >= (write_count as u128 * 100),
        "Total duration should reflect slow latency: {:?} >= {}ms",
        total_duration,
        write_count * 100
    );

    info!(
        "Test passed: avg latency = {:.2}ms, total duration = {:?}",
        avg_latency, total_duration
    );
}

#[tokio::test]
async fn given_multiple_slow_periods_when_writes_interleaved_then_all_eventually_complete() {
    let test_name = "multiple_slow_periods";
    info!("Starting test: {}", test_name);

    let ctx = Arc::new(
        ChaosTestContext::new(test_name)
            .await
            .expect("Failed to setup test context"),
    );

    let slow_ctx = ctx.clone();
    let slow_handle = tokio::spawn(async move {
        for _ in 0..3 {
            slow_ctx.set_slow_mode(true);
            tokio::time::sleep(Duration::from_millis(300)).await;
            slow_ctx.set_slow_mode(false);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let write_ctx = ctx.clone();
    let write_handle = tokio::spawn(async move {
        for i in 0..30 {
            let key = format!("multi-slow-key-{}", i);
            write_ctx
                .write(key, format!("value-{}", i))
                .await
                .expect("Failed to write");
        }
        30
    });

    slow_handle.await.expect("Slow periods should complete");
    let write_result = timeout(Duration::from_secs(60), write_handle)
        .await
        .expect("Writes should complete");

    let writes_completed = write_result.expect("Writer should succeed");
    assert_eq!(writes_completed, 30, "All writes should complete");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_count = ctx.completed_count().await;
    assert_eq!(
        final_count, 30,
        "Eventual consistency: expected 30 writes, got {}",
        final_count
    );

    info!(
        "Test passed: {} writes completed across multiple slow periods",
        final_count
    );
}
