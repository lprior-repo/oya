//! API verification tests for `execution_state` module split (oya-frontend-2e4).
//!
//! These tests verify serde contracts, transition table correctness, and
//! lint attribute preservation after the refactoring.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use std::path::Path;

use super::super::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn execution_state_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/graph/execution_state")
}

fn read_required_file(relative_path: &str) -> String {
    let path = execution_state_dir().join(relative_path);
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("Required file '{relative_path}' not found — refactoring not yet complete.")
    })
}

fn assert_serde_round_trip(variant: ExecutionState) {
    let json = serde_json::to_string(&variant).unwrap();
    let recovered: ExecutionState = serde_json::from_str(&json).unwrap();
    assert_eq!(recovered, variant);
}

// ---------------------------------------------------------------------------
// INV-3: Serialization produces lowercase JSON
// ---------------------------------------------------------------------------

#[test]
fn serialization_produces_lowercase_json() {
    assert_eq!(serde_json::to_string(&ExecutionState::Idle).unwrap(), "\"idle\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Queued).unwrap(), "\"queued\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Running).unwrap(), "\"running\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Completed).unwrap(), "\"completed\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Failed).unwrap(), "\"failed\"");
    assert_eq!(serde_json::to_string(&ExecutionState::Skipped).unwrap(), "\"skipped\"");
}

// ---------------------------------------------------------------------------
// INV-3: Deserialization from lowercase JSON
// ---------------------------------------------------------------------------

#[test]
fn deserialization_from_lowercase_json() {
    assert_eq!(serde_json::from_str::<ExecutionState>("\"idle\"").unwrap(), ExecutionState::Idle);
    assert_eq!(
        serde_json::from_str::<ExecutionState>("\"queued\"").unwrap(),
        ExecutionState::Queued
    );
    assert_eq!(
        serde_json::from_str::<ExecutionState>("\"running\"").unwrap(),
        ExecutionState::Running
    );
    assert_eq!(
        serde_json::from_str::<ExecutionState>("\"completed\"").unwrap(),
        ExecutionState::Completed
    );
    assert_eq!(
        serde_json::from_str::<ExecutionState>("\"failed\"").unwrap(),
        ExecutionState::Failed
    );
    assert_eq!(
        serde_json::from_str::<ExecutionState>("\"skipped\"").unwrap(),
        ExecutionState::Skipped
    );
}

// ---------------------------------------------------------------------------
// INV-3: Round-trip serialization preserves exact value
// ---------------------------------------------------------------------------

#[test]
fn round_trip_preserves_all_variants() {
    assert_serde_round_trip(ExecutionState::Idle);
    assert_serde_round_trip(ExecutionState::Queued);
    assert_serde_round_trip(ExecutionState::Running);
    assert_serde_round_trip(ExecutionState::Completed);
    assert_serde_round_trip(ExecutionState::Failed);
    assert_serde_round_trip(ExecutionState::Skipped);
}

// ---------------------------------------------------------------------------
// INV-5: Valid transitions return correct variant
// ---------------------------------------------------------------------------

#[test]
fn valid_transitions_return_correct_variant() {
    assert_eq!(
        try_transition(ExecutionState::Idle, ExecutionState::Queued),
        Some(StateTransition::IdleToQueued)
    );
    assert_eq!(
        try_transition(ExecutionState::Idle, ExecutionState::Skipped),
        Some(StateTransition::IdleToSkipped)
    );
    assert_eq!(
        try_transition(ExecutionState::Queued, ExecutionState::Running),
        Some(StateTransition::QueuedToRunning)
    );
    assert_eq!(
        try_transition(ExecutionState::Queued, ExecutionState::Skipped),
        Some(StateTransition::QueuedToSkipped)
    );
    assert_eq!(
        try_transition(ExecutionState::Running, ExecutionState::Completed),
        Some(StateTransition::RunningToCompleted)
    );
    assert_eq!(
        try_transition(ExecutionState::Running, ExecutionState::Failed),
        Some(StateTransition::RunningToFailed)
    );
}

// ---------------------------------------------------------------------------
// INV-5: Illegal transitions return None
// ---------------------------------------------------------------------------

#[test]
fn invalid_transitions_return_none() {
    assert_eq!(try_transition(ExecutionState::Idle, ExecutionState::Running), None);
    assert_eq!(try_transition(ExecutionState::Running, ExecutionState::Idle), None);
    assert_eq!(try_transition(ExecutionState::Completed, ExecutionState::Queued), None);
    assert_eq!(try_transition(ExecutionState::Failed, ExecutionState::Idle), None);
    assert_eq!(try_transition(ExecutionState::Skipped, ExecutionState::Running), None);
    assert_eq!(try_transition(ExecutionState::Idle, ExecutionState::Idle), None);
}

// ---------------------------------------------------------------------------
// INV-5: can_transition consistent with try_transition
// ---------------------------------------------------------------------------

#[test]
fn can_transition_consistent_with_try_transition() {
    assert!(can_transition(ExecutionState::Idle, ExecutionState::Queued));
    assert!(can_transition(ExecutionState::Running, ExecutionState::Completed));
    assert!(!can_transition(ExecutionState::Idle, ExecutionState::Running));
    assert!(!can_transition(ExecutionState::Completed, ExecutionState::Idle));
}

// ---------------------------------------------------------------------------
// INV-8: Lint attributes preserved in mod.rs — one test per attribute
// ---------------------------------------------------------------------------

#[test]
fn deny_unwrap_used_preserved() {
    let mod_rs = read_required_file("mod.rs");
    assert!(
        mod_rs.contains("#![deny(clippy::unwrap_used)]"),
        "mod.rs must preserve #![deny(clippy::unwrap_used)] lint attribute (INV-8)"
    );
}

#[test]
fn deny_expect_used_preserved() {
    let mod_rs = read_required_file("mod.rs");
    assert!(
        mod_rs.contains("#![deny(clippy::expect_used)]"),
        "mod.rs must preserve #![deny(clippy::expect_used)] lint attribute (INV-8)"
    );
}

#[test]
fn deny_panic_preserved() {
    let mod_rs = read_required_file("mod.rs");
    assert!(
        mod_rs.contains("#![deny(clippy::panic)]"),
        "mod.rs must preserve #![deny(clippy::panic)] lint attribute (INV-8)"
    );
}

#[test]
fn warn_pedantic_preserved() {
    let mod_rs = read_required_file("mod.rs");
    assert!(
        mod_rs.contains("#![warn(clippy::pedantic)]"),
        "mod.rs must preserve #![warn(clippy::pedantic)] lint attribute (INV-8)"
    );
}

#[test]
fn warn_nursery_preserved() {
    let mod_rs = read_required_file("mod.rs");
    assert!(
        mod_rs.contains("#![warn(clippy::nursery)]"),
        "mod.rs must preserve #![warn(clippy::nursery)] lint attribute (INV-8)"
    );
}

#[test]
fn forbid_unsafe_code_preserved() {
    let mod_rs = read_required_file("mod.rs");
    assert!(
        mod_rs.contains("#![forbid(unsafe_code)]"),
        "mod.rs must preserve #![forbid(unsafe_code)] lint attribute (INV-8)"
    );
}
