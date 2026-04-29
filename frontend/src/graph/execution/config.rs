use super::super::execution_types::ExecutionConfig;
use super::super::Workflow;

// ===========================================================================
// Execution Configuration Management
// ===========================================================================

impl Workflow {
    /// Sets the execution configuration for this workflow.
    ///
    /// This method allows configuring memory limits, timeouts, and other
    /// runtime constraints before execution.
    ///
    /// # Arguments
    /// * `config` - The execution configuration to apply
    ///
    /// # Returns
    /// The workflow with the new configuration applied
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::graph::{Workflow, execution_types::ExecutionConfig};
    ///
    /// let workflow = Workflow::new().with_execution_config(
    ///     ExecutionConfig::new().with_memory_limit(1024 * 1024) // 1MB limit
    /// );
    /// assert_eq!(workflow.execution_config.memory_limit_bytes, Some(1024 * 1024));
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_execution_config(mut self, config: ExecutionConfig) -> Self {
        self.execution_config = config;
        self
    }

    /// Sets a memory limit for this workflow execution.
    ///
    /// When the total memory usage of all node outputs exceeds this limit,
    /// execution will stop and the workflow will be marked as failed.
    ///
    /// # Arguments
    /// * `memory_limit_bytes` - Maximum memory allowed in bytes
    ///
    /// # Returns
    /// The workflow with the memory limit configured
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::graph::Workflow;
    ///
    /// let workflow = Workflow::new().with_memory_limit(1024 * 1024); // 1MB
    /// assert_eq!(workflow.execution_config.memory_limit_bytes, Some(1024 * 1024));
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // uses RwLockWriteGuard which is not const-safe
    pub fn with_memory_limit(mut self, memory_limit_bytes: u64) -> Self {
        self.execution_config = self.execution_config.with_memory_limit(memory_limit_bytes);
        self
    }
}
