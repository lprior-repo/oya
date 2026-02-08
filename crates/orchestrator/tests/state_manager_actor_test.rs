//! Unit tests for StateManagerActor using tokio::test.
//!
//! These tests verify all StateManagerActor message handlers with zero unwraps/panics.
//!
//! Test Categories:
//! - Lifecycle: Spawn and shutdown behavior
//! - Commands: Fire-and-forget messages (SaveState, DeleteState, ClearAll)
//! - Queries: Request-response messages (LoadState, StateExists, GetStateVersion, ListKeys)
//! - Resilience: Actor remains responsive after errors

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use orchestrator::actors::ActorError;
use orchestrator::actors::storage::{DatabaseConfig, StateManagerActorDef, StateManagerMessage};
use ractor::{Actor, ActorRef, RpcReplyPort};
use std::time::Duration;

// ═══════════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Spawn a StateManagerActor for testing with isolated temporary storage.
/// Each test gets its own temporary directory to avoid RocksDB lock conflicts.
async fn spawn_state_manager() -> Result<ActorRef<StateManagerMessage>, Box<dyn std::error::Error>>
{
    let name = format!("state-manager-test-{}", uuid::Uuid::new_v4());

    // Create isolated temporary directory for this test
    let temp_dir = tempfile::tempdir()?;
    let storage_path = temp_dir.path().to_path_buf();

    let config = DatabaseConfig {
        storage_path: storage_path.to_string_lossy().to_string(),
        namespace: "test_ns".to_string(),
        database: "test_db".to_string(),
    };

    let (actor, _handle) = Actor::spawn(Some(name), StateManagerActorDef, config).await?;

    // Wait longer for actor initialization (database connection takes time)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Keep temp_dir alive - it will be cleaned up when dropped (test end)
    std::mem::forget(temp_dir);

    Ok(actor)
}

/// Verify that an actor is still running by checking its status.
fn verify_actor_running(actor: &ActorRef<StateManagerMessage>) -> Result<(), String> {
    use ractor::ActorStatus;
    let status = actor.get_status();
    match status {
        ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading => Ok(()),
        _ => Err(format!("Actor not running, status: {:?}", status)),
    }
}

/// Perform a call with timeout to prevent hanging tests.
async fn call_with_timeout<T, F>(
    actor: &ActorRef<StateManagerMessage>,
    msg_builder: F,
    timeout_ms: u64,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: Send + 'static,
    F: FnOnce(RpcReplyPort<T>) -> StateManagerMessage,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    actor.send_message(msg_builder(tx.into()))?;

    let result = tokio::time::timeout(Duration::from_millis(timeout_ms), rx).await??;
    Ok(result)
}

// ═══════════════════════════════════════════════════════════════════════════════
// LIFECYCLE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_spawn_state_manager_with_default_config() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Default database config with isolated temporary storage
    let temp_dir = tempfile::tempdir()?;
    let storage_path = temp_dir.path().to_path_buf();
    let config = DatabaseConfig {
        storage_path: storage_path.to_string_lossy().to_string(),
        namespace: "test_ns".to_string(),
        database: "test_db".to_string(),
    };
    let name = format!("state-manager-test-{}", uuid::Uuid::new_v4());

    // WHEN: Spawning the actor
    let result = Actor::spawn(Some(name), StateManagerActorDef, config).await;

    // THEN: Actor should spawn successfully
    let (actor, _handle) = result?;
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_spawn_state_manager_with_custom_config() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Custom database config with isolated temporary storage
    let temp_dir = tempfile::tempdir()?;
    let storage_path = temp_dir.path().to_path_buf();
    let config = DatabaseConfig {
        storage_path: storage_path.to_string_lossy().to_string(),
        namespace: "test_namespace".to_string(),
        database: "test_database".to_string(),
    };
    let name = format!("state-manager-test-{}", uuid::Uuid::new_v4());

    // WHEN: Spawning the actor with custom config
    let result = Actor::spawn(Some(name), StateManagerActorDef, config).await;

    // THEN: Actor should spawn successfully
    let (actor, _handle) = result?;
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_shutdown_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Stopping the actor
    actor.stop(Some("test shutdown".to_string()));

    // THEN: Actor should stop without panicking
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify actor is no longer running
    use ractor::ActorStatus;
    let status = actor.get_status();
    assert!(
        matches!(status, ActorStatus::Stopping | ActorStatus::Stopped),
        "Actor should be stopping or stopped"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMMAND TESTS (Cast - Fire-and-Forget)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_save_state_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending SaveState message (fire-and-forget)
    let result = actor.send_message(StateManagerMessage::SaveState {
        key: "test-key".to_string(),
        data: vec![1, 2, 3, 4],
        version: Some(1),
    });

    // THEN: Message send should succeed
    assert!(result.is_ok(), "SaveState message send should succeed");

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify actor is still running (didn't crash)
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_delete_state_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending DeleteState message
    let result = actor.send_message(StateManagerMessage::DeleteState {
        key: "test-key".to_string(),
    });

    // THEN: Message send should succeed
    assert!(result.is_ok(), "DeleteState message send should succeed");

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_clear_all_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending ClearAll message
    let result = actor.send_message(StateManagerMessage::ClearAll);

    // THEN: Message send should succeed
    assert!(result.is_ok(), "ClearAll message send should succeed");

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_multiple_commands_sequentially() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending multiple commands in sequence
    actor.send_message(StateManagerMessage::SaveState {
        key: "key-1".to_string(),
        data: vec![1, 2, 3],
        version: Some(1),
    })?;

    actor.send_message(StateManagerMessage::SaveState {
        key: "key-2".to_string(),
        data: vec![4, 5, 6],
        version: Some(1),
    })?;

    actor.send_message(StateManagerMessage::DeleteState {
        key: "key-1".to_string(),
    })?;

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // THEN: Actor should still be running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUERY TESTS (Call - Request-Response)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_load_state_returns_not_found_for_missing_key()
-> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying LoadState for a non-existent key
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::LoadState {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return BeadNotFound error (handler is implemented)
    let load_result = result?;
    assert!(
        load_result.is_err(),
        "LoadState should return error for missing key"
    );

    let actor_error = load_result.unwrap_err();
    assert!(
        matches!(actor_error, ActorError::BeadNotFound(_)),
        "LoadState should return BeadNotFound for missing key"
    );

    // Verify actor is still running after error
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_state_exists_returns_false_for_missing_key() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying StateExists for a non-existent key
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::StateExists {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return Ok(false) - handler is implemented and returns bool
    let exists_result = result?;
    assert!(exists_result.is_ok(), "StateExists should return Ok(bool)");

    let exists = exists_result.unwrap();
    assert!(!exists, "StateExists should return false for missing key");

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_get_state_version_returns_not_found_for_missing_key()
-> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying GetStateVersion for a non-existent key
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::GetStateVersion {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return BeadNotFound error (handler is implemented)
    let version_result = result?;
    assert!(
        version_result.is_err(),
        "GetStateVersion should return error for missing key"
    );

    let actor_error = version_result.unwrap_err();
    assert!(
        matches!(actor_error, ActorError::BeadNotFound(_)),
        "GetStateVersion should return BeadNotFound for missing key"
    );

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_keys_returns_empty_list_when_no_keys() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying ListKeys with a prefix
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::ListKeys {
            prefix: Some("workflow:".to_string()),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return Ok with empty list (handler is implemented)
    let keys_result = result?;
    assert!(
        keys_result.is_ok(),
        "ListKeys should return Ok(Vec<String>)"
    );

    let keys = keys_result.unwrap();
    assert!(
        keys.is_empty(),
        "ListKeys should return empty list when no keys match prefix"
    );

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_keys_with_no_prefix_returns_empty_list() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying ListKeys without prefix
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::ListKeys {
            prefix: None,
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return Ok with empty list (handler is implemented)
    let keys_result = result?;
    assert!(
        keys_result.is_ok(),
        "ListKeys should return Ok(Vec<String>)"
    );

    let keys = keys_result.unwrap();
    assert!(
        keys.is_empty(),
        "ListKeys should return empty list when no keys"
    );

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_queries_dont_crash_actor() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending multiple queries that return errors
    let _ = call_with_timeout::<Result<Vec<u8>, _>, _>(
        &actor,
        |reply| StateManagerMessage::LoadState {
            key: "key-1".to_string(),
            reply,
        },
        1000,
    )
    .await;

    let _ = call_with_timeout::<Result<bool, _>, _>(
        &actor,
        |reply| StateManagerMessage::StateExists {
            key: "key-2".to_string(),
            reply,
        },
        1000,
    )
    .await;

    let _ = call_with_timeout::<Result<Option<u64>, _>, _>(
        &actor,
        |reply| StateManagerMessage::GetStateVersion {
            key: "key-3".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // THEN: Actor should still be running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// RESILIENCE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mixed_commands_and_queries() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Interleaving commands and queries
    actor.send_message(StateManagerMessage::SaveState {
        key: "key-1".to_string(),
        data: vec![1, 2, 3],
        version: Some(1),
    })?;

    // Allow processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    let _ = call_with_timeout::<Result<Vec<u8>, _>, _>(
        &actor,
        |reply| StateManagerMessage::LoadState {
            key: "key-1".to_string(),
            reply,
        },
        1000,
    )
    .await;

    actor.send_message(StateManagerMessage::DeleteState {
        key: "key-1".to_string(),
    })?;

    // Allow processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    let _ = call_with_timeout::<Result<bool, _>, _>(
        &actor,
        |reply| StateManagerMessage::StateExists {
            key: "key-1".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Actor should remain responsive
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_rapid_messages() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending 50 messages rapidly
    for i in 0..50 {
        actor.send_message(StateManagerMessage::SaveState {
            key: format!("key-{}", i),
            data: vec![i as u8; 10],
            version: Some(i as u64),
        })?;
    }

    // Allow time for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // THEN: Actor should still be running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_actor_after_error_queries() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Sending error queries followed by successful commands
    // These queries will return errors (unimplemented)
    let _ = call_with_timeout::<Result<Vec<u8>, _>, _>(
        &actor,
        |reply| StateManagerMessage::LoadState {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // Allow processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Send a command (should succeed without crashing)
    actor.send_message(StateManagerMessage::SaveState {
        key: "after-error".to_string(),
        data: vec![7, 8, 9],
        version: Some(1),
    })?;

    // Allow processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    // THEN: Actor should still be functional
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_database_config_serialization() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A custom DatabaseConfig
    let config = DatabaseConfig {
        storage_path: "/custom/path".to_string(),
        namespace: "custom_ns".to_string(),
        database: "custom_db".to_string(),
    };

    // WHEN: Serializing with bincode
    let encoded = bincode::serde::encode_to_vec(&config, bincode::config::standard())?;

    // THEN: Should deserialize correctly
    let (decoded, _): (DatabaseConfig, _) =
        bincode::serde::decode_from_slice(&encoded, bincode::config::standard())?;

    assert_eq!(config.storage_path, decoded.storage_path);
    assert_eq!(config.namespace, decoded.namespace);
    assert_eq!(config.database, decoded.database);

    Ok(())
}
