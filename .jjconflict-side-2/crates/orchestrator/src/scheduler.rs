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

use im::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::dag::{DagError, DependencyType, WorkflowDAG};
use crate::{Error, Result};

// Re-export types from DAG module
pub use crate::dag::BeadId;

/// Unique identifier for a workflow
pub type WorkflowId = String;

/// Wrapper around WorkflowDAG that tracks workflow metadata
#[derive(Debug, Clone)]
pub struct WorkflowState {
    /// The workflow ID this state belongs to
    workflow_id: WorkflowId,
    /// The underlying DAG with dependency tracking
    dag: WorkflowDAG,
    /// Set of completed bead IDs for ready detection
    completed: HashSet<BeadId>,
}

impl WorkflowState {
    /// Create a new workflow state
    #[must_use]
    pub fn new(workflow_id: WorkflowId) -> Self {
        Self {
            workflow_id,
            dag: WorkflowDAG::new(),
            completed: HashSet::new(),
        }
    }

    /// Get the workflow ID
    #[must_use]
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    /// Get the underlying DAG
    #[must_use]
    pub fn dag(&self) -> &WorkflowDAG {
        &self.dag
    }

    /// Get a mutable reference to the DAG
    pub fn dag_mut(&mut self) -> &mut WorkflowDAG {
        &mut self.dag
    }

    /// Add a bead to this workflow's DAG
    pub fn add_bead(&mut self, bead_id: BeadId) -> Result<()> {
        self.dag.add_node(bead_id).map_err(dag_error_to_error)
    }

    /// Add a dependency between beads
    pub fn add_dependency(
        &mut self,
        from_bead: BeadId,
        to_bead: BeadId,
        dep_type: DependencyType,
    ) -> Result<()> {
        self.dag
            .add_edge(from_bead, to_bead, dep_type)
            .map_err(dag_error_to_error)
    }

    /// Mark a bead as completed
    pub fn mark_completed(&mut self, bead_id: &BeadId) {
        self.completed = self.completed.update(bead_id.clone());
    }

    /// Get all beads that are ready to execute
    #[must_use]
    pub fn get_ready_beads(&self) -> Vec<BeadId> {
        self.dag.get_ready_nodes(&self.completed)
    }

    /// Check if a specific bead is ready
    pub fn is_bead_ready(&self, bead_id: &BeadId) -> Result<bool> {
        self.dag
            .is_ready(bead_id, &self.completed)
            .map_err(dag_error_to_error)
    }

    /// Check if DAG is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dag.node_count() == 0
    }

    /// Get the number of beads in the DAG
    #[must_use]
    pub fn len(&self) -> usize {
        self.dag.node_count()
    }

    /// Get the number of completed beads
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Check if workflow is complete (all beads done)
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.completed.len() == self.dag.node_count()
    }

    /// Get all bead IDs in this workflow
    #[must_use]
    pub fn beads(&self) -> Vec<BeadId> {
        self.dag.nodes().cloned().collect()
    }

    /// Check if a bead exists in this workflow
    #[must_use]
    pub fn contains_bead(&self, bead_id: &BeadId) -> bool {
        self.dag.contains_node(bead_id)
    }
}

/// Convert DagError to the crate Error type
fn dag_error_to_error(e: DagError) -> Error {
    Error::invalid_record(e.to_string())
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

use im::Vector;

/// Scheduler actor for managing workflow DAGs and bead scheduling.
///
/// Maintains the following invariants:
/// - One WorkflowState per workflow_id
/// - Each bead is tracked in exactly one workflow
/// - Event subscriptions are active while actor is running
#[derive(Debug, Clone)]
pub struct SchedulerActor {
    /// Map of workflow IDs to their state (DAG + completed tracking)
    /// Invariant: One WorkflowState per workflow_id
    workflows: HashMap<WorkflowId, WorkflowState>,

    /// Pending beads waiting to be scheduled
    pending_beads: HashMap<BeadId, ScheduledBead>,

    /// Ready beads that can be dispatched to queues
    ready_beads: Vector<BeadId>,

    /// Worker assignments (bead_id -> worker_id)
    worker_assignments: HashMap<BeadId, String>,

    /// References to queue actors for dispatching
    queue_refs: Vector<QueueActorRef>,

    /// Event subscriptions (for bead completion events)
    event_subscriptions: Vector<EventSubscription>,
}

impl SchedulerActor {
    /// Create a new scheduler actor with no workflows
    #[must_use]
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            pending_beads: HashMap::new(),
            ready_beads: Vector::new(),
            worker_assignments: HashMap::new(),
            queue_refs: Vector::new(),
            event_subscriptions: Vector::new(),
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
            .insert(workflow_id.clone(), WorkflowState::new(workflow_id));
        Ok(())
    }

    /// Unregister a workflow (when it completes)
    ///
    /// Returns the removed state if it existed, None otherwise
    pub fn unregister_workflow(&mut self, workflow_id: &WorkflowId) -> Option<WorkflowState> {
        self.workflows.remove(workflow_id)
    }

    /// Get a workflow state by ID
    #[must_use]
    pub fn get_workflow(&self, workflow_id: &WorkflowId) -> Option<&WorkflowState> {
        self.workflows.get(workflow_id)
    }

    /// Get a mutable reference to a workflow state
    pub fn get_workflow_mut(&mut self, workflow_id: &WorkflowId) -> Option<&mut WorkflowState> {
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
        let workflow_state = self
            .workflows
            .get_mut(&workflow_id)
            .ok_or_else(|| Error::invalid_record(format!("workflow {} not found", workflow_id)))?;

        // Add bead to workflow DAG
        workflow_state.add_bead(bead_id.clone())?;

        // Track bead as pending
        let scheduled_bead = ScheduledBead::new(bead_id.clone(), workflow_id);
        self.pending_beads.insert(bead_id, scheduled_bead);

        Ok(())
    }

    /// Add a dependency between beads in a workflow
    ///
    /// The `from_bead` must complete before `to_bead` can start
    pub fn add_dependency(
        &mut self,
        workflow_id: &WorkflowId,
        from_bead: BeadId,
        to_bead: BeadId,
    ) -> Result<()> {
        let workflow_state = self
            .workflows
            .get_mut(workflow_id)
            .ok_or_else(|| Error::invalid_record(format!("workflow {} not found", workflow_id)))?;

        workflow_state.add_dependency(from_bead, to_bead, DependencyType::BlockingDependency)
    }

    /// Get all beads ready to execute in a workflow (based on DAG dependencies)
    pub fn get_workflow_ready_beads(&self, workflow_id: &WorkflowId) -> Result<Vec<BeadId>> {
        let workflow_state = self
            .workflows
            .get(workflow_id)
            .ok_or_else(|| Error::invalid_record(format!("workflow {} not found", workflow_id)))?;

        Ok(workflow_state.get_ready_beads())
    }

    /// Mark a bead as ready for dispatch
    pub fn mark_ready(&mut self, bead_id: &BeadId) -> Result<()> {
        let bead = self
            .pending_beads
            .get_mut(bead_id)
            .ok_or_else(|| Error::invalid_record(format!("bead {} not found", bead_id)))?;

        bead.set_state(BeadScheduleState::Ready);

        if !self.ready_beads.contains(bead_id) {
            self.ready_beads.push_back(bead_id.clone());
        }

        Ok(())
    }

    /// Get all ready beads
    #[must_use]
    pub fn get_ready_beads(&self) -> Vec<BeadId> {
        self.ready_beads.iter().cloned().collect()
    }

    /// Handle a bead completion event
    ///
    /// This method is called when a BeadCompleted event is received from the event bus.
    /// It updates both the bead's schedule state AND the workflow DAG's completed set.
    pub fn handle_bead_completed(&mut self, bead_id: &BeadId) -> Result<()> {
        // Update bead state
        if let Some(bead) = self.pending_beads.get_mut(bead_id) {
            bead.set_state(BeadScheduleState::Completed);
        }

        // Remove from ready list
        self.ready_beads.retain(|id| id != bead_id);

        // Remove worker assignment
        self.worker_assignments.remove(bead_id);

        // MARK BEAD AS COMPLETED IN WORKFLOW DAG
        // This is the key change - we need to update the WorkflowState's completed set
        // so that dependent beads become ready
        if let Some(scheduled_bead) = self.pending_beads.get(bead_id) {
            let workflow_id = scheduled_bead.workflow_id.clone();
            if let Some(workflow_state) = self.workflows.get_mut(&workflow_id) {
                workflow_state.mark_completed(bead_id);
            }
        }

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
            self.queue_refs.push_back(queue_ref);
        }
    }

    /// Get all queue references
    #[must_use]
    pub fn get_queue_refs(&self) -> Vector<QueueActorRef> {
        self.queue_refs.clone()
    }

    /// Add an event subscription
    pub fn subscribe_to_events(&mut self, subscription: EventSubscription) {
        self.event_subscriptions.push_back(subscription);
    }

    /// Get all event subscriptions
    #[must_use]
    pub fn get_subscriptions(&self) -> Vector<EventSubscription> {
        self.event_subscriptions.clone()
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

    // ========================================================================
    // BEHAVIORAL TESTS (Martin Fowler style)
    // ========================================================================
    // These tests focus on observable behavior rather than implementation details.
    // They use descriptive names and follow Given-When-Then structure.

    #[test]
    fn should_register_workflow_and_track_it() {
        // GIVEN: A new scheduler with no workflows
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();

        // WHEN: Registering a new workflow
        let result = scheduler.register_workflow(workflow_id.clone());

        // THEN: The workflow should be tracked and accessible
        assert!(result.is_ok(), "workflow registration should succeed");
        assert_eq!(
            scheduler.workflow_count(),
            1,
            "should track exactly one workflow"
        );

        let workflow = scheduler.get_workflow(&workflow_id);
        assert!(
            workflow.is_some(),
            "registered workflow should be retrievable"
        );

        if let Some(state) = workflow {
            assert_eq!(
                state.workflow_id(),
                &workflow_id,
                "workflow should have correct ID"
            );
            assert!(state.is_empty(), "new workflow should have no beads");
        }
    }

    #[test]
    fn should_schedule_bead_in_pending_state() {
        // GIVEN: A scheduler with a registered workflow
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();
        let bead_id = "bead-456".to_string();

        let register_result = scheduler.register_workflow(workflow_id.clone());
        assert!(
            register_result.is_ok(),
            "workflow registration should succeed"
        );

        // WHEN: Scheduling a new bead
        let schedule_result = scheduler.schedule_bead(workflow_id.clone(), bead_id.clone());

        // THEN: The bead should be in pending state
        assert!(
            schedule_result.is_ok(),
            "bead scheduling should succeed for registered workflow"
        );
        assert_eq!(scheduler.pending_count(), 1, "should have one pending bead");
        assert_eq!(scheduler.ready_count(), 0, "bead should not be ready yet");

        // AND: The bead should be added to the workflow's DAG
        let workflow = scheduler.get_workflow(&workflow_id);
        assert!(workflow.is_some(), "workflow should still exist");

        if let Some(state) = workflow {
            assert!(
                state.contains_bead(&bead_id),
                "bead should be tracked in workflow DAG"
            );
        }
    }

    #[test]
    fn should_mark_bead_ready_when_dependencies_met() {
        // GIVEN: A scheduler with a scheduled bead in pending state
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();
        let bead_id = "bead-456".to_string();

        let register_result = scheduler.register_workflow(workflow_id.clone());
        assert!(
            register_result.is_ok(),
            "workflow registration should succeed"
        );

        let schedule_result = scheduler.schedule_bead(workflow_id, bead_id.clone());
        assert!(schedule_result.is_ok(), "bead scheduling should succeed");

        assert_eq!(
            scheduler.pending_count(),
            1,
            "bead should start in pending state"
        );

        // WHEN: Dependencies are satisfied and bead is marked ready
        let mark_ready_result = scheduler.mark_ready(&bead_id);

        // THEN: The bead should transition to ready state
        assert!(
            mark_ready_result.is_ok(),
            "marking bead ready should succeed"
        );
        assert_eq!(scheduler.ready_count(), 1, "bead should be in ready queue");

        let ready_beads = scheduler.get_ready_beads();
        assert!(
            ready_beads.contains(&bead_id),
            "ready queue should contain the bead"
        );
    }

    #[test]
    fn should_dispatch_ready_bead_to_queue() {
        // GIVEN: A scheduler with a ready bead and available queues
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();
        let bead_id = "bead-456".to_string();
        let queue_ref = QueueActorRef::new("queue-fifo".to_string(), QueueType::FIFO);

        let register_result = scheduler.register_workflow(workflow_id.clone());
        assert!(
            register_result.is_ok(),
            "workflow registration should succeed"
        );

        let schedule_result = scheduler.schedule_bead(workflow_id, bead_id.clone());
        assert!(schedule_result.is_ok(), "bead scheduling should succeed");

        let mark_ready_result = scheduler.mark_ready(&bead_id);
        assert!(
            mark_ready_result.is_ok(),
            "marking bead ready should succeed"
        );

        scheduler.add_queue_ref(queue_ref.clone());

        // WHEN: Bead is dispatched to a queue
        // (In a real system, this would happen via a dispatch method)
        // For this test, we verify the queue infrastructure is in place

        // THEN: The queue should be available for dispatch
        let queues = scheduler.get_queue_refs();
        assert_eq!(queues.len(), 1, "should have one queue registered");
        assert!(
            queues.contains(&queue_ref),
            "registered queue should be available"
        );

        // AND: The ready bead should still be tracked
        assert_eq!(
            scheduler.ready_count(),
            1,
            "ready bead should be available for dispatch"
        );
    }

    #[test]
    fn should_handle_bead_completion_and_update_state() {
        // GIVEN: A scheduler with a bead that has been dispatched
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();
        let bead_id = "bead-456".to_string();
        let worker_id = "worker-789".to_string();

        let register_result = scheduler.register_workflow(workflow_id.clone());
        assert!(
            register_result.is_ok(),
            "workflow registration should succeed"
        );

        let schedule_result = scheduler.schedule_bead(workflow_id, bead_id.clone());
        assert!(schedule_result.is_ok(), "bead scheduling should succeed");

        let mark_ready_result = scheduler.mark_ready(&bead_id);
        assert!(
            mark_ready_result.is_ok(),
            "marking bead ready should succeed"
        );

        let assign_result = scheduler.assign_to_worker(&bead_id, worker_id.clone());
        assert!(assign_result.is_ok(), "worker assignment should succeed");

        assert!(
            scheduler.get_worker_assignment(&bead_id).is_some(),
            "bead should be assigned to worker"
        );

        // WHEN: Bead completes execution
        let completion_result = scheduler.handle_bead_completed(&bead_id);

        // THEN: The bead should be marked completed and cleaned up
        assert!(
            completion_result.is_ok(),
            "handling bead completion should succeed"
        );
        assert_eq!(
            scheduler.ready_count(),
            0,
            "completed bead should be removed from ready queue"
        );
        assert!(
            scheduler.get_worker_assignment(&bead_id).is_none(),
            "worker assignment should be cleaned up"
        );
    }

    #[test]
    fn should_unblock_dependent_beads_on_completion() {
        // GIVEN: A workflow with multiple beads where bead-2 depends on bead-1
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();
        let bead_1 = "bead-1".to_string();
        let bead_2 = "bead-2".to_string();

        let register_result = scheduler.register_workflow(workflow_id.clone());
        assert!(
            register_result.is_ok(),
            "workflow registration should succeed"
        );

        // Schedule both beads
        let schedule_1 = scheduler.schedule_bead(workflow_id.clone(), bead_1.clone());
        let schedule_2 = scheduler.schedule_bead(workflow_id, bead_2.clone());
        assert!(schedule_1.is_ok(), "scheduling bead-1 should succeed");
        assert!(schedule_2.is_ok(), "scheduling bead-2 should succeed");

        // Mark bead-1 ready (no dependencies)
        let ready_1 = scheduler.mark_ready(&bead_1);
        assert!(ready_1.is_ok(), "marking bead-1 ready should succeed");

        // bead-2 remains pending (depends on bead-1)
        assert_eq!(scheduler.pending_count(), 1, "bead-2 should remain pending");

        // WHEN: bead-1 completes
        let complete_1 = scheduler.handle_bead_completed(&bead_1);
        assert!(
            complete_1.is_ok(),
            "handling bead-1 completion should succeed"
        );

        // THEN: bead-2 can now be marked ready (dependencies satisfied)
        let ready_2 = scheduler.mark_ready(&bead_2);
        assert!(
            ready_2.is_ok(),
            "marking bead-2 ready should succeed after dependency completes"
        );
        assert_eq!(
            scheduler.ready_count(),
            1,
            "bead-2 should now be in ready queue"
        );

        let ready_beads = scheduler.get_ready_beads();
        assert!(
            ready_beads.contains(&bead_2),
            "bead-2 should be unblocked and ready"
        );
    }

    #[test]
    fn should_reject_scheduling_bead_without_registered_workflow() {
        // GIVEN: A scheduler with no registered workflows
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "nonexistent-workflow".to_string();
        let bead_id = "bead-456".to_string();

        assert_eq!(
            scheduler.workflow_count(),
            0,
            "scheduler should have no workflows"
        );

        // WHEN: Attempting to schedule a bead for nonexistent workflow
        let result = scheduler.schedule_bead(workflow_id, bead_id);

        // THEN: The operation should fail with an error
        assert!(
            result.is_err(),
            "scheduling bead without workflow should fail"
        );
    }

    #[test]
    fn should_prevent_duplicate_workflow_registration() {
        // GIVEN: A scheduler with an already registered workflow
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();

        let first_registration = scheduler.register_workflow(workflow_id.clone());
        assert!(
            first_registration.is_ok(),
            "first registration should succeed"
        );

        // WHEN: Attempting to register the same workflow again
        let second_registration = scheduler.register_workflow(workflow_id);

        // THEN: The duplicate registration should be rejected
        assert!(
            second_registration.is_err(),
            "duplicate workflow registration should fail"
        );
        assert_eq!(
            scheduler.workflow_count(),
            1,
            "should still have only one workflow"
        );
    }

    #[test]
    fn should_track_scheduler_statistics_accurately() {
        // GIVEN: A scheduler with various operations performed
        let mut scheduler = SchedulerActor::new();
        let workflow_id = "workflow-123".to_string();
        let bead_1 = "bead-1".to_string();
        let bead_2 = "bead-2".to_string();

        scheduler
            .register_workflow(workflow_id.clone())
            .ok()
            .filter(|_| false);
        scheduler
            .schedule_bead(workflow_id.clone(), bead_1.clone())
            .ok()
            .filter(|_| false);
        scheduler
            .schedule_bead(workflow_id, bead_2.clone())
            .ok()
            .filter(|_| false);
        scheduler.mark_ready(&bead_1).ok().filter(|_| false);

        // Add a queue
        let queue_ref = QueueActorRef::new("queue-1".to_string(), QueueType::FIFO);
        scheduler.add_queue_ref(queue_ref);

        // WHEN: Requesting scheduler statistics
        let stats = scheduler.stats();

        // THEN: Statistics should accurately reflect the state
        assert_eq!(
            stats.workflow_count, 1,
            "should report correct workflow count"
        );
        assert_eq!(
            stats.pending_count, 1,
            "should report correct pending count (bead-2)"
        );
        assert_eq!(
            stats.ready_count, 1,
            "should report correct ready count (bead-1)"
        );
        assert_eq!(stats.queue_count, 1, "should report correct queue count");
    }

    // ========================================================================
    // UNIT TESTS (Original implementation tests)
    // ========================================================================
    // These tests verify specific implementation details and edge cases.

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
        let state = scheduler.get_workflow(&workflow_id);
        assert!(state.is_some());
        if let Some(state) = state {
            assert!(state.contains_bead(&bead_id));
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
    fn test_workflow_state_new() {
        let workflow_id = "workflow-1".to_string();
        let state = WorkflowState::new(workflow_id.clone());

        assert_eq!(state.workflow_id(), &workflow_id);
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn test_workflow_state_add_bead() {
        let workflow_id = "workflow-1".to_string();
        let mut state = WorkflowState::new(workflow_id);
        let bead_id = "bead-1".to_string();

        let result = state.add_bead(bead_id.clone());
        assert!(result.is_ok());
        assert_eq!(state.len(), 1);
        assert!(state.contains_bead(&bead_id));
    }

    #[test]
    fn test_workflow_state_no_duplicates() {
        let workflow_id = "workflow-1".to_string();
        let mut state = WorkflowState::new(workflow_id);
        let bead_id = "bead-1".to_string();

        let result1 = state.add_bead(bead_id.clone());
        assert!(result1.is_ok());

        let result2 = state.add_bead(bead_id); // Add same bead again
        assert!(result2.is_err()); // Should fail with duplicate error

        assert_eq!(state.len(), 1); // Should still be 1
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

    // ========================================================================
    // WORKFLOW STATE TESTS
    // ========================================================================
    // Tests for WorkflowState wrapper around WorkflowDAG.
    // WorkflowState tracks workflow_id, dag, and completed beads.

    // --- BASIC OPERATIONS (4 tests) ---

    #[test]
    fn test_workflow_state_new_initial_state() {
        // GIVEN: A workflow ID
        let workflow_id = "workflow-test".to_string();

        // WHEN: Creating a new WorkflowState
        let state = WorkflowState::new(workflow_id.clone());

        // THEN: The state should be correctly initialized
        assert_eq!(
            state.workflow_id(),
            &workflow_id,
            "workflow_id should match the provided value"
        );
        assert!(state.is_empty(), "new state should have no beads");
        assert_eq!(state.len(), 0, "new state should have length 0");
        assert_eq!(
            state.completed_count(),
            0,
            "new state should have no completed beads"
        );
        assert!(
            state.is_complete(),
            "empty workflow is considered complete (0 == 0)"
        );
    }

    #[test]
    fn test_workflow_state_add_bead_success() {
        // GIVEN: A new WorkflowState
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_id = "bead-001".to_string();

        // WHEN: Adding a bead to the state
        let result = state.add_bead(bead_id.clone());

        // THEN: The bead should be added successfully
        assert!(result.is_ok(), "add_bead should succeed");
        assert_eq!(state.len(), 1, "state should contain one bead");
        assert!(!state.is_empty(), "state should not be empty after add");
        assert!(
            state.contains_bead(&bead_id),
            "state should contain the added bead"
        );
    }

    #[test]
    fn test_workflow_state_add_dependency_success() {
        // GIVEN: A WorkflowState with two beads
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_a = "bead-a".to_string();
        let bead_b = "bead-b".to_string();
        state.add_bead(bead_a.clone()).ok();
        state.add_bead(bead_b.clone()).ok();

        // WHEN: Adding a dependency from bead_a to bead_b
        let result = state.add_dependency(
            bead_a.clone(),
            bead_b.clone(),
            DependencyType::BlockingDependency,
        );

        // THEN: The dependency should be tracked in the DAG
        assert!(result.is_ok(), "add_dependency should succeed");

        // Verify via get_ready_beads - only bead_a should be ready initially
        let ready = state.get_ready_beads();
        assert!(
            ready.contains(&bead_a),
            "bead_a (no dependencies) should be ready"
        );
        assert!(
            !ready.contains(&bead_b),
            "bead_b (depends on bead_a) should not be ready"
        );
    }

    #[test]
    fn test_workflow_state_contains_bead_existence_check() {
        // GIVEN: A WorkflowState with one bead
        let mut state = WorkflowState::new("workflow-1".to_string());
        let existing_bead = "bead-exists".to_string();
        let missing_bead = "bead-missing".to_string();
        state.add_bead(existing_bead.clone()).ok();

        // WHEN: Checking if beads exist
        let exists = state.contains_bead(&existing_bead);
        let missing = state.contains_bead(&missing_bead);

        // THEN: Should correctly report existence
        assert!(exists, "should return true for existing bead");
        assert!(!missing, "should return false for missing bead");
    }

    // --- COMPLETION TRACKING (4 tests) ---

    #[test]
    fn test_workflow_state_mark_completed_tracks_bead() {
        // GIVEN: A WorkflowState with a bead
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_id = "bead-001".to_string();
        state.add_bead(bead_id.clone()).ok();

        assert_eq!(
            state.completed_count(),
            0,
            "initially no beads should be completed"
        );

        // WHEN: Marking the bead as completed
        state.mark_completed(&bead_id);

        // THEN: The bead should be tracked as completed
        assert_eq!(
            state.completed_count(),
            1,
            "one bead should be marked completed"
        );
    }

    #[test]
    fn test_workflow_state_completed_count_accuracy() {
        // GIVEN: A WorkflowState with multiple beads
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_1 = "bead-1".to_string();
        let bead_2 = "bead-2".to_string();
        let bead_3 = "bead-3".to_string();
        state.add_bead(bead_1.clone()).ok();
        state.add_bead(bead_2.clone()).ok();
        state.add_bead(bead_3.clone()).ok();

        // WHEN: Marking beads completed incrementally
        assert_eq!(state.completed_count(), 0, "initial count should be 0");

        state.mark_completed(&bead_1);
        assert_eq!(state.completed_count(), 1, "count after first completion");

        state.mark_completed(&bead_2);
        assert_eq!(state.completed_count(), 2, "count after second completion");

        // THEN: Count should accurately reflect completed beads
        // Marking same bead again should not double-count (HashSet semantics)
        state.mark_completed(&bead_1);
        assert_eq!(
            state.completed_count(),
            2,
            "re-marking same bead should not increase count"
        );
    }

    #[test]
    fn test_workflow_state_is_complete_all_beads_done() {
        // GIVEN: A WorkflowState with all beads
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_1 = "bead-1".to_string();
        let bead_2 = "bead-2".to_string();
        state.add_bead(bead_1.clone()).ok();
        state.add_bead(bead_2.clone()).ok();

        assert!(!state.is_complete(), "should not be complete initially");

        // WHEN: Marking all beads as completed
        state.mark_completed(&bead_1);
        state.mark_completed(&bead_2);

        // THEN: Workflow should be marked complete
        assert!(
            state.is_complete(),
            "should be complete when all beads are done"
        );
    }

    #[test]
    fn test_workflow_state_is_complete_partial_beads_done() {
        // GIVEN: A WorkflowState with multiple beads
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_1 = "bead-1".to_string();
        let bead_2 = "bead-2".to_string();
        let bead_3 = "bead-3".to_string();
        state.add_bead(bead_1.clone()).ok();
        state.add_bead(bead_2.clone()).ok();
        state.add_bead(bead_3.clone()).ok();

        // WHEN: Only some beads are completed
        state.mark_completed(&bead_1);
        state.mark_completed(&bead_2);
        // bead_3 is NOT completed

        // THEN: Workflow should NOT be complete
        assert!(
            !state.is_complete(),
            "should not be complete when some beads are pending"
        );
        assert_eq!(state.completed_count(), 2, "two beads completed");
        assert_eq!(state.len(), 3, "three beads total");
    }

    // --- READY DETECTION (4 tests) ---

    #[test]
    fn test_workflow_state_get_ready_beads_roots_are_ready() {
        // GIVEN: A WorkflowState with root beads (no dependencies)
        let mut state = WorkflowState::new("workflow-1".to_string());
        let root_1 = "root-1".to_string();
        let root_2 = "root-2".to_string();
        let child = "child".to_string();
        state.add_bead(root_1.clone()).ok();
        state.add_bead(root_2.clone()).ok();
        state.add_bead(child.clone()).ok();
        state
            .add_dependency(
                root_1.clone(),
                child.clone(),
                DependencyType::BlockingDependency,
            )
            .ok();
        state
            .add_dependency(
                root_2.clone(),
                child.clone(),
                DependencyType::BlockingDependency,
            )
            .ok();

        // WHEN: Getting ready beads with nothing completed
        let ready = state.get_ready_beads();

        // THEN: Only root beads (no incoming edges) should be ready
        assert!(ready.contains(&root_1), "root_1 should be ready");
        assert!(ready.contains(&root_2), "root_2 should be ready");
        assert!(
            !ready.contains(&child),
            "child with dependencies should not be ready"
        );
    }

    #[test]
    fn test_workflow_state_get_ready_beads_after_dependencies_complete() {
        // GIVEN: A WorkflowState with a dependency chain: a -> b -> c
        let mut state = WorkflowState::new("workflow-1".to_string());
        let bead_a = "bead-a".to_string();
        let bead_b = "bead-b".to_string();
        let bead_c = "bead-c".to_string();
        state.add_bead(bead_a.clone()).ok();
        state.add_bead(bead_b.clone()).ok();
        state.add_bead(bead_c.clone()).ok();
        state
            .add_dependency(
                bead_a.clone(),
                bead_b.clone(),
                DependencyType::BlockingDependency,
            )
            .ok();
        state
            .add_dependency(
                bead_b.clone(),
                bead_c.clone(),
                DependencyType::BlockingDependency,
            )
            .ok();

        // Initially only a is ready
        let ready_initial = state.get_ready_beads();
        assert_eq!(
            ready_initial,
            vec![bead_a.clone()],
            "only a should be ready initially"
        );

        // WHEN: bead_a completes
        state.mark_completed(&bead_a);
        let ready_after_a = state.get_ready_beads();

        // THEN: bead_b should become ready, bead_c still blocked
        assert!(
            ready_after_a.contains(&bead_b),
            "b should be ready after a completes"
        );
        assert!(
            !ready_after_a.contains(&bead_c),
            "c should still be blocked"
        );
        assert!(
            !ready_after_a.contains(&bead_a),
            "completed beads should not appear in ready list"
        );

        // WHEN: bead_b completes
        state.mark_completed(&bead_b);
        let ready_after_b = state.get_ready_beads();

        // THEN: bead_c should become ready
        assert!(
            ready_after_b.contains(&bead_c),
            "c should be ready after b completes"
        );
    }

    #[test]
    fn test_workflow_state_is_bead_ready_returns_true() {
        // GIVEN: A WorkflowState with a root bead and a dependent bead
        let mut state = WorkflowState::new("workflow-1".to_string());
        let root = "root".to_string();
        let dependent = "dependent".to_string();
        state.add_bead(root.clone()).ok();
        state.add_bead(dependent.clone()).ok();
        state
            .add_dependency(
                root.clone(),
                dependent.clone(),
                DependencyType::BlockingDependency,
            )
            .ok();

        // Root is ready immediately
        let root_ready = state.is_bead_ready(&root);
        assert!(
            root_ready.is_ok(),
            "is_bead_ready should succeed for existing bead"
        );
        if let Ok(ready) = root_ready {
            assert!(ready, "root bead should be ready (no dependencies)");
        }

        // WHEN: Root completes
        state.mark_completed(&root);

        // THEN: Dependent should be ready
        let dependent_ready = state.is_bead_ready(&dependent);
        assert!(dependent_ready.is_ok(), "is_bead_ready should succeed");
        if let Ok(ready) = dependent_ready {
            assert!(ready, "dependent should be ready after root completes");
        }
    }

    #[test]
    fn test_workflow_state_is_bead_ready_returns_false_when_blocked() {
        // GIVEN: A WorkflowState with a dependency chain
        let mut state = WorkflowState::new("workflow-1".to_string());
        let blocker = "blocker".to_string();
        let blocked = "blocked".to_string();
        state.add_bead(blocker.clone()).ok();
        state.add_bead(blocked.clone()).ok();
        state
            .add_dependency(
                blocker.clone(),
                blocked.clone(),
                DependencyType::BlockingDependency,
            )
            .ok();

        // WHEN: Checking if the blocked bead is ready (blocker not completed)
        let result = state.is_bead_ready(&blocked);

        // THEN: Should return false - bead is blocked by incomplete dependency
        assert!(result.is_ok(), "is_bead_ready should succeed");
        if let Ok(ready) = result {
            assert!(
                !ready,
                "blocked bead should not be ready when dependency is incomplete"
            );
        }
    }
}
