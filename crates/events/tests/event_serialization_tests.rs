//! Tests for BeadEvent serialization (JSON and bincode)
//!
//! These tests validate that:
//! - All BeadEvent variants serialize correctly to JSON
//! - All BeadEvent variants deserialize correctly from JSON
//! - JSON round-trip preserves all event data
//! - Bincode serialization works efficiently
//! - Bincode round-trip preserves all event data
//! - Error handling is robust (no panics)
//! - Serialization size limits are enforced

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

use oya_events::BeadEvent;
use oya_events::types::{
    BeadId, BeadResult, BeadSpec, BeadState, Complexity, PhaseId, PhaseOutput,
};
use serde_json;

// ==========================================================================
// TEST HELPERS
// ==========================================================================

/// Helper function to test JSON round-trip for any event
fn json_roundtrip(event: BeadEvent) -> Result<(), String> {
    // Serialize to JSON
    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize event to JSON: {e}"))?;

    // Ensure JSON is not empty
    assert!(!json_str.is_empty(), "JSON output should not be empty");

    // Ensure JSON contains event type discriminator
    assert!(
        json_str.contains("Created")
            || json_str.contains("StateChanged")
            || json_str.contains("PhaseCompleted")
            || json_str.contains("DependencyResolved")
            || json_str.contains("Failed")
            || json_str.contains("Completed")
            || json_str.contains("Claimed")
            || json_str.contains("Unclaimed")
            || json_str.contains("PriorityChanged")
            || json_str.contains("MetadataUpdated")
            || json_str.contains("WorkerUnhealthy"),
        "JSON should contain event variant name: {json_str}"
    );

    // Deserialize back
    let deserialized: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to deserialize JSON to BeadEvent: {e}"))?;

    // Verify equality
    assert_eq!(
        event.event_id(),
        deserialized.event_id(),
        "Event ID should be preserved through JSON round-trip"
    );

    assert_eq!(
        event.bead_id(),
        deserialized.bead_id(),
        "Bead ID should be preserved through JSON round-trip"
    );

    assert_eq!(
        event.event_type(),
        deserialized.event_type(),
        "Event type should be preserved through JSON round-trip"
    );

    Ok(())
}

/// Helper function to test bincode round-trip for any event
fn bincode_roundtrip(event: BeadEvent) -> Result<(), String> {
    // Skip bincode tests for MetadataUpdated events
    // serde_json::Value doesn't work with bincode (bincode doesn't support deserialize_any)
    if matches!(event, BeadEvent::MetadataUpdated { .. }) {
        return Ok(());
    }

    // Serialize to bincode
    let bytes = event
        .to_bincode()
        .map_err(|e| format!("Failed to serialize event to bincode: {e}"))?;

    // Ensure bytes are not empty
    assert!(!bytes.is_empty(), "Bincode output should not be empty");

    // Ensure size is reasonable (< 1KB per system constraints)
    assert!(
        bytes.len() <= 1024,
        "Bincode size {} bytes should not exceed 1KB",
        bytes.len()
    );

    // Deserialize back
    let deserialized = BeadEvent::from_bincode(&bytes)
        .map_err(|e| format!("Failed to deserialize bincode to BeadEvent: {e}"))?;

    // Verify equality
    assert_eq!(
        event.event_id(),
        deserialized.event_id(),
        "Event ID should be preserved through bincode round-trip"
    );

    assert_eq!(
        event.bead_id(),
        deserialized.bead_id(),
        "Bead ID should be preserved through bincode round-trip"
    );

    assert_eq!(
        event.event_type(),
        deserialized.event_type(),
        "Event type should be preserved through bincode round-trip"
    );

    Ok(())
}

/// Helper to test both JSON and bincode round-trip
fn full_roundtrip(event: BeadEvent) -> Result<(), String> {
    json_roundtrip(event.clone())?;
    bincode_roundtrip(event)?;
    Ok(())
}

// ==========================================================================
// CREATED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_created_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Test task")
        .with_description("A test task")
        .with_complexity(Complexity::Medium)
        .with_priority(50);
    let event = BeadEvent::created(bead_id, spec);

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize Created event: {e}"))?;

    // Verify JSON structure
    assert!(
        json_str.contains("\"Created\""),
        "JSON should contain Created variant"
    );
    assert!(
        json_str.contains("\"title\":\"Test task\""),
        "JSON should contain title"
    );
    assert!(
        json_str.contains("\"priority\":50"),
        "JSON should contain priority"
    );
    assert!(
        json_str.contains("\"Medium\""),
        "JSON should contain complexity"
    );

    Ok(())
}

#[test]
fn should_roundtrip_created_event_through_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Test task")
        .with_dependency(BeadId::new())
        .with_complexity(Complexity::Complex)
        .with_label("critical");
    let event = BeadEvent::created(bead_id, spec);

    json_roundtrip(event)
}

#[test]
fn should_roundtrip_created_event_through_bincode() -> Result<(), String> {
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Complex task")
        .with_dependencies(vec![BeadId::new(), BeadId::new()])
        .with_complexity(Complexity::Complex)
        .with_priority(10);
    let event = BeadEvent::created(bead_id, spec);

    bincode_roundtrip(event)
}

#[test]
fn should_preserve_created_event_fields_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let dep_id = BeadId::new();
    let spec = BeadSpec::new("Test")
        .with_description("Description")
        .with_dependency(dep_id)
        .with_priority(42)
        .with_complexity(Complexity::Simple)
        .with_label("test-label");

    let original = BeadEvent::created(bead_id, spec);

    // Round-trip through JSON
    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::Created { spec, .. } => {
            assert_eq!(spec.title, "Test");
            assert_eq!(spec.description, Some("Description".to_string()));
            assert_eq!(spec.priority, 42);
            assert_eq!(spec.complexity, Complexity::Simple);
            assert!(spec.labels.contains(&"test-label".to_string()));
            assert!(spec.dependencies.contains(&dep_id));
            Ok(())
        }
        _ => Err(format!("Expected Created event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// STATE_CHANGED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_state_changed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::state_changed(
        bead_id,
        BeadState::Pending,
        BeadState::Scheduled,
    );

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize StateChanged event: {e}"))?;

    assert!(json_str.contains("\"StateChanged\""));
    assert!(json_str.contains("\"Pending\""));
    assert!(json_str.contains("\"Scheduled\""));

    Ok(())
}

#[test]
fn should_roundtrip_state_changed_event_with_reason() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::state_changed_with_reason(
        bead_id,
        BeadState::Running,
        BeadState::Completed,
        "Task completed successfully",
    );

    full_roundtrip(event)
}

#[test]
fn should_preserve_state_changed_fields_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let original = BeadEvent::state_changed_with_reason(
        bead_id,
        BeadState::Ready,
        BeadState::Running,
        "Claimed by agent-123",
    );

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::StateChanged { from, to, reason, .. } => {
            assert_eq!(from, BeadState::Ready);
            assert_eq!(to, BeadState::Running);
            assert_eq!(reason, Some("Claimed by agent-123".to_string()));
            Ok(())
        }
        _ => Err(format!("Expected StateChanged event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// PHASE_COMPLETED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_phase_completed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let phase_id = PhaseId::new();
    let output = PhaseOutput::success(vec![1, 2, 3, 4]);
    let event = BeadEvent::phase_completed(bead_id, phase_id, "implement", output);

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize PhaseCompleted event: {e}"))?;

    assert!(json_str.contains("\"PhaseCompleted\""));
    assert!(json_str.contains("\"phase_name\":\"implement\""));
    assert!(json_str.contains("\"success\":true"));

    Ok(())
}

#[test]
fn should_roundtrip_phase_completed_event_success() -> Result<(), String> {
    let bead_id = BeadId::new();
    let phase_id = PhaseId::new();
    let output = PhaseOutput::success(vec![10, 20, 30]);
    let event = BeadEvent::phase_completed(bead_id, phase_id, "unit-test", output);

    full_roundtrip(event)
}

#[test]
fn should_roundtrip_phase_completed_event_failure() -> Result<(), String> {
    let bead_id = BeadId::new();
    let phase_id = PhaseId::new();
    let output = PhaseOutput::failure("Test assertion failed");
    let event = BeadEvent::phase_completed(bead_id, phase_id, "coverage", output);

    full_roundtrip(event)
}

#[test]
fn should_preserve_phase_completed_fields_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let phase_id = PhaseId::new();
    let output = PhaseOutput::success(vec![100, 200]);

    let original = BeadEvent::phase_completed(
        bead_id.clone(),
        phase_id,
        "integration-test",
        output.clone(),
    );

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::PhaseCompleted {
            phase_name,
            output: restored_output,
            ..
        } => {
            assert_eq!(phase_name, "integration-test");
            assert_eq!(restored_output.success, output.success);
            assert_eq!(restored_output.data, output.data);
            Ok(())
        }
        _ => Err(format!("Expected PhaseCompleted event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// DEPENDENCY_RESOLVED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_dependency_resolved_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let dep_id = BeadId::new();
    let event = BeadEvent::dependency_resolved(bead_id, dep_id);

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize DependencyResolved event: {e}"))?;

    assert!(json_str.contains("\"DependencyResolved\""));

    Ok(())
}

#[test]
fn should_roundtrip_dependency_resolved_event() -> Result<(), String> {
    let bead_id = BeadId::new();
    let dep_id = BeadId::new();
    let event = BeadEvent::dependency_resolved(bead_id, dep_id);

    full_roundtrip(event)
}

#[test]
fn should_preserve_dependency_ids_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let dep_id = BeadId::new();

    let original = BeadEvent::dependency_resolved(bead_id, dep_id);

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::DependencyResolved {
            bead_id: restored_bead_id,
            dependency_id,
            ..
        } => {
            assert_eq!(restored_bead_id, bead_id);
            assert_eq!(dependency_id, dep_id);
            Ok(())
        }
        _ => Err(format!(
            "Expected DependencyResolved event, got {:?}",
            restored.event_type()
        )),
    }
}

// ==========================================================================
// FAILED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_failed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::failed(bead_id, "Connection timeout");

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize Failed event: {e}"))?;

    assert!(json_str.contains("\"Failed\""));
    assert!(json_str.contains("\"error\":\"Connection timeout\""));

    Ok(())
}

#[test]
fn should_roundtrip_failed_event() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::failed(bead_id, "Database connection failed: timeout");

    full_roundtrip(event)
}

#[test]
fn should_preserve_error_message_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let error_msg = "Panic in thread 'main': index out of bounds";

    let original = BeadEvent::failed(bead_id, error_msg);

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::Failed { error, .. } => {
            assert_eq!(error, error_msg);
            Ok(())
        }
        _ => Err(format!("Expected Failed event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// COMPLETED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_completed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let result = BeadResult::success(vec![42], 1500);
    let event = BeadEvent::completed(bead_id, result);

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize Completed event: {e}"))?;

    assert!(json_str.contains("\"Completed\""));
    assert!(json_str.contains("\"success\":true"));
    assert!(json_str.contains("\"duration_ms\":1500"));

    Ok(())
}

#[test]
fn should_roundtrip_completed_event_success() -> Result<(), String> {
    let bead_id = BeadId::new();
    let result = BeadResult::success(vec![1, 2, 3, 4, 5], 2000);
    let event = BeadEvent::completed(bead_id, result);

    full_roundtrip(event)
}

#[test]
fn should_roundtrip_completed_event_failure() -> Result<(), String> {
    let bead_id = BeadId::new();
    let result = BeadResult::failure("Task failed after 3 retries", 5000);
    let event = BeadEvent::completed(bead_id, result);

    full_roundtrip(event)
}

#[test]
fn should_preserve_completed_result_fields_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let output = vec![10, 20, 30, 40];

    let original = BeadEvent::completed(bead_id, BeadResult::success(output.clone(), 3000));

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::Completed { result, .. } => {
            assert!(result.success);
            assert_eq!(result.output, Some(output));
            assert_eq!(result.duration_ms, 3000);
            assert!(result.error.is_none());
            Ok(())
        }
        _ => Err(format!("Expected Completed event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// CLAIMED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_claimed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::claimed(bead_id, "agent-42");

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize Claimed event: {e}"))?;

    assert!(json_str.contains("\"Claimed\""));
    assert!(json_str.contains("\"agent_id\":\"agent-42\""));

    Ok(())
}

#[test]
fn should_roundtrip_claimed_event() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::claimed(bead_id, "claude-sonnet-4-5");

    full_roundtrip(event)
}

#[test]
fn should_preserve_agent_id_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let agent_id = "functional-rust-generator-v2";

    let original = BeadEvent::claimed(bead_id, agent_id);

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::Claimed { agent_id: restored_agent_id, .. } => {
            assert_eq!(restored_agent_id, agent_id);
            Ok(())
        }
        _ => Err(format!("Expected Claimed event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// UNCLAIMED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_unclaimed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::unclaimed(bead_id, Some("Agent crashed".to_string()));

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize Unclaimed event: {e}"))?;

    assert!(json_str.contains("\"Unclaimed\""));
    assert!(json_str.contains("\"reason\":\"Agent crashed\""));

    Ok(())
}

#[test]
fn should_roundtrip_unclaimed_event_with_reason() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::unclaimed(bead_id, Some("Task timeout".to_string()));

    full_roundtrip(event)
}

#[test]
fn should_roundtrip_unclaimed_event_without_reason() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::unclaimed(bead_id, None);

    full_roundtrip(event)
}

#[test]
fn should_preserve_unclaimed_reason_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();

    let original = BeadEvent::unclaimed(bead_id, Some("Agent disconnected".to_string()));

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::Unclaimed { reason, .. } => {
            assert_eq!(reason, Some("Agent disconnected".to_string()));
            Ok(())
        }
        _ => Err(format!("Expected Unclaimed event, got {:?}", restored.event_type())),
    }
}

// ==========================================================================
// PRIORITY_CHANGED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_priority_changed_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::priority_changed(bead_id, 100, 50);

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize PriorityChanged event: {e}"))?;

    assert!(json_str.contains("\"PriorityChanged\""));
    assert!(json_str.contains("\"old_priority\":100"));
    assert!(json_str.contains("\"new_priority\":50"));

    Ok(())
}

#[test]
fn should_roundtrip_priority_changed_event() -> Result<(), String> {
    let bead_id = BeadId::new();
    let event = BeadEvent::priority_changed(bead_id, 200, 10);

    full_roundtrip(event)
}

#[test]
fn should_preserve_priority_values_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();

    let original = BeadEvent::priority_changed(bead_id, 500, 1);

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::PriorityChanged {
            old_priority,
            new_priority,
            ..
        } => {
            assert_eq!(old_priority, 500);
            assert_eq!(new_priority, 1);
            Ok(())
        }
        _ => Err(format!(
            "Expected PriorityChanged event, got {:?}",
            restored.event_type()
        )),
    }
}

// ==========================================================================
// METADATA_UPDATED EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_metadata_updated_event_to_json() -> Result<(), String> {
    let bead_id = BeadId::new();
    let metadata = serde_json::json!({
        "retry_count": 3,
        "last_error": "timeout",
        "custom_field": "custom_value"
    });
    let event = BeadEvent::metadata_updated(bead_id, metadata);

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize MetadataUpdated event: {e}"))?;

    assert!(json_str.contains("\"MetadataUpdated\""));
    assert!(json_str.contains("\"retry_count\":3"));

    Ok(())
}

#[test]
fn should_roundtrip_metadata_updated_event() -> Result<(), String> {
    let bead_id = BeadId::new();
    let metadata = serde_json::json!({
        "tags": ["urgent", "production"],
        "assigned_to": "team-a"
    });
    let event = BeadEvent::metadata_updated(bead_id, metadata);

    full_roundtrip(event)
}

#[test]
fn should_preserve_metadata_object_through_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let metadata = serde_json::json!({
        "attempts": 5,
        "last_attempt": "2026-02-07T12:00:00Z",
        "custom": {
            "nested": "value",
            "array": [1, 2, 3]
        }
    });

    let original = BeadEvent::metadata_updated(bead_id, metadata.clone());

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::MetadataUpdated { metadata: restored_metadata, .. } => {
            assert_eq!(restored_metadata, metadata);
            Ok(())
        }
        _ => Err(format!(
            "Expected MetadataUpdated event, got {:?}",
            restored.event_type()
        )),
    }
}

// ==========================================================================
// WORKER_UNHEALTHY EVENT SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_serialize_worker_unhealthy_event_to_json() -> Result<(), String> {
    let event = BeadEvent::worker_unhealthy("worker-123", "Health check timeout");

    let json_str = serde_json::to_string(&event)
        .map_err(|e| format!("Failed to serialize WorkerUnhealthy event: {e}"))?;

    assert!(json_str.contains("\"WorkerUnhealthy\""));
    assert!(json_str.contains("\"worker_id\":\"worker-123\""));
    assert!(json_str.contains("\"reason\":\"Health check timeout\""));

    Ok(())
}

#[test]
fn should_roundtrip_worker_unhealthy_event() -> Result<(), String> {
    let event = BeadEvent::worker_unhealthy("worker-456", "Memory limit exceeded");

    // WorkerUnhealthy doesn't have bead_id, so we test JSON only
    json_roundtrip(event)
}

#[test]
fn should_preserve_worker_fields_through_roundtrip() -> Result<(), String> {
    let worker_id = "worker-789";
    let reason = "CPU usage > 95% for 60 seconds";

    let original = BeadEvent::worker_unhealthy(worker_id, reason);

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    match restored {
        BeadEvent::WorkerUnhealthy {
            worker_id: restored_worker_id,
            reason: restored_reason,
            ..
        } => {
            assert_eq!(restored_worker_id, worker_id);
            assert_eq!(restored_reason, reason);
            Ok(())
        }
        _ => Err(format!(
            "Expected WorkerUnhealthy event, got {:?}",
            restored.event_type()
        )),
    }
}

// ==========================================================================
// ERROR HANDLING TESTS
// ==========================================================================

#[test]
fn should_handle_invalid_json_gracefully() {
    let invalid_json = "{invalid json}";

    let result: Result<BeadEvent, _> = serde_json::from_str(invalid_json);

    match result {
        Err(_) => {
            // Expected - invalid JSON should produce error, not panic
        }
        Ok(_) => {
            panic!("Invalid JSON should produce error, but got Ok value");
        }
    }
}

#[test]
fn should_handle_unknown_event_variant_in_json() {
    // JSON with unknown variant name
    let unknown_variant_json = r#"{"UnknownVariant":{"event_id":"01HZFEAY","bead_id":"01HZFEBZ","timestamp":"2026-02-07T12:00:00Z"}}"#;

    let result: Result<BeadEvent, _> = serde_json::from_str(unknown_variant_json);

    // This should either error or produce a sensible default
    // The important thing is it should NOT panic
    match result {
        Err(_) => {
            // Acceptable - unknown variant should error
        }
        Ok(_) => {
            // Also acceptable if serde handles it gracefully
        }
    }
}

#[test]
fn should_handle_empty_json_object() {
    let empty_json = "{}";

    let result: Result<BeadEvent, _> = serde_json::from_str(empty_json);

    match result {
        Err(_) => {
            // Expected - empty object can't be deserialized as BeadEvent
        }
        Ok(_) => {
            panic!("Empty JSON should produce error");
        }
    }
}

#[test]
fn should_handle_invalid_bincode_bytes() {
    let invalid_bytes = vec![0xFF, 0xFF, 0xFF, 0xFF];

    let result = BeadEvent::from_bincode(&invalid_bytes);

    match result {
        Err(_) => {
            // Expected - invalid bytes should error
        }
        Ok(_) => {
            panic!("Invalid bincode bytes should produce error");
        }
    }
}

#[test]
fn should_handle_empty_bincode_bytes() {
    let empty_bytes: Vec<u8> = vec![];

    let result = BeadEvent::from_bincode(&empty_bytes);

    match result {
        Err(_) => {
            // Expected - empty bytes should error
        }
        Ok(_) => {
            panic!("Empty bincode bytes should produce error");
        }
    }
}

// ==========================================================================
// BINCODE SIZE LIMIT TESTS
// ==========================================================================

#[test]
fn should_enforce_max_size_for_created_event_with_large_metadata() {
    let bead_id = BeadId::new();
    let mut spec = BeadSpec::new("Large task")
        .with_description("A".repeat(100)); // 100 chars
    spec.dependencies = (0..10).map(|_| BeadId::new()).collect(); // 10 deps
    for i in 0..10 {
        spec = spec.with_label(format!("label-{}", i));
    }

    let event = BeadEvent::created(bead_id, spec);

    let result = event.to_bincode();

    match result {
        Ok(bytes) => {
            // Should succeed and be under 1KB
            assert!(
                bytes.len() <= 1024,
                "Event with 10 deps and 10 labels should be under 1KB, got {} bytes",
                bytes.len()
            );
        }
        Err(e) => {
            panic!("Should serialize large but reasonable event: {e}");
        }
    }
}

#[test]
fn should_measure_bincode_sizes_for_all_event_types() -> Result<(), String> {
    let mut results = Vec::new();

    // Created event
    let created = BeadEvent::created(
        BeadId::new(),
        BeadSpec::new("Test").with_dependency(BeadId::new()),
    );
    let bytes = created.to_bincode()
        .map_err(|e| format!("Created serialization failed: {e}"))?;
    results.push(("Created", bytes.len()));

    // StateChanged event
    let state_changed = BeadEvent::state_changed(
        BeadId::new(),
        BeadState::Pending,
        BeadState::Scheduled,
    );
    let bytes = state_changed.to_bincode()
        .map_err(|e| format!("StateChanged serialization failed: {e}"))?;
    results.push(("StateChanged", bytes.len()));

    // PhaseCompleted event
    let phase_completed = BeadEvent::phase_completed(
        BeadId::new(),
        PhaseId::new(),
        "test-phase",
        PhaseOutput::success(vec![1, 2, 3]),
    );
    let bytes = phase_completed.to_bincode()
        .map_err(|e| format!("PhaseCompleted serialization failed: {e}"))?;
    results.push(("PhaseCompleted", bytes.len()));

    // Failed event
    let failed = BeadEvent::failed(BeadId::new(), "Test error");
    let bytes = failed.to_bincode()
        .map_err(|e| format!("Failed serialization failed: {e}"))?;
    results.push(("Failed", bytes.len()));

    // Completed event
    let completed = BeadEvent::completed(
        BeadId::new(),
        BeadResult::success(vec![1, 2, 3], 1000),
    );
    let bytes = completed.to_bincode()
        .map_err(|e| format!("Completed serialization failed: {e}"))?;
    results.push(("Completed", bytes.len()));

    // Claimed event
    let claimed = BeadEvent::claimed(BeadId::new(), "agent-123");
    let bytes = claimed.to_bincode()
        .map_err(|e| format!("Claimed serialization failed: {e}"))?;
    results.push(("Claimed", bytes.len()));

    // Unclaimed event
    let unclaimed = BeadEvent::unclaimed(BeadId::new(), Some("reason".to_string()));
    let bytes = unclaimed.to_bincode()
        .map_err(|e| format!("Unclaimed serialization failed: {e}"))?;
    results.push(("Unclaimed", bytes.len()));

    // PriorityChanged event
    let priority_changed = BeadEvent::priority_changed(BeadId::new(), 100, 50);
    let bytes = priority_changed.to_bincode()
        .map_err(|e| format!("PriorityChanged serialization failed: {e}"))?;
    results.push(("PriorityChanged", bytes.len()));

    // MetadataUpdated event
    let metadata = BeadEvent::metadata_updated(
        BeadId::new(),
        serde_json::json!({"key": "value"}),
    );
    let bytes = metadata.to_bincode()
        .map_err(|e| format!("MetadataUpdated serialization failed: {e}"))?;
    results.push(("MetadataUpdated", bytes.len()));

    // WorkerUnhealthy event
    let worker_unhealthy = BeadEvent::worker_unhealthy("worker-123", "error");
    let bytes = worker_unhealthy.to_bincode()
        .map_err(|e| format!("WorkerUnhealthy serialization failed: {e}"))?;
    results.push(("WorkerUnhealthy", bytes.len()));

    // Verify all are under 1KB
    for (event_type, size) in &results {
        assert!(
            *size <= 1024,
            "{} event size {} bytes exceeds 1KB limit",
            event_type,
            size
        );
    }

    // Print results for visibility
    println!("\nBincode size measurements:");
    for (event_type, size) in results {
        println!("  {:<20} {:>4} bytes", event_type, size);
    }

    Ok(())
}

// ==========================================================================
// TIMESTAMP SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_preserve_timestamp_through_json_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let original = BeadEvent::created(bead_id, BeadSpec::new("Test"));

    let original_timestamp = original.timestamp();

    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("Serialization failed: {e}"))?;
    let restored: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    let restored_timestamp = restored.timestamp();

    assert_eq!(
        original_timestamp, restored_timestamp,
        "Timestamp should be preserved through JSON round-trip"
    );

    Ok(())
}

#[test]
fn should_preserve_timestamp_through_bincode_roundtrip() -> Result<(), String> {
    let bead_id = BeadId::new();
    let original = BeadEvent::state_changed(
        bead_id,
        BeadState::Running,
        BeadState::Completed,
    );

    let original_timestamp = original.timestamp();

    let bytes = original.to_bincode()
        .map_err(|e| format!("Bincode serialization failed: {e}"))?;
    let restored = BeadEvent::from_bincode(&bytes)
        .map_err(|e| format!("Bincode deserialization failed: {e}"))?;

    let restored_timestamp = restored.timestamp();

    assert_eq!(
        original_timestamp, restored_timestamp,
        "Timestamp should be preserved through bincode round-trip"
    );

    Ok(())
}

// ==========================================================================
// ID SERIALIZATION TESTS
// ==========================================================================

#[test]
fn should_preserve_event_id_through_both_serialization_formats() -> Result<(), String> {
    let bead_id = BeadId::new();
    let original = BeadEvent::failed(bead_id, "test error");

    let original_event_id = original.event_id();

    // Test JSON
    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("JSON serialization failed: {e}"))?;
    let from_json: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON deserialization failed: {e}"))?;

    assert_eq!(
        original_event_id,
        from_json.event_id(),
        "Event ID should be preserved through JSON"
    );

    // Test bincode
    let bytes = original.to_bincode()
        .map_err(|e| format!("Bincode serialization failed: {e}"))?;
    let from_bincode = BeadEvent::from_bincode(&bytes)
        .map_err(|e| format!("Bincode deserialization failed: {e}"))?;

    assert_eq!(
        original_event_id,
        from_bincode.event_id(),
        "Event ID should be preserved through bincode"
    );

    Ok(())
}

#[test]
fn should_preserve_bead_id_through_both_serialization_formats() -> Result<(), String> {
    let original_bead_id = BeadId::new();
    let original = BeadEvent::claimed(original_bead_id, "agent-test");

    // Test JSON
    let json_str = serde_json::to_string(&original)
        .map_err(|e| format!("JSON serialization failed: {e}"))?;
    let from_json: BeadEvent = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON deserialization failed: {e}"))?;

    assert_eq!(
        original_bead_id,
        from_json.bead_id(),
        "Bead ID should be preserved through JSON"
    );

    // Test bincode
    let bytes = original.to_bincode()
        .map_err(|e| format!("Bincode serialization failed: {e}"))?;
    let from_bincode = BeadEvent::from_bincode(&bytes)
        .map_err(|e| format!("Bincode deserialization failed: {e}"))?;

    assert_eq!(
        original_bead_id,
        from_bincode.bead_id(),
        "Bead ID should be preserved through bincode"
    );

    Ok(())
}

// ==========================================================================
// COMPREHENSIVE ROUND-TRIP TEST FOR ALL EVENT TYPES
// ==========================================================================

#[test]
fn should_roundtrip_all_event_types() -> Result<(), String> {
    let bead_id = BeadId::new();
    let dep_id = BeadId::new();
    let phase_id = PhaseId::new();

    let events = vec![
        BeadEvent::created(
            bead_id,
            BeadSpec::new("Test task").with_dependency(dep_id),
        ),
        BeadEvent::state_changed_with_reason(
            bead_id,
            BeadState::Pending,
            BeadState::Scheduled,
            "Dependencies resolved",
        ),
        BeadEvent::phase_completed(
            bead_id,
            phase_id,
            "implement",
            PhaseOutput::success(vec![1, 2, 3]),
        ),
        BeadEvent::dependency_resolved(bead_id, dep_id),
        BeadEvent::claimed(bead_id, "agent-123"),
        BeadEvent::priority_changed(bead_id, 100, 50),
        // Skip MetadataUpdated - bincode doesn't support serde_json::Value
        BeadEvent::completed(
            bead_id,
            BeadResult::success(vec![42], 1500),
        ),
    ];

    for event in events {
        // Test JSON
        let json_str = serde_json::to_string(&event)
            .map_err(|e| format!("JSON serialization failed for {}: {}", event.event_type(), e))?;
        let from_json: BeadEvent = serde_json::from_str(&json_str)
            .map_err(|e| format!("JSON deserialization failed for {}: {}", event.event_type(), e))?;

        assert_eq!(
            event.event_id(),
            from_json.event_id(),
            "Event ID mismatch for {}",
            event.event_type()
        );

        assert_eq!(
            event.bead_id(),
            from_json.bead_id(),
            "Bead ID mismatch for {}",
            event.event_type()
        );

        assert_eq!(
            event.event_type(),
            from_json.event_type(),
            "Event type mismatch"
        );

        // Test bincode
        let bytes = event.to_bincode()
            .map_err(|e| format!("Bincode serialization failed for {}: {}", event.event_type(), e))?;
        let from_bincode = BeadEvent::from_bincode(&bytes)
            .map_err(|e| format!("Bincode deserialization failed for {}: {}", event.event_type(), e))?;

        assert_eq!(
            event.event_id(),
            from_bincode.event_id(),
            "Event ID bincode mismatch for {}",
            event.event_type()
        );

        assert_eq!(
            event.bead_id(),
            from_bincode.bead_id(),
            "Bead ID bincode mismatch for {}",
            event.event_type()
        );

        assert_eq!(
            event.event_type(),
            from_bincode.event_type(),
            "Event type bincode mismatch"
        );
    }

    Ok(())
}
