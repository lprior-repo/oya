//! Quality Gates: Gate Report Generation
//!
//! Builds a quality gate report from aggregated results.
//! Pure function: no I/O, deterministic report construction.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::{FailureCategory, StageName};
use im::Vector;
use thiserror::Error;

// Re-export types from other modules
pub use crate::beads::gate_aggregation::AggregatedGateResult;
pub use crate::beads::gate_execution::GateExecutionResult;

/// Error types for report generation
#[derive(Debug, Error)]
pub enum GateReportError {
    #[error("Missing aggregated result for stage {0}")]
    MissingResult(String),
}

/// Individual gate result in report
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateReportEntry {
    pub gate_name: String,
    pub passed: bool,
    pub exit_code: i32,
}

/// Quality gate report for a stage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateReport {
    pub stage: StageName,
    pub passed: bool,
    pub total_gates: usize,
    pub passed_gates: usize,
    pub gate_entries: Vector<GateReportEntry>,
    pub failure_category: Option<FailureCategory>,
}

/// Build a gate report from aggregated results
/// Pure function: constructs report structure from data
#[must_use]
pub fn build_gate_report(stage: StageName, aggregated: &AggregatedGateResult) -> GateReport {
    let total_gates = aggregated.total_count;
    let passed_gates = aggregated.passed_count;
    let passed = aggregated.passed;

    let gate_entries: Vector<_> = aggregated
        .gate_results
        .iter()
        .map(|gate_name| GateReportEntry {
            gate_name: gate_name.clone(),
            passed,
            exit_code: i32::from(!passed),
        })
        .collect();

    let failure_category = if passed { None } else { Some(FailureCategory::TestFailed) };

    GateReport { stage, passed, total_gates, passed_gates, gate_entries, failure_category }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_report_passed() {
        let aggregated = AggregatedGateResult {
            stage: StageName::Plan,
            passed: true,
            passed_count: 1,
            total_count: 1,
            gate_results: Vector::from(vec!["compiles".to_string()]),
        };

        let report = build_gate_report(StageName::Plan, &aggregated);
        assert!(report.passed);
        assert_eq!(report.total_gates, 1);
        assert_eq!(report.passed_gates, 1);
        assert!(report.failure_category.is_none());
    }

    #[test]
    fn build_report_failed() {
        let aggregated = AggregatedGateResult {
            stage: StageName::Tdd15,
            passed: false,
            passed_count: 1,
            total_count: 2,
            gate_results: Vector::from(vec!["compiles".to_string(), "tests_pass".to_string()]),
        };

        let report = build_gate_report(StageName::Tdd15, &aggregated);
        assert!(!report.passed);
        assert_eq!(report.total_gates, 2);
        assert_eq!(report.passed_gates, 1);
        assert!(report.failure_category.is_some());
    }
}
