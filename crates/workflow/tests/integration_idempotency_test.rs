//! Idempotency validation test for Event Sourcing.
//!
//! This integration test validates that operations are truly idempotent:
//! - Generate UUID v5 key from bead_id + input
//! - Execute operation 3 times with same key
//! - Verify operation executes only once (remaining 2 return cached result)
//! - Result is identical across all 3 calls
//! - Test with concurrent execution (tokio::spawn)
//! - Verify cache + DB consistency

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use itertools::Itertools;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use oya_workflow::idempotent::{hash_input, idempotency_key_from_bytes, IdempotencyKey};

/// Mock operation result
#[derive(Debug, Clone, PartialEq, Eq)]
struct OperationResult {
    key: IdempotencyKey,
    value: String,
    execution_count: u32,
}

/// In-memory cache for idempotency
#[derive(Clone)]
struct Cache {
    results: Arc<RwLock<HashMap<IdempotencyKey, OperationResult>>>,
}

impl Cache {
    fn new() -> Self {
        Self {
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get(&self, key: &IdempotencyKey) -> Option<OperationResult> {
        let results = self.results.read().await;
        results.get(key).cloned()
    }

    async fn set(&self, key: IdempotencyKey, result: OperationResult) {
        let mut results = self.results.write().await;
        results.insert(key, result);
    }

    async fn contains_key(&self, key: &IdempotencyKey) -> bool {
        let results = self.results.read().await;
        results.contains_key(key)
    }
}

/// Mock database for persistence
#[derive(Clone)]
struct Database {
    records: Arc<RwLock<HashMap<IdempotencyKey, OperationResult>>>,
}

impl Database {
    fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get(&self, key: &IdempotencyKey) -> Option<OperationResult> {
        let records = self.records.read().await;
        records.get(key).cloned()
    }

    async fn insert(&self, key: IdempotencyKey, result: OperationResult) {
        let mut records = self.records.write().await;
        records.insert(key, result);
    }

    async fn get_execution_count(&self, key: &IdempotencyKey) -> u32 {
        let records = self.records.read().await;
        records.get(key).map(|r| r.execution_count).map_or(0, |v| v)
    }
}

/// Idempotent executor with cache + DB persistence
struct IdempotentExecutor {
    cache: Cache,
    db: Database,
    execution_count: Arc<RwLock<HashMap<IdempotencyKey, u32>>>,
}

impl IdempotentExecutor {
    fn new(cache: Cache, db: Database) -> Self {
        Self {
            cache,
            db,
            execution_count: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute operation idempotently
    async fn execute(&self, bead_id: &str, input: &str) -> Result<OperationResult, String> {
        // Generate idempotency key
        let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));

        // Check cache first
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        // Check database
        if let Some(persisted) = self.db.get(&key).await {
            // Populate cache from DB
            self.cache.set(key, persisted.clone()).await;
            return Ok(persisted);
        }

        // Execute operation (only if not cached/persisted)
        let result = self.execute_operation(&key, input).await?;

        // Persist to database
        self.db.insert(key, result.clone()).await;

        // Update cache
        self.cache.set(key, result.clone()).await;

        Ok(result)
    }

    /// Execute the actual operation (increment counter)
    async fn execute_operation(
        &self,
        key: &IdempotencyKey,
        input: &str,
    ) -> Result<OperationResult, String> {
        // Increment execution count
        let mut counts = self.execution_count.write().await;
        let count = counts.entry(*key).or_insert(0);
        *count += 1;
        let execution_count = *count;
        drop(counts);

        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(OperationResult {
            key: *key,
            value: format!("processed:{}", input),
            execution_count,
        })
    }

    /// Get actual execution count from internal tracker
    async fn get_actual_execution_count(&self, key: &IdempotencyKey) -> u32 {
        let counts = self.execution_count.read().await;
        counts.get(key).copied().map_or(0, |v| v)
    }
}

// ============================================================================
// IDEMPOTENCY VALIDATION TESTS
// ============================================================================

#[tokio::test]
async fn test_idempotency_key_from_bead_id_and_input() {
    // Given: bead_id and input
    let bead_id = "src-2xc4";
    let input = "test-operation";

    // When: Generate idempotency key
    let key = idempotency_key_from_bytes(bead_id, input.as_bytes());

    // Then: Key should be valid UUID v5
    assert_eq!(key.get_version(), Some(uuid::Version::Sha1));
    assert_eq!(key.get_variant(), uuid::Variant::RFC4122);

    // And: Same bead_id + input should produce same key
    let key2 = idempotency_key_from_bytes(bead_id, input.as_bytes());
    assert_eq!(key, key2);
}

#[tokio::test]
async fn test_execute_three_times_same_key_only_executes_once() {
    // Given: Idempotent executor
    let cache = Cache::new();
    let db = Database::new();
    let executor = IdempotentExecutor::new(cache, db);

    let bead_id = "src-2xc4";
    let input = "test-operation";

    // When: Execute operation 3 times with same key
    let result1 = executor.execute(bead_id, input).await;
    let result2 = executor.execute(bead_id, input).await;
    let result3 = executor.execute(bead_id, input).await;

    // Then: All executions should succeed
    assert!(result1.is_ok(), "First execution should succeed");
    assert!(result2.is_ok(), "Second execution should succeed");
    assert!(result3.is_ok(), "Third execution should succeed");

    // And: Results should be identical
    let r1 = result1.ok();
    let r2 = result2.ok();
    let r3 = result3.ok();
    assert_eq!(r1, r2, "First and second results should be identical");
    assert_eq!(r2, r3, "Second and third results should be identical");

    // And: Operation should only execute once
    let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));
    let actual_count = executor.get_actual_execution_count(&key).await;
    assert_eq!(actual_count, 1, "Operation should only execute once");

    // And: DB should show execution count of 1
    let db_count = executor.db.get_execution_count(&key).await;
    assert_eq!(db_count, 1, "DB should record 1 execution");
}

#[tokio::test]
async fn test_concurrent_execution_with_same_key_only_executes_once() {
    // Given: Idempotent executor
    let cache = Cache::new();
    let db = Database::new();
    let executor = Arc::new(IdempotentExecutor::new(cache, db));

    let bead_id = "src-2xc4-concurrent";
    let input = "concurrent-test";

    // When: Execute concurrently 10 times with same key
    let mut handles = Vec::new();
    for _ in 0..10 {
        let executor_clone = Arc::clone(&executor);
        let bead_id_owned = bead_id.to_string();
        let input_owned = input.to_string();

        handles.push(tokio::spawn(async move {
            executor_clone.execute(&bead_id_owned, &input_owned).await
        }));
    }

    // Then: All executions should succeed and results should be identical
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(results.len(), 10, "All tasks should complete successfully");

    // And: All results should be identical
    assert!(
        results.windows(2).all(|w| w[0] == w[1]),
        "All concurrent executions should return identical result"
    );

    // And: Operation should only execute once (not 10 times)
    let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));
    let actual_count = executor.get_actual_execution_count(&key).await;
    assert_eq!(
        actual_count, 1,
        "Only one execution should occur despite 10 concurrent calls"
    );

    // And: Cache should contain the result
    assert!(
        executor.cache.contains_key(&key).await,
        "Cache should contain the result"
    );

    // And: DB should contain the result
    let db_result = executor.db.get(&key).await;
    assert!(
        db_result.is_some(),
        "DB should contain the persisted result"
    );
}

#[tokio::test]
async fn test_cache_db_consistency() {
    // Given: Idempotent executor
    let cache = Cache::new();
    let db = Database::new();
    let executor = IdempotentExecutor::new(cache.clone(), db.clone());

    let bead_id = "src-2xc4-consistency";
    let input = "consistency-test";

    // When: Execute operation
    let result = executor.execute(bead_id, input).await;
    assert!(result.is_ok(), "Execution should succeed");

    let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));

    // Then: Cache and DB should both contain the result
    let cached = cache.get(&key).await;
    let persisted = db.get(&key).await;

    assert!(
        cached.is_some(),
        "Cache should contain the result after execution"
    );
    assert!(
        persisted.is_some(),
        "DB should contain the result after execution"
    );

    // And: Cache and DB results should be identical
    assert_eq!(
        cached, persisted,
        "Cache and DB should contain identical results"
    );

    // And: Both should match the returned result
    let returned = result.ok();
    assert_eq!(
        cached, returned,
        "Cached result should match returned result"
    );
    assert_eq!(
        persisted, returned,
        "Persisted result should match returned result"
    );
}

#[tokio::test]
async fn test_different_inputs_produce_different_keys() {
    // Given: Same bead_id, different inputs
    let bead_id = "src-2xc4-different";
    let input1 = "input-one";
    let input2 = "input-two";

    // When: Generate keys
    let key1 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input1.as_bytes()));
    let key2 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input2.as_bytes()));

    // Then: Keys should be different
    assert_ne!(key1, key2, "Different inputs should produce different keys");
}

#[tokio::test]
async fn test_different_bead_ids_produce_different_keys() {
    // Given: Different bead_ids, same input
    let bead_id1 = "src-2xc4-a";
    let bead_id2 = "src-2xc4-b";
    let input = "shared-input";

    // When: Generate keys
    let key1 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id1, input.as_bytes()));
    let key2 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id2, input.as_bytes()));

    // Then: Keys should be different
    assert_ne!(
        key1, key2,
        "Different bead_ids should produce different keys"
    );
}

#[tokio::test]
async fn test_sequential_operations_maintain_independence() {
    // Given: Idempotent executor
    let cache = Cache::new();
    let db = Database::new();
    let executor = IdempotentExecutor::new(cache, db);

    let bead_id = "src-2xc4-sequential";

    // When: Execute 3 different operations
    let result1 = executor.execute(bead_id, "operation-1").await;
    let result2 = executor.execute(bead_id, "operation-2").await;
    let result3 = executor.execute(bead_id, "operation-3").await;

    // Then: All should succeed
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());

    // And: Each should execute exactly once
    let key1 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, b"operation-1"));
    let key2 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, b"operation-2"));
    let key3 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, b"operation-3"));

    assert_eq!(executor.get_actual_execution_count(&key1).await, 1);
    assert_eq!(executor.get_actual_execution_count(&key2).await, 1);
    assert_eq!(executor.get_actual_execution_count(&key3).await, 1);

    // And: Results should be different
    let r1 = result1.ok();
    let r2 = result2.ok();
    let r3 = result3.ok();
    assert_ne!(
        r1, r2,
        "Different operations should produce different results"
    );
    assert_ne!(
        r2, r3,
        "Different operations should produce different results"
    );
}

#[tokio::test]
async fn test_repeated_execution_returns_cached_result() {
    // Given: Idempotent executor with executed operation
    let cache = Cache::new();
    let db = Database::new();
    let executor = IdempotentExecutor::new(cache.clone(), db.clone());

    let bead_id = "src-2xc4-cached";
    let input = "cached-operation";

    // Execute first time
    let result1 = executor.execute(bead_id, input).await;
    assert!(result1.is_ok());

    let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));
    let count_after_first = executor.get_actual_execution_count(&key).await;
    assert_eq!(count_after_first, 1, "Should execute once on first call");

    // When: Execute 5 more times
    for i in 2..=6 {
        let result = executor.execute(bead_id, input).await;
        assert!(
            result.is_ok(),
            "Execution {} should succeed (from cache)",
            i
        );
    }

    // Then: Execution count should still be 1 (all subsequent calls cached)
    let count_after_more = executor.get_actual_execution_count(&key).await;
    assert_eq!(
        count_after_more, 1,
        "Should not re-execute, should return cached result"
    );

    // And: All results should be identical
    let result_later = executor.execute(bead_id, input).await;
    assert_eq!(result1, result_later, "Cached result should match original");
}

#[tokio::test]
async fn test_idempotency_key_determinism() {
    // Given: Same bead_id and input
    let bead_id = "src-2xc4-determinism";
    let input = "determinism-test";

    // When: Generate keys multiple times
    let key1 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));
    let key2 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));
    let key3 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));

    // Then: All keys should be identical
    assert_eq!(key1, key2, "Key generation should be deterministic");
    assert_eq!(key2, key3, "Key generation should be deterministic");
    assert_eq!(key1, key3, "Key generation should be deterministic");
}

#[tokio::test]
async fn test_hash_input_consistency() {
    // Given: Input data
    let input = b"consistency-test-data";

    // When: Hash multiple times
    let hash1 = hash_input(input);
    let hash2 = hash_input(input);
    let hash3 = hash_input(input);

    // Then: All hashes should be identical
    assert_eq!(hash1, hash2, "Hash should be consistent");
    assert_eq!(hash2, hash3, "Hash should be consistent");

    // And: Hash should be fixed size (32 bytes for SHA-256)
    assert_eq!(hash1.len(), 32, "SHA-256 hash should be 32 bytes");
}

#[tokio::test]
async fn test_empty_input_generates_valid_key() {
    // Given: Empty input
    let bead_id = "src-2xc4-empty";
    let input = "";

    // When: Generate key
    let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));

    // Then: Key should be valid UUID v5
    assert_eq!(key.version(), Some(uuid::Version::Sha1));
    assert_eq!(key.variant(), uuid::Variant::RFC4122);

    // And: Key should not be nil
    assert!(!key.is_nil(), "Empty input should still generate valid key");

    // And: Should be deterministic
    let key2 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, input.as_bytes()));
    assert_eq!(key, key2, "Empty input key should be deterministic");
}

#[tokio::test]
async fn test_large_input_generates_valid_key() {
    // Given: Large input (10KB)
    let bead_id = "src-2xc4-large";
    let large_input = vec![42u8; 10_000];

    // When: Generate key
    let key = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, &large_input));

    // Then: Key should be valid UUID v5
    assert_eq!(key.version(), Some(uuid::Version::Sha1));
    assert_eq!(key.variant(), uuid::Variant::RFC4122);

    // And: Should be deterministic
    let key2 = IdempotencyKey::new(idempotency_key_from_bytes(bead_id, &large_input));
    assert_eq!(key, key2, "Large input key should be deterministic");
}
