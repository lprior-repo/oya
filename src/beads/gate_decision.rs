//! Quality Gates: Decision Making
//!
//! Makes pass/fail decision based on gate reports.
//! Pure function: no I/O, deterministic decision logic.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::{FailureCategory, StageName};
use thiserror::Error;

// Re-export types from other modules
pub use crate::beads::gate_report::GateReport;

/// Error types for decision making
#[derive(Debug, Error)]
pub enum GateDecisionError {
    #[error("Empty report for stage {0}")]
    EmptyReport(String),
}

/// Quality gate decision for a stage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDecision {
    pub stage: StageName,
    pub passed: bool,
    pub reason: String,
    pub failure_category: Option<FailureCategory>,
}

/// Make pass/fail decision based on gate report
/// Pure function: evaluates report and returns decision
#[must_use]
pub fn make_gate_decision(report: &GateReport) -> GateDecision {
    let reason = if report.passed {
        format!("All {} gates passed for stage {}", report.passed_gates, report.stage.as_str())
    } else {
        format!(
            "{}/{} gates failed for stage {}",
            report.total_gates - report.passed_gates,
            report.total_gates,
            report.stage.as_str()
        )
    };

    GateDecision {
        stage: report.stage.clone(),
        passed: report.passed,
        reason,
        failure_category: report.failure_category.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_passed() {
        let report = GateReport {
            stage: StageName::Plan,
            passed: true,
            total_gates: 1,
            passed_gates: 1,
            gate_entries: Vec::new(),
            failure_category: None,
        };

        let decision = make_gate_decision(&report);
        assert!(decision.passed);
        assert!(decision.reason.contains("All"));
        assert!(decision.reason.contains("passed"));
        assert!(decision.failure_category.is_none());
    }

    #[test]
    fn decision_failed() {
        let report = GateReport {
            stage: StageName::Tdd15,
            passed: false,
            total_gates: 2,
            passed_gates: 1,
            gate_entries: Vec::new(),
            failure_category: Some(FailureCategory::TestFailed),
        };

        let decision = make_gate_decision(&report);
        assert!(!decision.passed);
        assert!(decision.reason.contains("failed"));
        assert!(decision.failure_category.is_some());
    }
}
