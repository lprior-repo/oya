//! Scheduler actor for managing workflow DAGs and bead scheduling.
//!
//! The SchedulerActor maintains one WorkflowDAG per workflow and orchestrates
//! bead scheduling by:
//! - Tracking workflow DAGs and their ready beads
//! - Subscribing to bead completion events
//! - Dispatching ready beads to appropriate queue actors
//! - Rebuilding DAG state from database on restart

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// Re-export types from other crates for convenience
// Note: These would normally come from the events and workflow crates
// but we'll define the minimal types we need for this actor's state

/// Unique identifier for a workflow (placeholder until we can import from workflow crate)
pub type WorkflowId = String;

/// Unique identifier for a bead (placeholder until we can import from events crate)
pub type BeadId = String;

/// Reference to a WorkflowDAG (placeholder - actual implementation would be in a dag module)
/// This represents a directed acyclic graph of beads with their dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDAG {
    /// Workflow this DAG represents
    workflow_id: WorkflowId,
    /// Beads in this workflow (simplified - full implementation would use petgraph)
    beads: Vec<BeadId>,
}

impl WorkflowDAG {
    /// Create a new empty workflow DAG
    #[must_use]
    pub fn new(workflow_id: WorkflowId) -> Self {
        Self {
            workflow_id,
            beads: Vec::new(),
        }
    }

    /// Get the workflow ID
    #[must_use]
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    /// Get all beads in this DAG
    #[must_use]
    pub fn beads(&self) -> &[BeadId] {
        &self.beads
    }

    /// Add a bead to the DAG
    pub fn add_bead(&mut self, bead_id: BeadId) {
        if !self.beads.contains(&bead_id) {
            self.beads.push(bead_id);
        }
    }

    /// Check if DAG is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.beads.is_empty()
    }

    /// Get the number of beads in the DAG
    #[must_use]
    pub fn len(&self) -> usize {
        self.beads.len()
    }
}

/// Reference to a queue actor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueActorRef {
    /// Queue identifier
    pub queue_id: String,
    /// Queue type (FIFO, LIFO, RoundRobin, Priority)
    pub queue_type: QueueType,
}

impl QueueActorRef {
    /// Create a new queue actor reference
    #[must_use]
    pub fn new(queue_id: String, queue_type: QueueType) -> Self {
        Self {
            queue_id,
            queue_type,
        }
    }
}

/// Queue type for routing beads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueueType {
    /// First-in-first-out queue
    FIFO,
    /// Last-in-first-out queue (depth-first)
    LIFO,
    /// Round-robin fair queue
    RoundRobin,
    /// Priority-based queue
    Priority,
}

/// Event subscription handle (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    /// Subscription identifier
    pub subscription_id: String,
    /// Event types subscribed to
    pub event_types: Vec<String>,
}

impl EventSubscription {
    /// Create a new event subscription
    #[must_use]
    pub fn new(subscription_id: String, event_types: Vec<String>) -> Self {
        Self {
            subscription_id,
            event_types,
        }
    }
}

/// Messages that can be sent to the scheduler actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulerMessage {
    /// Schedule a new bead in a workflow
    ScheduleBead {
        workflow_id: WorkflowId,
        bead_id: BeadId,
    },
    /// Handle a bead completion event
    BeadCompleted {
        workflow_id: WorkflowId,
        bead_id: BeadId,
    },
    /// Register a workflow DAG
    RegisterWorkflow { workflow_id: WorkflowId },
    /// Unregister a workflow (workflow completed)
    UnregisterWorkflow { workflow_id: WorkflowId },
    /// Get ready beads for a workflow
    GetReadyBeads { workflow_id: WorkflowId },
    /// Rebuild DAG from database
    RebuildDAG { workflow_id: WorkflowId },
}

/// State of a scheduled bead
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadScheduleState {
    /// Bead is pending (waiting for dependencies)
    Pending,
    /// Bead is ready to be dispatched
    Ready,
    /// Bead has been dispatched to a queue
    Dispatched,
    /// Bead has been assigned to a worker
    Assigned,
    /// Bead is currently executing
    Running,
    /// Bead has completed
    Completed,
}

impl BeadScheduleState {
    /// Check if the bead is in a terminal state
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Check if the bead is ready to be scheduled
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Information about a scheduled bead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledBead {
    /// Bead identifier
    pub bead_id: BeadId,
    /// Workflow this bead belongs to
    pub workflow_id: WorkflowId,
    /// Current schedule state
    pub state: BeadScheduleState,
    /// Queue assigned to (if dispatched)
    pub assigned_queue: Option<String>,
}

impl ScheduledBead {
    /// Create a new scheduled bead in pending state
    #[must_use]
    pub fn new(bead_id: BeadId, workflow_id: WorkflowId) -> Self {
        Self {
            bead_id,
            workflow_id,
            state: BeadScheduleState::Pending,
            assigned_queue: None,
        }
    }

    /// Update the state of this bead
    pub fn set_state(&mut self, state: BeadScheduleState) {
        self.state = state;
    }

    /// Assign this bead to a queue
    pub fn assign_to_queue(&mut self, queue_id: String) {
        self.assigned_queue = Some(queue_id);
        self.state = BeadScheduleState::Dispatched;
    }
}

/// Scheduler actor for managing workflow DAGs and bead scheduling.
///
/// Maintains the following invariants:
/// - One WorkflowDAG per workflow_id
/// - Each bead is tracked in exactly one workflow
/// - Event subscriptions are active while actor is running
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerActor {
    /// Map of workflow IDs to their DAGs
    /// Invariant: One WorkflowDAG per workflow_id
    workflows: HashMap<WorkflowId, WorkflowDAG>,

    /// Pending beads waiting to be scheduled
    pending_beads: HashMap<BeadId, ScheduledBead>,

    /// Ready beads that can be dispatched to queues
    ready_beads: Vec<BeadId>,

    /// Worker assignments (bead_id -> worker_id)
    worker_assignments: HashMap<BeadId, String>,

    /// References to queue actors for dispatching
    queue_refs: Vec<QueueActorRef>,

    /// Event subscriptions (for bead completion events)
    event_subscriptions: Vec<EventSubscription>,
}

impl SchedulerActor {
    /// Create a new scheduler actor with no workflows
    #[must_use]
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            pending_beads: HashMap::new(),
            ready_beads: Vec::new(),
            worker_assignments: HashMap::new(),
            queue_refs: Vec::new(),
            event_subscriptions: Vec::new(),
        }
    }

    /// Register a new workflow with the scheduler
    ///
    /// Returns Ok(()) if registered successfully, Err if workflow already exists
    pub fn register_workflow(&mut self, workflow_id: WorkflowId) -> Result<()> {
        if self.workflows.contains_key(&workflow_id) {
            return Err(Error::invalid_record(format!(
                "workflow {} already registered",
                workflow_id
            )));
        }

        self.workflows
            .insert(workflow_id.clone(), WorkflowDAG::new(workflow_id));
        Ok(())
    }

    /// Unregister a workflow (when it completes)
    ///
    /// Returns the removed DAG if it existed, None otherwise
    pub fn unregister_workflow(&mut self, workflow_id: &WorkflowId) -> Option<WorkflowDAG> {
        self.workflows.remove(workflow_id)
    }

    /// Get a workflow DAG by ID
    #[must_use]
    pub fn get_workflow(&self, workflow_id: &WorkflowId) -> Option<&WorkflowDAG> {
        self.workflows.get(workflow_id)
    }

    /// Get a mutable reference to a workflow DAG
    pub fn get_workflow_mut(&mut self, workflow_id: &WorkflowId) -> Option<&mut WorkflowDAG> {
        self.workflows.get_mut(workflow_id)
    }

    /// Get the number of registered workflows
    #[must_use]
    pub fn workflow_count(&self) -> usize {
        self.workflows.len()
    }

    /// Schedule a bead in a workflow
    ///
    /// Adds the bead to the workflow's DAG and marks it as pending
    pub fn schedule_bead(&mut self, workflow_id: WorkflowId, bead_id: BeadId) -> Result<()> {
        // Ensure workflow exists
        if !self.workflows.contains_key(&workflow_id) {
            return Err(Error::invalid_record(format!(
                "workflow {} not found",
                workflow_id
            )));
        }

        // Add bead to workflow DAG
        if let Some(dag) = self.workflows.get_mut(&workflow_id) {
            dag.add_bead(bead_id.clone());
        }

        // Track bead as pending
        let scheduled_bead = ScheduledBead::new(bead_id.clone(), workflow_id);
        self.pending_beads.insert(bead_id, scheduled_bead);

        Ok(())
    }

    /// Mark a bead as ready for dispatch
    pub fn mark_ready(&mut self, bead_id: &BeadId) -> Result<()> {
        let bead = self
            .pending_beads
            .get_mut(bead_id)
            .ok_or_else(|| Error::invalid_record(format!("bead {} not found", bead_id)))?;

        bead.set_state(BeadScheduleState::Ready);

        if !self.ready_beads.contains(bead_id) {
            self.ready_beads.push(bead_id.clone());
        }

        Ok(())
    }

    /// Get all ready beads
    #[must_use]
    pub fn get_ready_beads(&self) -> &[BeadId] {
        &self.ready_beads
    }

    /// Handle a bead completion event
    pub fn handle_bead_completed(&mut self, bead_id: &BeadId) -> Result<()> {
        // Update bead state
        if let Some(bead) = self.pending_beads.get_mut(bead_id) {
            bead.set_state(BeadScheduleState::Completed);
        }

        // Remove from ready list
        self.ready_beads.retain(|id| id != bead_id);

        // Remove worker assignment
        self.worker_assignments.remove(bead_id);

        Ok(())
    }

    /// Assign a bead to a worker
    pub fn assign_to_worker(&mut self, bead_id: &BeadId, worker_id: String) -> Result<()> {
        if !self.pending_beads.contains_key(bead_id) {
            return Err(Error::invalid_record(format!("bead {} not found", bead_id)));
        }

        self.worker_assignments.insert(bead_id.clone(), worker_id);

        if let Some(bead) = self.pending_beads.get_mut(bead_id) {
            bead.set_state(BeadScheduleState::Assigned);
        }

        Ok(())
    }

    /// Get worker assignment for a bead
    #[must_use]
    pub fn get_worker_assignment(&self, bead_id: &BeadId) -> Option<&String> {
        self.worker_assignments.get(bead_id)
    }

    /// Add a queue actor reference
    pub fn add_queue_ref(&mut self, queue_ref: QueueActorRef) {
        if !self.queue_refs.contains(&queue_ref) {
            self.queue_refs.push(queue_ref);
        }
    }

    /// Get all queue references
    #[must_use]
    pub fn get_queue_refs(&self) -> &[QueueActorRef] {
        &self.queue_refs
    }

    /// Add an event subscription
    pub fn subscribe_to_events(&mut self, subscription: EventSubscription) {
        self.event_subscriptions.push(subscription);
    }

    /// Get all event subscriptions
    #[must_use]
    pub fn get_subscriptions(&self) -> &[EventSubscription] {
        &self.event_subscriptions
    }

    /// Get the number of pending beads
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_beads
            .values()
            .filter(|b| matches!(b.state, BeadScheduleState::Pending))
            .count()
    }

    /// Get the number of ready beads
    #[must_use]
    pub fn ready_count(&self) -> usize {
        self.ready_beads.len()
    }

    /// Get statistics about the scheduler state
    #[must_use]
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            workflow_count: self.workflows.len(),
            pending_count: self.pending_count(),
            ready_count: self.ready_count(),
            assigned_count: self.worker_assignments.len(),
            queue_count: self.queue_refs.len(),
        }
    }

    /// Clear all state (for testing)
    pub fn clear(&mut self) {
        self.workflows.clear();
        self.pending_beads.clear();
        self.ready_beads.clear();
        self.worker_assignments.clear();
        self.queue_refs.clear();
        self.event_subscriptions.clear();
    }
}

impl Default for SchedulerActor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about scheduler state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchedulerStats {
    /// Number of registered workflows
    pub workflow_count: usize,
    /// Number of pending beads
    pub pending_count: usize,
    /// Number of ready beads
    pub ready_count: usize,
    /// Number of assigned beads
    pub assigned_count: usize,
    /// Number of queue references
    pub queue_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_scheduler() {
        let scheduler = SchedulerActor::new();
        assert_eq!(scheduler.workflow_count(), 0);
        assert_eq!(scheduler.pending_count(), 0);
        assert_eq!(scheduler.ready_count(), 0);
    }

    #[test]
    fn test_register_workflow() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();

        let result = scheduler.register_workflow(workflow_id.clone());
        assert!(result.is_ok());
        assert_eq!(scheduler.workflow_count(), 1);
        assert!(scheduler.get_workflow(&workflow_id).is_some());
    }

    #[test]
    fn test_register_duplicate_workflow_fails() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();

        let first = scheduler.register_workflow(workflow_id.clone());
        assert!(first.is_ok());

        let second = scheduler.register_workflow(workflow_id);
        assert!(second.is_err());
    }

    #[test]
    fn test_unregister_workflow() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();

        scheduler.register_workflow(workflow_id.clone()).ok();
        let removed = scheduler.unregister_workflow(&workflow_id);

        assert!(removed.is_some());
        assert_eq!(scheduler.workflow_count(), 0);
    }

    #[test]
    fn test_schedule_bead() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();
        let bead_id = "bead-1".to_string();

        scheduler.register_workflow(workflow_id.clone()).ok();
        let result = scheduler.schedule_bead(workflow_id.clone(), bead_id.clone());

        assert!(result.is_ok());
        assert_eq!(scheduler.pending_count(), 1);

        // Verify bead was added to DAG
        let dag = scheduler.get_workflow(&workflow_id);
        assert!(dag.is_some());
        if let Some(dag) = dag {
            assert!(dag.beads().contains(&bead_id));
        }
    }

    #[test]
    fn test_schedule_bead_without_workflow_fails() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();
        let bead_id = "bead-1".to_string();

        let result = scheduler.schedule_bead(workflow_id, bead_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_mark_ready() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();
        let bead_id = "bead-1".to_string();

        scheduler.register_workflow(workflow_id.clone()).ok();
        scheduler.schedule_bead(workflow_id, bead_id.clone()).ok();

        let result = scheduler.mark_ready(&bead_id);
        assert!(result.is_ok());
        assert_eq!(scheduler.ready_count(), 1);
        assert!(scheduler.get_ready_beads().contains(&bead_id));
    }

    #[test]
    fn test_handle_bead_completed() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();
        let bead_id = "bead-1".to_string();

        scheduler.register_workflow(workflow_id.clone()).ok();
        scheduler.schedule_bead(workflow_id, bead_id.clone()).ok();
        scheduler.mark_ready(&bead_id).ok();

        let result = scheduler.handle_bead_completed(&bead_id);
        assert!(result.is_ok());
        assert_eq!(scheduler.ready_count(), 0);
    }

    #[test]
    fn test_assign_to_worker() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();
        let bead_id = "bead-1".to_string();
        let worker_id = "worker-1".to_string();

        scheduler.register_workflow(workflow_id.clone()).ok();
        scheduler.schedule_bead(workflow_id, bead_id.clone()).ok();

        let result = scheduler.assign_to_worker(&bead_id, worker_id.clone());
        assert!(result.is_ok());
        assert_eq!(scheduler.get_worker_assignment(&bead_id), Some(&worker_id));
    }

    #[test]
    fn test_add_queue_ref() {
        let mut scheduler = SchedulerActor::new();
        let queue_ref = QueueActorRef::new("queue-1".to_string(), QueueType::FIFO);

        scheduler.add_queue_ref(queue_ref.clone());
        assert_eq!(scheduler.get_queue_refs().len(), 1);
        assert!(scheduler.get_queue_refs().contains(&queue_ref));
    }

    #[test]
    fn test_subscribe_to_events() {
        let mut scheduler = SchedulerActor::new();
        let subscription =
            EventSubscription::new("sub-1".to_string(), vec!["BeadCompleted".to_string()]);

        scheduler.subscribe_to_events(subscription);
        assert_eq!(scheduler.get_subscriptions().len(), 1);
    }

    #[test]
    fn test_scheduler_stats() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();
        let bead_id = "bead-1".to_string();

        scheduler.register_workflow(workflow_id.clone()).ok();
        scheduler.schedule_bead(workflow_id, bead_id.clone()).ok();
        scheduler.mark_ready(&bead_id).ok();

        let stats = scheduler.stats();
        assert_eq!(stats.workflow_count, 1);
        assert_eq!(stats.pending_count, 0); // Marked as ready, not pending
        assert_eq!(stats.ready_count, 1);
    }

    #[test]
    fn test_clear() {
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-1".to_string();

        scheduler.register_workflow(workflow_id).ok();
        assert_eq!(scheduler.workflow_count(), 1);

        scheduler.clear();
        assert_eq!(scheduler.workflow_count(), 0);
    }

    #[test]
    fn test_workflow_dag_new() {
        let workflow_id = "workflow-1".to_string();
        let dag = WorkflowDAG::new(workflow_id.clone());

        assert_eq!(dag.workflow_id(), &workflow_id);
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
    }

    #[test]
    fn test_workflow_dag_add_bead() {
        let workflow_id = "workflow-1".to_string();
        let mut dag = WorkflowDAG::new(workflow_id);
        let bead_id = "bead-1".to_string();

        dag.add_bead(bead_id.clone());
        assert_eq!(dag.len(), 1);
        assert!(dag.beads().contains(&bead_id));
    }

    #[test]
    fn test_workflow_dag_no_duplicates() {
        let workflow_id = "workflow-1".to_string();
        let mut dag = WorkflowDAG::new(workflow_id);
        let bead_id = "bead-1".to_string();

        dag.add_bead(bead_id.clone());
        dag.add_bead(bead_id); // Add same bead again
        assert_eq!(dag.len(), 1); // Should still be 1
    }

    #[test]
    fn test_bead_schedule_state_is_terminal() {
        assert!(!BeadScheduleState::Pending.is_terminal());
        assert!(!BeadScheduleState::Ready.is_terminal());
        assert!(BeadScheduleState::Completed.is_terminal());
    }

    #[test]
    fn test_bead_schedule_state_is_ready() {
        assert!(!BeadScheduleState::Pending.is_ready());
        assert!(BeadScheduleState::Ready.is_ready());
        assert!(!BeadScheduleState::Completed.is_ready());
    }

    #[test]
    fn test_scheduled_bead_new() {
        let bead_id = "bead-1".to_string();
        let workflow_id = "workflow-1".to_string();

        let bead = ScheduledBead::new(bead_id.clone(), workflow_id.clone());
        assert_eq!(bead.bead_id, bead_id);
        assert_eq!(bead.workflow_id, workflow_id);
        assert_eq!(bead.state, BeadScheduleState::Pending);
        assert!(bead.assigned_queue.is_none());
    }

    #[test]
    fn test_scheduled_bead_set_state() {
        let bead_id = "bead-1".to_string();
        let workflow_id = "workflow-1".to_string();
        let mut bead = ScheduledBead::new(bead_id, workflow_id);

        bead.set_state(BeadScheduleState::Ready);
        assert_eq!(bead.state, BeadScheduleState::Ready);
    }

    #[test]
    fn test_scheduled_bead_assign_to_queue() {
        let bead_id = "bead-1".to_string();
        let workflow_id = "workflow-1".to_string();
        let mut bead = ScheduledBead::new(bead_id, workflow_id);
        let queue_id = "queue-1".to_string();

        bead.assign_to_queue(queue_id.clone());
        assert_eq!(bead.assigned_queue, Some(queue_id));
        assert_eq!(bead.state, BeadScheduleState::Dispatched);
    }
}
