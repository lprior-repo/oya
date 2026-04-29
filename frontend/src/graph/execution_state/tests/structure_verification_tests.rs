//! Structure verification tests for `execution_state` module split (oya-frontend-2e4).
//!
//! These tests verify file structure, line counts, module organization,
//! and test count preservation after the refactoring.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MAX_LINES_PER_FILE: usize = 300;
/// After splitting can_transition multi-assertion tests: 41+20+36+36+25 = 158
const EXPECTED_TEST_COUNT: usize = 158;

fn execution_state_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/graph/execution_state")
}

fn read_required_file(relative_path: &str) -> String {
    let path = execution_state_dir().join(relative_path);
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("Required file '{relative_path}' not found — refactoring not yet complete.")
    })
}

fn count_lines(content: &str) -> usize {
    content.lines().count()
}

fn count_test_annotations(content: &str) -> usize {
    content.lines().filter(|line| line.trim() == "#[test]").count()
}

fn assert_file_under_limit(relative_path: &str) {
    let content = read_required_file(relative_path);
    let lines = count_lines(&content);
    assert!(
        lines <= MAX_LINES_PER_FILE,
        "{relative_path} has {lines} lines, exceeds {MAX_LINES_PER_FILE}-line limit (INV-6)."
    );
}

fn assert_file_exists(relative_path: &str) {
    let path = execution_state_dir().join(relative_path);
    assert!(
        path.exists(),
        "{relative_path} must exist in execution_state/ directory per contract."
    );
}

// ---------------------------------------------------------------------------
// Line count constraint (INV-6) — one test per file, no loops
// ---------------------------------------------------------------------------

#[test]
fn mod_rs_under_300_lines() {
    assert_file_under_limit("mod.rs");
}

#[test]
fn tests_mod_rs_under_300_lines() {
    assert_file_under_limit("tests/mod.rs");
}

#[test]
fn state_tests_under_300_lines() {
    assert_file_under_limit("tests/state_tests.rs");
}

#[test]
fn transition_tests_under_300_lines() {
    assert_file_under_limit("tests/transition_tests.rs");
}

#[test]
fn try_transition_tests_under_300_lines() {
    assert_file_under_limit("tests/try_transition_tests.rs");
}

#[test]
fn can_transition_tests_under_300_lines() {
    assert_file_under_limit("tests/can_transition_tests.rs");
}

#[test]
fn type_state_tests_under_300_lines() {
    assert_file_under_limit("tests/type_state_tests.rs");
}

// ---------------------------------------------------------------------------
// Test count preserved — one test per file + total, no loops
// ---------------------------------------------------------------------------

#[test]
fn state_tests_has_tests() {
    let content = read_required_file("tests/state_tests.rs");
    let count = count_test_annotations(&content);
    assert!(count > 0, "state_tests.rs must contain at least one test function.");
}

#[test]
fn transition_tests_has_tests() {
    let content = read_required_file("tests/transition_tests.rs");
    let count = count_test_annotations(&content);
    assert!(count > 0, "transition_tests.rs must contain at least one test function.");
}

#[test]
fn try_transition_tests_has_tests() {
    let content = read_required_file("tests/try_transition_tests.rs");
    let count = count_test_annotations(&content);
    assert!(count > 0, "try_transition_tests.rs must contain at least one test function.");
}

#[test]
fn can_transition_tests_has_tests() {
    let content = read_required_file("tests/can_transition_tests.rs");
    let count = count_test_annotations(&content);
    assert!(count > 0, "can_transition_tests.rs must contain at least one test function.");
}

#[test]
fn type_state_tests_has_tests() {
    let content = read_required_file("tests/type_state_tests.rs");
    let count = count_test_annotations(&content);
    assert!(count > 0, "type_state_tests.rs must contain at least one test function.");
}

#[test]
fn total_test_count_preserved() {
    let total = count_test_annotations(&read_required_file("tests/state_tests.rs"))
        + count_test_annotations(&read_required_file("tests/transition_tests.rs"))
        + count_test_annotations(&read_required_file("tests/try_transition_tests.rs"))
        + count_test_annotations(&read_required_file("tests/can_transition_tests.rs"))
        + count_test_annotations(&read_required_file("tests/type_state_tests.rs"));
    assert_eq!(
        total, EXPECTED_TEST_COUNT,
        "Expected {EXPECTED_TEST_COUNT} test functions, found {total}."
    );
}

// ---------------------------------------------------------------------------
// Module structure matches contract — one test per file, no loops
// ---------------------------------------------------------------------------

#[test]
fn mod_rs_exists() {
    assert_file_exists("mod.rs");
}

#[test]
fn tests_mod_rs_exists() {
    assert_file_exists("tests/mod.rs");
}

#[test]
fn state_tests_rs_exists() {
    assert_file_exists("tests/state_tests.rs");
}

#[test]
fn transition_tests_rs_exists() {
    assert_file_exists("tests/transition_tests.rs");
}

#[test]
fn try_transition_tests_rs_exists() {
    assert_file_exists("tests/try_transition_tests.rs");
}

#[test]
fn can_transition_tests_rs_exists() {
    assert_file_exists("tests/can_transition_tests.rs");
}

#[test]
fn type_state_tests_rs_exists() {
    assert_file_exists("tests/type_state_tests.rs");
}
