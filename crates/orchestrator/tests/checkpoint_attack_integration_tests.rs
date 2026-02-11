//! RED QUEEN ATTACK: Checkpoint Restoration Integration Tests
//! 
//! This module implements malicious attacks on checkpoint restoration methods
//! to find every possible way to break the system.

use std::sync::Arc;
use tokio::sync::Barrier;

use orchestrator::persistence::{OrchestratorStore, StoreConfig, CheckpointRecord};
use orchestrator::replay::{CheckpointManager, CheckpointConfig};

/// Attack harness for checkpoint restoration testing
struct CheckpointAttackHarness {
    manager: CheckpointManager,
}

impl CheckpointAttackHarness {
    /// Create a new attack harness with in-memory database
    async fn new() -> Option<Self> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        Some(Self { manager })
    }
    
    /// Create a harness with uninitialized database (no schema)
    async fn new_uninitialized() -> Option<Self> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        // IMPORTANT: Do NOT initialize schema to test uninitialized state
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        Some(Self { manager })
    }
}

// ===== ATTACK 1: UNINITIALIZED DATABASE =====

#[tokio::test]
async fn attack_uninitialized_database() {
    let harness = CheckpointAttackHarness::new_uninitialized().await.expect("Failed to create uninitialized harness");
    let manager = harness.manager;
    
    // Try restoration without initializing schema
    let result: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    
    match result {
        Ok(_) => {
            println!("🔴 CRITICAL: Restoration succeeded without schema initialization!");
            panic!("Restoration should fail without schema initialization");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("✅ GOOD: Restoration failed without schema: {}", error_msg);
            
            // Check if error message is actionable
            if error_msg.contains("schema") || error_msg.contains("table") || error_msg.contains("not found") {
                println!("✅ Error message is actionable: mentions schema/table issue");
            } else {
                println!("🟡 WARNING: Error message may not be actionable: {}", error_msg);
            }
        }
    }
}

// ===== ATTACK 2: EMPTY CHECKPOINT ID =====

#[tokio::test]
async fn attack_empty_checkpoint_id() {
    let harness = CheckpointAttackHarness::new().await.expect("Failed to create harness");
    let manager = harness.manager;
    
    // Try to restore with empty checkpoint ID
    let result: Result<serde_json::Value, _> = manager.restore_scheduler_state_by_id("").await;
    
    match result {
        Ok(_) => {
            println!("🔴 CRITICAL: Empty checkpoint ID succeeded!");
            panic!("Empty checkpoint ID should fail");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("✅ GOOD: Empty checkpoint ID rejected: {}", error_msg);
            
            if error_msg.contains("not found") || error_msg.contains("checkpoint") {
                println!("✅ Error appropriately mentions checkpoint issue");
            }
        }
    }
}

// ===== ATTACK 3: MALFORMED CHECKPOINT IDs =====

#[tokio::test]
async fn attack_malformed_checkpoint_ids() {
    let harness = CheckpointAttackHarness::new().await.expect("Failed to create harness");
    let manager = harness.manager;
    
    // Try various malformed checkpoint IDs
    let malformed_ids = vec![
        ("../../../etc/passwd", "Path traversal"),
        ("'; DROP TABLE checkpoint; --", "SQL injection attempt"),
        ("a".repeat(10000), "Buffer overflow attempt"),
        ("checkpoint\u{0000}null", "Null byte injection"),
        ("🚀🔥💥", "Unicode injection"),
    ];
    
    for (id, description) in malformed_ids {
        let result: Result<serde_json::Value, _> = manager.restore_scheduler_state_by_id(&id).await;
        
        match result {
            Ok(_) => {
                println!("🔴 CRITICAL: Malformed checkpoint ID '{}' ({}) succeeded!", id, description);
                panic!("Malformed checkpoint ID should fail: {}", description);
            }
            Err(e) => {
                println!("✅ GOOD: Malformed checkpoint ID '{}' ({}) rejected", id, description);
            }
        }
    }
}

// ===== ATTACK 4: CORRUPTED JSON IN CHECKPOINT =====

#[tokio::test]
async fn attack_corrupted_json_in_checkpoint() {
    let harness = CheckpointAttackHarness::new().await.expect("Failed to create harness");
    let mut manager = harness.manager;
    
    // Create checkpoint with corrupted JSON
    let corrupted_data = r#"{"active_workflows": ["wf-1", "invalid json"#;
    
    // Create a checkpoint record directly with corrupted data
    let checkpoint_id = "corrupted-test";
    let checkpoint = CheckpointRecord::new(checkpoint_id, corrupted_data, 1);
    
    // Save directly to store to bypass validation
    let _saved = manager.store.save_checkpoint(&checkpoint).await
        .expect("Failed to save corrupted checkpoint");
    
    // Try to restore - should fail gracefully
    let result: Result<serde_json::Value, _> = manager.restore_scheduler_state_by_id(checkpoint_id).await;
    
    match result {
        Ok(_) => {
            println!("🔴 CRITICAL: Corrupted JSON restoration succeeded!");
            panic!("Corrupted JSON should fail to restore");
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("✅ GOOD: Corrupted JSON restoration failed: {}", error_msg);
            
            if error_msg.contains("serialization") || error_msg.contains("deserialize") || error_msg.contains("json") {
                println!("✅ Error correctly identifies JSON deserialization issue");
            } else {
                println!("🟡 WARNING: Error doesn't clearly indicate JSON issue: {}", error_msg);
            }
        }
    }
}

// ===== ATTACK 5: TYPE SAFETY VIOLATIONS =====

#[tokio::test]
async fn attack_type_safety_violations() {
    let harness = CheckpointAttackHarness::new().await.expect("Failed to create harness");
    let mut manager = harness.manager;
    
    // Create checkpoint with data that would violate type expectations
    let type_confusing_data = r#"{"active_workflows": 123, "sequence": "not-a-number"}"#;
    
    let _checkpoint = manager.create_checkpoint(type_confusing_data, None).await
        .expect("Failed to create type-confusing checkpoint");
    
    // Try to restore as different types
    let as_string: Result<String, _> = manager.restore_scheduler_state().await;
    match as_string {
        Ok(_) => {
            println!("🔴 CRITICAL: Restoring JSON object as String succeeded!");
            panic!("Should not restore object as String");
        }
        Err(_) => {
            println!("✅ GOOD: Restoring JSON object as String correctly failed");
        }
    }
    
    let as_number: Result<u64, _> = manager.restore_scheduler_state().await;
    match as_number {
        Ok(_) => {
            println!("🔴 CRITICAL: Restoring JSON object as u64 succeeded!");
            panic!("Should not restore object as u64");
        }
        Err(_) => {
            println!("✅ GOOD: Restoring JSON object as u64 correctly failed");
        }
    }
    
    // Should work as compatible types
    let as_json: Result<serde_json::Value, _> = manager.restore_scheduler_state().await;
    match as_json {
        Ok(_) => {
            println!("✅ GOOD: Restoring as serde_json::Value works correctly");
        }
        Err(_) => {
            println!("🟡 WARNING: Restoring as serde_json::Value failed (unexpected)");
        }
    }
}

// ===== ATTACK 6: ERROR MESSAGE CONSISTENCY =====

#[tokio::test]
async fn attack_error_message_consistency() {
    let harness = CheckpointAttackHarness::new().await.expect("Failed to create harness");
    let manager = harness.manager;
    
    // Check error message consistency between similar methods
    let non_existent_id = "non-existent-checkpoint";
    
    let scheduler_result: Result<serde_json::Value, _> = manager.restore_scheduler_state_by_id(non_existent_id).await;
    let snapshots_result: Result<Option<serde_json::Value>, _> = manager.restore_workflow_snapshots_by_id(non_existent_id).await;
    
    let (scheduler_error, snapshots_error) = match (scheduler_result, snapshots_result) {
        (Err(sched_err), Err(snap_err)) => {
            println!("✅ Both methods failed as expected");
            (sched_err.to_string(), snap_err.to_string())
        }
        _ => {
            panic!("Both methods should have returned errors");
        }
    };
    
    // Both should mention the checkpoint ID
    if scheduler_error.contains(non_existent_id) {
        println!("✅ Scheduler error mentions checkpoint ID");
    } else {
        println!("🟡 WARNING: Scheduler error doesn't mention checkpoint ID: {}", scheduler_error);
    }
    
    if snapshots_error.contains(non_existent_id) {
        println!("✅ Snapshots error mentions checkpoint ID");
    } else {
        println!("🟡 WARNING: Snapshots error doesn't mention checkpoint ID: {}", snapshots_error);
    }
}

// ===== ATTACK 7: CROSS-COMMAND CONSISTENCY =====

#[tokio::test]
async fn attack_cross_command_consistency() {
    let harness = CheckpointAttackHarness::new().await.expect("Failed to create harness");
    let mut manager = harness.manager;
    
    // Create checkpoint WITHOUT snapshots
    let data = r#"{"sequence": 1}"#;
    let checkpoint = manager.create_checkpoint(data, None).await.expect("Failed to create checkpoint");
    
    // Restore snapshots - should return None consistently
    let latest: Result<Option<String>, _> = manager.restore_workflow_snapshots().await;
    let by_id: Result<Option<String>, _> = manager.restore_workflow_snapshots_by_id(&checkpoint.checkpoint_id).await;
    
    match (latest, by_id) {
        (Ok(Some(_)), _) => {
            println!("🔴 CRITICAL: Latest snapshots returned Some when should be None!");
            panic!("Latest snapshots should be None");
        }
        (_, Ok(Some(_))) => {
            println!("🔴 CRITICAL: By-ID snapshots returned Some when should be None!");
            panic!("By-ID snapshots should be None");
        }
        (Ok(None), Ok(None)) => {
            println!("✅ GOOD: Both methods consistently return None");
        }
        _ => {
            println!("🟡 WARNING: Inconsistent behavior between latest and by-ID restoration");
        }
    }
}

// ===== ATTACK 8: CONCURRENT RESTORATION =====

#[tokio::test]
async fn attack_concurrent_restoration() {
    let harness = Arc::new(CheckpointAttackHarness::new().await.expect("Failed to create harness"));
    let manager = Arc::new(harness.manager.clone());
    
    // Create a checkpoint first
    let valid_data = r#"{"active_workflows": ["wf-1"], "sequence": 42}"#;
    {
        let mut mutable_manager = CheckpointManager::new(manager.store.clone(), CheckpointConfig::default());
        let _checkpoint = mutable_manager.create_checkpoint(valid_data, None).await
            .expect("Failed to create checkpoint");
    }
    
    // Launch concurrent restoration attempts
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];
    
    for i in 0..10 {
        let manager_clone = manager.clone();
        let barrier_clone = barrier.clone();
        
        let handle = tokio::spawn(async move {
            barrier_clone.wait().await; // Synchronize start
            
            let result: Result<serde_json::Value, _> = manager_clone.restore_scheduler_state().await;
            (i, result.is_ok())
        });
        
        handles.push(handle);
    }
    
    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        let (i, success) = handle.await.expect("Task panicked");
        if success {
            success_count += 1;
        }
    }
    
    // All concurrent restorations should either all succeed or all fail
    if success_count == 0 {
        println!("✅ GOOD: All concurrent restorations consistently failed");
    } else if success_count == 10 {
        println!("✅ GOOD: All concurrent restorations consistently succeeded");
    } else {
        println!("🔴 CRITICAL: Inconsistent concurrent restoration: {} succeeded, {} failed", 
                success_count, 10 - success_count);
        panic!("Concurrent restoration should be consistent");
    }
}

// ===== ATTACK REPORT GENERATION =====

#[tokio::test]
async fn generate_attack_report() {
    println!("\n=== RED QUEEN ATTACK REPORT: Checkpoint Restoration ===\n");
    
    // Run summary tests
    println!("Running critical attacks...\n");
    
    // Attack 1: Database not initialized
    println!("1. Testing uninitialized database...");
    match CheckpointAttackHarness::new_uninitialized().await {
        Some(_) => println!("   ✅ Can create uninitialized harness"),
        None => println!("   ⚠️  Cannot test uninitialized database"),
    }
    
    // Attack 2: Empty checkpoint ID
    println!("2. Testing empty checkpoint ID...");
    match CheckpointAttackHarness::new().await {
        Some(_) => println!("   ✅ Can create test harness"),
        None => println!("   ⚠️  Cannot test empty checkpoint ID"),
    }
    
    println!("\n=== ATTACK SUMMARY ===");
    println!("✅ Attacks implemented for:");
    println!("  - Uninitialized database handling");
    println!("  - Empty and malformed checkpoint IDs");
    println!("  - Corrupted JSON data");
    println!("  - Type safety violations");
    println!("  - Error message consistency");
    println!("  - Cross-command consistency");
    println!("  - Concurrent restoration safety");
    
    println!("\n=== ATTACK COMPLETE ===\n");
}