//! HTTP request correlation via context
//!
//! Provides request tracking and correlation using immutable BTreeMap-based context.
//! This enables tracing requests through the system for debugging and observability.

use std::collections::BTreeMap;
use std::fmt;

/// Unique identifier for a request
///
/// Thread-safe and copyable for easy propagation through the system.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(String);

impl RequestId {
    /// Generate a new unique request ID
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_nanos());
        Self(format!("req-{}", nanos))
    }

    /// Create a RequestId from a string
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the underlying string value as owned
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// Correlation context for tracking requests
///
/// Provides an immutable map of key-value pairs for request metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationContext {
    request_id: RequestId,
    metadata: BTreeMap<String, String>,
}

impl CorrelationContext {
    /// Create a new correlation context with a generated request ID
    pub fn new() -> Self {
        Self {
            request_id: RequestId::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Create a correlation context with a specific request ID
    pub fn with_request_id(request_id: RequestId) -> Self {
        Self {
            request_id,
            metadata: BTreeMap::new(),
        }
    }

    /// Get the request ID
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Get metadata value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Add metadata to the context
    ///
    /// Returns a new context with the metadata added.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add multiple metadata entries to the context
    ///
    /// Returns a new context with the metadata added.
    pub fn with_metadata_iter(
        mut self,
        iter: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (key, value) in iter {
            self.metadata.insert(key.into(), value.into());
        }
        self
    }

    /// Get all metadata as a reference
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }

    /// Create a child context with inherited metadata
    ///
    /// Useful for creating sub-requests that should inherit
    /// correlation data from their parent.
    pub fn child_context(&self) -> Self {
        Self {
            request_id: RequestId::new(),
            metadata: self.metadata.clone(),
        }
    }

    /// Format the context for logging
    pub fn format_log(&self) -> String {
        let mut parts = vec![format!("request_id={}", self.request_id.as_str())];

        for (key, value) in self.metadata.iter().take(5) {
            parts.push(format!("{}={}", key, value));
        }

        if self.metadata.len() > 5 {
            parts.push(format!("(+{} more)", self.metadata.len() - 5));
        }

        parts.join(" ")
    }
}

impl Default for CorrelationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CorrelationContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.format_log())
    }
}

/// Common correlation metadata keys
pub mod keys {
    /// Source component making the request
    pub const SOURCE: &str = "source";

    /// Target component or API endpoint
    pub const TARGET: &str = "target";

    /// HTTP method (GET, POST, etc.)
    pub const METHOD: &str = "method";

    /// Request path or URL
    pub const PATH: &str = "path";

    /// User or actor identifier
    pub const USER_ID: &str = "user_id";

    /// Session identifier
    pub const SESSION_ID: &str = "session_id";

    /// Workflow or bead identifier
    pub const WORKFLOW_ID: &str = "workflow_id";

    /// Bead identifier
    pub const BEAD_ID: &str = "bead_id";

    /// Parent request ID for tracing request chains
    pub const PARENT_REQUEST_ID: &str = "parent_request_id";

    /// Request start timestamp
    pub const START_TIME: &str = "start_time";

    /// Request duration in milliseconds
    pub const DURATION_MS: &str = "duration_ms";

    /// HTTP status code
    pub const STATUS_CODE: &str = "status_code";

    /// Error message (if request failed)
    pub const ERROR: &str = "error";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();

        assert_ne!(id1, id2);
        assert!(id1.as_str().starts_with("req-"));
        assert!(id2.as_str().starts_with("req-"));
    }

    #[test]
    fn test_request_id_from_string() {
        let id = RequestId::from_string("custom-id-123");
        assert_eq!(id.as_str(), "custom-id-123");
        assert_eq!(id.into_string(), "custom-id-123");
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::from_string("test-123");
        assert_eq!(format!("{}", id), "test-123");
        assert_eq!(id.to_string(), "test-123");
    }

    #[test]
    fn test_correlation_context_new() {
        let ctx = CorrelationContext::new();

        assert!(ctx.request_id().as_str().starts_with("req-"));
        assert_eq!(ctx.metadata().len(), 0);
    }

    #[test]
    fn test_correlation_context_with_request_id() {
        let req_id = RequestId::from_string("specific-id");
        let ctx = CorrelationContext::with_request_id(req_id.clone());

        assert_eq!(ctx.request_id(), &req_id);
        assert_eq!(ctx.metadata().len(), 0);
    }

    #[test]
    fn test_correlation_context_with_metadata() {
        let ctx = CorrelationContext::new()
            .with_metadata("source", "ui")
            .with_metadata("path", "/api/agents");

        assert_eq!(ctx.get("source"), Some(&"ui".to_string()));
        assert_eq!(ctx.get("path"), Some(&"/api/agents".to_string()));
        assert_eq!(ctx.metadata().len(), 2);
    }

    #[test]
    fn test_correlation_context_with_metadata_iter() {
        let metadata = vec![
            ("source", "worker"),
            ("target", "orchestrator"),
            ("action", "assign_bead"),
        ];

        let ctx = CorrelationContext::new().with_metadata_iter(metadata);

        assert_eq!(ctx.get("source"), Some(&"worker".to_string()));
        assert_eq!(ctx.get("target"), Some(&"orchestrator".to_string()));
        assert_eq!(ctx.get("action"), Some(&"assign_bead".to_string()));
        assert_eq!(ctx.metadata().len(), 3);
    }

    #[test]
    fn test_correlation_context_get_nonexistent() {
        let ctx = CorrelationContext::new().with_metadata("existing", "value");

        assert_eq!(ctx.get("existing"), Some(&"value".to_string()));
        assert_eq!(ctx.get("nonexistent"), None);
    }

    #[test]
    fn test_correlation_context_child_context() {
        let parent = CorrelationContext::new()
            .with_metadata("source", "ui")
            .with_metadata("user_id", "user-123");

        let child = parent.child_context();

        assert_ne!(child.request_id(), parent.request_id());
        assert_eq!(child.get("source"), Some(&"ui".to_string()));
        assert_eq!(child.get("user_id"), Some(&"user-123".to_string()));
    }

    #[test]
    fn test_correlation_context_child_context_independent() {
        let parent = CorrelationContext::new().with_metadata("source", "ui");

        let child = parent.child_context().with_metadata("action", "get_agents");

        assert_eq!(parent.get("action"), None);
        assert_eq!(child.get("action"), Some(&"get_agents".to_string()));
        assert_eq!(parent.get("source"), Some(&"ui".to_string()));
        assert_eq!(child.get("source"), Some(&"ui".to_string()));
    }

    #[test]
    fn test_correlation_context_format_log() {
        let ctx = CorrelationContext::new()
            .with_metadata("source", "ui")
            .with_metadata("path", "/api/agents");

        let formatted = ctx.format_log();

        assert!(formatted.contains("request_id="));
        assert!(formatted.contains("source=ui"));
        assert!(formatted.contains("path=/api/agents"));
    }

    #[test]
    fn test_correlation_context_format_log_many_fields() {
        let mut ctx = CorrelationContext::new();
        for i in 0..10 {
            ctx = ctx.with_metadata(format!("key{}", i), format!("value{}", i));
        }

        let formatted = ctx.format_log();

        assert!(formatted.contains("request_id="));
        assert!(formatted.contains("(+5 more)"));
    }

    #[test]
    fn test_correlation_context_display() {
        let ctx = CorrelationContext::new().with_metadata("source", "test");

        let display = format!("{}", ctx);

        assert!(display.starts_with("["));
        assert!(display.ends_with("]"));
        assert!(display.contains("request_id="));
        assert!(display.contains("source=test"));
    }

    #[test]
    fn test_correlation_context_default() {
        let ctx = CorrelationContext::default();

        assert!(ctx.request_id().as_str().starts_with("req-"));
        assert_eq!(ctx.metadata().len(), 0);
    }

    #[test]
    fn test_correlation_context_clone() {
        let ctx1 = CorrelationContext::new()
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        let ctx2 = ctx1.clone();

        assert_eq!(ctx1, ctx2);
        assert_eq!(ctx2.get("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx2.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_correlation_context_equality() {
        let req_id = RequestId::from_string("same-id");
        let ctx1 =
            CorrelationContext::with_request_id(req_id.clone()).with_metadata("key", "value");

        let ctx2 = CorrelationContext::with_request_id(req_id).with_metadata("key", "value");

        assert_eq!(ctx1, ctx2);
    }

    #[test]
    fn test_common_keys() {
        assert_eq!(keys::SOURCE, "source");
        assert_eq!(keys::TARGET, "target");
        assert_eq!(keys::METHOD, "method");
        assert_eq!(keys::PATH, "path");
        assert_eq!(keys::USER_ID, "user_id");
        assert_eq!(keys::SESSION_ID, "session_id");
        assert_eq!(keys::WORKFLOW_ID, "workflow_id");
        assert_eq!(keys::BEAD_ID, "bead_id");
        assert_eq!(keys::PARENT_REQUEST_ID, "parent_request_id");
        assert_eq!(keys::START_TIME, "start_time");
        assert_eq!(keys::DURATION_MS, "duration_ms");
        assert_eq!(keys::STATUS_CODE, "status_code");
        assert_eq!(keys::ERROR, "error");
    }

    #[test]
    fn test_request_id_default() {
        let id1 = RequestId::default();
        let id2 = RequestId::default();

        assert_ne!(id1, id2);
    }
}
