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
/// - All messages use bincode serialization for storage
/// - Business errors are returned in RPC replies, NOT as actor crashes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        state: &mut Self::State,
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

#[derive(Clone, Default)]
pub struct EventStoreActorDef;

/// Event store state for the EventStoreActor.
pub struct EventStoreState {
    /// The durable event store backend.
    store: Option<std::sync::Arc<DurableEventStore>>,
}

/// Messages for the EventStoreActor.
///
/// This actor manages event persistence using the DurableEventStore.
/// All events are serialized using bincode before storage.
///
/// # Design Principles
/// - Commands are fire-and-forget (use `cast!`)
/// - Queries return responses (use `call!`)
/// - Business errors are returned in RPC replies, NOT as actor crashes
#[derive(Clone, Debug)]
pub enum EventStoreMessage {
    // COMMANDS (fire-and-forget via cast!)
    /// Append a bead event to durable storage.
    ///
    /// This will:
    /// 1. Serialize the event using bincode
    /// 2. Write to WAL (write-ahead log)
    /// 3. Persist to SurrealDB
    AppendEvent {
        /// The event to append.
        event: BeadEvent,
    },

    /// Initiate graceful shutdown.
    Shutdown,

    // QUERIES (request-response via call! / call_t!)
    /// Read all events for a specific bead.
    ReadEvents {
        /// The bead ID to read events for.
        bead_id: BeadId,
        /// Reply port for the response.
        reply: RpcReplyPort<Result<Vec<BeadEvent>, ActorError>>,
    },

    /// Replay events from a specific checkpoint.
    ReplayFrom {
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
        Ok(EventStoreState { store: Some(args) })
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

                let result = if let Some(store) = &state.store {
                    store
                        .append_event(&event)
                        .await
                        .map_err(|e| ActorError::internal(format!("Failed to append event: {}", e)))
                } else {
                    Err(ActorError::internal("EventStore not initialized"))
                };

                if result.is_ok() {
                    info!(
                        "Successfully appended event: {} for bead {}",
                        event.event_id(),
                        event.bead_id()
                    );
                } else {
                    tracing::error!(
                        "Failed to append event {} for bead {}: {:?}",
                        event.event_id(),
                        event.bead_id(),
                        result
                    );
                }

                let _ = reply.send(result);
            }

            EventStoreMessage::Shutdown => {
                info!("EventStoreActor shutting down");
                // TODO: Implement graceful shutdown if needed
            }

            EventStoreMessage::ReadEvents { bead_id, reply } => {
                let result = if let Some(store) = &state.store {
                    store
                        .read_events(&bead_id)
                        .await
                        .map_err(|e| ActorError::internal(format!("Failed to read events: {}", e)))
                } else {
                    Err(ActorError::internal("EventStore not initialized"))
                };

                info!(
                    "Read events for bead {}: result={}",
                    bead_id,
                    if result.is_ok() { "ok" } else { "err" }
                );

                let _ = reply.send(result);
            }

            EventStoreMessage::ReplayFrom {
                checkpoint_id,
                reply,
            } => {
                let result = if let Some(store) = &state.store {
                    store.replay_from(&checkpoint_id).await.map_err(|e| {
                        ActorError::internal(format!("Failed to replay from checkpoint: {}", e))
                    })
                } else {
                    Err(ActorError::internal("EventStore not initialized"))
                };

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
        // TODO: This is a placeholder - in production, the EventStoreActor
        // should be initialized with an actual DurableEventStore instance
        // For now, we create a dummy store since DurableEventStore::new() requires async
        panic!(
            "EventStoreActor requires a DurableEventStore instance. Use spawn_event_store() helper instead."
        );
    }
}

#[cfg(test)]
mod tests {
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
        let decoded: DatabaseConfig = bincode::serde::decode_from_slice(
            &encoded,
            bincode::config::standard(),
        )
        .expect("Failed to decode config");

        assert_eq!(config.storage_path, decoded.storage_path);
        assert_eq!(config.namespace, decoded.namespace);
        assert_eq!(config.database, decoded.database);
    }
}
