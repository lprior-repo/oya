//! Unit tests for StateManagerActor using tokio::test.
//!
//! These tests verify all StateManagerActor message handlers with zero unwraps/panics.
//!
//! Test Categories:
//! - Lifecycle: Spawn and shutdown behavior
//! - Commands: Fire-and-forget messages (SaveState, DeleteState, ClearAll)
//! - Queries: Request-response messages (LoadState, StateExists, GetStateVersion, ListKeys)
//! - Resilience: Actor remains responsive after errors

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::actors::ActorError;
use orchestrator::actors::storage::{DatabaseConfig, StateManagerActorDef, StateManagerMessage};
use ractor::{Actor, ActorRef, RpcReplyPort};
use std::time::Duration;

// ═══════════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Spawn a StateManagerActor for testing with a unique name.
async fn spawn_state_manager() -> Result<ActorRef<StateManagerMessage>, Box<dyn std::error::Error>>
{
    let name = format!("state-manager-test-{}", uuid::Uuid::new_v4());
    let config = DatabaseConfig::default();

    let (actor, _handle) = Actor::spawn(Some(name), StateManagerActorDef, config).await?;

    // Give actor a moment to start
    tokio::time::sleep(Duration::from_millis(10)).await;

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
    // GIVEN: Default database config
    let config = DatabaseConfig::default();
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
    // GIVEN: Custom database config
    let config = DatabaseConfig {
        storage_path: "/tmp/test_state".to_string(),
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
async fn test_load_state_returns_actor_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying LoadState (unimplemented)
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::LoadState {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return ActorUnavailable error (not panic or crash)
    // The RPC call succeeds, but the result contains the ActorError
    let load_result = result?;
    let actor_error = match load_result {
        Ok(_) => return Err("LoadState should return ActorError".into()),
        Err(e) => e,
    };

    assert!(
        matches!(actor_error, ActorError::ActorUnavailable),
        "LoadState should return ActorUnavailable for unimplemented handler"
    );

    // Verify actor is still running after error
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_state_exists_returns_actor_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying StateExists
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::StateExists {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return ActorUnavailable error
    let exists_result = result?;
    let actor_error = match exists_result {
        Ok(_) => return Err("StateExists should return ActorError".into()),
        Err(e) => e,
    };

    assert!(
        matches!(actor_error, ActorError::ActorUnavailable),
        "StateExists should return ActorUnavailable"
    );

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_get_state_version_returns_actor_unavailable() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying GetStateVersion
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::GetStateVersion {
            key: "test-key".to_string(),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return ActorUnavailable error
    let version_result = result?;
    let actor_error = match version_result {
        Ok(_) => return Err("GetStateVersion should return ActorError".into()),
        Err(e) => e,
    };

    assert!(
        matches!(actor_error, ActorError::ActorUnavailable),
        "GetStateVersion should return ActorUnavailable"
    );

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_keys_returns_actor_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A running StateManagerActor
    let actor = spawn_state_manager().await?;

    // WHEN: Querying ListKeys
    let result = call_with_timeout(
        &actor,
        |reply| StateManagerMessage::ListKeys {
            prefix: Some("workflow:".to_string()),
            reply,
        },
        1000,
    )
    .await;

    // THEN: Should return ActorUnavailable error
    let keys_result = result?;
    let actor_error = match keys_result {
        Ok(_) => return Err("ListKeys should return ActorError".into()),
        Err(e) => e,
    };

    assert!(
        matches!(actor_error, ActorError::ActorUnavailable),
        "ListKeys should return ActorUnavailable"
    );

    // Verify actor is still running
    verify_actor_running(&actor)?;

    // Cleanup
    actor.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_keys_with_no_prefix_returns_actor_unavailable()
-> Result<(), Box<dyn std::error::Error>> {
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

    // THEN: Should return ActorUnavailable error
    let keys_result = result?;
    assert!(keys_result.is_err(), "ListKeys should return ActorError");

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
