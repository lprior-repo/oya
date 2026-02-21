//! Quality Gates: Gate Result Aggregation
//!
//! Aggregates individual gate results into a summary.
//! Pure function: no I/O, stable aggregation.
//!
//! # Design (Scott Wlaschin DDD)
//!
//! - `AggregatedGateResult` is a sum type: `Passed` or `Failed`
//! - Illegal states are unrepresentable: `Passed` has no failure details
//! - `Failed` captures which gates failed

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

/// Statistics for aggregated gate results
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregationStats {
    pub total_count: usize,
    pub passed_count: usize,
}

impl AggregationStats {
    /// Calculate failed count
    #[must_use]
    pub const fn failed_count(&self) -> usize {
        self.total_count - self.passed_count
    }

    /// Check if all gates passed
    #[must_use]
    pub const fn all_passed(&self) -> bool {
        self.passed_count == self.total_count
    }
}

/// Aggregated result for a stage
///
/// Sum type encoding: Passed and Failed are mutually exclusive states.
/// No `bool` flags - illegal states are unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregatedGateResult {
    Passed {
        stage: StageName,
        stats: AggregationStats,
        gate_names: Vector<String>,
    },
    Failed {
        stage: StageName,
        stats: AggregationStats,
        gate_names: Vector<String>,
        failed_gate_names: Vector<String>,
    },
}

impl AggregatedGateResult {
    /// Get the stage name regardless of variant
    #[must_use]
    pub const fn stage(&self) -> &StageName {
        match self {
            Self::Passed { stage, .. } | Self::Failed { stage, .. } => stage,
        }
    }

    /// Get the stats regardless of variant
    #[must_use]
    pub const fn stats(&self) -> &AggregationStats {
        match self {
            Self::Passed { stats, .. } | Self::Failed { stats, .. } => stats,
        }
    }

    /// Get all gate names regardless of variant
    #[must_use]
    pub const fn gate_names(&self) -> &Vector<String> {
        match self {
            Self::Passed { gate_names, .. } | Self::Failed { gate_names, .. } => gate_names,
        }
    }

    /// Get failed gate names if any failed
    #[must_use]
    pub const fn failed_gate_names(&self) -> Option<&Vector<String>> {
        match self {
            Self::Failed { failed_gate_names, .. } => Some(failed_gate_names),
            Self::Passed { .. } => None,
        }
    }

    /// Check if result is passed
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// Check if result is failed
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
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
    let passed_count = results.iter().filter(|r| r.is_passed()).count();
    let all_passed = passed_count == total_count;

    let gate_names = results.iter().map(|r| r.gate_name().to_string()).collect::<Vector<_>>();

    let stats = AggregationStats { total_count, passed_count };

    if all_passed {
        Ok(AggregatedGateResult::Passed { stage, stats, gate_names })
    } else {
        let failed_gate_names = results
            .iter()
            .filter(|r| r.is_failed())
            .map(|r| r.gate_name().to_string())
            .collect::<Vector<_>>();
        Ok(AggregatedGateResult::Failed { stage, stats, gate_names, failed_gate_names })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec_passed(name: &str) -> GateExecutionResult {
        GateExecutionResult::Passed { gate_name: name.to_string() }
    }

    fn exec_failed(name: &str, exit_code: i32) -> GateExecutionResult {
        GateExecutionResult::Failed { gate_name: name.to_string(), exit_code }
    }

    #[test]
    fn aggregate_all_passed() {
        let results = Vector::from(vec![exec_passed("compiles"), exec_passed("tests_pass")]);

        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        assert!(aggregated.is_passed());
        assert!(!aggregated.is_failed());
        assert_eq!(aggregated.stats().passed_count, 2);
        assert_eq!(aggregated.stats().total_count, 2);
        assert!(aggregated.stats().all_passed());
        assert_eq!(aggregated.stats().failed_count(), 0);

        match aggregated {
            AggregatedGateResult::Passed { stage, stats, gate_names } => {
                assert_eq!(stage, StageName::Implementation);
                assert_eq!(stats.total_count, 2);
                assert_eq!(gate_names.len(), 2);
            }
            AggregatedGateResult::Failed { .. } => panic!("expected Passed variant"),
        }
    }

    #[test]
    fn aggregate_some_failed() {
        let results = Vector::from(vec![exec_passed("compiles"), exec_failed("tests_pass", 1)]);

        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        assert!(!aggregated.is_passed());
        assert!(aggregated.is_failed());
        assert_eq!(aggregated.stats().passed_count, 1);
        assert_eq!(aggregated.stats().total_count, 2);
        assert_eq!(aggregated.stats().failed_count(), 1);

        match aggregated {
            AggregatedGateResult::Failed { stage, stats, gate_names, failed_gate_names } => {
                assert_eq!(stage, StageName::Implementation);
                assert_eq!(stats.total_count, 2);
                assert_eq!(stats.passed_count, 1);
                assert_eq!(gate_names.len(), 2);
                assert_eq!(failed_gate_names.len(), 1);
                assert!(failed_gate_names.contains(&"tests_pass".to_string()));
            }
            AggregatedGateResult::Passed { .. } => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn aggregate_empty_results_error() {
        let results = Vector::new();
        let result = aggregate_gate_results(StageName::Contract, &results);
        assert!(matches!(result, Err(GateAggregationError::EmptyResults(_))));
    }

    #[test]
    fn aggregate_all_failed() {
        let results = Vector::from(vec![exec_failed("compiles", 1), exec_failed("tests_pass", 2)]);

        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        assert!(!aggregated.is_passed());
        assert!(aggregated.is_failed());
        assert_eq!(aggregated.stats().passed_count, 0);
        assert_eq!(aggregated.stats().total_count, 2);
        assert_eq!(aggregated.stats().failed_count(), 2);

        match aggregated {
            AggregatedGateResult::Failed { failed_gate_names, .. } => {
                assert_eq!(failed_gate_names.len(), 2);
                assert!(failed_gate_names.contains(&"compiles".to_string()));
                assert!(failed_gate_names.contains(&"tests_pass".to_string()));
            }
            _ => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn aggregate_single_passed() {
        let results = Vector::from(vec![exec_passed("compiles")]);

        let aggregated = aggregate_gate_results(StageName::Contract, &results).unwrap();
        assert!(aggregated.is_passed());
        assert_eq!(aggregated.stats().passed_count, 1);
        assert_eq!(aggregated.stats().total_count, 1);
    }

    #[test]
    fn aggregate_preserves_gate_names() {
        let results = Vector::from(vec![exec_passed("compiles"), exec_passed("tests_pass")]);

        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        let gate_names = aggregated.gate_names();
        assert_eq!(gate_names.len(), 2);
        assert!(gate_names.contains(&"compiles".to_string()));
        assert!(gate_names.contains(&"tests_pass".to_string()));
    }

    #[test]
    fn aggregate_stable() {
        let results = Vector::from(vec![exec_passed("compiles"), exec_failed("tests_pass", 1)]);

        let agg1 = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        let agg2 = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        assert_eq!(agg1, agg2);
    }

    #[test]
    fn aggregate_empty_results_error_contains_stage_name() {
        let results = Vector::new();
        let result = aggregate_gate_results(StageName::Contract, &results);
        match result {
            Err(GateAggregationError::EmptyResults(stage_name)) => {
                assert_eq!(stage_name, "contract");
            }
            _ => panic!("Expected EmptyResults error"),
        }
    }

    #[test]
    fn passed_cannot_have_failed_gate_names() {
        // Demonstrates compile-time guarantee: Passed variant has no failed_gate_names field
        let results = Vector::from(vec![exec_passed("compiles"), exec_passed("tests_pass")]);
        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();

        match aggregated {
            AggregatedGateResult::Passed { .. } => {
                // No failed_gate_names field available - compile-time guarantee
                assert!(aggregated.failed_gate_names().is_none());
            }
            AggregatedGateResult::Failed { .. } => panic!("expected Passed variant"),
        }
    }

    #[test]
    fn failed_must_have_failed_gate_names() {
        // Failed variant always has failed_gate_names - no implicit failure state
        let results = Vector::from(vec![exec_passed("compiles"), exec_failed("tests_pass", 1)]);
        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();

        match aggregated {
            AggregatedGateResult::Failed { ref failed_gate_names, .. } => {
                // failed_gate_names is directly available
                assert_eq!(failed_gate_names.len(), 1);
                assert_eq!(aggregated.failed_gate_names(), Some(failed_gate_names));
            }
            AggregatedGateResult::Passed { .. } => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn stats_all_passed_true() {
        let stats = AggregationStats { total_count: 5, passed_count: 5 };
        assert!(stats.all_passed());
        assert_eq!(stats.failed_count(), 0);
    }

    #[test]
    fn stats_all_passed_false() {
        let stats = AggregationStats { total_count: 5, passed_count: 3 };
        assert!(!stats.all_passed());
        assert_eq!(stats.failed_count(), 2);
    }

    #[test]
    fn stage_accessible_from_both_variants() {
        let results_pass = Vector::from(vec![exec_passed("g1")]);
        let agg_pass = aggregate_gate_results(StageName::Contract, &results_pass).unwrap();
        assert_eq!(agg_pass.stage(), &StageName::Contract);

        let results_fail = Vector::from(vec![exec_failed("g1", 1)]);
        let agg_fail = aggregate_gate_results(StageName::Implementation, &results_fail).unwrap();
        assert_eq!(agg_fail.stage(), &StageName::Implementation);
    }

    #[test]
    fn multiple_failed_gates_tracked() {
        let results = Vector::from(vec![
            exec_passed("compiles"),
            exec_failed("test1", 1),
            exec_failed("test2", 2),
            exec_passed("lints"),
            exec_failed("test3", 3),
        ]);

        let aggregated = aggregate_gate_results(StageName::Implementation, &results).unwrap();
        assert!(aggregated.is_failed());
        assert_eq!(aggregated.stats().passed_count, 2);
        assert_eq!(aggregated.stats().total_count, 5);
        assert_eq!(aggregated.stats().failed_count(), 3);

        let failed_names = aggregated.failed_gate_names().unwrap();
        assert_eq!(failed_names.len(), 3);
        assert!(failed_names.contains(&"test1".to_string()));
        assert!(failed_names.contains(&"test2".to_string()));
        assert!(failed_names.contains(&"test3".to_string()));
    }
}
