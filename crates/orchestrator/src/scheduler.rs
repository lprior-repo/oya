//! Scheduler for dispatching ready beads to queues.
//!
//! This module implements the scheduler that:
//! - Queries ready beads from state
//! - Dispatches to appropriate queue based on strategy
//! - Maintains <50ms dispatch latency
//! - Ensures exactly-once dispatch semantics

use oya_core::{Error, Result};
use oya_events::BeadId;
use std::collections::HashSet;

/// Queue selection strategy for dispatching ready beads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStrategy {
    /// First-in-first-out queue.
    Fifo,
    /// Last-in-first-out queue (depth-first).
    Lifo,
    /// Round-robin across tenants.
    RoundRobin,
    /// Priority-based queue.
    Priority,
}

impl QueueStrategy {
    /// Get the queue name for this strategy.
    pub fn queue_name(&self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::Lifo => "lifo",
            Self::RoundRobin => "roundrobin",
            Self::Priority => "priority",
        }
    }
}

/// Dispatcher for routing beads to queues.
#[derive(Debug)]
pub struct Dispatcher {
    /// Strategy for queue selection.
    strategy: QueueStrategy,
    /// Tenant ID for round-robin routing (if applicable).
    tenant_id: Option<String>,
    /// Dispatched beads (for exactly-once semantics).
    dispatched: HashSet<BeadId>,
}

impl Dispatcher {
    /// Create a new dispatcher with the given strategy.
    pub fn new(strategy: QueueStrategy) -> Self {
        Self {
            strategy,
            tenant_id: None,
            dispatched: HashSet::new(),
        }
    }

    /// Create a new dispatcher with round-robin strategy and tenant ID.
    pub fn with_tenant(tenant_id: String) -> Self {
        Self {
            strategy: QueueStrategy::RoundRobin,
            tenant_id: Some(tenant_id),
            dispatched: HashSet::new(),
        }
    }

    /// Set the queue strategy.
    pub fn set_strategy(&mut self, strategy: QueueStrategy) {
        self.strategy = strategy;
    }

    /// Set the tenant ID for round-robin routing.
    pub fn set_tenant_id(&mut self, tenant_id: Option<String>) {
        self.tenant_id = tenant_id;
    }

    /// Dispatch a ready bead to the appropriate queue.
    ///
    /// Returns the queue name the bead was dispatched to, or an error if:
    /// - Bead was already dispatched
    /// - Round-robin strategy requires tenant_id but none was set
    pub fn dispatch(&mut self, bead_id: BeadId) -> Result<DispatchResult> {
        // Ensure exactly-once semantics
        if self.dispatched.contains(&bead_id) {
            return Err(Error::InvalidState(format!(
                "bead {} already dispatched",
                bead_id
            )));
        }

        // Validate round-robin has tenant_id
        if self.strategy == QueueStrategy::RoundRobin && self.tenant_id.is_none() {
            return Err(Error::InvalidState(
                "round-robin strategy requires tenant_id".to_string(),
            ));
        }

        // Mark as dispatched
        self.dispatched.insert(bead_id);

        // Route to queue
        let queue_name = self.strategy.queue_name();
        let tenant_id = self.tenant_id.clone();

        Ok(DispatchResult {
            bead_id,
            queue_name: queue_name.to_string(),
            tenant_id,
        })
    }

    /// Dispatch multiple ready beads.
    ///
    /// Returns a vector of successful dispatches and a vector of errors.
    /// This continues dispatching even if some beads fail.
    pub fn dispatch_batch(
        &mut self,
        bead_ids: &[BeadId],
    ) -> (Vec<DispatchResult>, Vec<(BeadId, Error)>) {
        let mut successes = Vec::with_capacity(bead_ids.len());
        let mut failures = Vec::new();

        for &bead_id in bead_ids {
            match self.dispatch(bead_id) {
                Ok(result) => successes.push(result),
                Err(e) => failures.push((bead_id, e)),
            }
        }

        (successes, failures)
    }

    /// Check if a bead has been dispatched.
    pub fn is_dispatched(&self, bead_id: &BeadId) -> bool {
        self.dispatched.contains(bead_id)
    }

    /// Clear the dispatched set (for testing or reset).
    pub fn clear_dispatched(&mut self) {
        self.dispatched.clear();
    }

    /// Get the current strategy.
    pub fn strategy(&self) -> QueueStrategy {
        self.strategy
    }

    /// Get the current tenant ID.
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }

    /// Get the number of dispatched beads.
    pub fn dispatched_count(&self) -> usize {
        self.dispatched.len()
    }
}

/// Result of dispatching a bead to a queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchResult {
    /// The bead that was dispatched.
    pub bead_id: BeadId,
    /// The queue name the bead was dispatched to.
    pub queue_name: String,
    /// The tenant ID (for round-robin routing).
    pub tenant_id: Option<String>,
}

impl DispatchResult {
    /// Create a new dispatch result.
    pub fn new(bead_id: BeadId, queue_name: String, tenant_id: Option<String>) -> Self {
        Self {
            bead_id,
            queue_name,
            tenant_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fifo_dispatch() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Fifo);
        let bead_id = BeadId::new();

        let result = dispatcher.dispatch(bead_id);
        assert!(result.is_ok());

        let dispatch_result = result.ok();
        assert!(dispatch_result.is_some());

        let dispatch_result = dispatch_result.as_ref();
        assert!(dispatch_result.is_some());

        if let Some(dr) = dispatch_result {
            assert_eq!(dr.bead_id, bead_id);
            assert_eq!(dr.queue_name, "fifo");
            assert!(dr.tenant_id.is_none());
        }
    }

    #[test]
    fn test_exactly_once_semantics() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Fifo);
        let bead_id = BeadId::new();

        // First dispatch succeeds
        let result1 = dispatcher.dispatch(bead_id);
        assert!(result1.is_ok());

        // Second dispatch fails
        let result2 = dispatcher.dispatch(bead_id);
        assert!(result2.is_err());
    }

    #[test]
    fn test_roundrobin_requires_tenant() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::RoundRobin);
        let bead_id = BeadId::new();

        // Fails without tenant_id
        let result = dispatcher.dispatch(bead_id);
        assert!(result.is_err());

        // Succeeds with tenant_id
        dispatcher.set_tenant_id(Some("tenant-123".to_string()));
        let bead_id2 = BeadId::new();
        let result2 = dispatcher.dispatch(bead_id2);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_with_tenant_constructor() {
        let mut dispatcher = Dispatcher::with_tenant("tenant-456".to_string());
        assert_eq!(dispatcher.strategy(), QueueStrategy::RoundRobin);
        assert_eq!(dispatcher.tenant_id(), Some("tenant-456"));

        let bead_id = BeadId::new();
        let result = dispatcher.dispatch(bead_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dispatch_batch() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Priority);
        let beads = vec![BeadId::new(), BeadId::new(), BeadId::new()];

        let (successes, failures) = dispatcher.dispatch_batch(&beads);

        assert_eq!(successes.len(), 3);
        assert_eq!(failures.len(), 0);

        for (i, success) in successes.iter().enumerate() {
            assert_eq!(success.bead_id, beads[i]);
            assert_eq!(success.queue_name, "priority");
        }
    }

    #[test]
    fn test_dispatch_batch_with_duplicates() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Lifo);
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();
        let beads = vec![bead1, bead2, bead1]; // bead1 appears twice

        let (successes, failures) = dispatcher.dispatch_batch(&beads);

        assert_eq!(successes.len(), 2); // Only first occurrence of each bead
        assert_eq!(failures.len(), 1); // Second occurrence of bead1 fails
    }

    #[test]
    fn test_is_dispatched() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Fifo);
        let bead_id = BeadId::new();

        assert!(!dispatcher.is_dispatched(&bead_id));

        let result = dispatcher.dispatch(bead_id);
        assert!(result.is_ok());
        assert!(dispatcher.is_dispatched(&bead_id));
    }

    #[test]
    fn test_clear_dispatched() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Fifo);
        let bead_id = BeadId::new();

        let result = dispatcher.dispatch(bead_id);
        assert!(result.is_ok());
        assert_eq!(dispatcher.dispatched_count(), 1);

        dispatcher.clear_dispatched();
        assert_eq!(dispatcher.dispatched_count(), 0);
        assert!(!dispatcher.is_dispatched(&bead_id));
    }

    #[test]
    fn test_queue_strategy_names() {
        assert_eq!(QueueStrategy::Fifo.queue_name(), "fifo");
        assert_eq!(QueueStrategy::Lifo.queue_name(), "lifo");
        assert_eq!(QueueStrategy::RoundRobin.queue_name(), "roundrobin");
        assert_eq!(QueueStrategy::Priority.queue_name(), "priority");
    }

    #[test]
    fn test_set_strategy() {
        let mut dispatcher = Dispatcher::new(QueueStrategy::Fifo);
        assert_eq!(dispatcher.strategy(), QueueStrategy::Fifo);

        dispatcher.set_strategy(QueueStrategy::Priority);
        assert_eq!(dispatcher.strategy(), QueueStrategy::Priority);
    }
}
