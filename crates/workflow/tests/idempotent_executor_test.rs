//! Tests for IdempotentExecutor with concurrent execution.
//!
//! These tests validate that:
//! - Same key + same input = same result (determinism)
//! - Concurrent execution → only one executes, others get cached result
//! - Execute-once guarantee under load

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock executor that tracks execution count
#[derive(Clone)]
struct MockExecutor {
    execution_count: Arc<RwLock<HashMap<String, usize>>>,
    results: Arc<RwLock<HashMap<String, String>>>,
}

impl MockExecutor {
    fn new() -> Self {
        Self {
            execution_count: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn execute(&self, key: &str, input: &str) -> Result<String, String> {
        // Increment execution count
        let mut counts = self.execution_count.write().await;
        *counts.entry(key.to_string()).or_insert(0) += 1;

        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Store and return result
        let result = format!("processed:{}", input);
        let mut results = self.results.write().await;
        results.insert(key.to_string(), result.clone());
        Ok(result)
    }

    async fn execution_count(&self, key: &str) -> usize {
        let counts = self.execution_count.read().await;
        *counts.get(key).unwrap_or(&0)
    }

    async fn get_result(&self, key: &str) -> Option<String> {
        let results = self.results.read().await;
        results.get(key).cloned()
    }
}

/// Simple idempotent executor wrapper
struct IdempotentExecutor {
    executor: MockExecutor,
    cache: Arc<RwLock<HashMap<String, String>>>,
    in_progress: Arc<RwLock<HashMap<String, ()>>>,
}

impl IdempotentExecutor {
    fn new(executor: MockExecutor) -> Self {
        Self {
            executor,
            cache: Arc::new(RwLock::new(HashMap::new())),
            in_progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn execute(&self, key: &str, input: &str) -> Result<String, String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                return Ok(cached.clone());
            }
        }

        // Mark as in-progress
        {
            let mut in_progress = self.in_progress.write().await;
            if in_progress.contains_key(key) {
                // Wait for other execution to complete
                drop(in_progress);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // Check cache again
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(key) {
                    return Ok(cached.clone());
                }
                return Err("Execution in progress but not cached".to_string());
            }
            in_progress.insert(key.to_string(), ());
        }

        // Execute
        let result = self.executor.execute(key, input).await;

        // Cache result
        if let Ok(ref res) = result {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), res.clone());
        }

        // Remove from in-progress
        let mut in_progress = self.in_progress.write().await;
        in_progress.remove(key);

        result
    }
}

#[tokio::test]
async fn test_same_key_same_input_deterministic() {
    // Given: IdempotentExecutor with same key and input
    let executor = MockExecutor::new();
    let idem_executor = IdempotentExecutor::new(executor);
    let key = "test-key-1";
    let input = "test-input";

    // When: Execute twice sequentially
    let result1 = idem_executor.execute(key, input).await;
    let result2 = idem_executor.execute(key, input).await;

    // Then: Results should be identical
    assert_eq!(result1, result2, "Results should be deterministic");
    assert_eq!(
        result1,
        Ok("processed:test-input".to_string()),
        "Result should match expected"
    );

    // And: Only one execution should have occurred
    let executor = &idem_executor.executor;
    let count = executor.execution_count(key).await;
    assert_eq!(count, 1, "Should only execute once due to caching");
}

#[tokio::test]
async fn test_concurrent_execution_only_one_executes() {
    // Given: IdempotentExecutor
    let executor = MockExecutor::new();
    let idem_executor = IdempotentExecutor::new(executor);
    let key = "concurrent-key";
    let input = "concurrent-input";

    // When: Execute concurrently from multiple tasks
    let mut handles = Vec::new();
    for _ in 0..10 {
        let executor_clone = idem_executor.clone();
        let key_owned = key.to_string();
        let input_owned = input.to_string();

        let handle =
            tokio::spawn(async move { executor_clone.execute(&key_owned, &input_owned).await });
        handles.push(handle);
    }

    // Then: All results should be identical
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Task should complete");
        results.push(result.ok());
    }

    let first_result = &results[0];
    for result in &results[1..] {
        assert_eq!(
            result, first_result,
            "All concurrent executions should return same result"
        );
    }

    // And: Only ONE execution should have occurred (not 10)
    let count = idem_executor.executor.execution_count(key).await;
    assert_eq!(
        count, 1,
        "Only one execution should occur despite 10 concurrent calls"
    );
}

#[tokio::test]
async fn test_different_keys_execute_independently() {
    // Given: IdempotentExecutor
    let executor = MockExecutor::new();
    let idem_executor = IdempotentExecutor::new(executor);

    // When: Execute with different keys
    let result1 = idem_executor.execute("key-1", "input-1").await;
    let result2 = idem_executor.execute("key-2", "input-2").await;
    let result3 = idem_executor.execute("key-3", "input-3").await;

    // Then: All should succeed
    assert!(result1.is_ok(), "First execution should succeed");
    assert!(result2.is_ok(), "Second execution should succeed");
    assert!(result3.is_ok(), "Third execution should succeed");

    // And: Results should be different
    assert_ne!(
        result1, result2,
        "Different keys should produce different results"
    );
    assert_ne!(
        result2, result3,
        "Different keys should produce different results"
    );

    // And: Each key should execute once
    assert_eq!(idem_executor.executor.execution_count("key-1").await, 1);
    assert_eq!(idem_executor.executor.execution_count("key-2").await, 1);
    assert_eq!(idem_executor.executor.execution_count("key-3").await, 1);
}

#[tokio::test]
async fn test_execute_once_under_load() {
    // Given: IdempotentExecutor
    let executor = MockExecutor::new();
    let idem_executor = IdempotentExecutor::new(executor);
    let key = "load-key";
    let input = "load-input";

    // When: Execute 50 times concurrently
    let mut handles = Vec::new();
    for _ in 0..50 {
        let executor_clone = idem_executor.clone();
        let key_owned = key.to_string();
        let input_owned = input.to_string();

        handles.push(tokio::spawn(async move {
            executor_clone.execute(&key_owned, &input_owned).await
        }));
    }

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // Then: All results should be identical
    let first = &results[0];
    for result in &results[1..] {
        assert_eq!(result, first, "All results should match");
    }

    // And: Despite 50 concurrent calls, only 1 execution
    let count = idem_executor.executor.execution_count(key).await;
    assert_eq!(count, 1, "Should only execute once under heavy load");
}

#[tokio::test]
async fn test_cache_persists_across_calls() {
    // Given: IdempotentExecutor
    let executor = MockExecutor::new();
    let idem_executor = IdempotentExecutor::new(executor);
    let key = "persist-key";
    let input = "persist-input";

    // When: Execute first time
    let result1 = idem_executor.execute(key, input).await;
    assert!(result1.is_ok(), "First execution should succeed");

    // Check execution count
    let count1 = idem_executor.executor.execution_count(key).await;
    assert_eq!(count1, 1, "Should execute once");

    // When: Execute 5 more times
    for _ in 0..5 {
        let result = idem_executor.execute(key, input).await;
        assert!(
            result.is_ok(),
            "Subsequent executions should return cached result"
        );
    }

    // Then: Execution count should still be 1 (cached)
    let count2 = idem_executor.executor.execution_count(key).await;
    assert_eq!(count2, 1, "Should not re-execute, should use cache");
}

#[tokio::test]
async fn test_error_handling_doesnt_cache() {
    // Given: Executor that fails
    #[derive(Clone)]
    struct FailingExecutor;

    impl FailingExecutor {
        async fn execute(&self, _key: &str, _input: &str) -> Result<String, String> {
            Err("Execution failed".to_string())
        }
    }

    struct FailingIdempotentExecutor {
        cache: Arc<RwLock<HashMap<String, String>>>,
    }

    impl FailingIdempotentExecutor {
        fn new() -> Self {
            Self {
                cache: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        async fn execute(&self, key: &str, input: &str) -> Result<String, String> {
            // Check cache
            {
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(key) {
                    return Ok(cached.clone());
                }
            }

            // Execute (always fails)
            let result = FailingExecutor.execute(key, input).await;

            // Only cache on success
            if let Ok(ref res) = result {
                let mut cache = self.cache.write().await;
                cache.insert(key.to_string(), res.clone());
            }

            result
        }
    }

    let executor = FailingIdempotentExecutor::new();

    // When: Execute twice with failing executor
    let result1 = executor.execute("fail-key", "fail-input").await;
    let result2 = executor.execute("fail-key", "fail-input").await;

    // Then: Both should fail
    assert!(result1.is_err(), "First execution should fail");
    assert!(result2.is_err(), "Second execution should also fail");

    // And: Nothing should be cached
    let cache = executor.cache.read().await;
    assert!(cache.is_empty(), "Errors should not be cached");
}

// Helper: Clone for IdempotentExecutor
impl Clone for IdempotentExecutor {
    fn clone(&self) -> Self {
        Self {
            executor: self.executor.clone(),
            cache: Arc::clone(&self.cache),
            in_progress: Arc::clone(&self.in_progress),
        }
    }
}
