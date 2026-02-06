//! Virtual Object trait and management.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::messaging::{Message, MessagePayload};
use crate::persistence::{OrchestratorStore, PersistenceResult};

use super::state::ObjectState;

/// Unique identifier for a virtual object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId(String);

impl ObjectId {
    /// Create a new object ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ObjectId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ObjectId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Response from a message handler.
#[derive(Debug, Clone)]
pub enum HandlerResponse {
    /// Successful response with payload.
    Success(MessagePayload),
    /// Error response.
    Error {
        /// Error code
        code: String,
        /// Error message
        message: String,
    },
    /// No response (for one-way messages).
    NoResponse,
}

impl HandlerResponse {
    /// Create a success response.
    #[must_use]
    pub fn success(payload: MessagePayload) -> Self {
        Self::Success(payload)
    }

    /// Create an error response.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a no-response.
    #[must_use]
    pub fn no_response() -> Self {
        Self::NoResponse
    }

    /// Check if this is a success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if this is an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

/// Configuration for a virtual object.
#[derive(Debug, Clone)]
pub struct ObjectConfig {
    /// Whether to persist state.
    pub persist_state: bool,
    /// State snapshot interval (0 = never).
    pub snapshot_interval: u64,
    /// Maximum state size in bytes.
    pub max_state_size: usize,
}

impl Default for ObjectConfig {
    fn default() -> Self {
        Self {
            persist_state: true,
            snapshot_interval: 100,           // Every 100 operations
            max_state_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Context provided to message handlers.
pub struct HandlerContext<'a> {
    /// The object's state
    pub state: &'a mut ObjectState,
    /// The object ID
    pub object_id: &'a ObjectId,
    /// Timestamp of the message
    pub timestamp: DateTime<Utc>,
}

/// Trait for implementing virtual object message handlers.
///
/// Implement this trait to create custom virtual objects with
/// specific message handling logic.
#[async_trait]
pub trait ObjectHandler: Send + Sync {
    /// Handle an incoming message.
    ///
    /// The handler has mutable access to the object's state through
    /// the context parameter.
    async fn handle_message(
        &self,
        message: &Message,
        context: HandlerContext<'_>,
    ) -> HandlerResponse;

    /// Called when the object is initialized.
    ///
    /// Default implementation does nothing.
    async fn on_init(&self, _context: HandlerContext<'_>) {}

    /// Called before the object is destroyed.
    ///
    /// Default implementation does nothing.
    async fn on_destroy(&self, _context: HandlerContext<'_>) {}
}

/// A virtual object instance.
pub struct VirtualObject {
    id: ObjectId,
    config: ObjectConfig,
    state: ObjectState,
    handler: Arc<dyn ObjectHandler>,
    operation_count: u64,
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
}

impl VirtualObject {
    /// Create a new virtual object.
    #[must_use]
    pub fn new(
        id: impl Into<ObjectId>,
        config: ObjectConfig,
        handler: Arc<dyn ObjectHandler>,
    ) -> Self {
        let id = id.into();
        let now = Utc::now();

        Self {
            id: id.clone(),
            config,
            state: ObjectState::new(id),
            handler,
            operation_count: 0,
            created_at: now,
            last_accessed: now,
        }
    }

    /// Create a virtual object with persistent storage.
    #[must_use]
    pub fn with_store(
        id: impl Into<ObjectId>,
        config: ObjectConfig,
        handler: Arc<dyn ObjectHandler>,
        store: OrchestratorStore,
    ) -> Self {
        let id = id.into();
        let now = Utc::now();

        Self {
            id: id.clone(),
            config,
            state: ObjectState::with_store(id, store),
            handler,
            operation_count: 0,
            created_at: now,
            last_accessed: now,
        }
    }

    /// Get the object ID.
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// Get the state.
    #[must_use]
    pub fn state(&self) -> &ObjectState {
        &self.state
    }

    /// Get mutable state.
    pub fn state_mut(&mut self) -> &mut ObjectState {
        &mut self.state
    }

    /// Get the operation count.
    #[must_use]
    pub fn operation_count(&self) -> u64 {
        self.operation_count
    }

    /// Get when the object was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get when the object was last accessed.
    #[must_use]
    pub fn last_accessed(&self) -> DateTime<Utc> {
        self.last_accessed
    }

    /// Initialize the object.
    ///
    /// # Errors
    ///
    /// Returns an error if state loading fails.
    pub async fn init(&mut self) -> PersistenceResult<()> {
        // Load existing state if persistent
        if self.config.persist_state {
            self.state.load().await?;
        }

        // Call handler's init
        let context = HandlerContext {
            state: &mut self.state,
            object_id: &self.id,
            timestamp: Utc::now(),
        };
        self.handler.on_init(context).await;

        Ok(())
    }

    /// Handle a message.
    ///
    /// # Errors
    ///
    /// Returns an error if state commit fails.
    pub async fn handle_message(
        &mut self,
        message: &Message,
    ) -> PersistenceResult<HandlerResponse> {
        self.last_accessed = Utc::now();
        self.operation_count = self.operation_count.saturating_add(1);

        let context = HandlerContext {
            state: &mut self.state,
            object_id: &self.id,
            timestamp: message.created_at(),
        };

        let response = self.handler.handle_message(message, context).await;

        // Commit state changes
        if self.config.persist_state && self.state.is_dirty() {
            self.state.commit().await?;
        }

        // Check if we should snapshot
        if self.config.snapshot_interval > 0
            && self.operation_count % self.config.snapshot_interval == 0
        {
            // Snapshot is automatic on commit, so nothing extra needed
        }

        Ok(response)
    }

    /// Destroy the object.
    pub async fn destroy(&mut self) {
        let context = HandlerContext {
            state: &mut self.state,
            object_id: &self.id,
            timestamp: Utc::now(),
        };
        self.handler.on_destroy(context).await;
    }
}

/// Manages virtual objects.
pub struct ObjectManager {
    config: ObjectConfig,
    store: Option<OrchestratorStore>,
    objects: Arc<RwLock<HashMap<String, Arc<RwLock<VirtualObject>>>>>,
}

impl ObjectManager {
    /// Create a new in-memory object manager.
    #[must_use]
    pub fn new(config: ObjectConfig) -> Self {
        Self {
            config,
            store: None,
            objects: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create an object manager with persistent storage.
    #[must_use]
    pub fn with_store(config: ObjectConfig, store: OrchestratorStore) -> Self {
        Self {
            config,
            store: Some(store),
            objects: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create an object.
    pub async fn get_or_create(
        &self,
        id: impl Into<ObjectId>,
        handler: Arc<dyn ObjectHandler>,
    ) -> PersistenceResult<Arc<RwLock<VirtualObject>>> {
        let id = id.into();
        let id_str = id.as_str().to_string();

        // Check if exists
        {
            let objects = self.objects.read().await;
            if let Some(object) = objects.get(&id_str) {
                return Ok(Arc::clone(object));
            }
        }

        // Create new object
        let mut object = if let Some(store) = &self.store {
            VirtualObject::with_store(id, self.config.clone(), handler, store.clone())
        } else {
            VirtualObject::new(id, self.config.clone(), handler)
        };

        // Initialize
        object.init().await?;

        let object = Arc::new(RwLock::new(object));

        // Register
        {
            let mut objects = self.objects.write().await;
            objects.insert(id_str, Arc::clone(&object));
        }

        Ok(object)
    }

    /// Get an existing object.
    pub async fn get(&self, id: &str) -> Option<Arc<RwLock<VirtualObject>>> {
        let objects = self.objects.read().await;
        objects.get(id).cloned()
    }

    /// Remove an object.
    pub async fn remove(&self, id: &str) -> Option<Arc<RwLock<VirtualObject>>> {
        let mut objects = self.objects.write().await;
        let object = objects.remove(id)?;

        // Call destroy
        {
            let mut obj = object.write().await;
            obj.destroy().await;
        }

        Some(object)
    }

    /// Get the number of managed objects.
    pub async fn object_count(&self) -> usize {
        let objects = self.objects.read().await;
        objects.len()
    }

    /// Initialize the object schema in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    pub async fn initialize_schema(store: &OrchestratorStore) -> PersistenceResult<()> {
        ObjectState::initialize_schema(store).await
    }
}

/// A simple echo handler for testing.
pub struct EchoHandler;

#[async_trait]
impl ObjectHandler for EchoHandler {
    async fn handle_message(
        &self,
        message: &Message,
        _context: HandlerContext<'_>,
    ) -> HandlerResponse {
        HandlerResponse::success(message.payload().clone())
    }
}

/// A counter handler that maintains a count in state.
pub struct CounterHandler;

#[async_trait]
impl ObjectHandler for CounterHandler {
    async fn handle_message(
        &self,
        message: &Message,
        context: HandlerContext<'_>,
    ) -> HandlerResponse {
        let payload = message.payload();

        // Get operation from payload
        let operation = payload.get("operation").and_then(|v| v.as_str());

        match operation {
            Some("increment") => {
                let current = context.state.get_i64("count").unwrap_or(0);
                let amount = payload.get("amount").and_then(|v| v.as_i64()).unwrap_or(1);
                context.state.set("count", current.saturating_add(amount));
                HandlerResponse::success(serde_json::json!({ "count": current + amount }))
            }
            Some("decrement") => {
                let current = context.state.get_i64("count").unwrap_or(0);
                let amount = payload.get("amount").and_then(|v| v.as_i64()).unwrap_or(1);
                context.state.set("count", current.saturating_sub(amount));
                HandlerResponse::success(serde_json::json!({ "count": current - amount }))
            }
            Some("get") | None => {
                let current = context.state.get_i64("count").unwrap_or(0);
                HandlerResponse::success(serde_json::json!({ "count": current }))
            }
            Some("reset") => {
                context.state.set("count", 0i64);
                HandlerResponse::success(serde_json::json!({ "count": 0 }))
            }
            Some(op) => {
                HandlerResponse::error("UNKNOWN_OPERATION", format!("Unknown operation: {}", op))
            }
        }
    }

    async fn on_init(&self, context: HandlerContext<'_>) {
        // Initialize count if not present
        if !context.state.contains_key("count") {
            context.state.set("count", 0i64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::Message;

    #[test]
    fn test_object_id_from_string() {
        let id: ObjectId = "test-obj".into();
        assert_eq!(id.as_str(), "test-obj");
    }

    #[test]
    fn test_object_id_display() {
        let id = ObjectId::new("display-test");
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn test_handler_response_success() {
        let response = HandlerResponse::success(serde_json::json!({"result": "ok"}));
        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_handler_response_error() {
        let response = HandlerResponse::error("ERR_001", "Something went wrong");
        assert!(response.is_error());
        assert!(!response.is_success());
    }

    #[test]
    fn test_object_config_default() {
        let config = ObjectConfig::default();
        assert!(config.persist_state);
        assert_eq!(config.snapshot_interval, 100);
    }

    #[tokio::test]
    async fn test_virtual_object_creation() {
        let handler = Arc::new(EchoHandler);
        let object = VirtualObject::new("obj-1", ObjectConfig::default(), handler);

        assert_eq!(object.id().as_str(), "obj-1");
        assert_eq!(object.operation_count(), 0);
    }

    #[tokio::test]
    async fn test_echo_handler() {
        let handler = Arc::new(EchoHandler);
        let mut object = VirtualObject::new("echo-obj", ObjectConfig::default(), handler);
        let _ = object.init().await;

        let msg = Message::request("reply", serde_json::json!({"echo": "test"}));
        let response = object.handle_message(&msg).await;

        assert!(response.is_ok());
        if let Ok(HandlerResponse::Success(payload)) = response {
            assert_eq!(payload.get("echo").and_then(|v| v.as_str()), Some("test"));
        }
    }

    #[tokio::test]
    async fn test_counter_handler_increment() {
        let handler = Arc::new(CounterHandler);
        let mut object = VirtualObject::new("counter", ObjectConfig::default(), handler);
        let _ = object.init().await;

        // Initial get
        let get_msg = Message::request("reply", serde_json::json!({"operation": "get"}));
        let response = object.handle_message(&get_msg).await;
        assert!(response.is_ok());

        // Increment
        let inc_msg = Message::request("reply", serde_json::json!({"operation": "increment"}));
        let response = object.handle_message(&inc_msg).await;
        assert!(response.is_ok());

        if let Ok(HandlerResponse::Success(payload)) = response {
            assert_eq!(payload.get("count").and_then(|v| v.as_i64()), Some(1));
        }
    }

    #[tokio::test]
    async fn test_counter_handler_operations() {
        let handler = Arc::new(CounterHandler);
        let mut object = VirtualObject::new("counter", ObjectConfig::default(), handler);
        let _ = object.init().await;

        // Increment by 5
        let msg = Message::request(
            "reply",
            serde_json::json!({"operation": "increment", "amount": 5}),
        );
        let _ = object.handle_message(&msg).await;

        // Decrement by 2
        let msg = Message::request(
            "reply",
            serde_json::json!({"operation": "decrement", "amount": 2}),
        );
        let response = object.handle_message(&msg).await;

        if let Ok(HandlerResponse::Success(payload)) = response {
            assert_eq!(payload.get("count").and_then(|v| v.as_i64()), Some(3));
        }
    }

    #[tokio::test]
    async fn test_object_manager_get_or_create() {
        let manager = ObjectManager::new(ObjectConfig::default());
        let handler: Arc<dyn ObjectHandler> = Arc::new(EchoHandler);

        let obj1 = manager.get_or_create("obj-1", Arc::clone(&handler)).await;
        assert!(obj1.is_ok());

        // Getting same ID should return same object
        let obj2 = manager.get_or_create("obj-1", handler).await;
        assert!(obj2.is_ok());

        assert_eq!(manager.object_count().await, 1);
    }

    #[tokio::test]
    async fn test_object_manager_remove() {
        let manager = ObjectManager::new(ObjectConfig::default());
        let handler: Arc<dyn ObjectHandler> = Arc::new(EchoHandler);

        let _ = manager.get_or_create("obj-1", handler).await;
        assert_eq!(manager.object_count().await, 1);

        let removed = manager.remove("obj-1").await;
        assert!(removed.is_some());
        assert_eq!(manager.object_count().await, 0);
    }
}
