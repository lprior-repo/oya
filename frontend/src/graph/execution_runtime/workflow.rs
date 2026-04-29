//! Workflow runner.

use crate::graph::{ExecutionState, NodeCategory, NodeId, RunRecord, Workflow};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

impl Workflow {
    // ===========================================================================
    // Workflow Runner
    // ===========================================================================

    pub async fn run(&mut self) {
        let _ = self.prepare_run();
        let start_time = Utc::now();
        let mut results = HashMap::new();

        if !self.has_entry_nodes() {
            self.push_run_record(start_time, results, false, None);
            return;
        }

        self.collect_run_results(&mut results).await;
        let success = self.run_succeeded();
        let restate_invocation_id = self.first_restate_invocation_id();

        self.push_run_record(start_time, results, success, restate_invocation_id);
    }

    fn has_entry_nodes(&self) -> bool {
        !self.nodes.is_empty() && self.nodes.iter().any(|node| node.category == NodeCategory::Entry)
    }

    async fn collect_run_results(&mut self, results: &mut HashMap<NodeId, Value>) {
        while !self.execution_failed && self.step().await {
            self.record_current_step_result(results);
        }
    }

    fn record_current_step_result(&self, results: &mut HashMap<NodeId, Value>) {
        let Some(id) = self.execution_queue.get(self.current_step.saturating_sub(1)) else {
            return;
        };
        let Some(node) = self.nodes.iter().find(|node| node.id == *id) else {
            return;
        };
        let Some(out) = &node.last_output else {
            return;
        };

        results.insert(*id, out.clone());
    }

    fn run_succeeded(&self) -> bool {
        self.nodes.iter().all(|node| {
            node.error.is_none()
                && matches!(
                    node.execution_state,
                    ExecutionState::Completed | ExecutionState::Skipped
                )
        })
    }

    fn first_restate_invocation_id(&self) -> Option<String> {
        self.nodes.iter().filter_map(|node| node.last_output.as_ref()).find_map(|output| {
            output
                .get("restate_invocation_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
    }

    fn push_run_record(
        &mut self,
        timestamp: DateTime<Utc>,
        results: HashMap<NodeId, Value>,
        success: bool,
        restate_invocation_id: Option<String>,
    ) {
        self.history.push(RunRecord {
            id: uuid::Uuid::new_v4(),
            timestamp,
            results,
            success,
            restate_invocation_id,
        });

        if self.history.len() > 10 {
            let _ = self.history.remove(0);
        }
    }
}
