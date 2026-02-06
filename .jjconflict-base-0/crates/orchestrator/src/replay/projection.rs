//! Projections for replaying orchestrator events.
//!
//! A projection transforms an event stream into a specific view of state.

// Allow dead_code until projections are fully integrated
#![allow(dead_code)]

use std::collections::HashMap;

use super::events::OrchestratorEvent;
use crate::persistence::{BeadState, WorkflowStatus};

/// A projection that builds state from events.
pub trait OrchestratorProjection: Send + Sync {
    /// Apply an event to update the projection's state.
    fn apply(&mut self, event: &OrchestratorEvent);

    /// Reset the projection to its initial state.
    fn reset(&mut self);
}

/// Projection that tracks workflow statuses.
#[derive(Debug, Default)]
pub struct WorkflowStatusProjection {
    /// Map of workflow ID to status
    pub statuses: HashMap<String, WorkflowStatus>,
    /// Map of workflow ID to DAG JSON
    pub dags: HashMap<String, String>,
}

impl OrchestratorProjection for WorkflowStatusProjection {
    fn apply(&mut self, event: &OrchestratorEvent) {
        match event {
            OrchestratorEvent::WorkflowRegistered {
                workflow_id,
                dag_json,
                ..
            } => {
                self.statuses
                    .insert(workflow_id.clone(), WorkflowStatus::Pending);
                self.dags.insert(workflow_id.clone(), dag_json.clone());
            }
            OrchestratorEvent::WorkflowUnregistered { workflow_id } => {
                self.statuses.remove(workflow_id);
                self.dags.remove(workflow_id);
            }
            OrchestratorEvent::WorkflowStatusChanged {
                workflow_id,
                status,
            } => {
                if let Some(ws) = parse_workflow_status(status) {
                    self.statuses.insert(workflow_id.clone(), ws);
                }
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.statuses.clear();
        self.dags.clear();
    }
}

impl WorkflowStatusProjection {
    /// Create a new workflow status projection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a workflow's current status.
    #[must_use]
    pub fn get_status(&self, workflow_id: &str) -> Option<&WorkflowStatus> {
        self.statuses.get(workflow_id)
    }

    /// Get all registered workflow IDs.
    #[must_use]
    pub fn workflow_ids(&self) -> Vec<&String> {
        self.statuses.keys().collect()
    }

    /// Get the count of workflows in each status.
    #[must_use]
    pub fn status_counts(&self) -> HashMap<WorkflowStatus, usize> {
        let mut counts = HashMap::new();
        for status in self.statuses.values() {
            *counts.entry(*status).or_insert(0) += 1;
        }
        counts
    }
}

/// Projection that tracks bead states.
#[derive(Debug, Default)]
pub struct BeadStateProjection {
    /// Map of bead ID to state
    pub states: HashMap<String, BeadState>,
    /// Map of bead ID to workflow ID
    pub workflow_assignments: HashMap<String, String>,
    /// Map of bead ID to worker ID
    pub worker_assignments: HashMap<String, String>,
}

impl OrchestratorProjection for BeadStateProjection {
    fn apply(&mut self, event: &OrchestratorEvent) {
        match event {
            OrchestratorEvent::BeadScheduled {
                bead_id,
                workflow_id,
            } => {
                self.states.insert(bead_id.clone(), BeadState::Ready);
                self.workflow_assignments
                    .insert(bead_id.clone(), workflow_id.clone());
            }
            OrchestratorEvent::BeadClaimed { bead_id, worker_id } => {
                self.states.insert(bead_id.clone(), BeadState::Assigned);
                self.worker_assignments
                    .insert(bead_id.clone(), worker_id.clone());
            }
            OrchestratorEvent::BeadStarted { bead_id, .. } => {
                self.states.insert(bead_id.clone(), BeadState::Running);
            }
            OrchestratorEvent::BeadCompleted { bead_id, .. } => {
                self.states.insert(bead_id.clone(), BeadState::Completed);
            }
            OrchestratorEvent::BeadFailed { bead_id, .. } => {
                self.states.insert(bead_id.clone(), BeadState::Failed);
            }
            OrchestratorEvent::BeadCancelled { bead_id, .. } => {
                self.states.insert(bead_id.clone(), BeadState::Cancelled);
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.states.clear();
        self.workflow_assignments.clear();
        self.worker_assignments.clear();
    }
}

impl BeadStateProjection {
    /// Create a new bead state projection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a bead's current state.
    #[must_use]
    pub fn get_state(&self, bead_id: &str) -> Option<&BeadState> {
        self.states.get(bead_id)
    }

    /// Get all beads in a specific state.
    #[must_use]
    pub fn beads_in_state(&self, state: BeadState) -> Vec<&String> {
        self.states
            .iter()
            .filter(|(_, s)| **s == state)
            .map(|(id, _)| id)
            .collect()
    }

    /// Get beads for a workflow.
    #[must_use]
    pub fn beads_for_workflow(&self, workflow_id: &str) -> Vec<&String> {
        self.workflow_assignments
            .iter()
            .filter(|(_, wf)| *wf == workflow_id)
            .map(|(bead_id, _)| bead_id)
            .collect()
    }
}

/// Projection that tracks agent health.
#[derive(Debug, Default)]
pub struct AgentHealthProjection {
    /// Map of agent ID to last heartbeat
    pub last_heartbeats: HashMap<String, chrono::DateTime<chrono::Utc>>,
    /// Map of agent ID to capabilities
    pub capabilities: HashMap<String, Vec<String>>,
    /// Set of registered agents
    pub registered: std::collections::HashSet<String>,
}

impl OrchestratorProjection for AgentHealthProjection {
    fn apply(&mut self, event: &OrchestratorEvent) {
        match event {
            OrchestratorEvent::AgentRegistered {
                agent_id,
                capabilities,
            } => {
                self.registered.insert(agent_id.clone());
                self.capabilities
                    .insert(agent_id.clone(), capabilities.clone());
            }
            OrchestratorEvent::AgentUnregistered { agent_id } => {
                self.registered.remove(agent_id);
                self.capabilities.remove(agent_id);
                self.last_heartbeats.remove(agent_id);
            }
            OrchestratorEvent::AgentHeartbeat {
                agent_id,
                timestamp,
            } => {
                self.last_heartbeats.insert(agent_id.clone(), *timestamp);
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.last_heartbeats.clear();
        self.capabilities.clear();
        self.registered.clear();
    }
}

impl AgentHealthProjection {
    /// Create a new agent health projection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an agent is registered.
    #[must_use]
    pub fn is_registered(&self, agent_id: &str) -> bool {
        self.registered.contains(agent_id)
    }

    /// Get all registered agent IDs.
    #[must_use]
    pub fn agent_ids(&self) -> Vec<&String> {
        self.registered.iter().collect()
    }
}

/// Parse workflow status from string.
fn parse_workflow_status(s: &str) -> Option<WorkflowStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Some(WorkflowStatus::Pending),
        "running" => Some(WorkflowStatus::Running),
        "completed" => Some(WorkflowStatus::Completed),
        "failed" => Some(WorkflowStatus::Failed),
        "cancelled" => Some(WorkflowStatus::Cancelled),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_projection() {
        let mut proj = WorkflowStatusProjection::new();

        proj.apply(&OrchestratorEvent::WorkflowRegistered {
            workflow_id: "wf-1".to_string(),
            name: "Test".to_string(),
            dag_json: "{}".to_string(),
        });

        assert_eq!(proj.get_status("wf-1"), Some(&WorkflowStatus::Pending));

        proj.apply(&OrchestratorEvent::WorkflowStatusChanged {
            workflow_id: "wf-1".to_string(),
            status: "running".to_string(),
        });

        assert_eq!(proj.get_status("wf-1"), Some(&WorkflowStatus::Running));
    }

    #[test]
    fn test_bead_projection() {
        let mut proj = BeadStateProjection::new();

        proj.apply(&OrchestratorEvent::BeadScheduled {
            workflow_id: "wf-1".to_string(),
            bead_id: "b-1".to_string(),
        });

        assert_eq!(proj.get_state("b-1"), Some(&BeadState::Ready));

        proj.apply(&OrchestratorEvent::BeadClaimed {
            bead_id: "b-1".to_string(),
            worker_id: "w-1".to_string(),
        });

        assert_eq!(proj.get_state("b-1"), Some(&BeadState::Assigned));
    }

    #[test]
    fn test_projection_reset() {
        let mut proj = WorkflowStatusProjection::new();

        proj.apply(&OrchestratorEvent::WorkflowRegistered {
            workflow_id: "wf-1".to_string(),
            name: "Test".to_string(),
            dag_json: "{}".to_string(),
        });

        assert!(!proj.statuses.is_empty());

        proj.reset();

        assert!(proj.statuses.is_empty());
    }
}
