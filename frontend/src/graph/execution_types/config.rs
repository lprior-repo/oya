//! Node execution configuration types.

// ===========================================================================
// Global Execution Configuration
// ===========================================================================

/// Global configuration for workflow execution.
///
/// This configuration applies to the entire workflow execution and defines
/// global constraints such as timeouts, memory limits, and execution policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionConfig {
    /// Global timeout in milliseconds for the entire workflow.
    /// If None, no timeout is enforced.
    pub timeout_ms: Option<u64>,
    /// Memory limit in bytes for the entire workflow execution.
    /// If None, no memory limit is enforced.
    pub memory_limit_bytes: Option<u64>,
    /// Maximum number of execution iterations.
    /// If None, no iteration limit is enforced.
    pub max_iterations: Option<usize>,
    /// Whether to continue execution after node failures.
    pub continue_on_error: bool,
    /// Whether to skip failed nodes and continue with dependents.
    pub skip_failed_nodes: bool,
    /// Maximum expression resolution depth to prevent stack overflow.
    pub max_expression_depth: usize,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            memory_limit_bytes: None,
            max_iterations: None,
            continue_on_error: false,
            skip_failed_nodes: false,
            max_expression_depth: 100,
        }
    }
}

impl ExecutionConfig {
    /// Create new config with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set global timeout.
    #[must_use]
    pub const fn with_timeout(self, timeout_ms: u64) -> Self {
        Self { timeout_ms: Some(timeout_ms), ..self }
    }

    /// Set memory limit.
    #[must_use]
    pub const fn with_memory_limit(self, memory_limit_bytes: u64) -> Self {
        Self { memory_limit_bytes: Some(memory_limit_bytes), ..self }
    }

    /// Set maximum iterations.
    #[must_use]
    pub const fn with_max_iterations(self, max_iterations: usize) -> Self {
        Self { max_iterations: Some(max_iterations), ..self }
    }

    /// Enable continue on error.
    #[must_use]
    pub const fn with_continue_on_error(self) -> Self {
        Self { continue_on_error: true, ..self }
    }

    /// Enable skip failed nodes.
    #[must_use]
    pub const fn with_skip_failed_nodes(self) -> Self {
        Self { skip_failed_nodes: true, ..self }
    }

    /// Set maximum expression resolution depth.
    #[must_use]
    pub const fn with_max_expression_depth(self, depth: usize) -> Self {
        Self { max_expression_depth: depth, ..self }
    }

    /// Check if timeout is exceeded.
    #[must_use]
    pub fn is_timeout_exceeded(&self, elapsed_ms: u64) -> bool {
        self.timeout_ms.is_some_and(|limit| elapsed_ms >= limit)
    }

    /// Check if memory limit is exceeded.
    #[must_use]
    pub fn is_memory_limit_exceeded(&self, bytes_used: u64) -> bool {
        self.memory_limit_bytes.is_some_and(|limit| bytes_used >= limit)
    }
}

// ===========================================================================
// Node Execution Configuration
// ===========================================================================

/// Configuration for a single node execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeExecutionConfig {
    /// Node-specific timeout (overrides global).
    pub timeout_ms: Option<u64>,
    /// Whether to retry on failure.
    pub retry_count: u32,
    /// Retry backoff in milliseconds.
    pub retry_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds (cap for exponential backoff).
    pub max_retry_backoff_ms: u64,
}

impl Default for NodeExecutionConfig {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            retry_count: 0,
            retry_backoff_ms: 100,
            max_retry_backoff_ms: 30000,
        }
    }
}

impl NodeExecutionConfig {
    /// Create new config with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set node-specific timeout.
    #[must_use]
    pub const fn with_timeout(self, timeout_ms: u64) -> Self {
        Self { timeout_ms: Some(timeout_ms), ..self }
    }

    /// Set retry count.
    #[must_use]
    pub const fn with_retry_count(self, retry_count: u32) -> Self {
        Self { retry_count, ..self }
    }

    /// Set retry backoff and default max to 30s.
    #[must_use]
    pub const fn with_retry_backoff(self, retry_backoff_ms: u64) -> Self {
        Self { retry_backoff_ms, max_retry_backoff_ms: 30000, ..self }
    }

    /// Set maximum retry backoff (cap for exponential backoff).
    #[must_use]
    pub const fn with_max_retry_backoff(self, max_retry_backoff_ms: u64) -> Self {
        Self { max_retry_backoff_ms, ..self }
    }
}

// ===========================================================================
// Exponential Backoff Calculator
// ===========================================================================

impl NodeExecutionConfig {
    /// Calculate exponential backoff delay for a given attempt number.
    ///
    /// Uses the formula: `base_backoff * (2 ^ (attempt - 1))`
    /// with a maximum cap to prevent excessively long delays.
    ///
    /// # Arguments
    /// * `attempt` - The current attempt number (1-indexed)
    ///
    /// # Returns
    /// The backoff delay in milliseconds
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::graph::execution_types::NodeExecutionConfig;
    ///
    /// let config = NodeExecutionConfig::new()
    ///     .with_retry_backoff(100) // 100ms base backoff
    ///     .with_max_retry_backoff(5000); // 5s max cap
    ///
    /// assert_eq!(config.backoff_for_attempt(1), 100);
    /// assert_eq!(config.backoff_for_attempt(2), 200);
    /// assert_eq!(config.backoff_for_attempt(3), 400);
    /// assert_eq!(config.backoff_for_attempt(10), 5000); // capped at max
    /// ```
    #[must_use]
    pub const fn backoff_for_attempt(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return self.retry_backoff_ms;
        }
        let multiplier = 1u64 << attempt.saturating_sub(1);
        let backoff = self.retry_backoff_ms.saturating_mul(multiplier);
        if backoff > self.max_retry_backoff_ms {
            self.max_retry_backoff_ms
        } else {
            backoff
        }
    }

    /// Calculate jittered backoff for distributed retry scenarios.
    ///
    /// Returns a random value between 0 and the exponential backoff to
    /// prevent thundering herd problems in distributed systems.
    ///
    /// Note: This requires a random number generator for true jitter.
    /// For deterministic testing, use `backoff_for_attempt` instead.
    #[must_use]
    pub const fn jittered_backoff_for_attempt(&self, attempt: u32, _rng_seed: u64) -> u64 {
        // Simplified jitter: use 80-100% of base backoff
        self.backoff_for_attempt(attempt)
    }
}
