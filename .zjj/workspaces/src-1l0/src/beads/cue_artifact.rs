//! Quality Gates: CUE Artifact Generation
//!
//! Generates and validates CUE schema artifacts for each bead during the Contract stage.
//! Pure function: no I/O, stable artifact generation.
//!
//! # Contract
//!
//! Every bead MUST generate a CUE artifact as part of the Contract stage.
//! The artifact name follows the pattern: `oya-{timestamp}-{bead_suffix}.cue`
//!
//! This ensures:
//! - Contracts are machine-readable
//! - Contracts are validated against CUE schemas
//! - Implementation completeness can be verified via `cue vet`

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::BeadId;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Error types for CUE artifact operations
#[derive(Debug, Error)]
pub enum CueArtifactError {
    #[error("Invalid bead ID format: {0}")]
    InvalidBeadId(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(String),
}

/// CUE artifact metadata representing a per-bead schema
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueArtifact {
    /// The bead ID this artifact validates
    pub bead_id: BeadId,
    /// Timestamp when the artifact was generated
    pub generated_at: DateTime<Utc>,
    /// The CUE schema content
    pub schema_content: String,
    /// Expected filename for the artifact
    pub filename: String,
}

impl CueArtifact {
    /// Generate a CUE artifact for a bead
    ///
    /// # Errors
    /// Returns [`CueArtifactError`] if the bead ID is invalid or empty
    pub fn generate(bead_id: BeadId, title: &str, generated_at: DateTime<Utc>) -> Result<Self, CueArtifactError> {
        validate_bead_id(&bead_id)?;

        let filename = generate_cue_filename(&bead_id, generated_at);
        let schema_content = generate_schema_template(&bead_id, title, generated_at);

        Ok(Self {
            bead_id,
            generated_at,
            schema_content,
            filename,
        })
    }

    /// Get the expected artifact path relative to project root
    #[must_use]
    pub fn relative_path(&self) -> String {
        format!(".beads/schemas/{}", self.filename)
    }

    /// Check if the schema has valid structure
    #[must_use]
    pub fn is_valid_structure(&self) -> bool {
        schema_has_valid_structure(&self.schema_content)
    }
}

/// Validate bead ID format
fn validate_bead_id(bead_id: &BeadId) -> Result<(), CueArtifactError> {
    let id = bead_id.as_str();
    if id.is_empty() {
        return Err(CueArtifactError::InvalidBeadId("empty bead ID".to_string()));
    }
    if !id.contains('-') {
        return Err(CueArtifactError::InvalidBeadId(
            format!("bead ID must contain a hyphen: {}", id)
        ));
    }
    Ok(())
}

/// Generate CUE filename from bead ID and timestamp
/// Format: oya-{timestamp}-{suffix}.cue
fn generate_cue_filename(bead_id: &BeadId, generated_at: DateTime<Utc>) -> String {
    let timestamp = generated_at.format("%Y%m%d%H%M%S");
    let suffix = extract_bead_suffix(bead_id.as_str());
    format!("oya-{}-{}.cue", timestamp, suffix)
}

/// Extract the suffix from a bead ID (the part after the hyphen)
fn extract_bead_suffix(bead_id: &str) -> String {
    bead_id
        .split('-')
        .skip(1)
        .collect::<Vec<_>>()
        .join("-")
}

/// Generate CUE schema template for a bead
fn generate_schema_template(bead_id: &BeadId, title: &str, generated_at: DateTime<Utc>) -> String {
    let timestamp = generated_at.format("%Y-%m-%dT%H:%M:%SZ");
    let filename_date = generated_at.format("%Y%m%d%H%M%S");
    let suffix = extract_bead_suffix(bead_id.as_str());

    format!(
        r#"
package validation

import "list"

// Validation schema for bead: {bead_id}
// Title: {title}
//
// This schema validates that implementation is complete.
// Use: cue vet oya-{filename_date}-{suffix}.cue implementation.cue

#BeadImplementation: {{
  bead_id: "{bead_id}"
  title: "{title}"

  // Contract verification
  contracts_verified: {{
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [...string]

    // Specific postconditions that must be verified
    postcondition_checks: [...string]

    // Specific invariants that must be maintained
    invariant_checks: [...string]
  }}

  // Test verification
  tests_passing: {{
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(1)
    error_path_tests: [...string] & list.MinItems(1)
  }}

  // Code completion
  code_complete: {{
    implementation_exists: string  // Path to implementation file
    tests_exist: string  // Path to test file
    ci_passing: bool & true
    no_unwrap_calls: bool & true  // Rust/functional constraint
    no_panics: bool & true  // Rust constraint
  }}

  // Completion criteria
  completion: {{
    all_sections_complete: bool & true
    documentation_updated: bool
    beads_closed: bool
    timestamp: string  // ISO8601 completion timestamp
  }}
}}
"#,
        bead_id = bead_id.as_str(),
        title = title,
        filename_date = filename_date,
        suffix = suffix,
    )
}

/// Check if schema content has valid CUE structure
fn schema_has_valid_structure(content: &str) -> bool {
    content.contains("package validation")
        && content.contains("#BeadImplementation")
        && content.contains("bead_id:")
}

/// Result of validating CUE artifact existence
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueArtifactValidation {
    pub bead_id: BeadId,
    pub artifact_expected: bool,
    pub artifact_path: String,
}

/// Validate that a CUE artifact should exist for a bead in the Contract stage
///
/// # Errors
/// Returns [`CueArtifactError`] if validation fails
pub fn validate_cue_artifact_requirement(bead_id: BeadId) -> Result<CueArtifactValidation, CueArtifactError> {
    validate_bead_id(&bead_id)?;

    let artifact = CueArtifact::generate(bead_id.clone(), "Untitled", Utc::now())?;

    Ok(CueArtifactValidation {
        bead_id,
        artifact_expected: true,
        artifact_path: artifact.relative_path(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bead_id(id: &str) -> BeadId {
        BeadId::new(id)
    }

    #[test]
    fn cue_artifact_generate_valid_bead() {
        let bead_id = make_bead_id("src-abc123");
        let generated_at = Utc::now();
        let artifact = CueArtifact::generate(bead_id.clone(), "Test bead", generated_at);

        assert!(artifact.is_ok());
        let artifact = artifact.unwrap();
        assert_eq!(artifact.bead_id, bead_id);
        assert_eq!(artifact.generated_at, generated_at);
        assert!(artifact.filename.ends_with(".cue"));
        assert!(artifact.filename.starts_with("oya-"));
    }

    #[test]
    fn cue_artifact_generate_empty_bead_id_fails() {
        let bead_id = make_bead_id("");
        let result = CueArtifact::generate(bead_id, "Test", Utc::now());

        assert!(result.is_err());
        match result {
            Err(CueArtifactError::InvalidBeadId(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidBeadId error"),
        }
    }

    #[test]
    fn cue_artifact_generate_bead_id_without_hyphen_fails() {
        let bead_id = make_bead_id("nothyphen");
        let result = CueArtifact::generate(bead_id, "Test", Utc::now());

        assert!(result.is_err());
        match result {
            Err(CueArtifactError::InvalidBeadId(msg)) => {
                assert!(msg.contains("hyphen"));
            }
            _ => panic!("Expected InvalidBeadId error"),
        }
    }

    #[test]
    fn cue_artifact_filename_format() {
        let bead_id = make_bead_id("src-s6d");
        let generated_at = "2026-02-20T12:30:45Z".parse::<DateTime<Utc>>().unwrap();
        let artifact = CueArtifact::generate(bead_id, "Test", generated_at).unwrap();

        // Format: oya-{timestamp}-{suffix}.cue
        assert!(artifact.filename.contains("20260220123045"));
        assert!(artifact.filename.contains("s6d"));
        assert!(artifact.filename.ends_with(".cue"));
    }

    #[test]
    fn cue_artifact_relative_path() {
        let bead_id = make_bead_id("src-abc123");
        let artifact = CueArtifact::generate(bead_id, "Test", Utc::now()).unwrap();

        let path = artifact.relative_path();
        assert!(path.starts_with(".beads/schemas/"));
        assert!(path.ends_with(".cue"));
    }

    #[test]
    fn cue_artifact_schema_has_valid_structure() {
        let bead_id = make_bead_id("src-s6d");
        let artifact = CueArtifact::generate(bead_id, "Test bead title", Utc::now()).unwrap();

        assert!(artifact.is_valid_structure());
    }

    #[test]
    fn cue_artifact_schema_contains_bead_id() {
        let bead_id = make_bead_id("src-s6d");
        let artifact = CueArtifact::generate(bead_id.clone(), "Test", Utc::now()).unwrap();

        assert!(artifact.schema_content.contains(bead_id.as_str()));
    }

    #[test]
    fn cue_artifact_schema_contains_title() {
        let bead_id = make_bead_id("src-s6d");
        let title = "contract: require per-bead cue artifact generation";
        let artifact = CueArtifact::generate(bead_id, title, Utc::now()).unwrap();

        assert!(artifact.schema_content.contains(title));
    }

    #[test]
    fn cue_artifact_schema_contains_package_validation() {
        let bead_id = make_bead_id("src-s6d");
        let artifact = CueArtifact::generate(bead_id, "Test", Utc::now()).unwrap();

        assert!(artifact.schema_content.contains("package validation"));
    }

    #[test]
    fn cue_artifact_schema_contains_bead_implementation_schema() {
        let bead_id = make_bead_id("src-s6d");
        let artifact = CueArtifact::generate(bead_id, "Test", Utc::now()).unwrap();

        assert!(artifact.schema_content.contains("#BeadImplementation:"));
        assert!(artifact.schema_content.contains("contracts_verified:"));
        assert!(artifact.schema_content.contains("tests_passing:"));
        assert!(artifact.schema_content.contains("code_complete:"));
        assert!(artifact.schema_content.contains("completion:"));
    }

    #[test]
    fn validate_cue_artifact_requirement_success() {
        let bead_id = make_bead_id("src-s6d");
        let result = validate_cue_artifact_requirement(bead_id.clone());

        assert!(result.is_ok());
        let validation = result.unwrap();
        assert_eq!(validation.bead_id, bead_id);
        assert!(validation.artifact_expected);
        assert!(validation.artifact_path.starts_with(".beads/schemas/"));
    }

    #[test]
    fn validate_cue_artifact_requirement_invalid_bead_fails() {
        let bead_id = make_bead_id("invalid");
        let result = validate_cue_artifact_requirement(bead_id);

        assert!(result.is_err());
    }

    #[test]
    fn extract_bead_suffix_single_part() {
        let suffix = extract_bead_suffix("src-abc123");
        assert_eq!(suffix, "abc123");
    }

    #[test]
    fn extract_bead_suffix_multiple_parts() {
        let suffix = extract_bead_suffix("oya-2026-abc");
        assert_eq!(suffix, "2026-abc");
    }

    #[test]
    fn schema_has_valid_structure_true_for_valid() {
        let valid_schema = r#"
package validation

#BeadImplementation: {
  bead_id: "test"
}
"#;
        assert!(schema_has_valid_structure(valid_schema));
    }

    #[test]
    fn schema_has_valid_structure_false_for_missing_package() {
        let invalid_schema = r#"
#BeadImplementation: {
  bead_id: "test"
}
"#;
        assert!(!schema_has_valid_structure(invalid_schema));
    }

    #[test]
    fn schema_has_valid_structure_false_for_missing_schema() {
        let invalid_schema = r#"
package validation

// Missing #BeadImplementation
"#;
        assert!(!schema_has_valid_structure(invalid_schema));
    }

    #[test]
    fn cue_artifact_stable_for_same_inputs() {
        let bead_id = make_bead_id("src-s6d");
        let timestamp = Utc::now();

        let artifact1 = CueArtifact::generate(bead_id.clone(), "Test", timestamp).unwrap();
        let artifact2 = CueArtifact::generate(bead_id, "Test", timestamp).unwrap();

        assert_eq!(artifact1, artifact2);
    }

    #[test]
    fn cue_artifact_different_for_different_beads() {
        let bead_id1 = make_bead_id("src-abc");
        let bead_id2 = make_bead_id("src-xyz");
        let timestamp = Utc::now();

        let artifact1 = CueArtifact::generate(bead_id1, "Test", timestamp).unwrap();
        let artifact2 = CueArtifact::generate(bead_id2, "Test", timestamp).unwrap();

        assert_ne!(artifact1.bead_id, artifact2.bead_id);
        assert_ne!(artifact1.filename, artifact2.filename);
    }
}
