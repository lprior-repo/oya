//! Phase handler trait and implementations.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

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
    fallback_chains: HashMap<String, HandlerChain>,
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

    /// Register a fallback chain for a phase name.
    ///
    /// If the primary handler fails, fallback handlers will be tried in order.
    pub fn register_fallback_chain(
        &mut self,
        name: impl Into<String>,
        primary: Arc<dyn PhaseHandler>,
        fallbacks: Vec<Arc<dyn PhaseHandler>>,
    ) {
        let name_string = name.into();
        let mut chain = HandlerChain::new(name_string.clone(), primary.clone());
        for fallback in fallbacks {
            chain = chain.with_fallback(fallback);
        }
        self.fallback_chains.insert(name_string.clone(), chain);
        // Also register primary as individual handler
        self.handlers.insert(name_string, primary);
    }

    /// Get a handler by phase name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn PhaseHandler>> {
        // First check for fallback chain
        if let Some(chain) = self.fallback_chains.get(name) {
            return Some(Arc::new(chain.clone()));
        }
        // Fall back to regular handler
        self.handlers.get(name).cloned()
    }

    /// Check if a handler exists for the given phase name.
    pub fn has(&self, name: &str) -> bool {
        self.handlers.contains_key(name) || self.fallback_chains.contains_key(name)
    }

    /// Get all registered handler names.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.handlers.keys().map(|s| s.as_str()).collect();
        let chain_names: Vec<&str> = self.fallback_chains.keys().map(|s| s.as_str()).collect();
        names.extend(chain_names);
        names
    }

    /// Get all registered handler keys.
    pub fn keys(&self) -> Vec<&String> {
        let mut keys: Vec<&String> = self.handlers.keys().collect();
        let chain_keys: Vec<&String> = self.fallback_chains.keys().collect();
        keys.extend(chain_keys);
        keys
    }

    /// Get the number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len() + self.fallback_chains.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty() && self.fallback_chains.is_empty()
    }

    /// Get all fallback chains.
    pub fn fallback_chains(&self) -> &HashMap<String, HandlerChain> {
        &self.fallback_chains
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

/// Handler chain with fallback support.
///
/// Executes the primary handler first, then falls back to fallback handlers
/// if the primary fails. All handlers must succeed for the chain to succeed.
#[derive(Clone)]
pub struct HandlerChain {
    name: String,
    primary: Arc<dyn PhaseHandler>,
    fallbacks: Vec<Arc<dyn PhaseHandler>>,
}

impl HandlerChain {
    /// Create a new handler chain.
    pub fn new(name: impl Into<String>, primary: Arc<dyn PhaseHandler>) -> Self {
        Self {
            name: name.into(),
            primary,
            fallbacks: Vec::new(),
        }
    }

    /// Add a fallback handler to the chain.
    pub fn with_fallback(mut self, fallback: Arc<dyn PhaseHandler>) -> Self {
        self.fallbacks.push(fallback);
        self
    }

    /// Add multiple fallback handlers to the chain.
    pub fn with_fallbacks(mut self, fallbacks: Vec<Arc<dyn PhaseHandler>>) -> Self {
        self.fallbacks.extend(fallbacks);
        self
    }

    /// Execute the handler chain.
    ///
    /// Tries the primary handler first, then falls back to fallback handlers
    /// in order until one succeeds or all fail.
    pub async fn execute_inner(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        // Try primary handler
        if let Ok(output) = self.primary.execute(ctx).await {
            return Ok(output);
        }

        // Try fallback handlers in order
        for fallback in &self.fallbacks {
            if let Ok(output) = fallback.execute(ctx).await {
                return Ok(output);
            }
        }

        // All handlers failed
        let fallback_names: Vec<String> = self
            .fallbacks
            .iter()
            .map(|h| h.name().to_string())
            .collect();
        Err(Error::all_handlers_failed(&self.name, fallback_names))
    }

    /// Rollback all handlers in the chain.
    async fn rollback_inner(&self, ctx: &PhaseContext) -> Result<()> {
        // Try to rollback primary handler
        if let Err(e) = self.primary.rollback(ctx).await {
            warn!(
                handler = %self.name,
                error = %e,
                "Primary handler rollback failed"
            );
        }

        // Try to rollback fallback handlers
        for fallback in &self.fallbacks {
            if let Err(e) = fallback.rollback(ctx).await {
                warn!(
                    handler = %fallback.name(),
                    error = %e,
                    "Fallback handler rollback failed"
                );
            }
        }

        Ok(())
    }
}

#[async_trait]
impl PhaseHandler for HandlerChain {
    async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
        self.execute_inner(ctx).await
    }

    async fn rollback(&self, ctx: &PhaseContext) -> Result<()> {
        self.rollback_inner(ctx).await
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
        let handler = FnHandler::new("double", |_ctx| Ok(PhaseOutput::success(vec![2, 4, 6])));
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_ok());
        assert_eq!(*result.map(|r| r.data).unwrap_or_default(), vec![2, 4, 6]);
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

    #[tokio::test]
    async fn test_handler_chain_primary_succeeds() {
        let primary = Arc::new(NoOpHandler::new("primary"));
        let fallback = Arc::new(FailingHandler::new("fallback", "should not be used"));

        let chain = HandlerChain::new("test-chain", primary).with_fallback(fallback);

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_ok());
        assert!(result.map(|r| r.success).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_handler_chain_fallback_succeeds() {
        let primary = Arc::new(FailingHandler::new("primary", "expected failure"));
        let fallback = Arc::new(NoOpHandler::new("fallback"));

        let chain = HandlerChain::new("test-chain", primary).with_fallback(fallback);

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_ok());
        assert!(result.map(|r| r.success).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_handler_chain_all_fail() {
        let primary = Arc::new(FailingHandler::new("primary", "primary failed"));
        let fallback = Arc::new(FailingHandler::new("fallback", "fallback failed"));

        let chain = HandlerChain::new("test-chain", primary).with_fallback(fallback);

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_err());
        if let Err(e) = &result {
            assert!(e.to_string().contains("all handlers"));
            assert!(e.to_string().contains("primary, fallback"));
        }
    }

    #[tokio::test]
    async fn test_handler_chain_multiple_fallbacks() {
        let primary = Arc::new(FailingHandler::new("primary", "primary failed"));
        let fallback1 = Arc::new(FailingHandler::new("fallback1", "fallback1 failed"));
        let fallback2 = Arc::new(NoOpHandler::new("fallback2"));
        let fallback3 = Arc::new(FailingHandler::new("fallback3", "should not be used"));

        let chain = HandlerChain::new("test-chain", primary)
            .with_fallback(fallback1)
            .with_fallback(fallback2)
            .with_fallback(fallback3);

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_ok());
        assert!(result.map(|r| r.success).unwrap_or(false));
    }

    #[tokio::test]
    async fn test_handler_registry_fallback_chain() {
        let mut registry = HandlerRegistry::new();
        let primary = Arc::new(NoOpHandler::new("primary"));
        let fallback = Arc::new(FailingHandler::new("fallback", "should not be used"));

        registry.register_fallback_chain("test", primary, vec![fallback]);

        assert!(registry.has("test"));
        let handler = registry.get("test").unwrap();
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handler_registry_fallback_chain_failure() {
        let mut registry = HandlerRegistry::new();
        let primary = Arc::new(FailingHandler::new("primary", "failed"));
        let fallback = Arc::new(FailingHandler::new("fallback", "also failed"));

        registry.register_fallback_chain("test", primary, vec![fallback]);

        assert!(registry.has("test"));
        let handler = registry.get("test").unwrap();
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handler_registry_fallback_chain_multiple() {
        let mut registry = HandlerRegistry::new();
        let primary = Arc::new(FailingHandler::new("primary", "failed"));
        let fallback1 = Arc::new(FailingHandler::new("fallback1", "failed"));
        let fallback2 = Arc::new(NoOpHandler::new("fallback2"));

        registry.register_fallback_chain("test", primary, vec![fallback1, fallback2]);

        assert!(registry.has("test"));
        let handler = registry.get("test").unwrap();
        let ctx = make_context();
        let result = handler.execute(&ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handler_chain_rollback() {
        let primary = Arc::new(FailingHandler::new("primary", "failed"));
        let fallback = Arc::new(NoOpHandler::new("fallback"));

        let chain = HandlerChain::new("test-chain", primary).with_fallback(fallback);

        let ctx = make_context();
        // Execute should succeed with fallback
        let result = chain.execute(&ctx).await;
        assert!(result.is_ok());

        // Test rollback
        let rollback_result = chain.rollback(&ctx).await;
        assert!(rollback_result.is_ok());
    }

    #[tokio::test]
    async fn test_handler_chain_convenience_method() {
        let primary = Arc::new(FailingHandler::new("primary", "failed"));
        let fallback1 = Arc::new(FailingHandler::new("fallback1", "failed"));
        let fallback2 = Arc::new(NoOpHandler::new("fallback2"));

        let fallbacks: Vec<Arc<dyn PhaseHandler>> = vec![fallback1, fallback2];
        let chain = HandlerChain::new("test-chain", primary).with_fallbacks(fallbacks);

        let ctx = make_context();
        let result = chain.execute(&ctx).await;
        assert!(result.is_ok());
        assert!(result.is_ok_and(|r| r.success));
    }
}
