//! RED QUEEN ATTACK: Checkpoint Restoration Test Suite
//!
//! This module implements malicious attacks on checkpoint restoration methods
//! to find every possible way to break the system.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

use crate::persistence::{CheckpointRecord, OrchestratorStore, StoreConfig};
use crate::replay::checkpoint::{CheckpointConfig, CheckpointManager};

/// Attack harness for checkpoint restoration testing
pub struct CheckpointAttackHarness {
    manager: CheckpointManager,
}

impl CheckpointAttackHarness {
    /// Create a new attack harness with in-memory database
    pub async fn new() -> Option<Self> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        Some(Self { manager })
    }

    /// Create a harness with uninitialized database (no schema)
    pub async fn new_uninitialized() -> Option<Self> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        // IMPORTANT: Do NOT initialize schema to test uninitialized state
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        Some(Self { manager })
    }

    /// Create a checkpoint with malformed data
    pub async fn create_malformed_checkpoint(
        &mut self,
        data: &str,
    ) -> Result<CheckpointRecord, Box<dyn std::error::Error + Send + Sync>> {
        self.manager
            .create_checkpoint(data, None)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

// ===== ATTACK 1: HAPPY PATH VERIFICATION =====

#[tokio::test]
async fn attack_happy_path_basic_restoration() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let mut manager = harness.manager;

    // Create valid checkpoint
    let valid_data = r#"{"active_workflows": ["wf-1"], "sequence": 42}"#;
    let _checkpoint = manager
        .create_checkpoint(valid_data, None)
        .await
        .map_err(|e| format!("Failed to create checkpoint: {e}"))?;

    // Test restoration - should succeed
    let restored: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    assert!(restored.is_ok(), "Happy path restoration should succeed");

    let restored_value = restored.map_err(|e| format!("Restore failed: {e}"))?;
    assert_eq!(
        restored_value
            .get("active_workflows")
            .ok_or("missing active_workflows")?
            .as_array()
            .ok_or("not an array")?
            .len(),
        1
    );
    assert_eq!(
        restored_value.get("sequence").ok_or("missing sequence")?.as_u64().ok_or("not a u64")?,
        42
    );
    Ok(())
}

#[tokio::test]
async fn attack_happy_path_with_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let mut manager = harness.manager;

    let scheduler_data = r#"{"active_workflows": ["wf-1"]}"#;
    let snapshots_data = r#"{"wf-1": {"beads": ["a", "b", "c"], "status": "running"}}"#;

    let _checkpoint = manager
        .create_checkpoint(scheduler_data, Some(snapshots_data))
        .await
        .map_err(|e| format!("Failed to create checkpoint: {e}"))?;

    // Test scheduler restoration
    let scheduler: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    assert!(scheduler.is_ok(), "Scheduler restoration should succeed");

    // Test snapshot restoration
    let snapshots: Result<Option<serde_json::Value>, _> =
        manager.restore_workflow_snapshots().await;
    assert!(snapshots.is_ok(), "Snapshot restoration should succeed");

    let snapshots_opt = snapshots.map_err(|e| format!("Snapshot restore failed: {e}"))?;
    assert!(snapshots_opt.is_some(), "Snapshots should be present");
    if let Some(s) = snapshots_opt {
        assert!(
            s.get("wf-1").is_some(),
            "wf-1 should be in snapshots"
        );
    }
    Ok(())
}

// ===== ATTACK 2: INPUT BOUNDARY ATTACKS =====

#[tokio::test]
async fn attack_empty_checkpoint_id() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let manager = harness.manager;

    // Try to restore with empty checkpoint ID
    let result: Result<serde_json::Value, _> = manager.restore_scheduler_state_by_id("").await;
    assert!(result.is_err(), "Empty checkpoint ID should fail");

    // Verify error is appropriate
    if let Err(error) = result {
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("not found") || error_msg.contains("checkpoint"),
            "Error should indicate checkpoint not found, got: {}",
            error_msg
        );
    }
    Ok(())
}

#[tokio::test]
async fn attack_malformed_checkpoint_id() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let manager = harness.manager;

    // Try various malformed checkpoint IDs
    let malformed_ids = [
        "../../../etc/passwd",
        "checkpoint/../../../etc/passwd",
        "'; DROP TABLE checkpoint; --",
        "\x00\x01\x02invalid",
        "a".repeat(10000).as_str(), // Very long ID
        "checkpoint\u{0000}null",   // Null byte injection
    ];

    for id in malformed_ids {
        let result: Result<serde_json::Value, _> = manager.restore_scheduler_state_by_id(id).await;
        assert!(
            result.is_err(),
            "Malformed checkpoint ID '{}' should fail",
            id
        );
    }
    Ok(())
}

// ===== ATTACK 3: STATE ATTACKS =====

#[tokio::test]
async fn attack_uninitialized_database() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new_uninitialized()
        .await
        .ok_or("Failed to create uninitialized harness")?;
    let manager = harness.manager;

    // Try restoration without initializing schema
    let result: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    assert!(
        result.is_err(),
        "Restoration should fail without initialized schema"
    );

    // Check if error is actionable
    if let Err(error) = result {
        let error_msg = error.to_string();
        println!("Uninitialized database error: {}", error_msg);
    }

    Ok(())
}

#[tokio::test]
async fn attack_corrupted_json_in_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let mut manager = harness.manager;

    // Create checkpoint with corrupted JSON
    let corrupted_data = r#"{"active_workflows": ["wf-1", "invalid json"#;

    // First, we need to directly create a checkpoint record with corrupted data
    // by accessing the store directly since create_checkpoint doesn't allow invalid JSON
    let checkpoint_id = "corrupted-test";
    let checkpoint = CheckpointRecord::new(checkpoint_id, corrupted_data, 1);

    // Save directly to store to bypass validation
    let _saved = manager
        .store
        .save_checkpoint(&checkpoint)
        .await
        .map_err(|e| format!("Failed to save corrupted checkpoint: {e}"))?;

    // Try to restore - should fail gracefully
    let result: Result<serde_json::Value, _> =
        manager.restore_scheduler_state_by_id(checkpoint_id).await;
    assert!(
        result.is_err(),
        "Restoration should fail with corrupted JSON"
    );

    // Verify error is a serialization error
    if let Err(error) = result {
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("serialization")
                || error_msg.contains("deserialize")
                || error_msg.contains("json"),
            "Error should indicate JSON deserialization failed, got: {}",
            error_msg
        );
    }
    Ok(())
}

#[tokio::test]
async fn attack_concurrent_restoration() -> Result<(), Box<dyn std::error::Error>> {
    let harness = Arc::new(
        CheckpointAttackHarness::new()
            .await
            .ok_or("Failed to create harness")?,
    );
    let manager = Arc::new(harness.manager.clone());

    // Create a checkpoint first
    let valid_data = r#"{"active_workflows": ["wf-1"], "sequence": 42}"#;
    {
        let mut mutable_manager =
            CheckpointManager::new(manager.store.clone(), CheckpointConfig::default());
        let _checkpoint = mutable_manager
            .create_checkpoint(valid_data, None)
            .await
            .map_err(|e| format!("Failed to create checkpoint: {e}"))?;
    }

    // Launch concurrent restoration attempts
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for i in 0..10 {
        let manager_clone = manager.clone();
        let barrier_clone = barrier.clone();

        let handle = tokio::spawn(async move {
            barrier_clone.wait().await; // Synchronize start

            let result: Result<serde_json::Value, _> =
                manager_clone.restore_scheduler_state().await;
            (i, result.is_ok())
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        let (i, success) = handle.await.map_err(|e| format!("Task failed: {e}"))?;
        if success {
            success_count += 1;
            println!("Concurrent restoration {} succeeded", i);
        } else {
            println!("Concurrent restoration {} failed", i);
        }
    }

    // All concurrent restorations should either all succeed or all fail
    // If some succeed and some fail, there's a race condition
    assert!(
        success_count == 0 || success_count == 10,
        "Inconsistent concurrent restoration: {} succeeded, {} failed",
        success_count,
        10 - success_count
    );
    Ok(())
}

// ===== ATTACK 4: OUTPUT CONTRACT ATTACKS =====

#[tokio::test]
async fn attack_type_safety_violations() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let mut manager = harness.manager;

    // Create checkpoint with data that would violate type expectations
    let type_confusing_data = r#"{"active_workflows": 123, "sequence": "not-a-number"}"#;

    let _checkpoint = manager
        .create_checkpoint(type_confusing_data, None)
        .await
        .map_err(|e| format!("Failed to create checkpoint: {e}"))?;

    // Try to restore as different types
    let as_string: Result<String, _> = manager.restore_scheduler_state().await;
    assert!(
        as_string.is_err(),
        "Restoring JSON object as String should fail"
    );

    let as_number: Result<u64, _> = manager.restore_scheduler_state().await;
    assert!(
        as_number.is_err(),
        "Restoring JSON object as u64 should fail"
    );

    // Should only work as compatible types
    let as_json: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    assert!(
        as_json.is_ok(),
        "Restoring as serde_json::Value should work"
    );
    Ok(())
}

#[tokio::test]
async fn attack_error_message_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let manager = harness.manager;

    // Check error message consistency between similar methods
    let non_existent_id = "non-existent-checkpoint";

    let scheduler_result: Result<serde_json::Value, _> =
        manager.restore_scheduler_state_by_id(non_existent_id).await;
    let snapshots_result: Result<Option<serde_json::Value>, _> = manager
        .restore_workflow_snapshots_by_id(non_existent_id)
        .await;

    assert!(
        scheduler_result.is_err(),
        "Scheduler restoration should fail"
    );
    assert!(
        snapshots_result.is_err(),
        "Snapshot restoration should fail"
    );

    // Error messages should be consistent
    if let (Err(scheduler_err), Err(snapshots_err)) = (&scheduler_result, &snapshots_result) {
        let scheduler_error = scheduler_err.to_string();
        let snapshots_error = snapshots_err.to_string();

        // Both should mention the checkpoint ID
        assert!(
            scheduler_error.contains(non_existent_id),
            "Scheduler error should mention checkpoint ID: {}",
            scheduler_error
        );
        assert!(
            snapshots_error.contains(non_existent_id),
            "Snapshots error should mention checkpoint ID: {}",
            snapshots_error
        );

        // Error types should be consistent
        assert!(
            std::mem::discriminant(scheduler_err) == std::mem::discriminant(snapshots_err),
            "Error types should be consistent between scheduler and snapshot restoration"
        );
    } else {
        panic!("Both methods should have returned errors");
    }
    Ok(())
}

// ===== ATTACK 5: CROSS-COMMAND CONSISTENCY =====

#[tokio::test]
async fn cross_command_consistency_latest_vs_by_id() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let mut manager = harness.manager;

    // Create multiple checkpoints
    let data1 = r#"{"sequence": 1}"#;
    let data2 = r#"{"sequence": 2}"#;

    let _cp1 = manager
        .create_checkpoint(data1, None)
        .await
        .map_err(|e| format!("Failed to create cp1: {e}"))?;
    let cp2 = manager
        .create_checkpoint(data2, None)
        .await
        .map_err(|e| format!("Failed to create cp2: {e}"))?;

    // Restore latest and by ID - should be consistent
    let latest: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    let by_id: Result<serde_json::Value, _> = manager
        .restore_scheduler_state_by_id(&cp2.checkpoint_id)
        .await;

    assert!(latest.is_ok(), "Latest restoration should succeed");
    assert!(by_id.is_ok(), "By ID restoration should succeed");

    let latest_value = latest.map_err(|e| format!("Latest restore failed: {e}"))?;
    let by_id_value = by_id.map_err(|e| format!("By ID restore failed: {e}"))?;

    // Should return the same data since cp2 is latest
    assert_eq!(
        latest_value, by_id_value,
        "Latest and by-ID restoration should be consistent"
    );
    Ok(())
}

#[tokio::test]
async fn attack_workflow_snapshots_none_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let harness = CheckpointAttackHarness::new()
        .await
        .ok_or("Failed to create harness")?;
    let mut manager = harness.manager;

    // Create checkpoint WITHOUT snapshots
    let data = r#"{"sequence": 1}"#;
    let _checkpoint = manager
        .create_checkpoint(data, None)
        .await
        .map_err(|e| format!("Failed to create checkpoint: {e}"))?;

    // Restore snapshots - should return None consistently
    let latest: Result<Option<serde_json::Value>, _> = manager.restore_workflow_snapshots().await;
    let by_id: Result<Option<serde_json::Value>, _> = manager
        .restore_workflow_snapshots_by_id(&_checkpoint.checkpoint_id)
        .await;

    assert!(
        latest.is_ok(),
        "Latest snapshots restoration should succeed"
    );
    assert!(by_id.is_ok(), "By-ID snapshots restoration should succeed");

    let latest_value = latest.map_err(|e| format!("Latest snapshots restore failed: {e}"))?;
    let by_id_value = by_id.map_err(|e| format!("By-ID snapshots restore failed: {e}"))?;

    // Both should return None
    assert!(latest_value.is_none(), "Latest snapshots should be None");
    assert!(by_id_value.is_none(), "By-ID snapshots should be None");
    assert_eq!(
        latest_value, by_id_value,
        "None values should be consistent"
    );
    Ok(())
}

#[cfg(test)]
mod attack_reports {
    use super::*;

    /// Generate attack report
    #[tokio::test]
    async fn generate_attack_report() {
        println!("=== RED QUEEN ATTACK REPORT: Checkpoint Restoration ===\n");

        // Run a subset of attacks and report findings
        println!("Running critical attacks...\n");

        // Attack 1: Database not initialized
        match CheckpointAttackHarness::new_uninitialized().await {
            Some(harness) => {
                let result: Result<serde_json::Value, _> =
                    harness.manager.restore_scheduler_state().await;
                match result {
                    Ok(_) => println!(
                        "🔴 CRITICAL: Restoration succeeded without schema initialization!"
                    ),
                    Err(e) => println!("✅ GOOD: Restoration failed without schema: {}", e),
                }
            }
            None => println!("⚠️  Could not test uninitialized database"),
        }

        // Attack 2: Empty checkpoint ID
        match CheckpointAttackHarness::new().await {
            Some(harness) => {
                let result: Result<serde_json::Value, _> =
                    harness.manager.restore_scheduler_state_by_id("").await;
                match result {
                    Ok(_) => println!("🔴 CRITICAL: Empty checkpoint ID succeeded!"),
                    Err(e) => println!("✅ GOOD: Empty checkpoint ID rejected: {}", e),
                }
            }
            None => println!("⚠️  Could not test empty checkpoint ID"),
        }

        println!("\n=== ATTACK COMPLETE ===");
    }
}
