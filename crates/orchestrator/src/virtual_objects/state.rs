//! State management for virtual objects.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::persistence::{OrchestratorStore, PersistenceError, PersistenceResult};

use super::object::ObjectId;

/// A typed state value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum StateValue {
    /// Null/absent value.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Integer value.
    Integer(i64),
    /// Float value.
    Float(f64),
    /// String value.
    String(String),
    /// Binary data.
    Bytes(Vec<u8>),
    /// JSON value.
    Json(serde_json::Value),
    /// List of values.
    List(Vec<StateValue>),
}

impl StateValue {
    /// Create a null value.
    #[must_use]
    pub fn null() -> Self {
        Self::Null
    }

    /// Create a boolean value.
    #[must_use]
    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Create an integer value.
    #[must_use]
    pub fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    /// Create a float value.
    #[must_use]
    pub fn float(value: f64) -> Self {
        Self::Float(value)
    }

    /// Create a string value.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// Create a bytes value.
    #[must_use]
    pub fn bytes(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }

    /// Create a JSON value.
    #[must_use]
    pub fn json(value: serde_json::Value) -> Self {
        Self::Json(value)
    }

    /// Create a list value.
    #[must_use]
    pub fn list(values: Vec<StateValue>) -> Self {
        Self::List(values)
    }

    /// Check if the value is null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Try to get as boolean.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get as integer.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get as float.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Try to get as string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get as JSON.
    #[must_use]
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Json(v) => Some(v),
            _ => None,
        }
    }
}

impl From<bool> for StateValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i64> for StateValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<i32> for StateValue {
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<f64> for StateValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for StateValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for StateValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<serde_json::Value> for StateValue {
    fn from(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

/// Pending state write operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateWrite {
    /// Key being written
    pub key: String,
    /// Value being written (None = delete)
    pub value: Option<StateValue>,
    /// When the write was requested
    pub requested_at: DateTime<Utc>,
}

/// A snapshot of object state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Object ID
    pub object_id: ObjectId,
    /// Snapshot ID
    pub snapshot_id: String,
    /// Full state as of snapshot
    pub state: HashMap<String, StateValue>,
    /// When the snapshot was created
    pub created_at: DateTime<Utc>,
    /// Version number
    pub version: u64,
}

impl StateSnapshot {
    /// Create a new snapshot.
    #[must_use]
    pub fn new(object_id: ObjectId, state: HashMap<String, StateValue>, version: u64) -> Self {
        Self {
            object_id: object_id.clone(),
            snapshot_id: format!("snap-{}-{}", object_id.as_str(), version),
            state,
            created_at: Utc::now(),
            version,
        }
    }
}

/// K/V state store for a virtual object.
pub struct ObjectState {
    object_id: ObjectId,
    kv_store: HashMap<String, StateValue>,
    pending_writes: Vec<StateWrite>,
    version: u64,
    store: Option<OrchestratorStore>,
    dirty: bool,
}

impl ObjectState {
    /// Create a new in-memory object state.
    #[must_use]
    pub fn new(object_id: impl Into<ObjectId>) -> Self {
        Self {
            object_id: object_id.into(),
            kv_store: HashMap::new(),
            pending_writes: Vec::new(),
            version: 0,
            store: None,
            dirty: false,
        }
    }

    /// Create an object state with persistent storage.
    #[must_use]
    pub fn with_store(object_id: impl Into<ObjectId>, store: OrchestratorStore) -> Self {
        Self {
            object_id: object_id.into(),
            kv_store: HashMap::new(),
            pending_writes: Vec::new(),
            version: 0,
            store: Some(store),
            dirty: false,
        }
    }

    /// Get the object ID.
    #[must_use]
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Get the current version.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Check if there are uncommitted changes.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get a value from the state.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&StateValue> {
        self.kv_store.get(key)
    }

    /// Get a value as a specific type.
    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.kv_store.get(key).and_then(StateValue::as_str)
    }

    /// Get a value as integer.
    #[must_use]
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.kv_store.get(key).and_then(StateValue::as_i64)
    }

    /// Get a value as boolean.
    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.kv_store.get(key).and_then(StateValue::as_bool)
    }

    /// Set a value in the state.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<StateValue>) {
        let key = key.into();
        let value = value.into();

        self.pending_writes.push(StateWrite {
            key: key.clone(),
            value: Some(value.clone()),
            requested_at: Utc::now(),
        });

        self.kv_store.insert(key, value);
        self.dirty = true;
    }

    /// Delete a value from the state.
    pub fn delete(&mut self, key: &str) {
        self.pending_writes.push(StateWrite {
            key: key.to_string(),
            value: None,
            requested_at: Utc::now(),
        });

        self.kv_store.remove(key);
        self.dirty = true;
    }

    /// Check if a key exists.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.kv_store.contains_key(key)
    }

    /// Get all keys.
    #[must_use]
    pub fn keys(&self) -> Vec<&String> {
        self.kv_store.keys().collect()
    }

    /// Get the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.kv_store.len()
    }

    /// Check if the state is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kv_store.is_empty()
    }

    /// Clear all state.
    pub fn clear(&mut self) {
        for key in self.kv_store.keys().cloned().collect::<Vec<_>>() {
            self.pending_writes.push(StateWrite {
                key,
                value: None,
                requested_at: Utc::now(),
            });
        }
        self.kv_store.clear();
        self.dirty = true;
    }

    /// Commit pending writes.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub async fn commit(&mut self) -> PersistenceResult<()> {
        if !self.dirty {
            return Ok(());
        }

        // Persist if store available
        if let Some(store) = &self.store {
            self.persist_state(store).await?;
        }

        // Clear pending writes
        self.pending_writes.clear();
        self.version = self.version.saturating_add(1);
        self.dirty = false;

        Ok(())
    }

    /// Rollback pending writes.
    pub fn rollback(&mut self) {
        // Reverse pending writes
        for write in self.pending_writes.drain(..).rev() {
            if write.value.is_some() {
                // Was a set, need to delete
                self.kv_store.remove(&write.key);
            }
            // Can't fully rollback deletes without tracking old values
            // For now, just clear pending writes
        }
        self.dirty = false;
    }

    /// Create a snapshot of current state.
    #[must_use]
    pub fn snapshot(&self) -> StateSnapshot {
        StateSnapshot::new(self.object_id.clone(), self.kv_store.clone(), self.version)
    }

    /// Restore from a snapshot.
    pub fn restore(&mut self, snapshot: StateSnapshot) {
        self.kv_store = snapshot.state;
        self.version = snapshot.version;
        self.pending_writes.clear();
        self.dirty = false;
    }

    /// Load state from persistence.
    ///
    /// # Errors
    ///
    /// Returns an error if loading fails.
    pub async fn load(&mut self) -> PersistenceResult<()> {
        let Some(store) = &self.store else {
            return Ok(());
        };

        #[derive(Deserialize)]
        struct StateRecord {
            state: String,
            version: u64,
        }

        let result: Option<StateRecord> = store
            .db()
            .select(("object_state", self.object_id.as_str()))
            .await
            .map_err(PersistenceError::from)?;

        if let Some(record) = result {
            let state: HashMap<String, StateValue> = serde_json::from_str(&record.state)?;

            self.kv_store = state;
            self.version = record.version;
        }

        Ok(())
    }

    /// Persist state to storage.
    async fn persist_state(&self, store: &OrchestratorStore) -> PersistenceResult<()> {
        #[derive(Serialize)]
        struct StateInput {
            object_id: String,
            state: String,
            version: u64,
            updated_at: DateTime<Utc>,
        }

        let state_json = serde_json::to_string(&self.kv_store)?;

        let object_id_str = self.object_id.as_str().to_string();

        let input = StateInput {
            object_id: object_id_str.clone(),
            state: state_json,
            version: self.version.saturating_add(1),
            updated_at: Utc::now(),
        };

        let _: Option<serde_json::Value> = store
            .db()
            .query("UPSERT type::thing('object_state', $id) CONTENT $data")
            .bind(("id", object_id_str))
            .bind(("data", serde_json::to_value(input)?))
            .await
            .map_err(PersistenceError::from)?
            .take(0)
            .map_err(PersistenceError::from)?;

        Ok(())
    }

    /// Initialize the object state schema in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    pub async fn initialize_schema(store: &OrchestratorStore) -> PersistenceResult<()> {
        let schema = r"
            DEFINE TABLE IF NOT EXISTS object_state SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS object_id ON object_state TYPE string;
            DEFINE FIELD IF NOT EXISTS state ON object_state TYPE string;
            DEFINE FIELD IF NOT EXISTS version ON object_state TYPE int;
            DEFINE FIELD IF NOT EXISTS updated_at ON object_state TYPE datetime;
            DEFINE INDEX IF NOT EXISTS object_state_object ON object_state FIELDS object_id;
        ";

        store
            .db()
            .query(schema)
            .await
            .map_err(PersistenceError::from)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_value_null() {
        let value = StateValue::null();
        assert!(value.is_null());
    }

    #[test]
    fn test_state_value_boolean() {
        let value = StateValue::boolean(true);
        assert_eq!(value.as_bool(), Some(true));
    }

    #[test]
    fn test_state_value_integer() {
        let value = StateValue::integer(42);
        assert_eq!(value.as_i64(), Some(42));
        assert_eq!(value.as_f64(), Some(42.0));
    }

    #[test]
    fn test_state_value_string() {
        let value = StateValue::string("hello");
        assert_eq!(value.as_str(), Some("hello"));
    }

    #[test]
    fn test_state_value_from_impls() {
        let _: StateValue = true.into();
        let _: StateValue = 42i64.into();
        let _: StateValue = 1.234f64.into();
        let _: StateValue = "hello".into();
        let _: StateValue = String::from("world").into();
    }

    #[test]
    fn test_object_state_get_set() {
        let mut state = ObjectState::new("obj-1");

        state.set("key1", "value1");
        assert_eq!(state.get_string("key1"), Some("value1"));

        state.set("count", 100i64);
        assert_eq!(state.get_i64("count"), Some(100));
    }

    #[test]
    fn test_object_state_delete() {
        let mut state = ObjectState::new("obj-1");

        state.set("key1", "value1");
        assert!(state.contains_key("key1"));

        state.delete("key1");
        assert!(!state.contains_key("key1"));
    }

    #[test]
    fn test_object_state_dirty_flag() {
        let mut state = ObjectState::new("obj-1");
        assert!(!state.is_dirty());

        state.set("key", "value");
        assert!(state.is_dirty());
    }

    #[test]
    fn test_object_state_snapshot() {
        let mut state = ObjectState::new("obj-1");
        state.set("key1", "value1");
        state.set("key2", 42i64);

        let snapshot = state.snapshot();
        assert_eq!(snapshot.object_id.as_str(), "obj-1");
        assert_eq!(snapshot.state.len(), 2);
    }

    #[test]
    fn test_object_state_restore() {
        let mut state1 = ObjectState::new("obj-1");
        state1.set("key1", "value1");
        let snapshot = state1.snapshot();

        let mut state2 = ObjectState::new("obj-1");
        state2.set("key2", "value2");

        state2.restore(snapshot);
        assert!(state2.contains_key("key1"));
        assert!(!state2.contains_key("key2"));
    }

    #[test]
    fn test_object_state_clear() {
        let mut state = ObjectState::new("obj-1");
        state.set("key1", "value1");
        state.set("key2", "value2");
        assert_eq!(state.len(), 2);

        state.clear();
        assert!(state.is_empty());
        assert!(state.is_dirty());
    }

    #[test]
    fn test_object_state_keys() {
        let mut state = ObjectState::new("obj-1");
        state.set("a", "1");
        state.set("b", "2");
        state.set("c", "3");

        let keys = state.keys();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_state_value_serialization() {
        let value = StateValue::Json(serde_json::json!({"nested": "value"}));
        let json = serde_json::to_string(&value);
        assert!(json.is_ok());

        if let Ok(serialized) = json {
            let deserialized: Result<StateValue, _> = serde_json::from_str(&serialized);
            assert!(deserialized.is_ok());
        }
    }

    #[tokio::test]
    async fn test_object_state_commit() {
        let mut state = ObjectState::new("obj-1");
        state.set("key", "value");
        assert!(state.is_dirty());

        // Commit without store should succeed
        let result = state.commit().await;
        assert!(result.is_ok());
        assert!(!state.is_dirty());
        assert_eq!(state.version(), 1);
    }
}
