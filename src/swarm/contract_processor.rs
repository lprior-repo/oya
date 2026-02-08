//! Contract processor for agent handoff workflow.
//!
//! Processes handoff files and generates test contracts following
//! Martin Fowler's test philosophy with Given-When-Then structure.
//!
//! # Zero-Unwrap Contract
//!
//! All functions return Result<T, E> and never use unwrap, expect, or panic.
//! Error handling follows Railway-Oriented Programming patterns.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Error types for contract processing.
#[derive(Debug, Error)]
pub enum ContractProcessorError {
    /// Handoff file not found at expected path.
    #[error("Handoff file not found: {file_path}")]
    HandoffFileNotFound { file_path: String },

    /// Contract file was not created after processing.
    #[error("Contract file not created: {file_path}")]
    ContractFileNotFound { file_path: String },

    /// Handoff JSON has invalid format or missing required fields.
    #[error("Invalid handoff format: {reason}")]
    InvalidHandoffFormat { reason: String },

    /// Generated contract is missing required sections.
    #[error("Missing contract sections: {reason}")]
    MissingContractSections { reason: String },

    /// IO error during file operations.
    #[error("IO error: {operation} failed for {file_path}: {reason}")]
    IoError {
        operation: String,
        file_path: String,
        reason: String,
    },
}

/// Result type for contract processing.
pub type ContractProcessorResult<T> = Result<T, ContractProcessorError>;

/// Validation error for handoff content.
#[derive(Debug, Clone)]
pub struct HandoffValidationError {
    /// Missing field name.
    pub field: String,

    /// Reason for validation failure.
    pub reason: String,
}

/// Handoff file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffFile {
    /// Bead identifier.
    pub bead_id: String,

    /// Feature title.
    pub title: String,

    /// Feature description.
    pub description: String,

    /// Optional: Error scenarios to enumerate.
    #[serde(default)]
    pub error_scenarios: Vec<String>,

    /// Optional: Integration points for break analysis.
    #[serde(default)]
    pub integration_points: Vec<String>,
}

/// Contract structure generated from handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractFile {
    /// Bead identifier.
    pub bead_id: String,

    /// Feature title.
    pub title: String,

    /// Contract content.
    pub contract: ContractContent,
}

/// Contract content with all required sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractContent {
    /// Error variants.
    pub errors: Vec<ErrorVariant>,

    /// Preconditions.
    pub preconditions: Vec<String>,

    /// Postconditions.
    pub postconditions: Vec<String>,

    /// Invariants.
    pub invariants: Vec<String>,

    /// Break analysis.
    pub break_analysis: Vec<BreakAnalysisEntry>,

    /// Test plan with Given-When-Then structure.
    pub test_plan: Vec<TestCase>,
}

/// Error variant definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorVariant {
    /// Error variant name.
    pub variant: String,

    /// Human-readable description.
    pub description: String,

    /// Whether error is recoverable.
    pub recoverable: bool,
}

/// Break analysis entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakAnalysisEntry {
    /// Failure scenario.
    pub scenario: String,

    /// Impact of failure.
    pub impact: String,

    /// Prevention strategy.
    pub prevention: String,

    /// Mitigation strategy.
    pub mitigation: String,
}

/// Test case with Given-When-Then structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Test name following Fowler conventions.
    pub test_name: String,

    /// Initial context.
    pub given: String,

    /// Action being tested.
    pub when: String,

    /// Expected outcome.
    pub then: String,

    /// Contract elements covered by this test.
    pub covers: String,
}

/// Contract processor for handoff-to-contract workflow.
#[derive(Debug, Clone)]
pub struct ContractProcessor {
    /// Output directory for contracts.
    output_dir: PathBuf,
}

impl ContractProcessor {
    /// Create a new contract processor.
    #[must_use]
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// Process a handoff file and generate contract.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Handoff file doesn't exist
    /// - Handoff JSON is malformed
    /// - Required fields are missing
    /// - Contract generation fails
    /// - Contract file write fails
    pub fn process_handoff(&self, handoff_path: &Path) -> ContractProcessorResult<PathBuf> {
        // Validate handoff exists
        if !handoff_path.exists() {
            return Err(ContractProcessorError::HandoffFileNotFound {
                file_path: handoff_path.display().to_string(),
            });
        }

        // Read handoff content
        let content =
            fs::read_to_string(handoff_path).map_err(|e| ContractProcessorError::IoError {
                operation: "read".to_string(),
                file_path: handoff_path.display().to_string(),
                reason: e.to_string(),
            })?;

        // Parse handoff JSON
        let handoff: HandoffFile = serde_json::from_str(&content).map_err(|e| {
            ContractProcessorError::InvalidHandoffFormat {
                reason: format!("JSON parse error: {e}"),
            }
        })?;

        // Validate required fields
        Self::validate_handoff(&handoff)?;

        // Generate contract
        let contract = self.generate_contract(&handoff)?;

        // Write contract file
        let contract_path = self
            .output_dir
            .join(format!("bead-contracts-{}.json", handoff.bead_id));
        let contract_json = serde_json::to_string_pretty(&contract).map_err(|e| {
            ContractProcessorError::IoError {
                operation: "serialize".to_string(),
                file_path: contract_path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        fs::write(&contract_path, contract_json).map_err(|e| ContractProcessorError::IoError {
            operation: "write".to_string(),
            file_path: contract_path.display().to_string(),
            reason: e.to_string(),
        })?;

        info!(
            bead_id = %handoff.bead_id,
            path = %contract_path.display(),
            "Contract generated successfully"
        );

        Ok(contract_path)
    }

    /// Validate handoff has required fields.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing.
    fn validate_handoff(handoff: &HandoffFile) -> ContractProcessorResult<()> {
        if handoff.bead_id.is_empty() {
            return Err(ContractProcessorError::InvalidHandoffFormat {
                reason: "bead_id is required and cannot be empty".to_string(),
            });
        }

        if handoff.title.is_empty() {
            return Err(ContractProcessorError::InvalidHandoffFormat {
                reason: "title is required and cannot be empty".to_string(),
            });
        }

        if handoff.description.is_empty() {
            return Err(ContractProcessorError::InvalidHandoffFormat {
                reason: "description is required and cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    /// Generate contract from handoff.
    ///
    /// # Errors
    ///
    /// Returns error if contract generation fails.
    fn generate_contract(&self, handoff: &HandoffFile) -> ContractProcessorResult<ContractFile> {
        let errors = self.generate_error_variants(handoff);
        let preconditions = self.generate_preconditions(handoff);
        let postconditions = self.generate_postconditions(handoff);
        let invariants = self.generate_invariants(handoff);
        let break_analysis = self.generate_break_analysis(handoff);
        let test_plan = self.generate_test_plan(handoff);

        let contract_content = ContractContent {
            errors,
            preconditions,
            postconditions,
            invariants,
            break_analysis,
            test_plan,
        };

        Ok(ContractFile {
            bead_id: handoff.bead_id.clone(),
            title: handoff.title.clone(),
            contract: contract_content,
        })
    }

    /// Generate error variants based on handoff.
    fn generate_error_variants(&self, handoff: &HandoffFile) -> Vec<ErrorVariant> {
        let mut errors = Vec::new();

        // Base errors for all contracts
        errors.push(ErrorVariant {
            variant: "HandoffFileNotFound".to_string(),
            description: "Handoff JSON file does not exist at expected path".to_string(),
            recoverable: false,
        });

        errors.push(ErrorVariant {
            variant: "ContractFileNotFound".to_string(),
            description: "Contract JSON file not created after processing".to_string(),
            recoverable: false,
        });

        errors.push(ErrorVariant {
            variant: "InvalidHandoffFormat".to_string(),
            description: "Handoff JSON missing required fields (bead_id, title)".to_string(),
            recoverable: false,
        });

        errors.push(ErrorVariant {
            variant: "MissingContractSections".to_string(),
            description: "Generated contract missing required sections (errors, test_plan)"
                .to_string(),
            recoverable: false,
        });

        // Add custom error scenarios from handoff
        for scenario in &handoff.error_scenarios {
            let variant_name = scenario
                .split_whitespace()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();

            errors.push(ErrorVariant {
                variant: variant_name.clone(),
                description: format!("Error scenario: {scenario}"),
                recoverable: false,
            });
        }

        errors
    }

    /// Generate preconditions.
    fn generate_preconditions(&self, handoff: &HandoffFile) -> Vec<String> {
        vec![
            format!(
                "Handoff file exists at /tmp/bead-handoff-{}.json",
                handoff.bead_id
            ),
            "Handoff file contains valid JSON with bead_id field".to_string(),
            format!(
                "Bead {} exists in .beads/beads.jsonl database",
                handoff.bead_id
            ),
        ]
    }

    /// Generate postconditions.
    fn generate_postconditions(&self, handoff: &HandoffFile) -> Vec<String> {
        vec![
            format!(
                "Contract file created at /tmp/bead-contracts-{}.json",
                handoff.bead_id
            ),
            "Contract contains exhaustive error variants".to_string(),
            "Contract contains preconditions, postconditions, and invariants".to_string(),
            "Contract contains Martin Fowler-style test plan with Given-When-Then structure"
                .to_string(),
            "Contract contains break analysis of all possible failure modes".to_string(),
        ]
    }

    /// Generate invariants.
    fn generate_invariants(&self, _handoff: &HandoffFile) -> Vec<String> {
        vec![
            "Contract file is always created for valid handoffs".to_string(),
            "Error variants are exhaustive (every possible failure mode enumerated)".to_string(),
            "Test names are expressive and describe WHAT and WHY (not just what)".to_string(),
            "All test names follow pattern: test_{verb}_{outcome}_when_{condition}".to_string(),
            "Each test covers one specific concept from contract/edge cases".to_string(),
        ]
    }

    /// Generate break analysis.
    fn generate_break_analysis(&self, handoff: &HandoffFile) -> Vec<BreakAnalysisEntry> {
        let mut analysis = Vec::new();

        // Base break analysis
        analysis.push(BreakAnalysisEntry {
            scenario: "Handoff file does not exist".to_string(),
            impact: "Cannot proceed - no requirements to analyze".to_string(),
            prevention: "Check file existence before reading".to_string(),
            mitigation: "Log error and skip to next handoff".to_string(),
        });

        analysis.push(BreakAnalysisEntry {
            scenario: "Handoff JSON is malformed".to_string(),
            impact: "Cannot parse requirements".to_string(),
            prevention: "Validate JSON structure before processing".to_string(),
            mitigation: "Log parse error and skip".to_string(),
        });

        analysis.push(BreakAnalysisEntry {
            scenario: "Contract file write fails".to_string(),
            impact: "Implementer cannot proceed".to_string(),
            prevention: "Verify write permissions on /tmp".to_string(),
            mitigation: "Retry write with error logging".to_string(),
        });

        analysis.push(BreakAnalysisEntry {
            scenario: "Missing required fields in handoff".to_string(),
            impact: "Incomplete contract generation".to_string(),
            prevention: "Validate bead_id, title, description exist".to_string(),
            mitigation: "Use defaults for optional fields, fail on required".to_string(),
        });

        // Add integration point specific analysis
        for integration in &handoff.integration_points {
            analysis.push(BreakAnalysisEntry {
                scenario: format!("Failure in {integration}"),
                impact: format!("Cannot process handoff via {integration}"),
                prevention: format!("Validate {integration} availability").to_string(),
                mitigation: format!("Fallback to alternative {integration}").to_string(),
            });
        }

        analysis
    }

    /// Generate test plan with Given-When-Then structure.
    fn generate_test_plan(&self, handoff: &HandoffFile) -> Vec<TestCase> {
        let mut tests = Vec::new();

        // Test 1: Creates contract file when handoff is valid
        tests.push(TestCase {
            test_name: "test_creates_contract_file_when_handoff_is_valid".to_string(),
            given: format!("A valid handoff JSON file exists at /tmp/bead-handoff-{}.json with bead_id, title, and description fields", handoff.bead_id),
            when: "The planner processes the handoff".to_string(),
            then: format!("A contract file is created at /tmp/bead-contracts-{}.json with all required sections populated", handoff.bead_id),
            covers: "precondition_valid_handoff, postcondition_contract_created".to_string(),
        });

        // Test 2: Enumerates all error variants
        tests.push(TestCase {
            test_name: "test_enumerates_all_error_variants_in_contract".to_string(),
            given: "A handoff describing a feature that can fail in multiple ways".to_string(),
            when: "The planner generates the contract".to_string(),
            then: "The contract errors array contains exhaustive variants covering: file IO, parse errors, validation errors, and state violations".to_string(),
            covers: "exhaustive_error_variants, break_analysis_completeness".to_string(),
        });

        // Test 3: Generates expressive test names
        tests.push(TestCase {
            test_name: "test_generates_expressive_test_names_following_fowler_conventions"
                .to_string(),
            given: "A contract with multiple edge cases and error conditions".to_string(),
            when: "The planner generates the test plan".to_string(),
            then: "All test names follow the pattern test_{verb}_{outcome}_when_{condition} and clearly describe WHAT behavior is tested and WHY the test exists".to_string(),
            covers: "martin_fowler_test_naming_convention, test_readability".to_string(),
        });

        // Test 4: Includes break analysis
        tests.push(TestCase {
            test_name: "test_includes_break_analysis_for_all_failure_modes".to_string(),
            given: "A feature with multiple integration points and external dependencies".to_string(),
            when: "The planner analyzes potential breaks".to_string(),
            then: "The break_analysis array contains entries for memory safety, logic errors, state violations, external failures, and error propagation".to_string(),
            covers: "break_analysis_completeness, rust_safety_guarantees".to_string(),
        });

        // Test 5: Returns contract path for notification
        tests.push(TestCase {
            test_name: "test_returns_contract_path_for_notification_on_success".to_string(),
            given: "A contract file has been successfully created".to_string(),
            when: "The planner completes processing".to_string(),
            then: "System returns contract path for printing CONTRACT_READY notification"
                .to_string(),
            covers: "postcondition_ready_notification, implementer_handoff_protocol".to_string(),
        });

        // Test 6: Handles missing handoff file
        tests.push(TestCase {
            test_name: "test_handles_missing_handoff_file_gracefully".to_string(),
            given: "No handoff file exists at the expected path".to_string(),
            when: "The planner attempts to process a non-existent handoff".to_string(),
            then: "Planner logs an error and continues monitoring without crashing".to_string(),
            covers: "error_variant_HandoffFileNotFound, graceful_error_handling".to_string(),
        });

        // Test 7: Validates handoff JSON structure
        tests.push(TestCase {
            test_name: "test_validates_handoff_json_structure_before_processing".to_string(),
            given: "A malformed JSON file exists at /tmp/bead-handoff-malformed.json".to_string(),
            when: "The planner attempts to read and parse the file".to_string(),
            then: "Planner detects invalid JSON, logs an error, and skips to the next handoff file"
                .to_string(),
            covers: "error_variant_InvalidHandoffFormat, input_validation".to_string(),
        });

        // Test 8: Ensures Given-When-Then structure
        tests.push(TestCase {
            test_name: "test_ensures_all_tests_use_given_when_then_structure".to_string(),
            given: "A contract with multiple test cases".to_string(),
            when: "The planner generates the test plan".to_string(),
            then: "Every test in test_plan has given, when, then, and covers fields populated with descriptive text".to_string(),
            covers: "martin_fowler_given_when_then, test_documentation".to_string(),
        });

        // Test 9: Defines invariants
        tests.push(TestCase {
            test_name: "test_defines_invariants_that_never_change".to_string(),
            given: "A feature with internal state and external interfaces".to_string(),
            when: "The planner analyzes the system".to_string(),
            then: "The contract invariants array includes properties that must remain true regardless of system state or operations".to_string(),
            covers: "invariant_definition, system_stability_guarantees".to_string(),
        });

        // Test 10: Maps tests to contract elements
        tests.push(TestCase {
            test_name: "test_maps_tests_to_contract_elements".to_string(),
            given: "A contract with preconditions, postconditions, error variants, and invariants".to_string(),
            when: "The planner generates the test plan".to_string(),
            then: "Each test's covers field explicitly references which precondition, postcondition, error variant, or invariant the test validates".to_string(),
            covers: "test_traceability, contract_coverage".to_string(),
        });

        tests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_processor_new() {
        let processor = ContractProcessor::new(PathBuf::from("/tmp"));
        assert_eq!(processor.output_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_validate_handoff_with_valid_data() {
        let handoff = HandoffFile {
            bead_id: "test-123".to_string(),
            title: "Test".to_string(),
            description: "A test".to_string(),
            error_scenarios: vec![],
            integration_points: vec![],
        };

        let result = ContractProcessor::validate_handoff(&handoff);
        assert!(result.is_ok(), "Valid handoff should pass validation");
    }

    #[test]
    fn test_validate_handoff_with_empty_bead_id() {
        let handoff = HandoffFile {
            bead_id: "".to_string(),
            title: "Test".to_string(),
            description: "A test".to_string(),
            error_scenarios: vec![],
            integration_points: vec![],
        };

        let result = ContractProcessor::validate_handoff(&handoff);
        assert!(result.is_err(), "Empty bead_id should fail validation");
    }

    #[test]
    fn test_generate_error_variants() {
        let processor = ContractProcessor::new(PathBuf::from("/tmp"));
        let handoff = HandoffFile {
            bead_id: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            error_scenarios: vec!["file not found".to_string()],
            integration_points: vec![],
        };

        let errors = processor.generate_error_variants(&handoff);
        assert!(!errors.is_empty(), "Should generate error variants");
        assert!(
            errors.len() >= 4,
            "Should have at least 4 base errors, got {}",
            errors.len()
        );
    }
}
