//! Phase handler trait and implementations.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::types::{PhaseContext, PhaseOutput};

/// Trait for phase execution handlers.
///
/// Each phase in a workflow is executed by a handler that implements
/// this trait. Handlers are responsible for:
/// - Executing the phase logic
/// - Optionally rolling back on failure
/// - Providing checkpoint data for recovery
#[async_trait]
pub trait PhaseHandler: Send + Sync {
    /// Execute the phase.
    ///
    /// # Arguments
    /// * `ctx` - The execution context including inputs and metadata
    ///
    /// # Returns
    /// The phase output on success, or an error on failure.
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput>;

    /// Roll back the phase.
    ///
    /// Called when a subsequent phase fails and rollback is requested.
    /// Default implementation does nothing.
    ///
    /// # Arguments
    /// * `ctx` - The execution context
    async fn rollback(&self, ctx: &PhaseContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Get checkpoint data for this phase.
    ///
    /// Called after successful execution to capture state that can
    /// be used for recovery. Default returns None.
    fn checkpoint_data(&self) -> Option<Vec<u8>> {
        None
    }

    /// Get the handler name (for logging/debugging).
    fn name(&self) -> &str;

    /// Check if this handler can handle the given phase.
    fn can_handle(&self, phase_name: &str) -> bool {
        self.name() == phase_name
    }
}

/// Registry of phase handlers.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, Arc<dyn PhaseHandler>>,
}

impl HandlerRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for a phase name.
    pub fn register(&mut self, name: impl Into<String>, handler: Arc<dyn PhaseHandler>) {
        self.handlers.insert(name.into(), handler);
    }

    /// Get a handler by phase name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn PhaseHandler>> {
        self.handlers.get(name).cloned()
    }

    /// Check if a handler exists for the given phase name.
    pub fn has(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// Get all registered handler names.
    pub fn names(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

/// A no-op handler for testing and placeholder phases.
pub struct NoOpHandler {
    name: String,
}

impl NoOpHandler {
    /// Create a new no-op handler with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl PhaseHandler for NoOpHandler {
    async fn execute(&self, _ctx: &PhaseContext) -> Result<PhaseOutput> {
        Ok(PhaseOutput::success(Vec::new()).with_message("No-op completed"))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A handler that always fails (for testing).
pub struct FailingHandler {
    name: String,
    error_message: String,
}

impl FailingHandler {
    /// Create a new failing handler.
    pub fn new(name: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            error_message: error_message.into(),
        }
    }
}

#[async_trait]
impl PhaseHandler for FailingHandler {
    async fn execute(&self, _ctx: &PhaseContext) -> Result<PhaseOutput> {
        Err(Error::phase_failed(&self.name, &self.error_message))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A handler that runs a closure.
pub struct FnHandler<F>
where
    F: Fn(&PhaseContext) -> Result<PhaseOutput> + Send + Sync,
{
    name: String,
    func: F,
}

impl<F> FnHandler<F>
where
    F: Fn(&PhaseContext) -> Result<PhaseOutput> + Send + Sync,
{
    /// Create a new function handler.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

#[async_trait]
impl<F> PhaseHandler for FnHandler<F>
where
    F: Fn(&PhaseContext) -> Result<PhaseOutput> + Send + Sync,
{
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        (self.func)(ctx)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A handler that delegates to an async function.
pub struct AsyncFnHandler<F, Fut>
where
    F: Fn(PhaseContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<PhaseOutput>> + Send,
{
    name: String,
    func: F,
}

impl<F, Fut> AsyncFnHandler<F, Fut>
where
    F: Fn(PhaseContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<PhaseOutput>> + Send,
{
    /// Create a new async function handler.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

#[async_trait]
impl<F, Fut> PhaseHandler for AsyncFnHandler<F, Fut>
where
    F: Fn(PhaseContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<PhaseOutput>> + Send,
{
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        (self.func)(ctx.clone()).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Composable handler that runs handlers in sequence.
pub struct ChainHandler {
    name: String,
    handlers: Vec<Arc<dyn PhaseHandler>>,
}

impl ChainHandler {
    /// Create a new chain handler.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            handlers: Vec::new(),
        }
    }

    /// Add a handler to the chain.
    pub fn then(mut self, handler: Arc<dyn PhaseHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
}

#[async_trait]
impl PhaseHandler for ChainHandler {
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        let mut final_output = PhaseOutput::success(Vec::new());

        for handler in &self.handlers {
            let output = handler.execute(ctx).await?;
            if !output.success {
                return Ok(output);
            }
            // Combine artifacts
            final_output.artifacts.extend(output.artifacts);
            final_output.duration_ms += output.duration_ms;
            // Use last output data
            if !output.data.is_empty() {
                final_output.data = output.data;
            }
            if output.message.is_some() {
                final_output.message = output.message;
            }
        }

        Ok(final_output)
    }

    async fn rollback(&self, ctx: &PhaseContext) -> Result<()> {
        // Rollback in reverse order
        for handler in self.handlers.iter().rev() {
            handler.rollback(ctx).await?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Phase, WorkflowId};

    fn make_context() -> PhaseContext {
        PhaseContext::new(WorkflowId::new(), Phase::new("test"))
    }

    #[tokio::test]
    async fn test_noop_handler() {
        let handler = NoOpHandler::new("test");
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_ok());
        assert!(result.map(|r| r.success).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_failing_handler() {
        let handler = FailingHandler::new("fail", "expected failure");
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fn_handler() {
        let handler = FnHandler::new("double", |_ctx| {
            Ok(PhaseOutput::success(vec![2, 4, 6]))
        });
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.map(|r| r.data).unwrap_or_default(), vec![2, 4, 6]);
    }

    #[tokio::test]
    async fn test_registry() {
        let mut registry = HandlerRegistry::new();
        registry.register("build", Arc::new(NoOpHandler::new("build")));
        registry.register("test", Arc::new(NoOpHandler::new("test")));

        assert!(registry.has("build"));
        assert!(registry.has("test"));
        assert!(!registry.has("deploy"));
        assert_eq!(registry.len(), 2);
    }

    #[tokio::test]
    async fn test_chain_handler() {
        let chain = ChainHandler::new("build-test")
            .then(Arc::new(NoOpHandler::new("build")))
            .then(Arc::new(NoOpHandler::new("test")));

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_chain_stops_on_failure() {
        let chain = ChainHandler::new("build-fail-test")
            .then(Arc::new(NoOpHandler::new("build")))
            .then(Arc::new(FailingHandler::new("fail", "stop here")))
            .then(Arc::new(NoOpHandler::new("test")));

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_err());
    }
}
