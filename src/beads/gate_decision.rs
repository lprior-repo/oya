//! Quality Gates: Decision Making
//!
//! Makes pass/fail decision based on gate reports.
//! Pure function: no I/O, stable decision logic.
//!
//! # Design (Scott Wlaschin DDD)
//!
//! - `GateDecision` is a sum type: `Passed` or `Failed`
//! - Illegal states are unrepresentable: `Passed` has no failure category
//! - `Failed` always has a `FailureCategory` (no Option)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::{FailureCategory, StageName};

// Re-export types from other modules
pub use crate::beads::gate_report::{GateReport, GateStats};

/// Gate execution statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionStats {
    pub total_gates: usize,
    pub passed_gates: usize,
}

impl DecisionStats {
    #[must_use]
    pub const fn failed_gates(&self) -> usize {
        self.total_gates - self.passed_gates
    }
}

impl From<&GateStats> for DecisionStats {
    fn from(stats: &GateStats) -> Self {
        Self { total_gates: stats.total, passed_gates: stats.passed }
    }
}

/// Quality gate decision for a stage
///
/// Sum type encoding: Passed and Failed are mutually exclusive states.
/// No `bool` flags, no `Option<FailureCategory>` - illegal states are unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Passed { stage: StageName, stats: DecisionStats },
    Failed { stage: StageName, stats: DecisionStats, category: FailureCategory },
}

impl GateDecision {
    /// Get the stage name regardless of variant
    #[must_use]
    pub const fn stage(&self) -> &StageName {
        match self {
            Self::Passed { stage, .. } | Self::Failed { stage, .. } => stage,
        }
    }

    /// Get the stats regardless of variant
    #[must_use]
    pub const fn stats(&self) -> &DecisionStats {
        match self {
            Self::Passed { stats, .. } | Self::Failed { stats, .. } => stats,
        }
    }

    /// Check if decision is passed
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// Check if decision is failed
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

    /// Generate human-readable reason string
    #[must_use]
    pub fn reason(&self) -> String {
        let stage = self.stage().as_str();
        let stats = self.stats();

        match self {
            Self::Passed { .. } => {
                format!("All {} gates passed for stage {}", stats.passed_gates, stage)
            }
            Self::Failed { .. } => {
                format!(
                    "{}/{} gates failed for stage {}",
                    stats.failed_gates(),
                    stats.total_gates,
                    stage
                )
            }
        }
    }
}

/// Make pass/fail decision based on gate report
///
/// Pure function: evaluates report and returns typed decision.
/// Illegal state combinations are prevented by the type system.
#[must_use]
pub fn make_gate_decision(report: &GateReport) -> GateDecision {
    let stats = DecisionStats::from(report.stats());

    match report {
        GateReport::Passed { stage, .. } => GateDecision::Passed { stage: stage.clone(), stats },
        GateReport::Failed { stage, category, .. } => {
            GateDecision::Failed { stage: stage.clone(), stats, category: category.clone() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use im::Vector;

    fn report_passed(stage: StageName, total: usize, passed: usize) -> GateReport {
        GateReport::Passed { stage, stats: GateStats { total, passed }, entries: Vector::new() }
    }

    fn report_failed(
        stage: StageName,
        total: usize,
        passed: usize,
        category: FailureCategory,
    ) -> GateReport {
        GateReport::Failed {
            stage,
            stats: GateStats { total, passed },
            entries: Vector::new(),
            category,
        }
    }

    #[test]
    fn decision_passed_variant() {
        let report = report_passed(StageName::Contract, 1, 1);
        let decision = make_gate_decision(&report);

        assert!(decision.is_passed());
        assert!(!decision.is_failed());
        assert!(decision.failure_category().is_none());

        match decision {
            GateDecision::Passed { stage, stats } => {
                assert_eq!(stage, StageName::Contract);
                assert_eq!(stats.total_gates, 1);
                assert_eq!(stats.passed_gates, 1);
            }
            GateDecision::Failed { .. } => panic!("expected Passed variant"),
        }
    }

    #[test]
    fn decision_failed_variant() {
        let report = report_failed(StageName::Implementation, 2, 1, FailureCategory::TestFailed);
        let decision = make_gate_decision(&report);

        assert!(!decision.is_passed());
        assert!(decision.is_failed());

        match decision {
            GateDecision::Failed { stage, stats, category } => {
                assert_eq!(stage, StageName::Implementation);
                assert_eq!(stats.total_gates, 2);
                assert_eq!(stats.passed_gates, 1);
                assert_eq!(category, FailureCategory::TestFailed);
            }
            GateDecision::Passed { .. } => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn decision_passed_all_gates() {
        let report = report_passed(StageName::Implementation, 2, 2);
        let decision = make_gate_decision(&report);

        assert!(decision.is_passed());
        assert!(decision.reason().contains("All 2 gates passed"));
    }

    #[test]
    fn decision_failed_all_gates() {
        let report = report_failed(StageName::Implementation, 2, 0, FailureCategory::TestFailed);
        let decision = make_gate_decision(&report);

        assert!(decision.is_failed());
        assert!(decision.reason().contains("2/2 gates failed"));
    }

    #[test]
    fn decision_preserves_stage() {
        let report = report_passed(StageName::Implementation, 2, 2);
        let decision = make_gate_decision(&report);
        assert_eq!(decision.stage(), &StageName::Implementation);
    }

    #[test]
    fn decision_failed_preserves_category() {
        let report = report_failed(StageName::ShipGate, 1, 0, FailureCategory::RateLimited);
        let decision = make_gate_decision(&report);

        assert_eq!(decision.failure_category(), Some(&FailureCategory::RateLimited));
    }

    #[test]
    fn decision_reason_format_passed() {
        let report = report_passed(StageName::Contract, 1, 1);
        let decision = make_gate_decision(&report);

        assert!(decision.reason().starts_with("All"));
        assert!(decision.reason().ends_with("passed for stage contract"));
    }

    #[test]
    fn decision_reason_format_failed() {
        let report = report_failed(StageName::Implementation, 3, 1, FailureCategory::TestFailed);
        let decision = make_gate_decision(&report);

        assert!(decision.reason().contains("2/3 gates failed"));
        assert!(decision.reason().ends_with("for stage implementation"));
    }

    #[test]
    fn decision_stable() {
        let report = report_failed(StageName::Implementation, 2, 1, FailureCategory::TestFailed);

        let decision1 = make_gate_decision(&report);
        let decision2 = make_gate_decision(&report);
        assert_eq!(decision1, decision2);
    }

    #[test]
    fn decision_cloneable() {
        let report = report_passed(StageName::Contract, 1, 1);
        let decision = make_gate_decision(&report);
        let cloned = decision.clone();
        assert_eq!(decision, cloned);
    }

    #[test]
    fn stats_failed_gates_calculation() {
        let stats = DecisionStats { total_gates: 5, passed_gates: 3 };
        assert_eq!(stats.failed_gates(), 2);
    }

    #[test]
    fn passed_cannot_have_failure_category() {
        // This test demonstrates that the type system prevents
        // constructing a Passed decision with a failure category.
        // The compiler enforces this - no runtime check needed.
        let report = report_passed(StageName::Contract, 1, 1);
        let decision = make_gate_decision(&report);

        // Can only get category from Failed variant
        match decision {
            GateDecision::Passed { .. } => {
                // No category field available - compile-time guarantee
                assert!(decision.failure_category().is_none());
            }
            _ => panic!("should be Passed"),
        }
    }

    #[test]
    fn failed_must_have_failure_category() {
        // Failed variant always has a category - no Option needed
        let report = report_failed(StageName::Implementation, 1, 0, FailureCategory::CompileFailed);
        let decision = make_gate_decision(&report);

        match decision {
            GateDecision::Failed { category, .. } => {
                // Category is directly available, not wrapped in Option
                assert_eq!(category, FailureCategory::CompileFailed);
            }
            _ => panic!("should be Failed"),
        }
    }

    #[test]
    fn stats_conversion_from_gate_stats() {
        let gate_stats = GateStats { total: 10, passed: 7 };
        let decision_stats = DecisionStats::from(&gate_stats);

        assert_eq!(decision_stats.total_gates, 10);
        assert_eq!(decision_stats.passed_gates, 7);
        assert_eq!(decision_stats.failed_gates(), 3);
    }
}
