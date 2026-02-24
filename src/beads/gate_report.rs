//! Quality Gates: Gate Report Generation
//!
//! Builds a quality gate report from aggregated results.
//! Pure function: no I/O, stable report construction.
//!
//! # Design (Scott Wlaschin DDD)
//!
//! - `GateReport` is a sum type: `Passed` or `Failed`
//! - Illegal states are unrepresentable: `Passed` has no failure category
//! - `Failed` always has a `FailureCategory` (no Option)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::{FailureCategory, StageName};
use im::Vector;

// Re-export types from other modules
pub use crate::beads::gate_aggregation::AggregatedGateResult;
pub use crate::beads::gate_execution::GateExecutionResult;

/// Gate result statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateStats {
    pub total: usize,
    pub passed: usize,
}

impl GateStats {
    #[must_use]
    pub const fn failed(&self) -> usize {
        self.total - self.passed
    }

    #[must_use]
    pub const fn all_passed(&self) -> bool {
        self.passed == self.total
    }
}

impl From<&crate::beads::gate_aggregation::AggregationStats> for GateStats {
    fn from(stats: &crate::beads::gate_aggregation::AggregationStats) -> Self {
        Self { total: stats.total_count, passed: stats.passed_count }
    }
}

/// Individual gate result entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateReportEntry {
    pub gate_name: String,
    pub passed: bool,
    pub exit_code: i32,
}

/// Quality gate report for a stage
///
/// Sum type encoding: Passed and Failed are mutually exclusive states.
/// No `bool` flags, no `Option<FailureCategory>` - illegal states are unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateReport {
    Passed {
        stage: StageName,
        stats: GateStats,
        entries: Vector<GateReportEntry>,
    },
    Failed {
        stage: StageName,
        stats: GateStats,
        entries: Vector<GateReportEntry>,
        category: FailureCategory,
    },
}

impl GateReport {
    /// Get the stage name regardless of variant
    #[must_use]
    pub const fn stage(&self) -> &StageName {
        match self {
            Self::Passed { stage, .. } | Self::Failed { stage, .. } => stage,
        }
    }

    /// Get the stats regardless of variant
    #[must_use]
    pub const fn stats(&self) -> &GateStats {
        match self {
            Self::Passed { stats, .. } | Self::Failed { stats, .. } => stats,
        }
    }

    /// Get the entries regardless of variant
    #[must_use]
    pub const fn entries(&self) -> &Vector<GateReportEntry> {
        match self {
            Self::Passed { entries, .. } | Self::Failed { entries, .. } => entries,
        }
    }

    /// Check if report is passed
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// Check if report is failed
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get failure category if failed
    #[must_use]
    pub const fn failure_category(&self) -> Option<&FailureCategory> {
        match self {
            Self::Failed { category, .. } => Some(category),
            Self::Passed { .. } => None,
        }
    }
}

/// Build a gate report from aggregated results
///
/// Pure function: constructs typed report from aggregated data.
/// Illegal state combinations are prevented by the type system.
#[must_use]
pub fn build_gate_report(stage: StageName, aggregated: &AggregatedGateResult) -> GateReport {
    let stats = GateStats::from(aggregated.stats());
    let gate_names = aggregated.gate_names();

    let entries: Vector<_> = gate_names
        .iter()
        .map(|gate_name| {
            // Determine if this specific gate passed by checking the aggregated result
            let (passed, exit_code) = match aggregated {
                AggregatedGateResult::Passed { .. } => (true, 0),
                AggregatedGateResult::Failed { failed_gate_names, .. } => {
                    let failed = failed_gate_names.contains(gate_name);
                    (!failed, i32::from(failed))
                }
            };
            GateReportEntry { gate_name: gate_name.clone(), passed, exit_code }
        })
        .collect();

    match aggregated {
        AggregatedGateResult::Passed { .. } => GateReport::Passed { stage, stats, entries },
        AggregatedGateResult::Failed { .. } => {
            GateReport::Failed { stage, stats, entries, category: FailureCategory::TestFailed }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beads::gate_aggregation::AggregationStats;

    fn agg_passed(stage: StageName, count: usize) -> AggregatedGateResult {
        AggregatedGateResult::Passed {
            stage: stage.clone(),
            stats: AggregationStats { total_count: count, passed_count: count },
            gate_names: (0..count).map(|i| format!("gate_{}", i)).collect::<Vector<_>>(),
        }
    }

    fn agg_failed(stage: StageName, passed: usize, total: usize) -> AggregatedGateResult {
        let gate_names: Vector<_> = (0..total).map(|i| format!("gate_{}", i)).collect();
        let failed_gate_names: Vector<_> = (passed..total).map(|i| format!("gate_{}", i)).collect();
        AggregatedGateResult::Failed {
            stage: stage.clone(),
            stats: AggregationStats { total_count: total, passed_count: passed },
            gate_names,
            failed_gate_names,
        }
    }

    #[test]
    fn build_report_passed_variant() {
        let aggregated = agg_passed(StageName::Implementation, 1);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        assert!(report.is_passed());
        assert!(!report.is_failed());
        assert!(report.failure_category().is_none());

        match report {
            GateReport::Passed { stage, stats, entries } => {
                assert_eq!(stage, StageName::Implementation);
                assert_eq!(stats.total, 1);
                assert_eq!(stats.passed, 1);
                assert_eq!(entries.len(), 1);
            }
            GateReport::Failed { .. } => panic!("expected Passed variant"),
        }
    }

    #[test]
    fn build_report_failed_variant() {
        let aggregated = agg_failed(StageName::Implementation, 1, 2);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        assert!(!report.is_passed());
        assert!(report.is_failed());

        match report {
            GateReport::Failed { stage, stats, category, .. } => {
                assert_eq!(stage, StageName::Implementation);
                assert_eq!(stats.total, 2);
                assert_eq!(stats.passed, 1);
                assert_eq!(stats.failed(), 1);
                assert_eq!(category, FailureCategory::TestFailed);
            }
            GateReport::Passed { .. } => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn build_report_all_passed() {
        let aggregated = agg_passed(StageName::Implementation, 2);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        assert!(report.is_passed());
        let stats = report.stats();
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.total, 2);
        assert!(stats.all_passed());
    }

    #[test]
    fn build_report_all_failed() {
        let aggregated = agg_failed(StageName::Implementation, 0, 2);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        assert!(report.is_failed());
        let stats = report.stats();
        assert_eq!(stats.passed, 0);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.failed(), 2);
    }

    #[test]
    fn build_report_preserves_stage() {
        let aggregated = agg_passed(StageName::Implementation, 2);
        let report = build_gate_report(StageName::Implementation, &aggregated);
        assert_eq!(report.stage(), &StageName::Implementation);
    }

    #[test]
    fn build_report_failure_category_set_when_failed() {
        let aggregated = agg_failed(StageName::Implementation, 1, 2);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        match report {
            GateReport::Failed { category, .. } => {
                assert_eq!(category, FailureCategory::TestFailed);
            }
            _ => panic!("should be Failed"),
        }
    }

    #[test]
    fn build_report_failure_category_none_when_passed() {
        let aggregated = agg_passed(StageName::Implementation, 1);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        match report {
            GateReport::Passed { .. } => {
                assert!(report.failure_category().is_none());
            }
            _ => panic!("should be Passed"),
        }
    }

    #[test]
    fn build_report_stable() {
        let aggregated = agg_failed(StageName::Implementation, 1, 2);
        let report1 = build_gate_report(StageName::Implementation, &aggregated);
        let report2 = build_gate_report(StageName::Implementation, &aggregated);
        assert_eq!(report1, report2);
    }

    #[test]
    fn stats_failed_calculation() {
        let stats = GateStats { total: 5, passed: 3 };
        assert_eq!(stats.failed(), 2);
        assert!(!stats.all_passed());
    }

    #[test]
    fn passed_cannot_have_failure_category() {
        // Demonstrates compile-time guarantee: Passed variant has no category field
        let aggregated = agg_passed(StageName::Implementation, 1);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        match report {
            GateReport::Passed { .. } => {
                // No category field available - compile-time guarantee
                assert!(report.failure_category().is_none());
            }
            GateReport::Failed { .. } => panic!("expected Passed"),
        }
    }

    #[test]
    fn failed_must_have_failure_category() {
        // Failed variant always has a category - no Option needed
        let aggregated = agg_failed(StageName::Implementation, 0, 1);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        match report {
            GateReport::Failed { category, .. } => {
                // Category is directly available, not wrapped in Option
                assert_eq!(category, FailureCategory::TestFailed);
            }
            _ => panic!("should be Failed"),
        }
    }

    #[test]
    fn entries_accessible_from_both_variants() {
        let agg_pass = agg_passed(StageName::Implementation, 2);
        let report_pass = build_gate_report(StageName::Implementation, &agg_pass);
        assert_eq!(report_pass.entries().len(), 2);

        let agg_fail = agg_failed(StageName::Implementation, 1, 2);
        let report_fail = build_gate_report(StageName::Implementation, &agg_fail);
        assert_eq!(report_fail.entries().len(), 2);
    }

    #[test]
    fn report_entries_show_correct_pass_fail_status() {
        let aggregated = agg_failed(StageName::Implementation, 1, 3);
        let report = build_gate_report(StageName::Implementation, &aggregated);

        let entries = report.entries();
        assert_eq!(entries.len(), 3);

        // First gate should have passed (index 0 is in the passed range)
        assert!(entries[0].passed);
        assert_eq!(entries[0].gate_name, "gate_0");

        // Second and third gates should have failed
        assert!(!entries[1].passed);
        assert_eq!(entries[1].gate_name, "gate_1");
        assert!(!entries[2].passed);
        assert_eq!(entries[2].gate_name, "gate_2");
    }

    #[test]
    fn stats_conversion_from_aggregation_stats() {
        let agg_stats = AggregationStats { total_count: 10, passed_count: 7 };
        let gate_stats = GateStats::from(&agg_stats);

        assert_eq!(gate_stats.total, 10);
        assert_eq!(gate_stats.passed, 7);
        assert_eq!(gate_stats.failed(), 3);
    }
}
