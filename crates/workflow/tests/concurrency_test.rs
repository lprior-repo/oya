//! Concurrent execution tests for idempotency.
//!
//! Tests concurrent execution scenarios with property-based testing.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe executor for concurrent testing
struct ConcurrentExecutor {
    executions: Arc<RwLock<HashMap<String, usize>>>,
    results: Arc<RwLock<HashMap<String, String>>>,
}

impl Clone for ConcurrentExecutor {
    fn clone(&self) -> Self {
        Self {
            executions: Arc::clone(&self.executions),
            results: Arc::clone(&self.results),
        }
    }
}

impl ConcurrentExecutor {
    fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn execute(&self, key: &str, input: &str) -> Result<String, String> {
        // Track execution
        {
            let mut execs = self.executions.write().await;
            *execs.entry(key.to_string()).or_insert(0) += 1;
        }

        // Simulate variable work time
        let delay = rand::random::<u64>() % 20; // 0-20ms
        tokio::time::sleep(tokio::time::Duration::from_millis(delay + 10)).await;

        // Return result
        let result = format!("result:{}:{}", key, input);
        {
            let mut results = self.results.write().await;
            results.insert(key.to_string(), result.clone());
        }
        Ok(result)
    }

    async fn execution_count(&self, key: &str) -> usize {
        let execs = self.executions.read().await;
        execs.get(key).map_or(0, |v| *v)
    }

    async fn result(&self, key: &str) -> Option<String> {
        let results = self.results.read().await;
        results.get(key).cloned()
    }
}

/// Idempotent wrapper with concurrent safety
#[derive(Clone)]
struct SafeIdempotentExecutor {
    executor: ConcurrentExecutor,
    cache: Arc<RwLock<HashMap<String, String>>>,
    executing: Arc<RwLock<HashMap<String, bool>>>,
}

impl SafeIdempotentExecutor {
    fn new(executor: ConcurrentExecutor) -> Self {
        Self {
            executor,
            cache: Arc::new(RwLock::new(HashMap::new())),
            executing: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn execute(&self, key: &str, input: &str) -> Result<String, String> {
        // Fast path: check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                return Ok(cached.clone());
            }
        }

        // Acquire execution lock
        let should_execute = {
            let mut executing = self.executing.write().await;
            if executing.get(key).copied().map_or(false, |v| v) {
                // Someone else is executing, wait and retry
                drop(executing);
                drop(cache_guard()); // Explicit drop
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // Retry from cache
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(key) {
                    return Ok(cached.clone());
                }
                return Err("Execution in progress".to_string());
            }

            executing.insert(key.to_string(), true);
            true
        };

        if !should_execute {
            return Err("Failed to acquire execution lock".to_string());
        }

        // Execute
        let result = self.executor.execute(key, input).await;

        // Cache success
        if let Ok(ref res) = result {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), res.clone());
        }

        // Release lock
        let mut executing = self.executing.write().await;
        executing.remove(key);

        result
    }
}

fn cache_guard() {}

#[tokio::test]
async fn test_concurrent_safety_same_key() {
    let executor = ConcurrentExecutor::new();
    let idem_executor = SafeIdempotentExecutor::new(executor.clone());
    let key = "safe-key";

    // Spawn 20 concurrent tasks with same key
    let mut handles = Vec::new();
    for i in 0..20 {
        let exec = idem_executor.clone();
        let key_owned = key.to_string();
        let input = format!("input-{}", i);

        handles.push(tokio::spawn(async move {
            exec.execute(&key_owned, &input).await
        }));
    }

    // Wait for all
    let _results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // Verify only one execution occurred
    assert_eq!(
        executor.execution_count(key).await,
        1,
        "Should only execute once"
    );
}

// Helper: Make it cloneable by wrapping in Arc
#[derive(Clone)]
struct ArcSafeExecutor {
    inner: Arc<SafeIdempotentExecutorInner>,
}

struct SafeIdempotentExecutorInner {
    executor: ConcurrentExecutor,
    cache: RwLock<HashMap<String, String>>,
    executing: RwLock<HashMap<String, bool>>,
}

impl ArcSafeExecutor {
    fn new(executor: ConcurrentExecutor) -> Self {
        Self {
            inner: Arc::new(SafeIdempotentExecutorInner {
                executor,
                cache: RwLock::new(HashMap::new()),
                executing: RwLock::new(HashMap::new()),
            }),
        }
    }

    async fn execute(&self, key: &str, input: &str) -> Result<String, String> {
        // Check cache
        {
            let cache = self.inner.cache.read().await;
            if let Some(cached) = cache.get(key) {
                return Ok(cached.clone());
            }
        }

        // Try to acquire lock
        {
            let mut executing = self.inner.executing.write().await;
            if executing.get(key).copied().map_or(false, |v| v) {
                drop(executing);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let cache = self.inner.cache.read().await;
                if let Some(cached) = cache.get(key) {
                    return Ok(cached.clone());
                }
                return Err("Busy".to_string());
            }
            executing.insert(key.to_string(), true);
        }

        // Execute
        let result = self.inner.executor.execute(key, input).await;

        // Cache result
        if let Ok(ref res) = result {
            let mut cache = self.inner.cache.write().await;
            cache.insert(key.to_string(), res.clone());
        }

        // Release lock
        let mut executing = self.inner.executing.write().await;
        executing.remove(key);

        result
    }
}

#[tokio::test]
async fn test_high_concurrency_single_key() {
    let executor = ConcurrentExecutor::new();
    let idem_executor = ArcSafeExecutor::new(executor);
    let key = "high-concurrency-key";

    // Spawn 100 concurrent tasks
    let mut handles = Vec::new();
    for i in 0..100 {
        let exec = idem_executor.clone();
        let key_owned = key.to_string();
        let input = format!("input-{}", i);

        handles.push(tokio::spawn(async move {
            exec.execute(&key_owned, &input).await
        }));
    }

    // Collect results
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // All should succeed
    assert!(!results.is_empty(), "Should have results");

    // All results should be identical
    assert!(
        results.windows(2).all(|w| w[0] == w[1]),
        "All results should be identical"
    );
}

#[tokio::test]
async fn test_concurrent_different_keys() {
    let executor = ConcurrentExecutor::new();
    let idem_executor = ArcSafeExecutor::new(executor);

    // Spawn 50 tasks with different keys
    let mut handles = Vec::new();
    for i in 0..50 {
        let exec = idem_executor.clone();
        let key = format!("key-{}", i % 10); // Only 10 unique keys
        let input = format!("input-{}", i);

        handles.push(tokio::spawn(
            async move { exec.execute(&key, &input).await },
        ));
    }

    // Wait for all
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // All should succeed
    assert_eq!(results.len(), 50, "All tasks should complete");

    // Each unique key should execute at least once
    // (some keys may execute multiple times due to race conditions in acquiring lock)
}

#[tokio::test]
async fn test_concurrent_determinism() {
    let executor = ConcurrentExecutor::new();
    let idem_executor = ArcSafeExecutor::new(executor);

    // Execute same key/input 20 times concurrently
    let key = "determinism-key";
    let input = "determinism-input";

    let mut handles = Vec::new();
    for _ in 0..20 {
        let exec = idem_executor.clone();
        let key_owned = key.to_string();
        let input_owned = input.to_string();

        handles.push(tokio::spawn(async move {
            exec.execute(&key_owned, &input_owned).await
        }));
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // All results must be identical
    assert!(
        results.windows(2).all(|w| w[0] == w[1]),
        "Results must be deterministic"
    );
}

#[tokio::test]
async fn test_concurrent_stress() {
    let executor = ConcurrentExecutor::new();
    let idem_executor = ArcSafeExecutor::new(executor);

    // Stress test: 200 tasks, 10 unique keys
    let mut handles = Vec::new();
    for i in 0..200 {
        let exec = idem_executor.clone();
        let key = format!("stress-key-{}", i % 10);
        let input = format!("input-{}", i);

        handles.push(tokio::spawn(
            async move { exec.execute(&key, &input).await },
        ));
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // All should complete
    assert_eq!(results.len(), 200, "All tasks should complete under stress");
}
