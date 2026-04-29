//! Execution step runner.
//!
//! Implements phase-based parallel execution for workflow orchestrator.
//! Nodes are grouped into phases based on x-coordinate proximity and
//! executed concurrently within each phase.

use crate::graph::{ExecutionState, NodeId, Workflow};
use std::collections::HashMap;

const X_COORDINATE_TOLERANCE: f32 = 50.0;

impl Workflow {
    // ===========================================================================
    // Phase Detection
    // ===========================================================================

    #[allow(clippy::cast_possible_truncation)]
    fn group_nodes_by_phase(&self, execution_queue: &[NodeId]) -> Vec<Vec<NodeId>> {
        if execution_queue.is_empty() {
            return vec![];
        }

        let node_map: HashMap<NodeId, &crate::graph::Node> =
            self.nodes.iter().map(|n| (n.id, n)).collect();

        let mut phase_map: HashMap<i32, Vec<NodeId>> = HashMap::new();

        for &node_id in execution_queue {
            if let Some(node) = node_map.get(&node_id) {
                let bucket = (node.x / X_COORDINATE_TOLERANCE).floor();
                let bucket_key = bucket as i32;
                phase_map.entry(bucket_key).or_default().push(node_id);
            }
        }

        let mut phases: Vec<Vec<NodeId>> = phase_map.into_values().collect();
        phases.sort_by(|a, b| {
            let x_a = node_map.get(&a[0]).map_or(f32::MAX, |n| n.x);
            let x_b = node_map.get(&b[0]).map_or(f32::MAX, |n| n.x);
            x_a.partial_cmp(&x_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        phases
    }

    // ===========================================================================
    // Condition Branch Skipping
    // ===========================================================================

    fn execute_condition_and_skip_branches(&mut self, node_id: NodeId, output: &serde_json::Value) {
        let result =
            output.get("result").and_then(serde_json::Value::as_bool).is_some_and(|value| value);
        let skip_port = if result { "false" } else { "true" };

        let branch_targets: Vec<NodeId> = self
            .connections
            .iter()
            .filter(|c| c.source == node_id && c.source_port.0 == skip_port)
            .map(|c| c.target)
            .collect();

        let branch_descendants = self.collect_descendants(&branch_targets);

        let mut skip_set: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        skip_set.extend(branch_targets);
        skip_set.extend(branch_descendants);

        for skip_id in &skip_set {
            if let Some(skip_node) = self.nodes.iter_mut().find(|n| n.id == *skip_id) {
                if !skip_node.skipped {
                    skip_node.skipped = true;
                    let _ = Self::set_node_status(skip_node, ExecutionState::Skipped);
                }
            }
        }

        let target_ids: Vec<NodeId> = self.nodes.iter().map(|n| n.id).collect();
        for target_id in target_ids {
            if skip_set.contains(&target_id) {
                continue;
            }
            let incoming: Vec<NodeId> = self
                .connections
                .iter()
                .filter(|c| c.target == target_id)
                .map(|c| c.source)
                .collect();

            if !incoming.is_empty() && incoming.iter().all(|src| skip_set.contains(src)) {
                if let Some(target_node) = self.nodes.iter_mut().find(|n| n.id == target_id) {
                    target_node.skipped = true;
                    let _ = Self::set_node_status(target_node, ExecutionState::Skipped);
                }
            }
        }
    }

    // ===========================================================================
    // Execution Step Runner
    // ===========================================================================

    #[allow(clippy::too_many_lines)]
    pub async fn step(&mut self) -> bool {
        let phases = self.group_nodes_by_phase(&self.execution_queue);

        if self.current_step >= phases.len() {
            self.nodes.iter_mut().for_each(|node| {
                node.executing = false;
            });
            return false;
        }

        let phase_nodes = &phases[self.current_step];

        if phase_nodes.is_empty() {
            self.current_step += 1;
            return true;
        }

        let all_skipped = phase_nodes
            .iter()
            .all(|&node_id| self.nodes.iter().find(|n| n.id == node_id).is_some_and(|n| n.skipped));

        if all_skipped {
            eprintln!("  Skipping phase {} (all skipped)", self.current_step);
            for &node_id in phase_nodes {
                if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                    let _ = Self::set_node_status(node, ExecutionState::Skipped);
                }
            }
            self.current_step += 1;
            return true;
        }

        let mut results: HashMap<NodeId, serde_json::Value> = HashMap::new();

        for &node_id in phase_nodes {
            if self.nodes.iter().find(|n| n.id == node_id).is_some_and(|n| n.skipped) {
                continue;
            }

            if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                node.executing = true;
                let _ = Self::set_node_status(node, ExecutionState::Running);
            }

            let parent_outputs: Vec<serde_json::Value> = self
                .connections
                .iter()
                .filter(|c| c.target == node_id)
                .filter_map(|c| {
                    self.nodes.iter().find(|n| n.id == c.source).and_then(|n| n.last_output.clone())
                })
                .collect();

            if let Some(node) = self.nodes.iter().find(|n| n.id == node_id) {
                let node_type = node.node_type.clone();
                let node_config_json = node.config.clone();
                let resolved_config = self.resolve_expressions(&node_config_json);

                let output =
                    self.execute_node_type(&node_type, &resolved_config, &parent_outputs).await;

                results.insert(node_id, output.clone());

                if let Err(memory_error) = self.check_and_update_memory(&output) {
                    if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                        n.error = Some(memory_error.to_string());
                        let _ = Self::set_node_status(n, ExecutionState::Failed);
                        n.executing = false;
                        n.last_output = Some(output.clone());
                    }
                    self.execution_failed = true;
                }
            }
        }

        for (node_id, output) in results {
            let is_condition = self
                .nodes
                .iter()
                .find(|n| n.id == node_id)
                .map(|n| n.node_type.as_str())
                .is_some_and(|s| s == "condition");

            if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                n.executing = false;
                n.last_output = Some(output.clone());

                if let Some(err) = output.get("error").and_then(serde_json::Value::as_str) {
                    n.error = Some(err.to_string());
                    let _ = Self::set_node_status(n, ExecutionState::Failed);
                } else {
                    let _ = Self::set_node_status(n, ExecutionState::Completed);
                }
            }

            if is_condition {
                self.execute_condition_and_skip_branches(node_id, &output);
            }
        }

        self.current_step += 1;
        true
    }
}
