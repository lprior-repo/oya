//! Validation types and main validation function.

use std::collections::HashSet;
use std::fmt;

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}

/// A validation issue found during workflow validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub message: String,
    pub node_id: Option<super::NodeId>,
    pub severity: ValidationSeverity,
}

impl ValidationIssue {
    pub fn error(message: impl Into<String>) -> Self {
        Self { message: message.into(), node_id: None, severity: ValidationSeverity::Error }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self { message: message.into(), node_id: None, severity: ValidationSeverity::Warning }
    }

    pub fn error_for_node(message: impl Into<String>, node_id: super::NodeId) -> Self {
        Self {
            message: message.into(),
            node_id: Some(node_id),
            severity: ValidationSeverity::Error,
        }
    }

    pub fn warning_for_node(message: impl Into<String>, node_id: super::NodeId) -> Self {
        Self {
            message: message.into(),
            node_id: Some(node_id),
            severity: ValidationSeverity::Warning,
        }
    }
}

/// Result of validating a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    #[must_use]
    pub const fn new(valid: bool, issues: Vec<ValidationIssue>) -> Self {
        Self { valid, issues }
    }

    #[must_use]
    pub fn from_issues(issues: Vec<ValidationIssue>) -> Self {
        let valid = issues.iter().all(|issue| issue.severity != ValidationSeverity::Error);
        Self { valid, issues }
    }

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == ValidationSeverity::Error).count()
    }

    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == ValidationSeverity::Warning).count()
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        // Inline the error_count logic to avoid calling a non-const method
        self.issues.iter().any(|i| i.severity == ValidationSeverity::Error)
    }
}

// Re-export validation functions from validation_checks module
pub use crate::graph::validation_checks::structural::{
    validate_entry_points, validate_orphan_nodes, validate_reachability,
};

/// Validates that all node IDs in the workflow are unique.
///
/// # Returns
///
/// Returns a `ValidationIssue` with severity ERROR if duplicate node IDs are found.
#[must_use]
pub fn validate_unique_node_ids(workflow: &super::Workflow) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut seen_ids = HashSet::new();

    for node in &workflow.nodes {
        if !seen_ids.insert(node.id) {
            issues.push(ValidationIssue::error_for_node(
                format!("Duplicate node ID: {}", node.id),
                node.id,
            ));
        }
    }

    issues
}

// The main validation function
#[must_use]
pub fn validate_workflow(workflow: &super::Workflow) -> ValidationResult {
    let mut issues = Vec::new();

    validate_entry_points(workflow, &mut issues);
    validate_reachability(workflow, &mut issues);
    validate_orphan_nodes(workflow, &mut issues);
    issues.extend(validate_unique_node_ids(workflow));

    // Config validation would go here
    // for node in &workflow.nodes {
    //     match workflow_node_from_persisted(node) {
    //         Ok(workflow_node) => validate_node_config(&workflow_node, node, &mut issues),
    //         Err(_) => issues.push(ValidationIssue::error_for_node(
    //             format!("Unknown node type"),
    //             node.id,
    //         )),
    //     }
    // }

    ValidationResult::from_issues(issues)
}
