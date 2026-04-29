//! Performance benchmarks for ExecutionState machine
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oya_frontend::graph::{can_transition, try_transition, ExecutionState};

// ===========================================================================
// Benchmark: try_transition performance
// ===========================================================================

fn bench_try_transition_valid(c: &mut Criterion) {
    c.bench_function("try_transition_valid_idle_queued", |b| {
        b.iter(|| {
            let from = ExecutionState::Idle;
            let to = ExecutionState::Queued;
            black_box(try_transition(from, to))
        })
    });
}

fn bench_try_transition_invalid(c: &mut Criterion) {
    c.bench_function("try_transition_invalid_completed_running", |b| {
        b.iter(|| {
            let from = ExecutionState::Completed;
            let to = ExecutionState::Running;
            black_box(try_transition(from, to))
        })
    });
}

fn bench_try_transition_all_states(c: &mut Criterion) {
    c.bench_function("try_transition_all_36_pairs", |b| {
        b.iter(|| {
            let states = [
                ExecutionState::Idle,
                ExecutionState::Queued,
                ExecutionState::Running,
                ExecutionState::Completed,
                ExecutionState::Failed,
                ExecutionState::Skipped,
            ];

            let mut count = 0;
            for from in states.iter() {
                for to in states.iter() {
                    let _ = try_transition(*from, *to);
                    count += 1;
                }
            }
            black_box(count);
        })
    });
}

// ===========================================================================
// Benchmark: can_transition performance
// ===========================================================================

fn bench_can_transition_valid(c: &mut Criterion) {
    c.bench_function("can_transition_valid", |b| {
        b.iter(|| {
            let from = ExecutionState::Idle;
            let to = ExecutionState::Queued;
            black_box(can_transition(from, to))
        })
    });
}

fn bench_can_transition_invalid(c: &mut Criterion) {
    c.bench_function("can_transition_invalid", |b| {
        b.iter(|| {
            let from = ExecutionState::Completed;
            let to = ExecutionState::Running;
            black_box(can_transition(from, to))
        })
    });
}

// ===========================================================================
// Benchmark: Serialization performance
// ===========================================================================

fn bench_serialize_state(c: &mut Criterion) {
    c.bench_function("serialize_running_state", |b| {
        let state = ExecutionState::Running;
        b.iter(|| {
            let _ = serde_json::to_string(&state);
        })
    });
}

fn bench_deserialize_state(c: &mut Criterion) {
    c.bench_function("deserialize_running_state", |b| {
        let json = r#""running""#;
        b.iter(|| {
            let _ = serde_json::from_str::<ExecutionState>(json);
        })
    });
}

fn bench_serialize_roundtrip(c: &mut Criterion) {
    c.bench_function("serialize_deserialize_roundtrip", |b| {
        let state = ExecutionState::Running;
        let json = serde_json::to_string(&state)
            .expect("ExecutionState serialization must succeed for benchmark setup");
        b.iter(|| {
            let _ = serde_json::from_str::<ExecutionState>(&json);
        })
    });
}

// ===========================================================================
// Benchmark: is_terminal and is_active
// ===========================================================================

fn bench_is_terminal(c: &mut Criterion) {
    c.bench_function("is_terminal_all_states", |b| {
        b.iter(|| {
            let states = [
                ExecutionState::Idle,
                ExecutionState::Queued,
                ExecutionState::Running,
                ExecutionState::Completed,
                ExecutionState::Failed,
                ExecutionState::Skipped,
            ];

            let mut terminal_count = 0;
            for state in states.iter() {
                if state.is_terminal() {
                    terminal_count += 1;
                }
            }
            black_box(terminal_count);
        })
    });
}

fn bench_is_active(c: &mut Criterion) {
    c.bench_function("is_active_all_states", |b| {
        b.iter(|| {
            let states = [
                ExecutionState::Idle,
                ExecutionState::Queued,
                ExecutionState::Running,
                ExecutionState::Completed,
                ExecutionState::Failed,
                ExecutionState::Skipped,
            ];

            let mut active_count = 0;
            for state in states.iter() {
                if state.is_active() {
                    active_count += 1;
                }
            }
            black_box(active_count);
        })
    });
}

criterion_group!(
    benches,
    bench_try_transition_valid,
    bench_try_transition_invalid,
    bench_try_transition_all_states,
    bench_can_transition_valid,
    bench_can_transition_invalid,
    bench_serialize_state,
    bench_deserialize_state,
    bench_serialize_roundtrip,
    bench_is_terminal,
    bench_is_active,
);

criterion_main!(benches);
