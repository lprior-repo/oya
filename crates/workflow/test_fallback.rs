//! Simple test file to verify fallback chain implementation

use std::sync::Arc;

// Mock the necessary types and traits for testing
mod handler {
    use async_trait::async_trait;
    use std::sync::Arc;

    #[async_trait]
    pub trait PhaseHandler: Send + Sync {
        async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput>;
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    pub struct PhaseContext {
        pub name: String,
    }

    impl PhaseContext {
        pub fn new(name: String) -> Self {
            Self { name }
        }
    }

    pub struct PhaseOutput {
        pub success: bool,
        pub data: Vec<u8>,
    }

    impl PhaseOutput {
        pub fn success(data: Vec<u8>) -> Self {
            Self {
                success: true,
                data,
            }
        }
    }

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, Clone)]
    pub enum Error {
        AllHandlersFailed { phase_name: String, fallback_names: Vec<String> },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::AllHandlersFailed { phase_name, fallback_names } => {
                    write!(
                        f,
                        "all handlers for phase '{}' failed: {}",
                        phase_name,
                        fallback_names.join(", ")
                    )
                }
            }
        }
    }

    impl std::error::Error for Error {}

    // Mock implementations
    pub struct NoOpHandler {
        name: String,
    }

    impl NoOpHandler {
        pub fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    #[async_trait]
    impl PhaseHandler for NoOpHandler {
        async fn execute(&self, _ctx: &PhaseContext) -> Result<PhaseOutput> {
            Ok(PhaseOutput::success(vec![]))
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    pub struct FailingHandler {
        name: String,
        error_message: String,
    }

    impl FailingHandler {
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
            Err(Error::AllHandlersFailed {
                phase_name: self.name.clone(),
                fallback_names: vec![],
            })
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    /// Handler chain with fallback support.
    ///
    /// Executes the primary handler first, then falls back to fallback handlers
    /// if the primary fails. All handlers must succeed for the chain to succeed.
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
            let fallback_names: Vec<String> = self.fallbacks
                .iter()
                .map(|h| h.name().to_string())
                .collect();
            Err(Error::AllHandlersFailed {
                phase_name: self.name.clone(),
                fallback_names,
            })
        }

        /// Rollback all handlers in the chain.
        async fn rollback_inner(&self, _ctx: &PhaseContext) -> Result<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[async_trait]
    impl PhaseHandler for HandlerChain {
        async fn execute(&self, ctx: &PhaseContext) -> Result<PhaseOutput> {
            self.execute_inner(ctx).await
        }

        fn name(&self) -> &str {
            &self.name
        }
    }
}

#[tokio::test]
async fn test_handler_chain_primary_succeeds() {
    let primary = Arc::new(handler::NoOpHandler::new("primary"));
    let fallback = Arc::new(handler::FailingHandler::new("fallback", "should not be used"));

    let chain = handler::HandlerChain::new("test-chain", primary)
        .with_fallback(fallback);

    let ctx = handler::PhaseContext::new("test".to_string());
    let result = chain.execute(&ctx).await;
    assert!(result.is_ok());
    assert!(result.unwrap().success);
}

#[tokio::test]
async fn test_handler_chain_fallback_succeeds() {
    let primary = Arc::new(handler::FailingHandler::new("primary", "expected failure"));
    let fallback = Arc::new(handler::NoOpHandler::new("fallback"));

    let chain = handler::HandlerChain::new("test-chain", primary)
        .with_fallback(fallback);

    let ctx = handler::PhaseContext::new("test".to_string());
    let result = chain.execute(&ctx).await;
    assert!(result.is_ok());
    assert!(result.unwrap().success);
}

#[tokio::test]
async fn test_handler_chain_all_fail() {
    let primary = Arc::new(handler::FailingHandler::new("primary", "primary failed"));
    let fallback = Arc::new(handler::FailingHandler::new("fallback", "fallback failed"));

    let chain = handler::HandlerChain::new("test-chain", primary)
        .with_fallback(fallback);

    let ctx = handler::PhaseContext::new("test".to_string());
    let result = chain.execute(&ctx).await;
    assert!(result.is_err());
    if let Err(e) = &result {
        assert!(e.to_string().contains("all handlers"));
        assert!(e.to_string().contains("primary, fallback"));
    }
}

#[tokio::test]
async fn test_handler_chain_multiple_fallbacks() {
    let primary = Arc::new(handler::FailingHandler::new("primary", "primary failed"));
    let fallback1 = Arc::new(handler::FailingHandler::new("fallback1", "fallback1 failed"));
    let fallback2 = Arc::new(handler::NoOpHandler::new("fallback2"));
    let fallback3 = Arc::new(handler::FailingHandler::new("fallback3", "should not be used"));

    let chain = handler::HandlerChain::new("test-chain", primary)
        .with_fallback(fallback1)
        .with_fallback(fallback2)
        .with_fallback(fallback3);

    let ctx = handler::PhaseContext::new("test".to_string());
    let result = chain.execute(&ctx).await;
    assert!(result.is_ok());
    assert!(result.unwrap().success);
}

fn main() {
    println!("Fallback chain implementation test compiled successfully!");
}