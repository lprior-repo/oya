//! Quality Gates: Gate Result Aggregation
//!
//! Aggregates individual gate results into a summary.
//! Pure function: no I/O, deterministic aggregation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::StageName;
use im::Vector;
use thiserror::Error;

// Re-export types from other modules
pub use crate::beads::gate_execution::GateExecutionResult;

/// Error types for aggregation
#[derive(Debug, Error)]
pub enum GateAggregationError {
    #[error("Empty gate results for stage {0}")]
    EmptyResults(String),
}

/// Aggregated result for a stage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedGateResult {
    pub stage: StageName,
    pub passed: bool,
    pub passed_count: usize,
    pub total_count: usize,
    pub gate_results: Vector<String>,
}

/// Aggregate gate results for a stage
/// Pure function: combines individual results into summary
/// # Errors
/// Returns [`GateAggregationError::EmptyResults`] if results vector is empty
pub fn aggregate_gate_results(
    stage: StageName,
    results: &Vector<GateExecutionResult>,
) -> Result<AggregatedGateResult, GateAggregationError> {
    if results.is_empty() {
        return Err(GateAggregationError::EmptyResults(stage.as_str().to_string()));
    }

    let total_count = results.len();
    let passed_count = results.iter().filter(|r| r.passed).count();
    let passed = passed_count == total_count;

    let gate_results = results.iter().map(|r| r.gate_name.clone()).collect::<Vector<_>>();

    Ok(AggregatedGateResult { stage, passed, passed_count, total_count, gate_results })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_all_passed() {
        let results = Vector::from(vec![
            GateExecutionResult { gate_name: "compiles".to_string(), passed: true, exit_code: 0 },
            GateExecutionResult { gate_name: "tests_pass".to_string(), passed: true, exit_code: 0 },
        ]);

        let aggregated = aggregate_gate_results(StageName::Tdd15, &results).unwrap();
        assert!(aggregated.passed);
        assert_eq!(aggregated.passed_count, 2);
        assert_eq!(aggregated.total_count, 2);
    }

    #[test]
    fn aggregate_some_failed() {
        let results = Vector::from(vec![
            GateExecutionResult { gate_name: "compiles".to_string(), passed: true, exit_code: 0 },
            GateExecutionResult {
                gate_name: "tests_pass".to_string(),
                passed: false,
                exit_code: 1,
            },
        ]);

        let aggregated = aggregate_gate_results(StageName::Tdd15, &results).unwrap();
        assert!(!aggregated.passed);
        assert_eq!(aggregated.passed_count, 1);
        assert_eq!(aggregated.total_count, 2);
    }

    #[test]
    fn aggregate_empty_results_error() {
        let results = Vector::new();
        let result = aggregate_gate_results(StageName::Plan, &results);
        assert!(matches!(result, Err(GateAggregationError::EmptyResults(_))));
    }

    #[test]
    fn aggregate_all_failed() {
        let results = Vector::from(vec![
            GateExecutionResult { gate_name: "compiles".to_string(), passed: false, exit_code: 1 },
            GateExecutionResult {
                gate_name: "tests_pass".to_string(),
                passed: false,
                exit_code: 1,
            },
        ]);

        let aggregated = aggregate_gate_results(StageName::Tdd15, &results).unwrap();
        assert!(!aggregated.passed);
        assert_eq!(aggregated.passed_count, 0);
        assert_eq!(aggregated.total_count, 2);
    }

    #[test]
    fn aggregate_single_passed() {
        let results = Vector::from(vec![GateExecutionResult {
            gate_name: "compiles".to_string(),
            passed: true,
            exit_code: 0,
        }]);

        let aggregated = aggregate_gate_results(StageName::Plan, &results).unwrap();
        assert!(aggregated.passed);
        assert_eq!(aggregated.passed_count, 1);
        assert_eq!(aggregated.total_count, 1);
    }

    #[test]
    fn aggregate_preserves_gate_names() {
        let results = Vector::from(vec![
            GateExecutionResult { gate_name: "compiles".to_string(), passed: true, exit_code: 0 },
            GateExecutionResult { gate_name: "tests_pass".to_string(), passed: true, exit_code: 0 },
        ]);

        let aggregated = aggregate_gate_results(StageName::Tdd15, &results).unwrap();
        assert_eq!(aggregated.gate_results.len(), 2);
        assert!(aggregated.gate_results.contains(&"compiles".to_string()));
        assert!(aggregated.gate_results.contains(&"tests_pass".to_string()));
    }

    #[test]
    fn aggregate_deterministic() {
        let results = Vector::from(vec![
            GateExecutionResult { gate_name: "compiles".to_string(), passed: true, exit_code: 0 },
            GateExecutionResult {
                gate_name: "tests_pass".to_string(),
                passed: false,
                exit_code: 1,
            },
        ]);

        let agg1 = aggregate_gate_results(StageName::Tdd15, &results).unwrap();
        let agg2 = aggregate_gate_results(StageName::Tdd15, &results).unwrap();
        assert_eq!(agg1, agg2);
    }

    #[test]
    fn aggregate_empty_results_error_contains_stage_name() {
        let results = Vector::new();
        let result = aggregate_gate_results(StageName::Plan, &results);
        match result {
            Err(GateAggregationError::EmptyResults(stage_name)) => {
                assert_eq!(stage_name, "plan");
            }
            _ => panic!("Expected EmptyResults error"),
        }
    }
}
