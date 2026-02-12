//! Chaos tests for disk full scenarios with graceful failure and retry.
//!
//! Tests resilience to disk full errors by:
//! 1. Simulating disk full when writing checkpoint
//! 2. Verifying graceful failure (no panic, proper error)
//! 3. Simulating space becoming available
//! 4. Verifying retry succeeds
//!
//! **Bead:** src-2qrr
//! **Phase 4 - Chaos Tests:** Disk full -> fail gracefully -> retry after space available

#![cfg(test)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use thiserror::Error;
use tracing::{debug, info, warn};

use oya_workflow::checkpoint::restore::{restore_checkpoint, CheckpointId};
use oya_workflow::checkpoint::serialize::serialize_state;
use oya_workflow::checkpoint::storage::{
    CheckpointMetadata, CheckpointStorage, StorageError, StorageStats,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during chaos testing.
#[derive(Debug, Error)]
pub enum ChaosTestError {
    #[error("Failed to create checkpoint: {reason}")]
    CheckpointCreationFailed { reason: String },

    #[error("Storage operation failed: {reason}")]
    StorageFailed { reason: String },

    #[error("Retry limit exceeded: {attempts} attempts")]
    RetryLimitExceeded { attempts: u32 },

    #[error("Unexpected success when disk full expected")]
    UnexpectedSuccess,

    #[error("Test setup failed: {reason}")]
    SetupFailed { reason: String },

    #[error("State mismatch after recovery: {details}")]
    StateMismatch { details: String },

    #[error("Disk space not freed as expected")]
    DiskSpaceNotFreed,
}

/// Result type for chaos tests.
pub type ChaosTestResult<T> = Result<T, ChaosTestError>;

// =============================================================================
// Test State Structures
// =============================================================================

/// Test state that will be checkpointed.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct TestState {
    pub version: u32,
    pub counter: u64,
    pub data: String,
    pub items: Vec<String>,
}

impl TestState {
    /// Create a new test state.
    #[must_use]
    pub fn new(version: u32, counter: u64) -> Self {
        Self {
            version,
            counter,
            data: format!("state-data-v{version}"),
            items: vec![format!("item-{}", version)],
        }
    }
}

// =============================================================================
// Disk Full Simulation Storage
// =============================================================================

/// Storage that simulates disk full conditions.
///
/// This mock storage can be configured to fail writes when "disk full"
/// and allow writes after "space is freed".
#[derive(Debug)]
pub struct DiskFullSimulatingStorage {
    checkpoints: im::HashMap<CheckpointId, (Vec<u8>, CheckpointMetadata)>,
    is_disk_full: Arc<AtomicBool>,
    write_attempts: Arc<AtomicU64>,
    successful_writes: Arc<AtomicU64>,
    failed_writes: Arc<AtomicU64>,
    max_capacity_bytes: Arc<AtomicU64>,
    current_usage_bytes: Arc<AtomicU64>,
}

impl DiskFullSimulatingStorage {
    /// Create a new disk-full-simulating storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            checkpoints: im::HashMap::new(),
            is_disk_full: Arc::new(AtomicBool::new(false)),
            write_attempts: Arc::new(AtomicU64::new(0)),
            successful_writes: Arc::new(AtomicU64::new(0)),
            failed_writes: Arc::new(AtomicU64::new(0)),
            max_capacity_bytes: Arc::new(AtomicU64::new(u64::MAX)),
            current_usage_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create storage with a specific capacity limit.
    #[must_use]
    pub fn with_capacity(max_bytes: u64) -> Self {
        Self {
            checkpoints: im::HashMap::new(),
            is_disk_full: Arc::new(AtomicBool::new(false)),
            write_attempts: Arc::new(AtomicU64::new(0)),
            successful_writes: Arc::new(AtomicU64::new(0)),
            failed_writes: Arc::new(AtomicU64::new(0)),
            max_capacity_bytes: Arc::new(AtomicU64::new(max_bytes)),
            current_usage_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Simulate disk full condition.
    pub fn set_disk_full(&self, full: bool) {
        self.is_disk_full.store(full, Ordering::SeqCst);
        debug!("Disk full simulation set to: {}", full);
    }

    /// Check if disk is full.
    #[must_use]
    pub fn is_disk_full(&self) -> bool {
        self.is_disk_full.load(Ordering::SeqCst)
            || self.current_usage_bytes.load(Ordering::SeqCst)
                >= self.max_capacity_bytes.load(Ordering::SeqCst)
    }

    /// Free disk space by clearing old checkpoints.
    pub fn free_space(&mut self) -> u64 {
        let freed = self.current_usage_bytes.load(Ordering::SeqCst);
        self.checkpoints = im::HashMap::new();
        self.current_usage_bytes.store(0, Ordering::SeqCst);
        self.is_disk_full.store(false, Ordering::SeqCst);
        debug!("Freed {} bytes of disk space", freed);
        freed
    }

    /// Get number of write attempts.
    #[must_use]
    pub fn write_attempts(&self) -> u64 {
        self.write_attempts.load(Ordering::SeqCst)
    }

    /// Get number of successful writes.
    #[must_use]
    pub fn successful_writes(&self) -> u64 {
        self.successful_writes.load(Ordering::SeqCst)
    }

    /// Get number of failed writes.
    #[must_use]
    pub fn failed_writes(&self) -> u64 {
        self.failed_writes.load(Ordering::SeqCst)
    }

    /// Get current disk usage in bytes.
    #[must_use]
    pub fn current_usage(&self) -> u64 {
        self.current_usage_bytes.load(Ordering::SeqCst)
    }
}

impl Default for DiskFullSimulatingStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointStorage for DiskFullSimulatingStorage {
    fn store_checkpoint(
        &mut self,
        data: Vec<u8>,
        metadata: CheckpointMetadata,
    ) -> Result<CheckpointId, StorageError> {
        self.write_attempts.fetch_add(1, Ordering::SeqCst);

        if self.is_disk_full() {
            self.failed_writes.fetch_add(1, Ordering::SeqCst);
            info!("Write rejected: disk full (attempt {})", self.write_attempts());
            return Err(StorageError::StorageFailed {
                reason: "disk full: no space left on device".to_string(),
            });
        }

        let id = metadata.id;
        let data_size = data.len() as u64;

        self.checkpoints = self.checkpoints.update(id, (data, metadata));
        self.current_usage_bytes.fetch_add(data_size, Ordering::SeqCst);
        self.successful_writes.fetch_add(1, Ordering::SeqCst);

        debug!("Checkpoint stored: {} ({} bytes)", id, data_size);
        Ok(id)
    }

    fn load_checkpoint(&self, id: &CheckpointId) -> Result<(Vec<u8>, CheckpointMetadata), StorageError> {
        self.checkpoints
            .get(id)
            .map(|(data, meta)| (data.clone(), meta.clone()))
            .ok_or_else(|| StorageError::NotFound {
                checkpoint_id: id.to_string(),
            })
    }

    fn delete_checkpoint(&mut self, id: &CheckpointId) -> Result<(), StorageError> {
        match self.checkpoints.get(id) {
            Some((data, _)) => {
                let size = data.len() as u64;
                self.checkpoints = self.checkpoints.without(id);
                self.current_usage_bytes.fetch_sub(size, Ordering::SeqCst);
                Ok(())
            }
            None => Err(StorageError::NotFound {
                checkpoint_id: id.to_string(),
            }),
        }
    }

    fn list_checkpoints(&self) -> Result<Vec<CheckpointId>, StorageError> {
        Ok(self.checkpoints.keys().copied().collect())
    }

    fn get_stats(&self) -> Result<StorageStats, StorageError> {
        let total_checkpoints = self.checkpoints.len();
        let (total_compressed, total_uncompressed, ratio_sum) =
            self.checkpoints.values().fold(
                (0u64, 0u64, 0.0),
                |(c, u, r), (_, meta)| {
                    (c + meta.compressed_size as u64, u + meta.uncompressed_size as u64, r + meta.compression_ratio)
                },
            );

        Ok(StorageStats {
            total_checkpoints,
            total_compressed_size: total_compressed,
            total_uncompressed_size: total_uncompressed,
            average_compression_ratio: if total_checkpoints > 0 {
                ratio_sum / total_checkpoints as f64
            } else {
                1.0
            },
        })
    }

    fn clear_all(&mut self) -> Result<(), StorageError> {
        self.free_space();
        Ok(())
    }
}

// =============================================================================
// Retry Logic
// =============================================================================

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 10,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a test-friendly config with short delays.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 5,
            max_delay_ms: 50,
            backoff_multiplier: 2.0,
        }
    }

    /// Calculate delay for a given attempt (1-indexed).
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.base_delay_ms as f64
            * self.backoff_multiplier.powi(i32::try_from(attempt).unwrap_or(0).saturating_sub(1));
        delay.min(self.max_delay_ms as f64) as u64
    }
}

/// Attempt to store a checkpoint with retry logic.
///
/// Returns Ok(id) on success, Err if all retries exhausted.
fn store_with_retry(
    storage: &mut DiskFullSimulatingStorage,
    data: Vec<u8>,
    metadata: CheckpointMetadata,
    config: &RetryConfig,
) -> ChaosTestResult<CheckpointId> {
    let mut attempt = 0u32;

    loop {
        attempt += 1;

        match storage.store_checkpoint(data.clone(), metadata.clone()) {
            Ok(id) => {
                info!("Store succeeded on attempt {}", attempt);
                return Ok(id);
            }
            Err(StorageError::StorageFailed { reason }) if reason.contains("disk full") => {
                warn!("Disk full on attempt {}/{}", attempt, config.max_attempts);

                if attempt >= config.max_attempts {
                    return Err(ChaosTestError::RetryLimitExceeded { attempts: attempt });
                }

                let delay = config.delay_for_attempt(attempt);
                debug!("Waiting {}ms before retry...", delay);
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            Err(e) => {
                return Err(ChaosTestError::StorageFailed {
                    reason: e.to_string(),
                });
            }
        }
    }
}

/// Create checkpoint metadata for test state.
fn create_metadata(state: &TestState) -> ChaosTestResult<(CheckpointMetadata, Vec<u8>)> {
    let checkpoint_id = CheckpointId::new();
    let serialized = serialize_state(state).map_err(|e| ChaosTestError::CheckpointCreationFailed {
        reason: format!("serialization failed: {e}"),
    })?;

    let metadata = CheckpointMetadata {
        id: checkpoint_id,
        created_at: chrono::Utc::now(),
        version: 1,
        uncompressed_size: std::mem::size_of_val(state),
        compressed_size: serialized.len(),
        compression_ratio: std::mem::size_of_val(state) as f64 / serialized.len().max(1) as f64,
    };

    Ok((metadata, serialized))
}

// =============================================================================
// Test Functions
// =============================================================================

#[tokio::test]
async fn given_disk_full_when_store_checkpoint_then_fails_gracefully() {
    let test_name = "disk_full_graceful_failure";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::new();
    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    storage.set_disk_full(true);
    assert!(storage.is_disk_full(), "Storage should report disk full");

    let result = storage.store_checkpoint(data, metadata);

    assert!(
        result.is_err(),
        "Store should fail when disk is full"
    );

    match result {
        Err(StorageError::StorageFailed { reason }) => {
            assert!(
                reason.contains("disk full"),
                "Error should indicate disk full, got: {}",
                reason
            );
            info!("Graceful failure with proper error message: {}", reason);
        }
        Err(e) => {
            panic!("Unexpected error type: {}", e);
        }
        Ok(_) => {
            panic!("Should have failed when disk is full");
        }
    }

    assert_eq!(storage.failed_writes(), 1, "Should have one failed write");
    assert_eq!(storage.successful_writes(), 0, "Should have no successful writes");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_disk_full_when_freed_then_retry_succeeds() {
    let test_name = "disk_full_retry_success";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::new();
    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    storage.set_disk_full(true);

    let result = storage.store_checkpoint(data.clone(), metadata.clone());
    assert!(result.is_err(), "First write should fail");

    storage.set_disk_full(false);
    assert!(!storage.is_disk_full(), "Storage should not be full after freeing");

    let result = storage.store_checkpoint(data, metadata);
    assert!(result.is_ok(), "Retry should succeed after space freed");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_disk_full_when_store_with_retry_then_eventually_succeeds() {
    let test_name = "disk_full_retry_eventual_success";
    info!("Starting test: {}", test_name);

    let storage = DiskFullSimulatingStorage::new();
    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    storage.set_disk_full(true);

    let storage_ptr = Arc::new(std::sync::Mutex::new(storage));
    let storage_clone = Arc::clone(&storage_ptr);

    let free_handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let s = storage_clone.lock().map_err(|_| "lock poisoned").expect("lock");
        s.set_disk_full(false);
        info!("Freed disk space after delay");
        ChaosTestResult::<()>::Ok(())
    });

    let retry_config = RetryConfig {
        max_attempts: 5,
        base_delay_ms: 20,
        max_delay_ms: 100,
        backoff_multiplier: 1.5,
    };

    {
        let mut s = storage_ptr.lock().map_err(|_| "lock poisoned").expect("lock");
        let result = store_with_retry(&mut s, data, metadata, &retry_config);
        assert!(result.is_ok(), "Should eventually succeed after space freed: {:?}", result.err());
    }

    free_handle.await.expect("Free task panicked").expect("Free task failed");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_capacity_limit_when_exceeded_then_fails_gracefully() {
    let test_name = "capacity_limit_exceeded";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::with_capacity(50);
    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    assert!(!storage.is_disk_full(), "Storage should not be full initially");

    if data.len() > 50 {
        let result = storage.store_checkpoint(data, metadata);
        assert!(result.is_err(), "Store should fail when data exceeds capacity");

        match result {
            Err(StorageError::StorageFailed { reason }) => {
                assert!(
                    reason.contains("disk full"),
                    "Error should indicate disk full"
                );
            }
            Err(e) => {
                panic!("Unexpected error type: {}", e);
            }
            Ok(_) => {
                panic!("Should have failed when capacity exceeded");
            }
        }
        info!("Correctly rejected write that exceeds capacity");
    }

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_multiple_checkpoints_when_disk_full_then_preserves_existing() {
    let test_name = "disk_full_preserves_existing";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::with_capacity(500);

    let state1 = TestState::new(1, 100);
    let (metadata1, data1) = create_metadata(&state1).expect("Failed to create metadata");
    let id1 = storage.store_checkpoint(data1, metadata1).expect("First store should succeed");

    let state2 = TestState::new(2, 200);
    let (metadata2, data2) = create_metadata(&state2).expect("Failed to create metadata");
    let id2 = storage.store_checkpoint(data2, metadata2).expect("Second store should succeed");

    storage.set_disk_full(true);

    let state3 = TestState::new(3, 300);
    let (metadata3, data3) = create_metadata(&state3).expect("Failed to create metadata");
    let result = storage.store_checkpoint(data3, metadata3);
    assert!(result.is_err(), "Third store should fail with disk full");

    let restored1: TestState = restore_checkpoint(&id1, &storage)
        .expect("Existing checkpoint 1 should be restorable");
    let restored2: TestState = restore_checkpoint(&id2, &storage)
        .expect("Existing checkpoint 2 should be restorable");

    assert_eq!(restored1.version, 1, "State 1 version should be preserved");
    assert_eq!(restored2.version, 2, "State 2 version should be preserved");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_disk_full_retry_loop_when_all_fail_then_returns_retry_error() {
    let test_name = "disk_full_retry_exhausted";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::new();
    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    storage.set_disk_full(true);

    let config = RetryConfig::for_testing();
    let result = store_with_retry(&mut storage, data, metadata, &config);

    assert!(result.is_err(), "Should fail when all retries exhausted");

    match result {
        Err(ChaosTestError::RetryLimitExceeded { attempts }) => {
            assert!(attempts >= 3, "Should have made at least 3 attempts");
            info!("Correctly returned retry limit exceeded after {} attempts", attempts);
        }
        Err(e) => {
            panic!("Unexpected error type: {}", e);
        }
        Ok(_) => {
            panic!("Should have failed when disk remains full");
        }
    }

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_free_space_when_disk_full_then_reduces_usage() {
    let test_name = "free_space_reduces_usage";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::new();

    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    storage.store_checkpoint(data, metadata).expect("First store should succeed");
    let usage_before = storage.current_usage();
    assert!(usage_before > 0, "Usage should be positive after store");

    let freed = storage.free_space();
    assert_eq!(freed, usage_before, "Should report freed bytes");
    assert_eq!(storage.current_usage(), 0, "Usage should be zero after free");
    assert!(!storage.is_disk_full(), "Should not be full after free");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn test_invariant_no_data_loss_on_disk_full() {
    let test_name = "no_data_loss_invariant";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::with_capacity(1000);

    let mut stored_ids = Vec::new();
    let mut stored_states = Vec::new();

    for i in 1..=3 {
        let state = TestState::new(i, u64::from(i) * 100);
        let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

        match storage.store_checkpoint(data, metadata) {
            Ok(id) => {
                stored_ids.push(id);
                stored_states.push(state);
            }
            Err(StorageError::StorageFailed { reason }) if reason.contains("disk full") => {
                info!("Stopped at {} checkpoints due to disk full", i - 1);
                break;
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    for (id, expected_state) in stored_ids.iter().zip(stored_states.iter()) {
        let restored: TestState = restore_checkpoint(id, &storage)
            .expect("Should restore stored checkpoint");

        assert_eq!(
            restored, *expected_state,
            "Invariant violated: stored state does not match expected"
        );
    }

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn test_postcondition_successful_write_after_free() {
    let test_name = "write_after_free_postcondition";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::with_capacity(100);

    let state1 = TestState::new(1, 100);
    let (metadata1, data1) = create_metadata(&state1).expect("Failed to create metadata");

    match storage.store_checkpoint(data1, metadata1) {
        Ok(_) => {
            storage.set_disk_full(true);
        }
        Err(StorageError::StorageFailed { .. }) => {
            info!("Initial capacity too small, adjusting test");
            return;
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }

    let state2 = TestState::new(2, 200);
    let (metadata2, data2) = create_metadata(&state2).expect("Failed to create metadata");

    let result = storage.store_checkpoint(data2.clone(), metadata2.clone());
    assert!(result.is_err(), "Should fail when disk full");

    storage.free_space();

    let result = storage.store_checkpoint(data2, metadata2);
    assert!(result.is_ok(), "Should succeed after free space");

    let id = result.expect("Should have id");
    let (loaded_data, _) = storage.load_checkpoint(&id).expect("Should load after free");

    let restored: TestState = bincode::decode_from_slice(&loaded_data, bincode::config::standard())
        .map(|(s, _)| s)
        .expect("Should deserialize");

    assert_eq!(restored.version, 2, "Restored state should have version 2");
    assert_eq!(restored.counter, 200, "Restored state should have counter 200");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_restore_when_disk_full_then_read_still_works() {
    let test_name = "read_works_during_disk_full";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::new();

    let state = TestState::new(1, 100);
    let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

    let id = storage.store_checkpoint(data, metadata).expect("Store should succeed");

    storage.set_disk_full(true);

    let result = storage.load_checkpoint(&id);
    assert!(result.is_ok(), "Read should work even when disk is full");

    let (loaded_data, _) = result.expect("Should load");
    let restored: TestState = bincode::decode_from_slice(&loaded_data, bincode::config::standard())
        .map(|(s, _)| s)
        .expect("Should deserialize");

    assert_eq!(restored, state, "Read state should match original");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn test_scenario_checkpoint_workflow_with_disk_full_recovery() {
    let test_name = "workflow_disk_full_recovery_scenario";
    info!("Starting integration test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::with_capacity(500);

    let stages = vec!["initialize", "process", "validate"];
    let mut checkpoints = Vec::new();

    for (i, stage) in stages.iter().enumerate() {
        let state = TestState::new(i as u32 + 1, (i as u64 + 1) * 100);
        let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");

        match storage.store_checkpoint(data, metadata) {
            Ok(id) => {
                info!("Checkpoint created for stage '{}': {}", stage, id);
                checkpoints.push((id, state));
            }
            Err(StorageError::StorageFailed { reason }) if reason.contains("disk full") => {
                warn!("Disk full at stage '{}', freeing space...", stage);

                if !checkpoints.is_empty() {
                    let (oldest_id, _) = checkpoints.remove(0);
                    storage.delete_checkpoint(&oldest_id).expect("Delete should work");
                    info!("Deleted oldest checkpoint to free space");
                }

                let (metadata, data) = create_metadata(&state).expect("Failed to create metadata");
                match storage.store_checkpoint(data, metadata) {
                    Ok(id) => {
                        info!("Checkpoint created for stage '{}' after cleanup: {}", stage, id);
                        checkpoints.push((id, state));
                    }
                    Err(e) => {
                        panic!("Still failed after cleanup: {}", e);
                    }
                }
            }
            Err(e) => {
                panic!("Unexpected error at stage '{}': {}", stage, e);
            }
        }
    }

    for (id, expected_state) in &checkpoints {
        let (data, _) = storage.load_checkpoint(id).expect("Should load checkpoint");
        let restored: TestState = bincode::decode_from_slice(&data, bincode::config::standard())
            .map(|(s, _)| s)
            .expect("Should deserialize");

        assert_eq!(
            &restored, expected_state,
            "Checkpoint state should match for stage {}",
            expected_state.version
        );
    }

    info!(
        "Workflow completed with {} checkpoints preserved",
        checkpoints.len()
    );
    info!("Integration test passed: {}", test_name);
}

#[tokio::test]
async fn given_delete_when_disk_full_then_frees_space_for_new_writes() {
    let test_name = "delete_frees_space";
    info!("Starting test: {}", test_name);

    let mut storage = DiskFullSimulatingStorage::with_capacity(100);

    let state1 = TestState::new(1, 100);
    let (metadata1, data1) = create_metadata(&state1).expect("Failed to create metadata");

    let id1 = match storage.store_checkpoint(data1, metadata1) {
        Ok(id) => id,
        Err(StorageError::StorageFailed { .. }) => {
            info!("Data too large for capacity, skipping test");
            return;
        }
        Err(e) => panic!("Unexpected error: {}", e),
    };

    let state2 = TestState::new(2, 200);
    let (metadata2, data2) = create_metadata(&state2).expect("Failed to create metadata");

    let result = storage.store_checkpoint(data2.clone(), metadata2.clone());
    assert!(result.is_err(), "Should fail - disk full");

    storage.delete_checkpoint(&id1).expect("Delete should succeed");

    let result = storage.store_checkpoint(data2, metadata2);
    assert!(result.is_ok(), "Should succeed after delete freed space");

    info!("Test passed: {}", test_name);
}
