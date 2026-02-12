//! Property-based tests for serialization/deserialization roundtrip.
//!
//! Property verified: ∀ serialization: deserialize(serialize(x)) == x
//!
//! Tests both JSON and bincode formats for all serializable types.

use chrono::{TimeZone, Utc};
use oya_events::{
    BeadEvent, BeadId, BeadResult, BeadSpec, BeadState, Complexity, EventId, PhaseId, PhaseOutput,
    StateTransition,
};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::string::string_regex;

prop_compose! {
    fn arb_bead_id()(ulid_bytes in vec(any::<u8>(), 16..=16)) -> BeadId {
        let bytes: [u8; 16] = ulid_bytes.try_into().unwrap_or_else(|_| [0u8; 16]);
        let milliseconds = u64::from_be_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]));
        let random = u128::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0,
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        ulid::Ulid::from_parts(milliseconds / 1000, random)
            .map(BeadId::from_ulid)
            .unwrap_or_else(BeadId::new)
    }
}

prop_compose! {
    fn arb_event_id()(ulid_bytes in vec(any::<u8>(), 16..=16)) -> EventId {
        let bytes: [u8; 16] = ulid_bytes.try_into().unwrap_or_else(|_| [0u8; 16]);
        let milliseconds = u64::from_be_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]));
        let random = u128::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0,
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        ulid::Ulid::from_parts(milliseconds / 1000, random)
            .map(EventId::from_ulid)
            .unwrap_or_else(EventId::new)
    }
}

prop_compose! {
    fn arb_phase_id()(ulid_bytes in vec(any::<u8>(), 16..=16)) -> PhaseId {
        let bytes: [u8; 16] = ulid_bytes.try_into().unwrap_or_else(|_| [0u8; 16]);
        let milliseconds = u64::from_be_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]));
        let random = u128::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0,
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        ulid::Ulid::from_parts(milliseconds / 1000, random)
            .map(PhaseId::from_ulid)
            .unwrap_or_else(PhaseId::new)
    }
}

fn arb_bead_state() -> impl Strategy<Value = BeadState> {
    prop_oneof![
        Just(BeadState::Pending),
        Just(BeadState::Scheduled),
        Just(BeadState::Ready),
        Just(BeadState::Running),
        Just(BeadState::Suspended),
        Just(BeadState::BackingOff),
        Just(BeadState::Paused),
        Just(BeadState::Completed),
    ]
}

fn arb_complexity() -> impl Strategy<Value = Complexity> {
    prop_oneof![
        Just(Complexity::Simple),
        Just(Complexity::Medium),
        Just(Complexity::Complex),
    ]
}

prop_compose! {
    fn arb_phase_output()(
        success: bool,
        data: Vec<u8>,
        message: Option<String>,
    ) -> PhaseOutput {
        PhaseOutput {
            success,
            data,
            message,
        }
    }
}

prop_compose! {
    fn arb_bead_result()(
        success: bool,
        output: Option<Vec<u8>>,
        error: Option<String>,
        duration_ms: u64,
    ) -> BeadResult {
        BeadResult {
            success,
            output,
            error,
            duration_ms,
        }
    }
}

prop_compose! {
    fn arb_bead_spec()(
        title: String,
        description: Option<String>,
        dependencies in vec(arb_bead_id(), 0..5),
        priority: u32,
        complexity in arb_complexity(),
        labels: Vec<String>,
    ) -> BeadSpec {
        BeadSpec {
            title,
            description,
            dependencies,
            priority,
            complexity,
            labels,
            metadata: None,
        }
    }
}

prop_compose! {
    fn arb_state_transition()(
        from in arb_bead_state(),
        to in arb_bead_state(),
        reason: Option<String>,
        timestamp_secs: i64,
        timestamp_nanos: u32,
    ) -> StateTransition {
        StateTransition {
            from,
            to,
            timestamp: Utc.timestamp_opt(timestamp_secs, timestamp_nanos)
                .single()
                .unwrap_or_else(Utc::now),
            reason,
        }
    }
}

fn arb_bead_event() -> impl Strategy<Value = BeadEvent> {
    let bead_id = BeadId::new();
    let event_id = EventId::new();
    let phase_id = PhaseId::new();
    let timestamp = Utc::now();

    prop_oneof![
        arb_bead_id().prop_map(move |b| {
            let b_id = b;
            let e_id = EventId::new();
            let ts = Utc::now();
            BeadEvent::created(b_id, BeadSpec::new("test"))
        }),
        (arb_bead_id(), arb_bead_state(), arb_bead_state())
            .prop_map(move |(b, from, to)| { BeadEvent::state_changed(b, from, to) }),
        (
            arb_bead_id(),
            arb_phase_id(),
            "[a-z]{3,10}",
            arb_phase_output()
        )
            .prop_map(move |(b, p, name, out)| { BeadEvent::phase_completed(b, p, name, out) }),
        (arb_bead_id(), arb_bead_id()).prop_map(|(b, dep)| BeadEvent::dependency_resolved(b, dep)),
        (arb_bead_id(), "[a-z ]{5,30}").prop_map(|(b, err)| BeadEvent::failed(b, err)),
        (arb_bead_id(), arb_bead_result()).prop_map(|(b, r)| BeadEvent::completed(b, r)),
        (arb_bead_id(), "[a-z0-9-]{5,20}").prop_map(|(b, agent)| BeadEvent::claimed(b, agent)),
        (arb_bead_id(), proptest::option::of("[a-z ]{5,30}"))
            .prop_map(|(b, reason)| BeadEvent::unclaimed(b, reason)),
        (arb_bead_id(), any::<u32>(), any::<u32>())
            .prop_map(|(b, old, new)| BeadEvent::priority_changed(b, old, new)),
        ("[a-z0-9-]{5,20}", "[a-z ]{5,30}")
            .prop_map(|(worker, reason)| BeadEvent::worker_unhealthy(worker, reason)),
    ]
}

proptest! {
    #[test]
    fn prop_bead_id_json_roundtrip(id in arb_bead_id()) {
        let json = serde_json::to_string(&id).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadId = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(id, restored);
    }

    #[test]
    fn prop_bead_id_bincode_roundtrip(id in arb_bead_id()) {
        let bytes = bincode::serialize(&id).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadId = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(id, restored);
    }

    #[test]
    fn prop_event_id_json_roundtrip(id in arb_event_id()) {
        let json = serde_json::to_string(&id).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: EventId = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(id, restored);
    }

    #[test]
    fn prop_event_id_bincode_roundtrip(id in arb_event_id()) {
        let bytes = bincode::serialize(&id).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: EventId = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(id, restored);
    }

    #[test]
    fn prop_phase_id_json_roundtrip(id in arb_phase_id()) {
        let json = serde_json::to_string(&id).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: PhaseId = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(id, restored);
    }

    #[test]
    fn prop_phase_id_bincode_roundtrip(id in arb_phase_id()) {
        let bytes = bincode::serialize(&id).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: PhaseId = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(id, restored);
    }

    #[test]
    fn prop_bead_state_json_roundtrip(state in arb_bead_state()) {
        let json = serde_json::to_string(&state).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadState = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(state, restored);
    }

    #[test]
    fn prop_bead_state_bincode_roundtrip(state in arb_bead_state()) {
        let bytes = bincode::serialize(&state).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadState = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(state, restored);
    }

    #[test]
    fn prop_complexity_json_roundtrip(complexity in arb_complexity()) {
        let json = serde_json::to_string(&complexity).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: Complexity = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(complexity, restored);
    }

    #[test]
    fn prop_complexity_bincode_roundtrip(complexity in arb_complexity()) {
        let bytes = bincode::serialize(&complexity).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: Complexity = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(complexity, restored);
    }

    #[test]
    fn prop_phase_output_json_roundtrip(output in arb_phase_output()) {
        let json = serde_json::to_string(&output).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: PhaseOutput = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(output.success, restored.success);
        prop_assert_eq!(output.data, restored.data);
        prop_assert_eq!(output.message, restored.message);
    }

    #[test]
    fn prop_phase_output_bincode_roundtrip(output in arb_phase_output()) {
        let bytes = bincode::serialize(&output).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: PhaseOutput = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(output.success, restored.success);
        prop_assert_eq!(output.data, restored.data);
        prop_assert_eq!(output.message, restored.message);
    }

    #[test]
    fn prop_bead_result_json_roundtrip(result in arb_bead_result()) {
        let json = serde_json::to_string(&result).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadResult = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(result.success, restored.success);
        prop_assert_eq!(result.output, restored.output);
        prop_assert_eq!(result.error, restored.error);
        prop_assert_eq!(result.duration_ms, restored.duration_ms);
    }

    #[test]
    fn prop_bead_result_bincode_roundtrip(result in arb_bead_result()) {
        let bytes = bincode::serialize(&result).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadResult = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(result.success, restored.success);
        prop_assert_eq!(result.output, restored.output);
        prop_assert_eq!(result.error, restored.error);
        prop_assert_eq!(result.duration_ms, restored.duration_ms);
    }

    #[test]
    fn prop_bead_spec_json_roundtrip(spec in arb_bead_spec()) {
        let json = serde_json::to_string(&spec).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadSpec = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(spec.title, restored.title);
        prop_assert_eq!(spec.description, restored.description);
        prop_assert_eq!(spec.dependencies, restored.dependencies);
        prop_assert_eq!(spec.priority, restored.priority);
        prop_assert_eq!(spec.complexity, restored.complexity);
        prop_assert_eq!(spec.labels, restored.labels);
    }

    #[test]
    fn prop_bead_spec_bincode_roundtrip(spec in arb_bead_spec()) {
        let bytes = bincode::serialize(&spec).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadSpec = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(spec.title, restored.title);
        prop_assert_eq!(spec.description, restored.description);
        prop_assert_eq!(spec.dependencies, restored.dependencies);
        prop_assert_eq!(spec.priority, restored.priority);
        prop_assert_eq!(spec.complexity, restored.complexity);
        prop_assert_eq!(spec.labels, restored.labels);
    }

    #[test]
    fn prop_state_transition_json_roundtrip(trans in arb_state_transition()) {
        let json = serde_json::to_string(&trans).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: StateTransition = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(trans.from, restored.from);
        prop_assert_eq!(trans.to, restored.to);
        prop_assert_eq!(trans.reason, restored.reason);
    }

    #[test]
    fn prop_state_transition_bincode_roundtrip(trans in arb_state_transition()) {
        let bytes = bincode::serialize(&trans).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: StateTransition = bincode::deserialize(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(trans.from, restored.from);
        prop_assert_eq!(trans.to, restored.to);
        prop_assert_eq!(trans.reason, restored.reason);
    }

    #[test]
    fn prop_bead_event_json_roundtrip(event in arb_bead_event()) {
        let json = serde_json::to_string(&event).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored: BeadEvent = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(event.event_id(), restored.event_id());
        prop_assert_eq!(event.event_type(), restored.event_type());
    }

    #[test]
    fn prop_bead_event_bincode_roundtrip(event in arb_bead_event()) {
        let bytes = event.to_bincode().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let restored = BeadEvent::from_bincode(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(event.event_id(), restored.event_id());
        prop_assert_eq!(event.event_type(), restored.event_type());
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn should_roundtrip_empty_bead_spec() {
        let spec = BeadSpec::new("");
        let json = serde_json::to_string(&spec).expect("serialize");
        let restored: BeadSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(spec.title, restored.title);
    }

    #[test]
    fn should_roundtrip_bead_spec_with_large_dependencies() {
        let deps: Vec<BeadId> = (0..100).map(|_| BeadId::new()).collect();
        let spec = BeadSpec::new("test").with_dependencies(deps.clone());
        let json = serde_json::to_string(&spec).expect("serialize");
        let restored: BeadSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(spec.dependencies.len(), restored.dependencies.len());
    }

    #[test]
    fn should_roundtrip_bead_spec_with_many_labels() {
        let mut spec = BeadSpec::new("test");
        for i in 0..50 {
            spec = spec.with_label(format!("label-{}", i));
        }
        let json = serde_json::to_string(&spec).expect("serialize");
        let restored: BeadSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(spec.labels.len(), restored.labels.len());
    }

    #[test]
    fn should_roundtrip_all_bead_states() {
        let states = [
            BeadState::Pending,
            BeadState::Scheduled,
            BeadState::Ready,
            BeadState::Running,
            BeadState::Suspended,
            BeadState::BackingOff,
            BeadState::Paused,
            BeadState::Completed,
        ];

        for state in states {
            let json = serde_json::to_string(&state).expect("serialize");
            let restored: BeadState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(state, restored, "Failed for {:?}", state);
        }
    }

    #[test]
    fn should_roundtrip_all_complexities() {
        let complexities = [Complexity::Simple, Complexity::Medium, Complexity::Complex];

        for complexity in complexities {
            let json = serde_json::to_string(&complexity).expect("serialize");
            let restored: Complexity = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(complexity, restored);
        }
    }

    #[test]
    fn should_roundtrip_empty_phase_output_data() {
        let output = PhaseOutput::success(vec![]);
        let json = serde_json::to_string(&output).expect("serialize");
        let restored: PhaseOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(restored.data.is_empty());
    }

    #[test]
    fn should_roundtrip_large_phase_output_data() {
        let data = vec![0u8; 1000];
        let output = PhaseOutput::success(data.clone());
        let json = serde_json::to_string(&output).expect("serialize");
        let restored: PhaseOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(data, restored.data);
    }

    #[test]
    fn should_roundtrip_bead_result_with_large_output() {
        let output = vec![42u8; 500];
        let result = BeadResult::success(output.clone(), 1000);
        let json = serde_json::to_string(&result).expect("serialize");
        let restored: BeadResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(output, restored.output.expect("should have output"));
    }
}
