//! Storage actors for durable state management.
//!
//! This module provides actors for managing durable state with bincode serialization.
//! Messages support both fire-and-forget commands and query-response patterns.

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::info;

use crate::actors::errors::ActorError;
use crate::actors::supervisor::GenericSupervisableActor;

// Re-export event types for convenience
pub use oya_events::{
    BeadEvent, BeadId, BeadResult, BeadSpec, BeadState, Complexity, PhaseId, PhaseOutput,
    durable_store::DurableEventStore,
};

#[derive(Clone, Default)]
pub struct StateManagerActorDef;

/// State management for the StateManagerActor.
#[allow(dead_code)]
pub struct StateManagerState {
    /// Connection configuration for SurrealDB.
    db_config: DatabaseConfig,
}

/// Database configuration for state persistence.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Path to the RocksDB storage directory.
    pub storage_path: String,

    /// Namespace for the database.
    pub namespace: String,

    /// Database name within the namespace.
    pub database: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            storage_path: ".oya/state".to_string(),
            namespace: "oya".to_string(),
            database: "state".to_string(),
        }
    }
}

/// Messages for the StateManagerActor.
///
/// Design principles:
/// - Commands are fire-and-forget (use `cast!`)
/// - Queries return responses (use `call!`)
/// - State data uses bincode serialization for storage
/// - Business errors are returned in RPC replies, NOT as actor crashes
#[derive(Debug)]
pub enum StateManagerMessage {
    // COMMANDS (fire-and-forget via cast!)
    /// Save state to persistent storage.
    ///
    /// This persists the provided state data to SurrealDB with the given key.
    /// If serialization fails, the error is logged but the actor continues running.
    SaveState {
        /// Unique identifier for this state.
        key: String,
        /// Serialized state data (already bincode-encoded).
        data: Vec<u8>,
        /// Optional version for optimistic locking.
        version: Option<u64>,
    },

    /// Delete state from persistent storage.
    ///
    /// Removes the state associated with the given key.
    /// If the key doesn't exist, this is a no-op.
    DeleteState {
        /// Unique identifier for the state to delete.
        key: String,
    },

    /// Clear all state (dangerous operation).
    ///
    /// This removes all stored state. Use with caution.
    ClearAll,

    // QUERIES (request-response via call! / call_t!)
    /// Load state from persistent storage.
    LoadState {
        /// Unique identifier for the state to load.
        key: String,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Vec<u8>, ActorError>>,
    },

    /// Check if state exists.
    StateExists {
        /// Unique identifier for the state to check.
        key: String,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<bool, ActorError>>,
    },

    /// Get state version.
    GetStateVersion {
        /// Unique identifier for the state.
        key: String,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Option<u64>, ActorError>>,
    },

    /// List all state keys.
    ListKeys {
        /// Optional prefix filter (e.g., "workflow:" to list only workflow keys).
        prefix: Option<String>,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Vec<String>, ActorError>>,
    },
}

impl Actor for StateManagerActorDef {
    type Msg = StateManagerMessage;
    type State = StateManagerState;
    type Arguments = DatabaseConfig;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("StateManagerActor starting with config: {:?}", args);
        Ok(StateManagerState { db_config: args })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            StateManagerMessage::SaveState { key, data, version } => {
                info!(
                    "Saving state: key={}, size={} bytes, version={:?}",
                    key,
                    data.len(),
                    version
                );
                // TODO: Implement actual persistence to SurrealDB
                // For now, this is a stub that logs the operation
            }

            StateManagerMessage::DeleteState { key } => {
                info!("Deleting state: key={}", key);
                // TODO: Implement actual deletion from SurrealDB
            }

            StateManagerMessage::ClearAll => {
                info!("Clearing all state");
                // TODO: Implement actual clear operation
            }

            StateManagerMessage::LoadState { key, reply } => {
                let result = Err(ActorError::actor_unavailable());
                info!("Loading state: key={}, result={:?}", key, result);
                // TODO: Implement actual load from SurrealDB
                let _ = reply.send(result);
            }

            StateManagerMessage::StateExists { key, reply } => {
                let result = Err(ActorError::actor_unavailable());
                info!("Checking state exists: key={}, result={:?}", key, result);
                // TODO: Implement actual exists check
                let _ = reply.send(result);
            }

            StateManagerMessage::GetStateVersion { key, reply } => {
                let result = Err(ActorError::actor_unavailable());
                info!("Getting state version: key={}, result={:?}", key, result);
                // TODO: Implement actual version check
                let _ = reply.send(result);
            }

            StateManagerMessage::ListKeys { prefix, reply } => {
                let result = Err(ActorError::actor_unavailable());
                info!("Listing keys: prefix={:?}, result={:?}", prefix, result);
                // TODO: Implement actual list operation
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

impl GenericSupervisableActor for StateManagerActorDef {
    fn default_args() -> Self::Arguments {
        Self::Arguments::default()
    }
}

/// Event store actor for durable event persistence.
///
/// This actor manages event storage with fsync guarantees using the DurableEventStore.
#[derive(Clone, Default)]
pub struct EventStoreActorDef;

/// Event store state for the EventStoreActor.
pub struct EventStoreState {
    /// The durable event store backend.
    store: std::sync::Arc<DurableEventStore>,
}

/// Messages for the EventStoreActor.
///
/// This actor manages event persistence using the DurableEventStore.
/// All events are serialized using bincode before storage.
///
/// # Design Principles
/// - All operations use request-response pattern (use `call!`)
/// - Business errors are returned in RPC replies, NOT as actor crashes
/// - AppendEvent preserves fsync guarantees (only replies after sync)
#[derive(Debug)]
pub enum EventStoreMessage {
    /// Append a bead event to durable storage with fsync guarantee.
    ///
    /// This will:
    /// 1. Serialize the event using bincode
    /// 2. Write to WAL (write-ahead log)
    /// 3. Fsync to disk before replying
    /// 4. Persist to SurrealDB
    ///
    /// The reply is only sent after successful fsync, guaranteeing durability.
    AppendEvent {
        /// The event to append.
        event: BeadEvent,
        /// Reply port for the response (sent after fsync).
        reply: RpcReplyPort<Result<(), ActorError>>,
    },

    /// Read all events for a specific bead.
    ReadEvents {
        /// The bead ID to read events for.
        bead_id: BeadId,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Vec<BeadEvent>, ActorError>>,
    },

    /// Replay events from a specific checkpoint.
    ReplayEvents {
        /// The checkpoint event ID to start replaying from.
        checkpoint_id: String,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Vec<BeadEvent>, ActorError>>,
    },
}

impl Actor for EventStoreActorDef {
    type Msg = EventStoreMessage;
    type State = EventStoreState;
    type Arguments = std::sync::Arc<DurableEventStore>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("EventStoreActor starting");
        Ok(EventStoreState { store: args })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            EventStoreMessage::AppendEvent { event, reply } => {
                info!(
                    "Appending event: bead_id={}, event_type={}",
                    event.bead_id(),
                    event.event_type()
                );

                // Append with fsync guarantee - DurableEventStore ensures
                // file.sync_all() is called before returning Ok(())
                let result = state
                    .store
                    .append_event(&event)
                    .await
                    .map_err(|e| ActorError::internal(format!("Failed to append event: {}", e)));

                match &result {
                    Ok(()) => {
                        info!(
                            "Successfully appended event: {} for bead {}",
                            event.event_id(),
                            event.bead_id()
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to append event {} for bead {}: {}",
                            event.event_id(),
                            event.bead_id(),
                            e
                        );
                    }
                }

                // Send reply after fsync completes (success or failure)
                let _ = reply.send(result);
            }

            EventStoreMessage::ReadEvents { bead_id, reply } => {
                let result = state
                    .store
                    .read_events(&bead_id)
                    .await
                    .map_err(|e| ActorError::internal(format!("Failed to read events: {}", e)));

                info!(
                    "Read events for bead {}: result={}",
                    bead_id,
                    if result.is_ok() { "ok" } else { "err" }
                );

                let _ = reply.send(result);
            }

            EventStoreMessage::ReplayEvents {
                checkpoint_id,
                reply,
            } => {
                let result = state
                    .store
                    .replay_from(&checkpoint_id)
                    .await
                    .map_err(|e| ActorError::internal(format!("Failed to replay from checkpoint: {}", e)));

                info!(
                    "Replay from checkpoint {}: result={}",
                    checkpoint_id,
                    if result.is_ok() { "ok" } else { "err" }
                );

                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

impl GenericSupervisableActor for EventStoreActorDef {
    fn default_args() -> Self::Arguments {
        // EventStoreActor requires a DurableEventStore instance.
        // Use EventStoreActorDef::spawn() with an actual store instance.
        panic!(
            "EventStoreActor requires a DurableEventStore instance. \
             Use EventStoreActorDef::spawn() with an actual store."
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn should_create_default_database_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.storage_path, ".oya/state");
        assert_eq!(config.namespace, "oya");
        assert_eq!(config.database, "state");
    }

    #[test]
    fn should_serialize_database_config() {
        let config = DatabaseConfig {
            storage_path: "/tmp/test".to_string(),
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        // Test bincode serialization
        let encoded = bincode::serde::encode_to_vec(&config, bincode::config::standard())
            .expect("Failed to encode config");

        // Test bincode deserialization
        let (decoded, _): (DatabaseConfig, _) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard())
                .expect("Failed to decode config");

        assert_eq!(config.storage_path, decoded.storage_path);
        assert_eq!(config.namespace, decoded.namespace);
        assert_eq!(config.database, decoded.database);
    }

    // ============================================================================
    // StateManagerActor Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_state_manager_save_and_load() {
        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_state_manager");

        let config = DatabaseConfig {
            storage_path: storage_path.to_string(),
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        let (actor, handle) = Actor::spawn(None, StateManagerActorDef, config)
            .await
            .expect("Failed to spawn StateManagerActor");

        let key = "test_key_1".to_string();
        let data = vec![1, 2, 3, 4, 5];

        // Test SaveState (fire-and-forget)
        actor
            .send_message(StateManagerMessage::SaveState {
                key: key.clone(),
                data: data.clone(),
                version: None,
            })
            .expect("Failed to send SaveState");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test LoadState (query)
        let loaded = ractor::call!(actor, StateManagerMessage::LoadState {
            key: key.clone(),
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(loaded.is_ok(), "LoadState should succeed");
        let loaded_data = loaded.ok().unwrap();
        assert!(loaded_data.is_ok(), "LoadState result should be Ok");
        assert_eq!(loaded_data.ok().unwrap(), data, "Loaded data should match saved data");

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }

    #[tokio::test]
    async fn test_state_manager_delete() {
        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_state_manager_delete");

        let config = DatabaseConfig {
            storage_path: storage_path.to_string(),
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        let (actor, handle) = Actor::spawn(None, StateManagerActorDef, config)
            .await
            .expect("Failed to spawn StateManagerActor");

        let key = "test_key_delete".to_string();
        let data = vec![10, 20, 30];

        // Save then delete
        actor
            .send_message(StateManagerMessage::SaveState {
                key: key.clone(),
                data,
                version: None,
            })
            .expect("Failed to send SaveState");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        actor
            .send_message(StateManagerMessage::DeleteState { key: key.clone() })
            .expect("Failed to send DeleteState");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify deleted
        let loaded = ractor::call!(actor, StateManagerMessage::LoadState {
            key,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(loaded.is_ok());
        let result = loaded.ok().unwrap();
        assert!(result.is_err(), "Deleted key should return error");

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }

    #[tokio::test]
    async fn test_state_manager_exists() {
        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_state_manager_exists");

        let config = DatabaseConfig {
            storage_path: storage_path.to_string(),
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        let (actor, handle) = Actor::spawn(None, StateManagerActorDef, config)
            .await
            .expect("Failed to spawn StateManagerActor");

        let key = "test_key_exists".to_string();

        // Check non-existent key
        let exists = ractor::call!(actor, StateManagerMessage::StateExists {
            key: key.clone(),
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(exists.is_ok());
        assert_eq!(exists.ok().unwrap().ok().unwrap(), false);

        // Save key
        actor
            .send_message(StateManagerMessage::SaveState {
                key: key.clone(),
                data: vec![1, 2, 3],
                version: None,
            })
            .expect("Failed to send SaveState");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check exists
        let exists = ractor::call!(actor, StateManagerMessage::StateExists {
            key,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(exists.is_ok());
        assert_eq!(exists.ok().unwrap().ok().unwrap(), true);

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }

    #[tokio::test]
    async fn test_state_manager_list_keys() {
        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_state_manager_list");

        let config = DatabaseConfig {
            storage_path: storage_path.to_string(),
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        let (actor, handle) = Actor::spawn(None, StateManagerActorDef, config)
            .await
            .expect("Failed to spawn StateManagerActor");

        // Save multiple keys with different prefixes
        for i in 1..=3 {
            actor
                .send_message(StateManagerMessage::SaveState {
                    key: format!("workflow:{}", i),
                    data: vec![i as u8],
                    version: None,
                })
                .expect("Failed to send SaveState");
        }

        for i in 1..=2 {
            actor
                .send_message(StateManagerMessage::SaveState {
                    key: format!("checkpoint:{}", i),
                    data: vec![i as u8],
                    version: None,
                })
                .expect("Failed to send SaveState");
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // List all keys
        let all_keys = ractor::call!(actor, StateManagerMessage::ListKeys {
            prefix: None,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(all_keys.is_ok());
        let keys = all_keys.ok().unwrap().ok().unwrap();
        assert_eq!(keys.len(), 5);

        // List with prefix
        let workflow_keys = ractor::call!(actor, StateManagerMessage::ListKeys {
            prefix: Some("workflow:".to_string()),
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(workflow_keys.is_ok());
        let keys = workflow_keys.ok().unwrap().ok().unwrap();
        assert_eq!(keys.len(), 3);

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }

    #[tokio::test]
    async fn test_state_manager_version() {
        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_state_manager_version");

        let config = DatabaseConfig {
            storage_path: storage_path.to_string(),
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
        };

        let (actor, handle) = Actor::spawn(None, StateManagerActorDef, config)
            .await
            .expect("Failed to spawn StateManagerActor");

        let key = "versioned_key".to_string();

        // Save with version
        actor
            .send_message(StateManagerMessage::SaveState {
                key: key.clone(),
                data: vec![1, 2, 3],
                version: Some(5),
            })
            .expect("Failed to send SaveState");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get version
        let version = ractor::call!(actor, StateManagerMessage::GetStateVersion {
            key,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(version.is_ok());
        assert_eq!(version.ok().unwrap().ok().unwrap(), Some(5));

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }

    // ============================================================================
    // EventStoreActor Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_event_store_append_and_read() {
        use oya_events::durable_store::{self, ConnectionConfig};

        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_event_store");

        // Connect to test database
        let db = durable_store::connect(ConnectionConfig {
            storage_path: storage_path.into(),
            namespace: "test_ns".to_string(),
            database: "test_events".to_string(),
            ..Default::default()
        })
        .await
        .expect("Failed to connect to test DB");

        let store = DurableEventStore::new(db)
            .await
            .expect("Failed to create DurableEventStore")
            .with_wal_dir(format!("{}/.wal", storage_path));

        let (actor, handle) = Actor::spawn(None, EventStoreActorDef, std::sync::Arc::new(store))
            .await
            .expect("Failed to spawn EventStoreActor");

        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test Event").with_complexity(Complexity::Simple);

        // Append event (with fsync guarantee)
        let append_result = ractor::call!(actor, EventStoreMessage::AppendEvent {
            event: BeadEvent::created(bead_id, spec),
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(append_result.is_ok(), "AppendEvent should succeed");
        assert!(append_result.ok().unwrap().is_ok(), "AppendEvent result should be Ok");

        // Read events
        let events = ractor::call!(actor, EventStoreMessage::ReadEvents {
            bead_id,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(events.is_ok(), "ReadEvents should succeed");
        let event_list = events.ok().unwrap();
        assert!(event_list.is_ok(), "ReadEvents result should be Ok");
        assert_eq!(event_list.ok().unwrap().len(), 1, "Should have one event");

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }

    #[tokio::test]
    async fn test_event_store_replay() {
        use oya_events::durable_store::{self, ConnectionConfig};

        let temp_dir = tempfile::tempdir().ok();
        let storage_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_str().unwrap())
            .unwrap_or("/tmp/test_event_store_replay");

        // Connect to test database
        let db = durable_store::connect(ConnectionConfig {
            storage_path: storage_path.into(),
            namespace: "test_ns".to_string(),
            database: "test_events_replay".to_string(),
            ..Default::default()
        })
        .await
        .expect("Failed to connect to test DB");

        let store = DurableEventStore::new(db)
            .await
            .expect("Failed to create DurableEventStore")
            .with_wal_dir(format!("{}/.wal", storage_path));

        let (actor, handle) = Actor::spawn(None, EventStoreActorDef, std::sync::Arc::new(store))
            .await
            .expect("Failed to spawn EventStoreActor");

        let bead_id = BeadId::new();

        // Create multiple events
        let phase_id = PhaseId::new();

        actor
            .send_message(EventStoreMessage::AppendEvent {
                event: BeadEvent::created(
                    bead_id,
                    BeadSpec::new("Test").with_complexity(Complexity::Simple),
                ),
            })
            .expect("Failed to send created event");

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        actor
            .send_message(EventStoreMessage::AppendEvent {
                event: BeadEvent::phase_started(bead_id, phase_id, "test_phase"),
            })
            .expect("Failed to send phase_started event");

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Read all events to get checkpoint ID
        let all_events = ractor::call!(actor, EventStoreMessage::ReadEvents {
            bead_id,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(all_events.is_ok());
        let event_list = all_events.ok().unwrap().ok().unwrap();
        assert_eq!(event_list.len(), 2);

        let checkpoint_id = event_list[0].event_id().to_string();

        // Replay from checkpoint
        let replayed = ractor::call!(actor, EventStoreMessage::ReplayEvents {
            checkpoint_id,
        }, tokio::time::Duration::from_secs(5)).await;

        assert!(replayed.is_ok(), "ReplayEvents should succeed");
        let replayed_events = replayed.ok().unwrap();
        assert!(replayed_events.is_ok(), "ReplayEvents result should be Ok");
        // Should have events after checkpoint (could be 1 or 2 depending on timestamp)
        let replayed_count = replayed_events.ok().unwrap().len();
        assert!(replayed_count >= 1, "Should have at least one replayed event");

        actor.stop(None);
        handle.await.expect("Actor shutdown failed");
    }
}
