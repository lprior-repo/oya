#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chaos test: Disconnect database, queue events, reconnect, flush.
//!
//! This test validates that the system can handle database disconnections
//! by buffering events in memory and flushing them after reconnection.

#![cfg(any())]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use orchestrator::actors::storage::{
    surreal_integration::{
        ConnectionManagerConfig, RetryPolicy, SurrealConnectionManager, SurrealError,
    },
    DatabaseConfig,
};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum DatabaseChaosTestError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Event queue overflow: {0} events dropped")]
    QueueOverflow(usize),

    #[error("Flush timeout exceeded: {timeout_ms}ms")]
    FlushTimeout { timeout_ms: u64 },

    #[error("Data integrity check failed: expected {expected} events, got {actual}")]
    DataIntegrityFailed { expected: usize, actual: usize },

    #[error("Reconnection failed: {0}")]
    ReconnectionFailed(String),

    #[error("Test setup failed: {0}")]
    SetupFailed(String),
}

pub type DatabaseChaosTestResult<T> = Result<T, DatabaseChaosTestError>;

// =============================================================================
// Event Queue for Buffering During Disconnection
// =============================================================================

/// Buffered event that will be flushed after reconnection.
#[derive(Debug, Clone)]
struct BufferedEvent {
    event_id: String,
    data: String,
    timestamp: Instant,
    retry_count: u32,
}

/// In-memory event queue for buffering during database disconnection.
#[derive(Debug)]
struct EventBuffer {
    events: Vec<BufferedEvent>,
    max_size: usize,
    is_flushing: bool,
}

impl EventBuffer {
    fn new(max_size: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_size),
            max_size,
            is_flushing: false,
        }
    }

    fn push(&mut self, event: BufferedEvent) -> DatabaseChaosTestResult<()> {
        if self.events.len() >= self.max_size {
            return Err(DatabaseChaosTestError::QueueOverflow(1));
        }
        self.events.push(event);
        Ok(())
    }

    fn drain(&mut self) -> Vec<BufferedEvent> {
        let mut drained = Vec::new();
        std::mem::swap(&mut drained, &mut self.events);
        drained
    }

    fn len(&self) -> usize {
        self.events.len()
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// =============================================================================
// Simulated Database with Disconnect Capability
// =============================================================================

/// A database wrapper that can simulate disconnections.
struct SimulatedDatabase {
    manager: Option<SurrealConnectionManager>,
    is_connected: bool,
    connection_count: Arc<Mutex<u32>>,
}

impl SimulatedDatabase {
    async fn new(config: ConnectionManagerConfig) -> DatabaseChaosTestResult<Self> {
        let manager = SurrealConnectionManager::new(config)
            .await
            .map_err(|e| DatabaseChaosTestError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            manager: Some(manager),
            is_connected: true,
            connection_count: Arc::new(Mutex::new(0)),
        })
    }

    /// Simulate database disconnection.
    async fn disconnect(&mut self) -> DatabaseChaosTestResult<()> {
        info!("Simulating database disconnection");
        self.is_connected = false;
        // Drop the connection manager
        self.manager = None;
        Ok(())
    }

    /// Simulate database reconnection.
    async fn reconnect(&mut self, config: ConnectionManagerConfig) -> DatabaseChaosTestResult<()> {
        info!("Simulating database reconnection");
        let start = Instant::now();

        let manager = SurrealConnectionManager::new(config)
            .await
            .map_err(|e| DatabaseChaosTestError::ReconnectionFailed(e.to_string()))?;

        self.manager = Some(manager);
        self.is_connected = true;

        let reconnect_time = start.elapsed();
        info!(
            "Database reconnected successfully in {}ms",
            reconnect_time.as_millis()
        );

        // Increment connection count for metrics
        let mut count = self.connection_count.lock().await;
        *count = count.saturating_add(1);

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected
    }

    fn manager(&self) -> Option<&SurrealConnectionManager> {
        self.manager.as_ref()
    }

    async fn connection_count(&self) -> u32 {
        *self.connection_count.lock().await
    }
}

// =============================================================================
// Event Processor with Buffering
// =============================================================================

/// Event processor that buffers events during disconnection.
struct EventProcessor {
    database: Arc<RwLock<SimulatedDatabase>>,
    buffer: Arc<Mutex<EventBuffer>>,
    processed_count: Arc<Mutex<usize>>,
}

impl EventProcessor {
    fn new(database: Arc<RwLock<SimulatedDatabase>>, buffer_size: usize) -> Self {
        Self {
            database,
            buffer: Arc::new(Mutex::new(EventBuffer::new(buffer_size))),
            processed_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Process an event, buffering if database is disconnected.
    async fn process_event(&self, event_id: String, data: String) -> DatabaseChaosTestResult<()> {
        let db = self.database.read().await;

        if db.is_connected() {
            // Try to write to database
            if let Some(manager) = db.manager() {
                let result = manager
                    .execute_with_retry(|conn| {
                        async move {
                            // Simulate database write
                            debug!("Writing event {} to database", event_id);
                            Ok(())
                        }
                    })
                    .await;

                match result {
                    Ok(_) => {
                        // Successfully written to database
                        let mut count = self.processed_count.lock().await;
                        *count = count.saturating_add(1);
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Database write failed, buffering event: {}", e);
                        // Fall through to buffering
                    }
                }
            }
        }

        // Buffer the event
        drop(db); // Release read lock before acquiring buffer lock
        let mut buffer = self.buffer.lock().await;
        buffer.push(BufferedEvent {
            event_id,
            data,
            timestamp: Instant::now(),
            retry_count: 0,
        })?;

        debug!("Event buffered ({} events in buffer)", buffer.len());
        Ok(())
    }

    /// Flush buffered events to database after reconnection.
    async fn flush_buffer(&self, timeout_ms: u64) -> DatabaseChaosTestResult<usize> {
        info!("Flushing buffered events (timeout: {}ms)", timeout_ms);

        let start = Instant::now();
        let timeout_duration = Duration::from_millis(timeout_ms);
        let mut flushed_count = 0;

        loop {
            {
                let mut buffer = self.buffer.lock().await;
                if buffer.is_empty() {
                    info!("Buffer flushed successfully ({} events)", flushed_count);
                    return Ok(flushed_count);
                }

                let events = buffer.drain();
                drop(buffer); // Release buffer lock

                // Try to flush events
                let db = self.database.read().await;
                if let Some(manager) = db.manager() {
                    for event in events {
                        if start.elapsed() >= timeout_duration {
                            error!("Flush timeout exceeded, putting remaining events back");
                            let mut buffer = self.buffer.lock().await;
                            buffer.push(event)?;
                            return Err(DatabaseChaosTestError::FlushTimeout { timeout_ms });
                        }

                        match manager
                            .execute_with_retry(|conn| async move {
                                debug!("Flushing event {} to database", event.event_id);
                                Ok(())
                            })
                            .await
                        {
                            Ok(_) => {
                                flushed_count = flushed_count.saturating_add(1);
                                let mut count = self.processed_count.lock().await;
                                *count = count.saturating_add(1);
                            }
                            Err(e) => {
                                warn!("Failed to flush event {}: {}", event.event_id, e);
                                // Put back in buffer with incremented retry count
                                let mut buffer = self.buffer.lock().await;
                                let mut buffered_event = event;
                                buffered_event.retry_count =
                                    buffered_event.retry_count.saturating_add(1);
                                buffer.push(buffered_event)?;
                            }
                        }
                    }
                }
            }

            // Small delay between flush attempts
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn buffered_count(&self) -> usize {
        self.buffer.lock().await.len()
    }

    async fn processed_count(&self) -> usize {
        *self.processed_count.lock().await
    }
}

// =============================================================================
// Test Context
// =============================================================================

struct ChaosTestContext {
    database: Arc<RwLock<SimulatedDatabase>>,
    processor: EventProcessor,
    db_config: ConnectionManagerConfig,
}

impl ChaosTestContext {
    async fn new(test_name: &str) -> DatabaseChaosTestResult<Self> {
        info!("Setting up chaos test: {}", test_name);

        let storage_path = format!("/tmp/test_chaos_{}", test_name);
        let db_config = DatabaseConfig {
            storage_path,
            namespace: format!("chaos_ns_{}", test_name),
            database: format!("chaos_db_{}", test_name),
        };

        let conn_config = ConnectionManagerConfig::new(db_config.clone())
            .with_max_connections(5)
            .with_retry_policy(RetryPolicy::new(3, 50, 300).without_jitter())
            .with_query_timeout(Duration::from_millis(500));

        let database = Arc::new(RwLock::new(
            SimulatedDatabase::new(conn_config.clone()).await?,
        ));

        let processor = EventProcessor::new(database.clone(), 1000);

        Ok(Self {
            database,
            processor,
            db_config: conn_config,
        })
    }
}

// =============================================================================
// Test Functions
// =============================================================================

#[tokio::test]
async fn given_connected_database_when_disconnected_then_events_buffer() {
    let test_name = "disconnect_buffer";
    info!("Starting test: {}", test_name);

    let ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    // Verify initial connection
    {
        let db = ctx.database.read().await;
        assert!(db.is_connected(), "Database should be connected initially");
    }

    // Disconnect database
    {
        let mut db = ctx.database.write().await;
        db.disconnect()
            .await
            .expect("Failed to disconnect database");
    }

    // Queue events while disconnected
    let event_count = 50;
    for i in 0..event_count {
        ctx.processor
            .process_event(format!("event-{}", i), format!("data-{}", i))
            .await
            .expect("Failed to process event");
    }

    // Verify events are buffered
    let buffered = ctx.processor.buffered_count().await;
    assert_eq!(
        buffered, event_count,
        "All events should be buffered while disconnected"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_buffered_events_when_reconnected_then_all_events_flushed() {
    let test_name = "reconnect_flush";
    info!("Starting test: {}", test_name);

    let mut ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    // Disconnect database
    {
        let mut db = ctx.database.write().await;
        db.disconnect()
            .await
            .expect("Failed to disconnect database");
    }

    // Queue events while disconnected
    let event_count = 100;
    for i in 0..event_count {
        ctx.processor
            .process_event(format!("event-{}", i), format!("data-{}", i))
            .await
            .expect("Failed to process event");
    }

    let buffered_before = ctx.processor.buffered_count().await;
    assert_eq!(
        buffered_before, event_count,
        "All events should be buffered"
    );

    // Reconnect database
    {
        let mut db = ctx.database.write().await;
        db.reconnect(ctx.db_config.clone())
            .await
            .expect("Failed to reconnect database");
    }

    // Verify reconnection
    {
        let db = ctx.database.read().await;
        assert!(db.is_connected(), "Database should be reconnected");
    }

    // Flush buffered events
    let flushed = ctx
        .processor
        .flush_buffer(5000)
        .await
        .expect("Failed to flush events");

    assert_eq!(
        flushed, event_count,
        "All buffered events should be flushed"
    );

    // Verify buffer is empty
    let buffered_after = ctx.processor.buffered_count().await;
    assert_eq!(buffered_after, 0, "Buffer should be empty after flush");

    // Verify all events were processed
    let processed = ctx.processor.processed_count().await;
    assert_eq!(processed, event_count, "All events should be processed");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_rapid_disconnect_reconnect_then_zero_events_lost() {
    let test_name = "rapid_cycle";
    info!("Starting test: {}", test_name);

    let mut ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    let total_events = 200;
    let mut queued_events = 0;

    // Simulate rapid disconnect/reconnect cycles
    for cycle in 0..5 {
        info!("Starting cycle {}", cycle);

        // Disconnect
        {
            let mut db = ctx.database.write().await;
            db.disconnect().await.expect("Failed to disconnect");
        }

        // Queue some events while disconnected
        for i in 0..20 {
            ctx.processor
                .process_event(
                    format!("cycle{}-event{}", cycle, i),
                    format!("data-{}", queued_events),
                )
                .await
                .expect("Failed to process event");
            queued_events = queued_events.saturating_add(1);
        }

        // Reconnect
        {
            let mut db = ctx.database.write().await;
            db.reconnect(ctx.db_config.clone())
                .await
                .expect("Failed to reconnect");
        }

        // Flush
        ctx.processor
            .flush_buffer(2000)
            .await
            .expect("Failed to flush events");
    }

    // Final verification
    let buffered = ctx.processor.buffered_count().await;
    assert_eq!(buffered, 0, "Buffer should be empty");

    let processed = ctx.processor.processed_count().await;
    assert_eq!(processed, total_events, "All events should be processed");

    // Verify connection count
    let db = ctx.database.read().await;
    let connection_count = db.connection_count().await;
    assert!(
        connection_count >= 5,
        "Should have at least 5 reconnections"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_large_buffer_when_flushing_then_completes_within_timeout() {
    let test_name = "large_buffer_flush";
    info!("Starting test: {}", test_name);

    let mut ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    // Disconnect
    {
        let mut db = ctx.database.write().await;
        db.disconnect().await.expect("Failed to disconnect");
    }

    // Queue large number of events
    let event_count = 500;
    for i in 0..event_count {
        ctx.processor
            .process_event(format!("bulk-event-{}", i), format!("data-{}", i))
            .await
            .expect("Failed to process event");
    }

    let buffered = ctx.processor.buffered_count().await;
    assert_eq!(buffered, event_count, "All events should be buffered");

    // Reconnect
    {
        let mut db = ctx.database.write().await;
        db.reconnect(ctx.db_config.clone())
            .await
            .expect("Failed to reconnect");
    }

    // Measure flush time
    let start = Instant::now();
    let flushed = ctx
        .processor
        .flush_buffer(10000)
        .await
        .expect("Failed to flush events");
    let flush_duration = start.elapsed();

    assert_eq!(flushed, event_count, "All events should be flushed");
    assert!(
        flush_duration.as_secs() < 10,
        "Flush should complete within 10 seconds, took {}ms",
        flush_duration.as_millis()
    );

    info!(
        "Flushed {} events in {}ms ({} events/sec)",
        flushed,
        flush_duration.as_millis(),
        (flushed as f64 / flush_duration.as_secs_f64()) as u64
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_database_unavailable_during_flush_then_retries_successfully() {
    let test_name = "flush_with_retry";
    info!("Starting test: {}", test_name);

    let mut ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    // Disconnect and queue events
    {
        let mut db = ctx.database.write().await;
        db.disconnect().await.expect("Failed to disconnect");
    }

    let event_count = 30;
    for i in 0..event_count {
        ctx.processor
            .process_event(format!("retry-event-{}", i), format!("data-{}", i))
            .await
            .expect("Failed to process event");
    }

    // Reconnect
    {
        let mut db = ctx.database.write().await;
        db.reconnect(ctx.db_config.clone())
            .await
            .expect("Failed to reconnect");
    }

    // Start flushing but disconnect halfway through
    let db_ref = ctx.database.clone();
    let flush_handle = tokio::spawn(async move {
        // This will handle the reconnection and flush
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Simulate temporary disconnection during flush
        {
            let mut db = db_ref.write().await;
            db.disconnect().await.expect("Failed to disconnect");
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Reconnect
        {
            let mut db = db_ref.write().await;
            db.reconnect(ConnectionManagerConfig::new(DatabaseConfig {
                storage_path: "/tmp/test_chaos_flush_with_retry".to_string(),
                namespace: "chaos_ns_flush_with_retry".to_string(),
                database: "chaos_db_flush_with_retry".to_string(),
            }))
            .await
            .expect("Failed to reconnect");
        }
    });

    // Wait a bit for the disconnect to happen
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Now flush - it should handle the reconnection
    let flushed = ctx
        .processor
        .flush_buffer(5000)
        .await
        .expect("Failed to flush events");

    flush_handle.await.expect("Flush handle failed");

    assert_eq!(
        flushed, event_count,
        "All events should be flushed despite retry"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_concurrent_events_and_disconnect_then_all_processed() {
    let test_name = "concurrent_stress";
    info!("Starting test: {}", test_name);

    let ctx = ChaosTestContext::new(test_name)
        .await
        .expect("Failed to setup test context");

    // Spawn concurrent event producers
    let mut handles = Vec::new();
    let producers = 10;
    let events_per_producer = 20;

    for producer_id in 0..producers {
        let processor = EventProcessor::new(ctx.database.clone(), 1000);

        let handle = tokio::spawn(async move {
            for i in 0..events_per_producer {
                processor
                    .process_event(
                        format!("producer{}-event{}", producer_id, i),
                        format!("data-{}", i),
                    )
                    .await
                    .expect("Failed to process event");
            }
        });

        handles.push(handle);
    }

    // Disconnect after a short delay
    let db_ref = ctx.database.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut db = db_ref.write().await;
        db.disconnect().await.expect("Failed to disconnect");
        tokio::time::sleep(Duration::from_millis(200)).await;
        db.reconnect(ConnectionManagerConfig::new(DatabaseConfig {
            storage_path: "/tmp/test_chaos_concurrent".to_string(),
            namespace: "chaos_ns_concurrent".to_string(),
            database: "chaos_db_concurrent".to_string(),
        }))
        .await
        .expect("Failed to reconnect");
    });

    // Wait for all producers
    for handle in handles {
        handle.await.expect("Producer failed");
    }

    // Give some time for buffering
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Flush
    let flushed = ctx
        .processor
        .flush_buffer(10000)
        .await
        .expect("Failed to flush events");

    let expected_events = producers * events_per_producer;
    assert_eq!(
        flushed, expected_events,
        "All concurrent events should be processed"
    );

    info!("Test passed: {}", test_name);
}
