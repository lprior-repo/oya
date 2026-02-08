//! Contract processor tests using Martin Fowler Given-When-Then style.
//!
//! Tests verify that handoff files are correctly processed into contracts
//! following the zero-unwrap functional contract.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::swarm::contract_processor::{
    ContractProcessor, ContractProcessorError, HandoffValidationError,
};

/// Test: Creates contract file when handoff is valid
///
/// Given: A valid handoff JSON file exists with bead_id, title, and description fields
/// When: The processor processes the handoff
/// Then: A contract file is created with all required sections populated
#[test]
fn test_creates_contract_file_when_handoff_is_valid() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-test-123.json");
    let handoff_content = serde_json::json!({
        "bead_id": "test-123",
        "title": "Test Feature",
        "description": "A test feature for contract generation",
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            assert!(contract_path.exists(), "Contract file should exist");
            assert_eq!(
                contract_path.file_name(),
                std::ffi::OsStr::new("bead-contracts-test-123.json")
            );

            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            assert!(
                contract["bead_id"].is_string(),
                "Contract should have bead_id"
            );
            assert!(contract["title"].is_string(), "Contract should have title");
            assert!(
                contract["contract"]["errors"].is_array(),
                "Contract should have errors array"
            );
            assert!(
                contract["contract"]["preconditions"].is_array(),
                "Contract should have preconditions"
            );
            assert!(
                contract["contract"]["postconditions"].is_array(),
                "Contract should have postconditions"
            );
            assert!(
                contract["contract"]["invariants"].is_array(),
                "Contract should have invariants"
            );
            assert!(
                contract["contract"]["test_plan"].is_array(),
                "Contract should have test_plan"
            );
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Enumerates all error variants in contract
///
/// Given: A handoff describing a feature that can fail in multiple ways
/// When: The processor generates the contract
/// Then: The contract errors array contains exhaustive variants
#[test]
fn test_enumerates_all_error_variants_in_contract() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-error-variants.json");
    let handoff_content = serde_json::json!({
        "bead_id": "error-variants",
        "title": "Error Variants Test",
        "description": "Test error variant enumeration",
        "error_scenarios": [
            "file not found",
            "parse error",
            "validation failed",
            "state violation"
        ],
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            let errors = contract["contract"]["errors"]
                .as_array()
                .expect("errors should be array");

            assert!(!errors.is_empty(), "Errors array should not be empty");
            assert!(
                errors.len() >= 4,
                "Should enumerate at least 4 error variants, got {}",
                errors.len()
            );

            // Each error should have required fields
            for error in errors {
                assert!(
                    error["variant"].is_string(),
                    "Error should have variant field"
                );
                assert!(
                    error["description"].is_string(),
                    "Error should have description field"
                );
                assert!(
                    error["recoverable"].is_boolean(),
                    "Error should have recoverable field"
                );
            }
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Generates expressive test names following Fowler conventions
///
/// Given: A contract with multiple edge cases and error conditions
/// When: The processor generates the test plan
/// Then: All test names follow the pattern test_{verb}_{outcome}_when_{condition}
#[test]
fn test_generates_expressive_test_names_following_fowler_conventions() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-test-names.json");
    let handoff_content = serde_json::json!({
        "bead_id": "test-names",
        "title": "Test Naming Convention",
        "description": "Verify Fowler-style test names",
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            let test_plan = contract["contract"]["test_plan"]
                .as_array()
                .expect("test_plan should be array");

            // Check that test names follow the pattern
            for test_entry in test_plan {
                let test_name = test_entry["test_name"]
                    .as_str()
                    .expect("test_name should be string");

                // Should follow pattern: test_{verb}_{outcome}_when_{condition}
                assert!(
                    test_name.starts_with("test_"),
                    "Test name should start with 'test_': {test_name}"
                );

                // Should contain _when_ separator
                assert!(
                    test_name.contains("_when_"),
                    "Test name should contain '_when_': {test_name}"
                );

                // Each test should have Given-When-Then structure
                assert!(
                    test_entry["given"].is_string(),
                    "Test should have 'given' field"
                );
                assert!(
                    test_entry["when"].is_string(),
                    "Test should have 'when' field"
                );
                assert!(
                    test_entry["then"].is_string(),
                    "Test should have 'then' field"
                );
                assert!(
                    test_entry["covers"].is_string(),
                    "Test should have 'covers' field"
                );
            }
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Includes break analysis for all failure modes
///
/// Given: A feature with multiple integration points and external dependencies
/// When: The processor analyzes potential breaks
/// Then: The break_analysis array contains entries for various failure modes
#[test]
fn test_includes_break_analysis_for_all_failure_modes() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-break-analysis.json");
    let handoff_content = serde_json::json!({
        "bead_id": "break-analysis",
        "title": "Break Analysis Test",
        "description": "Verify break analysis completeness",
        "integration_points": ["file system", "JSON parsing", "network"],
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            let break_analysis = contract["contract"]["break_analysis"]
                .as_array()
                .expect("break_analysis should be array");

            assert!(
                !break_analysis.is_empty(),
                "Break analysis should not be empty"
            );

            // Each break analysis entry should have required fields
            for entry in break_analysis {
                assert!(
                    entry["scenario"].is_string(),
                    "Break analysis should have scenario field"
                );
                assert!(
                    entry["impact"].is_string(),
                    "Break analysis should have impact field"
                );
                assert!(
                    entry["prevention"].is_string(),
                    "Break analysis should have prevention field"
                );
                assert!(
                    entry["mitigation"].is_string(),
                    "Break analysis should have mitigation field"
                );
            }
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Prints contract ready notification on success
///
/// Given: A contract file has been successfully created
/// When: The processor completes processing
/// Then: System returns contract path for notification
#[test]
fn test_returns_contract_path_for_notification_on_success() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-notification.json");
    let handoff_content = serde_json::json!({
        "bead_id": "notification-test",
        "title": "Notification Test",
        "description": "Verify contract path is returned",
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            // Verify the path format matches expected pattern
            let path_str = contract_path.to_string_lossy();
            assert!(
                path_str.contains("bead-contracts-notification-test.json"),
                "Contract path should contain expected filename: {path_str}"
            );
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Handles missing handoff file gracefully
///
/// Given: No handoff file exists at the expected path
/// When: The processor attempts to process a non-existent handoff
/// Then: Processor returns HandoffFileNotFound error without crashing
#[test]
fn test_handles_missing_handoff_file_gracefully() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let non_existent_path = handoff_dir.join("bead-handoff-missing.json");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&non_existent_path);

    // Then
    match result {
        Err(ContractProcessorError::HandoffFileNotFound { file_path }) => {
            assert!(file_path.contains("bead-handoff-missing.json"));
        }
        Ok(_) => panic!("Expected Err, got Ok"),
        Err(e) => panic!("Expected HandoffFileNotFound, got: {e}"),
    }
}

/// Test: Validates handoff JSON structure before processing
///
/// Given: A malformed JSON file exists
/// When: The processor attempts to read and parse the file
/// Then: Processor detects invalid JSON and returns InvalidHandoffFormat error
#[test]
fn test_validates_handoff_json_structure_before_processing() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-malformed.json");
    fs::write(&handoff_path, "{ invalid json }")
        .map_err(|e| format!("Failed to write malformed handoff: {e}"))
        .expect("malformed handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Err(ContractProcessorError::InvalidHandoffFormat { reason }) => {
            assert!(!reason.is_empty(), "Error reason should not be empty");
        }
        Ok(_) => panic!("Expected Err, got Ok"),
        Err(e) => panic!("Expected InvalidHandoffFormat, got: {e}"),
    }
}

/// Test: Ensures all tests use Given-When-Then structure
///
/// Given: A contract with multiple test cases
/// When: The processor generates the test plan
/// Then: Every test has given, when, then, and covers fields populated
#[test]
fn test_ensures_all_tests_use_given_when_then_structure() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-gwt-structure.json");
    let handoff_content = serde_json::json!({
        "bead_id": "gwt-structure",
        "title": "Given-When-Then Structure Test",
        "description": "Verify GWT structure in test plan",
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            let test_plan = contract["contract"]["test_plan"]
                .as_array()
                .expect("test_plan should be array");

            for test_entry in test_plan {
                assert!(
                    test_entry["given"].is_string(),
                    "Test should have 'given' field"
                );
                assert!(
                    test_entry["when"].is_string(),
                    "Test should have 'when' field"
                );
                assert!(
                    test_entry["then"].is_string(),
                    "Test should have 'then' field"
                );
                assert!(
                    test_entry["covers"].is_string(),
                    "Test should have 'covers' field"
                );
            }
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Defines invariants that never change
///
/// Given: A feature with internal state and external interfaces
/// When: The processor analyzes the system
/// Then: The contract invariants array includes properties that must remain true
#[test]
fn test_defines_invariants_that_never_change() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-invariants.json");
    let handoff_content = serde_json::json!({
        "bead_id": "invariants",
        "title": "Invariants Test",
        "description": "Verify invariant definitions",
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            let invariants = contract["contract"]["invariants"]
                .as_array()
                .expect("invariants should be array");

            assert!(!invariants.is_empty(), "Invariants should not be empty");

            for invariant in invariants {
                assert!(
                    invariant.is_string(),
                    "Invariant should be a string describing a property"
                );
            }
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Maps tests to contract elements
///
/// Given: A contract with preconditions, postconditions, error variants, and invariants
/// When: The processor generates the test plan
/// Then: Each test's covers field explicitly references contract elements
#[test]
fn test_maps_tests_to_contract_elements() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    let handoff_path = handoff_dir.join("bead-handoff-test-mapping.json");
    let handoff_content = serde_json::json!({
        "bead_id": "test-mapping",
        "title": "Test Mapping Test",
        "description": "Verify test to contract element mapping",
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Ok(contract_path) => {
            let contract_content = fs::read_to_string(&contract_path)
                .map_err(|e| format!("Failed to read contract: {e}"))
                .expect("contract read");
            let contract: serde_json::Value = serde_json::from_str(&contract_content)
                .map_err(|e| format!("Failed to parse contract JSON: {e}"))
                .expect("contract parse");

            let test_plan = contract["contract"]["test_plan"]
                .as_array()
                .expect("test_plan should be array");

            for test_entry in test_plan {
                let covers = test_entry["covers"]
                    .as_str()
                    .expect("covers should be string");

                assert!(
                    !covers.is_empty(),
                    "Covers field should reference contract elements"
                );

                // Should reference known contract elements
                assert!(
                    covers.contains("precondition")
                        || covers.contains("postcondition")
                        || covers.contains("error_variant")
                        || covers.contains("invariant")
                        || covers.contains("test_plan"),
                    "Covers field should reference known contract elements: {covers}"
                );
            }
        }
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }
}

/// Test: Validates required fields in handoff
///
/// Given: A handoff file missing required fields
/// When: The processor attempts to process
/// Then: Processor returns InvalidHandoffFormat error
#[test]
fn test_validates_required_fields_in_handoff() {
    // Given
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temp dir: {e}"))
        .expect("temp dir creation");
    let handoff_dir = temp_dir.path();

    // Missing bead_id field
    let handoff_path = handoff_dir.join("bead-handoff-missing-fields.json");
    let handoff_content = serde_json::json!({
        "title": "Missing Bead ID",
        // bead_id is missing
    });
    fs::write(&handoff_path, handoff_content.to_string())
        .map_err(|e| format!("Failed to write handoff: {e}"))
        .expect("handoff write");

    // When
    let processor = ContractProcessor::new(handoff_dir.to_path_buf());
    let result = processor.process_handoff(&handoff_path);

    // Then
    match result {
        Err(ContractProcessorError::InvalidHandoffFormat { reason }) => {
            assert!(
                reason.contains("bead_id") || reason.contains("required"),
                "Error should mention missing bead_id or required fields: {reason}"
            );
        }
        Ok(_) => panic!("Expected Err, got Ok"),
        Err(e) => panic!("Expected InvalidHandoffFormat, got: {e}"),
    }
}
