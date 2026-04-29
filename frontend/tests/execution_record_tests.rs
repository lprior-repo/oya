//! Tests for execution_record_types: serialization roundtrips, identifier
//! uniqueness, and record ordering for execution history.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use chrono::Utc;
use oya_frontend::graph::{
    AttemptNumber, EmptyErrorMessage, ExecutionError, ExecutionOverallStatus, ExecutionRecord,
    ExecutionRecordId, ExecutionState, NodeId, StepCount, StepName, StepOutput, StepRecord,
    StepType, WorkflowName,
};
use serde_json::json;

// ===========================================================================
// ExecutionError & EmptyErrorMessage
// ===========================================================================

#[test]
fn given_non_empty_message_when_execution_error_created_then_succeeds() {
    let err = ExecutionError::new("something went wrong");
    assert_eq!(err.as_str(), "something went wrong");
}

#[test]
fn given_empty_string_when_execution_error_try_from_then_rejects() {
    let result = ExecutionError::try_from(String::new());
    assert_eq!(result.unwrap_err(), EmptyErrorMessage);
}

#[test]
fn given_execution_error_when_serialized_then_roundtrips() {
    let err = ExecutionError::new("node 42 failed");
    let json = serde_json::to_string(&err).unwrap();
    let parsed: ExecutionError = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, err);
}

#[test]
fn given_execution_error_when_try_from_empty_json_then_fails() {
    let result: Result<ExecutionError, _> = serde_json::from_str("\"\"");
    assert!(result.is_err());
}

// ===========================================================================
// ExecutionRecordId
// ===========================================================================

#[test]
fn given_two_new_ids_when_compared_then_different() {
    let id1 = ExecutionRecordId::new();
    let id2 = ExecutionRecordId::new();
    assert_ne!(id1, id2);
}

#[test]
fn given_record_id_when_serialized_then_roundtrips() {
    let id = ExecutionRecordId::new();
    let json = serde_json::to_string(&id).unwrap();
    let parsed: ExecutionRecordId = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, id);
}

#[test]
fn given_uuid_when_converted_to_record_id_then_preserves_value() {
    let uuid = uuid::Uuid::new_v4();
    let id: ExecutionRecordId = uuid.into();
    assert_eq!(id.as_uuid(), uuid);
}

// ===========================================================================
// WorkflowName
// ===========================================================================

#[test]
fn given_workflow_name_when_displayed_then_shows_name() {
    let name = WorkflowName::new("My Workflow");
    assert_eq!(format!("{name}"), "My Workflow");
}

#[test]
fn given_workflow_name_when_serialized_then_roundtrips() {
    let name = WorkflowName::new("order-pipeline");
    let json = serde_json::to_string(&name).unwrap();
    let parsed: WorkflowName = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, name);
}

#[test]
fn given_different_workflow_names_when_compared_then_not_equal() {
    let a = WorkflowName::new("alpha");
    let b = WorkflowName::new("beta");
    assert_ne!(a, b);
}

// ===========================================================================
// StepCount
// ===========================================================================

#[test]
fn given_step_count_zero_when_incremented_then_becomes_one() {
    let count = StepCount::zero().increment();
    assert_eq!(count.get(), 1);
}

#[test]
fn given_step_count_max_when_incremented_then_saturates() {
    let count = StepCount(u32::MAX).increment();
    assert_eq!(count.get(), u32::MAX);
}

#[test]
fn given_step_count_when_serialized_then_roundtrips() {
    let count = StepCount(42);
    let json = serde_json::to_string(&count).unwrap();
    let parsed: StepCount = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, count);
}

// ===========================================================================
// ExecutionOverallStatus
// ===========================================================================

#[test]
fn given_terminal_statuses_when_is_terminal_then_true() {
    assert!(ExecutionOverallStatus::Succeeded.is_terminal());
    assert!(ExecutionOverallStatus::Failed.is_terminal());
    assert!(ExecutionOverallStatus::Cancelled.is_terminal());
}

#[test]
fn given_running_status_when_is_terminal_then_false() {
    assert!(!ExecutionOverallStatus::Running.is_terminal());
}

#[test]
fn given_all_statuses_when_serialized_then_roundtrip() {
    for status in [
        ExecutionOverallStatus::Running,
        ExecutionOverallStatus::Succeeded,
        ExecutionOverallStatus::Failed,
        ExecutionOverallStatus::Cancelled,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let parsed: ExecutionOverallStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status, "Roundtrip failed for {status:?}");
    }
}

// ===========================================================================
// StepOutput
// ===========================================================================

#[test]
fn given_success_output_when_serialized_then_roundtrips() {
    let output = StepOutput::success(json!({"result": 42}));
    let json_str = serde_json::to_string(&output).unwrap();
    let parsed: StepOutput = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed, output);
}

#[test]
fn given_failure_output_when_serialized_then_roundtrips() {
    let output = StepOutput::failure(ExecutionError::new("timeout"));
    let json_str = serde_json::to_string(&output).unwrap();
    let parsed: StepOutput = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed, output);
}

#[test]
fn given_cancelled_output_when_serialized_then_roundtrips() {
    let output = StepOutput::Cancelled;
    let json_str = serde_json::to_string(&output).unwrap();
    let parsed: StepOutput = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed, output);
}

// ===========================================================================
// StepName & StepType (identifier uniqueness)
// ===========================================================================

#[test]
fn given_different_step_names_when_compared_then_distinct() {
    let a = StepName::new("fetch-order");
    let b = StepName::new("process-payment");
    assert_ne!(a, b);
    assert_ne!(a.as_str(), b.as_str());
}

#[test]
fn given_same_step_name_when_compared_then_equal() {
    let a = StepName::new("validate");
    let b = StepName::new("validate");
    assert_eq!(a, b);
}

#[test]
fn given_step_name_from_str_when_serialized_then_roundtrips() {
    let name = StepName::new("step-1");
    let json = serde_json::to_string(&name).unwrap();
    let parsed: StepName = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, name);
}

#[test]
fn given_step_type_from_str_when_serialized_then_roundtrips() {
    let stype = StepType::new("service-call");
    let json = serde_json::to_string(&stype).unwrap();
    let parsed: StepType = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, stype);
}

#[test]
fn given_step_names_when_used_as_hash_keys_then_distinct() {
    use std::collections::HashSet;
    let names: HashSet<StepName> =
        [StepName::new("a"), StepName::new("b"), StepName::new("c"), StepName::new("a")]
            .into_iter()
            .collect();
    assert_eq!(names.len(), 3);
}

#[test]
fn given_step_types_when_used_as_hash_keys_then_distinct() {
    use std::collections::HashSet;
    let types: HashSet<StepType> =
        [StepType::new("handler"), StepType::new("workflow"), StepType::new("handler")]
            .into_iter()
            .collect();
    assert_eq!(types.len(), 2);
}

// ===========================================================================
// AttemptNumber
// ===========================================================================

#[test]
fn given_first_attempt_when_next_called_then_increments() {
    let first = AttemptNumber::first();
    assert_eq!(first.get(), 1);
    let second = first.next();
    assert_eq!(second.get(), 2);
}

#[test]
fn given_max_attempt_when_next_called_then_saturates() {
    let max = AttemptNumber(u32::MAX);
    let next = max.next();
    assert_eq!(next.get(), u32::MAX);
}

#[test]
fn given_attempt_number_when_serialized_then_roundtrips() {
    let attempt = AttemptNumber(5);
    let json = serde_json::to_string(&attempt).unwrap();
    let parsed: AttemptNumber = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, attempt);
}

// ===========================================================================
// StepRecord
// ===========================================================================

#[test]
fn given_step_record_when_created_then_status_is_idle() {
    let record = StepRecord::new(StepName::new("fetch"), StepType::new("service-call"));
    assert_eq!(record.status, ExecutionState::Idle);
    assert_eq!(record.attempt, AttemptNumber::first());
    assert!(record.start_time.is_none());
}

#[test]
fn given_step_record_when_serialized_then_roundtrips() {
    let record = StepRecord {
        step_name: StepName::new("transform"),
        step_type: StepType::new("handler"),
        status: ExecutionState::Completed,
        start_time: Some(Utc::now()),
        end_time: Some(Utc::now()),
        attempt: AttemptNumber(2),
        input: Some(json!({"key": "val"})),
        output: StepOutput::success(json!({"count": 10})),
    };
    let json = serde_json::to_string(&record).unwrap();
    let parsed: StepRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, record);
}

// ===========================================================================
// ExecutionRecord (full record roundtrip & ordering)
// ===========================================================================

#[test]
fn given_execution_record_when_serialized_then_roundtrips() {
    let now = Utc::now();
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("order-pipeline"),
        status: ExecutionOverallStatus::Succeeded,
        start_time: now,
        end_time: Some(now),
        steps: vec![(
            NodeId::new(),
            StepRecord {
                step_name: StepName::new("fetch"),
                step_type: StepType::new("service-call"),
                status: ExecutionState::Completed,
                start_time: Some(now),
                end_time: Some(now),
                attempt: AttemptNumber::first(),
                input: None,
                output: StepOutput::success(json!({"ok": true})),
            },
        )],
        steps_completed: StepCount(1),
        steps_failed: StepCount::zero(),
    };
    let json = serde_json::to_string(&record).unwrap();
    let parsed: ExecutionRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, record);
}

#[test]
fn given_execution_record_when_duration_calculated_then_correct() {
    let now = Utc::now();
    let later = now + chrono::Duration::milliseconds(1500);
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("test"),
        status: ExecutionOverallStatus::Succeeded,
        start_time: now,
        end_time: Some(later),
        steps: vec![],
        steps_completed: StepCount::zero(),
        steps_failed: StepCount::zero(),
    };
    assert_eq!(record.duration_ms(), Some(1500));
}

#[test]
fn given_running_record_when_duration_calculated_then_none() {
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("test"),
        status: ExecutionOverallStatus::Running,
        start_time: Utc::now(),
        end_time: None,
        steps: vec![],
        steps_completed: StepCount::zero(),
        steps_failed: StepCount::zero(),
    };
    assert_eq!(record.duration_ms(), None);
}

#[test]
fn given_terminal_record_when_is_frozen_then_true() {
    for status in [
        ExecutionOverallStatus::Succeeded,
        ExecutionOverallStatus::Failed,
        ExecutionOverallStatus::Cancelled,
    ] {
        let record = ExecutionRecord {
            id: ExecutionRecordId::new(),
            workflow_name: WorkflowName::new("test"),
            status,
            start_time: Utc::now(),
            end_time: None,
            steps: vec![],
            steps_completed: StepCount::zero(),
            steps_failed: StepCount::zero(),
        };
        assert!(record.is_frozen(), "{status:?} should be frozen");
    }
}

#[test]
fn given_running_record_when_is_frozen_then_false() {
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("test"),
        status: ExecutionOverallStatus::Running,
        start_time: Utc::now(),
        end_time: None,
        steps: vec![],
        steps_completed: StepCount::zero(),
        steps_failed: StepCount::zero(),
    };
    assert!(!record.is_frozen());
}

#[test]
fn given_record_with_steps_when_step_for_node_then_found() {
    let node_id = NodeId::new();
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("test"),
        status: ExecutionOverallStatus::Succeeded,
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        steps: vec![(node_id, StepRecord::new(StepName::new("step-a"), StepType::new("handler")))],
        steps_completed: StepCount(1),
        steps_failed: StepCount::zero(),
    };
    let found = record.step_for_node(node_id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().step_name.as_str(), "step-a");
}

#[test]
fn given_record_when_step_for_missing_node_then_none() {
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("test"),
        status: ExecutionOverallStatus::Succeeded,
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        steps: vec![],
        steps_completed: StepCount::zero(),
        steps_failed: StepCount::zero(),
    };
    assert!(record.step_for_node(NodeId::new()).is_none());
}

#[test]
fn given_multiple_records_when_sorted_by_start_time_then_ordered() {
    let base = Utc::now();
    let mut records: Vec<ExecutionRecord> = vec![
        ExecutionRecord {
            id: ExecutionRecordId::new(),
            workflow_name: WorkflowName::new("c"),
            status: ExecutionOverallStatus::Succeeded,
            start_time: base + chrono::Duration::seconds(2),
            end_time: None,
            steps: vec![],
            steps_completed: StepCount::zero(),
            steps_failed: StepCount::zero(),
        },
        ExecutionRecord {
            id: ExecutionRecordId::new(),
            workflow_name: WorkflowName::new("a"),
            status: ExecutionOverallStatus::Succeeded,
            start_time: base,
            end_time: None,
            steps: vec![],
            steps_completed: StepCount::zero(),
            steps_failed: StepCount::zero(),
        },
        ExecutionRecord {
            id: ExecutionRecordId::new(),
            workflow_name: WorkflowName::new("b"),
            status: ExecutionOverallStatus::Succeeded,
            start_time: base + chrono::Duration::seconds(1),
            end_time: None,
            steps: vec![],
            steps_completed: StepCount::zero(),
            steps_failed: StepCount::zero(),
        },
    ];
    records.sort_by_key(|r| r.start_time);
    assert_eq!(records[0].workflow_name.as_str(), "a");
    assert_eq!(records[1].workflow_name.as_str(), "b");
    assert_eq!(records[2].workflow_name.as_str(), "c");
}

#[test]
fn given_record_with_multiple_steps_when_serialized_then_roundtrips() {
    let now = Utc::now();
    let record = ExecutionRecord {
        id: ExecutionRecordId::new(),
        workflow_name: WorkflowName::new("multi-step"),
        status: ExecutionOverallStatus::Failed,
        start_time: now,
        end_time: Some(now + chrono::Duration::seconds(10)),
        steps: vec![
            (
                NodeId::new(),
                StepRecord {
                    step_name: StepName::new("fetch"),
                    step_type: StepType::new("service-call"),
                    status: ExecutionState::Completed,
                    start_time: Some(now),
                    end_time: Some(now),
                    attempt: AttemptNumber::first(),
                    input: Some(json!({"url": "https://api.example.com"})),
                    output: StepOutput::success(json!({"code": 200})),
                },
            ),
            (
                NodeId::new(),
                StepRecord {
                    step_name: StepName::new("transform"),
                    step_type: StepType::new("handler"),
                    status: ExecutionState::Failed,
                    start_time: Some(now),
                    end_time: Some(now),
                    attempt: AttemptNumber(3),
                    input: None,
                    output: StepOutput::failure(ExecutionError::new("invalid data")),
                },
            ),
        ],
        steps_completed: StepCount(1),
        steps_failed: StepCount(1),
    };
    let json = serde_json::to_string(&record).unwrap();
    let parsed: ExecutionRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, record);
    assert_eq!(parsed.steps.len(), 2);
}
