//! Execution record logic and conversions.

use super::{ExecutionState, RunRecord};
use crate::graph::execution_record_types::{
    AttemptNumber, ExecutionOverallStatus, ExecutionRecord, ExecutionRecordId, StepCount, StepName,
    StepOutput, StepRecord, StepType, WorkflowName,
};

// ============================================================================
// Conversion from Legacy RunRecord
// ============================================================================

/// Converts a legacy [`super::RunRecord`] into an [`ExecutionRecord`] for
/// display purposes.
///
/// Pure function — no side effects.
#[must_use]
pub fn from_run_record(record: &RunRecord) -> ExecutionRecord {
    let steps_completed = StepCount(u32::try_from(record.results.len()).unwrap_or(0));
    let steps_failed =
        if record.success { StepCount::zero() } else { StepCount::zero().increment() };
    let status = if record.success {
        ExecutionOverallStatus::Succeeded
    } else {
        ExecutionOverallStatus::Failed
    };

    ExecutionRecord {
        id: ExecutionRecordId::from(record.id),
        workflow_name: WorkflowName::default(),
        status,
        start_time: record.timestamp,
        end_time: None,
        steps: record
            .results
            .iter()
            .map(|(node_id, output)| {
                (
                    *node_id,
                    StepRecord {
                        step_name: StepName::new(""),
                        step_type: StepType::new(""),
                        status: if record.success {
                            ExecutionState::Completed
                        } else {
                            ExecutionState::Failed
                        },
                        start_time: Some(record.timestamp),
                        end_time: None,
                        attempt: AttemptNumber::first(),
                        input: None,
                        output: StepOutput::success(output.clone()),
                    },
                )
            })
            .collect(),
        steps_completed,
        steps_failed,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::graph::NodeId;
    use chrono::TimeZone;
    use chrono::Utc;

    #[test]
    fn given_run_record_with_results_when_converting_then_steps_are_created() {
        let timestamp = Utc.timestamp_opt(1, 0).unwrap();
        let record = RunRecord {
            id: uuid::Uuid::new_v4(),
            timestamp,
            results: std::collections::HashMap::new(),
            success: true,
            restate_invocation_id: None,
        };

        let execution_record = from_run_record(&record);

        assert_eq!(execution_record.status, ExecutionOverallStatus::Succeeded);
        assert!(execution_record.steps.is_empty());
    }

    #[test]
    fn given_run_record_with_one_result_when_converting_then_step_is_created() {
        let timestamp = Utc.timestamp_opt(1, 0).unwrap();
        let node_id = NodeId::new();
        let mut results = std::collections::HashMap::new();
        results.insert(node_id, serde_json::json!({"output": "value"}));

        let record = RunRecord {
            id: uuid::Uuid::new_v4(),
            timestamp,
            results,
            success: true,
            restate_invocation_id: None,
        };

        let execution_record = from_run_record(&record);

        assert_eq!(execution_record.steps.len(), 1);
        assert!(execution_record.step_for_node(node_id).is_some());
    }

    #[test]
    fn given_run_record_with_failure_when_converting_then_status_is_failed() {
        let timestamp = Utc.timestamp_opt(1, 0).unwrap();
        let record = RunRecord {
            id: uuid::Uuid::new_v4(),
            timestamp,
            results: std::collections::HashMap::new(),
            success: false,
            restate_invocation_id: None,
        };

        let execution_record = from_run_record(&record);

        assert_eq!(execution_record.status, ExecutionOverallStatus::Failed);
    }
}
