#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;
use thiserror::Error;

use super::{BeadId, GateId};

const MAX_REPAIR_BUDGET: u8 = 10;
const FORMAT_CATEGORY: &str = "format";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairBudget {
    Available { remaining: NonZeroU8 },
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRepairBudget {
    pub gate_id: GateId,
    pub budget: RepairBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadRepairBudget {
    pub bead_id: BeadId,
    pub budget: RepairBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairMutationScope {
    FormatOnly,
    GateOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairMutationKind {
    FormattingOnly,
    SourceLogic,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RepairBudgetError {
    #[error("repair budget must be greater than zero")]
    Zero,
    #[error("repair budget exceeds max count: {count} > {max}")]
    TooLarge { count: u8, max: u8 },
    #[error("repair budget is exhausted")]
    Exhausted,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MutationScopeViolation {
    #[error(
        "MutationScopeViolation: mutation 'source_logic' is outside repair scope 'format_only'"
    )]
    SourceLogicOutsideFormatOnly,
}

impl RepairBudget {
    /// Creates a repair budget from an initial retry count.
    ///
    /// # Errors
    /// Returns `RepairBudgetError` when the count is zero or exceeds the
    /// bounded per-scope retry maximum.
    pub fn new(count: u8) -> Result<Self, RepairBudgetError> {
        if count == 0 {
            return Err(RepairBudgetError::Zero);
        }
        if count > MAX_REPAIR_BUDGET {
            return Err(RepairBudgetError::TooLarge { count, max: MAX_REPAIR_BUDGET });
        }
        parse_available_budget(count)
    }

    #[must_use]
    pub fn remaining(&self) -> u8 {
        match self {
            Self::Available { remaining } => remaining.get(),
            Self::Exhausted => 0,
        }
    }

    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        matches!(self, Self::Exhausted)
    }

    /// Consumes one repair attempt from the budget.
    ///
    /// # Errors
    /// Returns `RepairBudgetError::Exhausted` when no attempts remain.
    pub fn decrement(&self) -> Result<Self, RepairBudgetError> {
        match self {
            Self::Available { remaining } => decrement_available_budget(remaining.get()),
            Self::Exhausted => Err(RepairBudgetError::Exhausted),
        }
    }
}

impl GateRepairBudget {
    /// Builds a per-gate repair budget.
    ///
    /// # Errors
    /// Returns `RepairBudgetError` when the count is outside the valid range.
    pub fn new(gate_id: GateId, count: u8) -> Result<Self, RepairBudgetError> {
        RepairBudget::new(count).map(|budget| Self { gate_id, budget })
    }

    /// Consumes one repair attempt from this gate budget.
    ///
    /// # Errors
    /// Returns `RepairBudgetError::Exhausted` when no attempts remain.
    pub fn decrement(&self) -> Result<Self, RepairBudgetError> {
        self.budget.decrement().map(|budget| Self { gate_id: self.gate_id, budget })
    }
}

impl BeadRepairBudget {
    /// Builds a per-bead repair budget.
    ///
    /// # Errors
    /// Returns `RepairBudgetError` when the count is outside the valid range.
    pub fn new(bead_id: BeadId, count: u8) -> Result<Self, RepairBudgetError> {
        RepairBudget::new(count).map(|budget| Self { bead_id, budget })
    }

    /// Consumes one repair attempt from this bead budget.
    ///
    /// # Errors
    /// Returns `RepairBudgetError::Exhausted` when no attempts remain.
    pub fn decrement(&self) -> Result<Self, RepairBudgetError> {
        self.budget.decrement().map(|budget| Self { bead_id: self.bead_id.clone(), budget })
    }
}

impl RepairMutationScope {
    #[must_use]
    pub fn from_failure_category(category: &str) -> Self {
        if category == FORMAT_CATEGORY {
            Self::FormatOnly
        } else {
            Self::GateOnly
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FormatOnly => "format_only",
            Self::GateOnly => "gate_only",
        }
    }

    #[must_use]
    pub fn policy_text(&self) -> &'static str {
        match self {
            Self::FormatOnly => "formatting_only_no_source_logic",
            Self::GateOnly => "gate_scoped_changes_only",
        }
    }

    /// Verifies a proposed repair mutation is inside this scope.
    ///
    /// # Errors
    /// Returns `MutationScopeViolation` when the proposed mutation would change
    /// source logic while the repair is limited to formatting-only changes.
    pub fn ensure_allows(
        &self,
        mutation: RepairMutationKind,
    ) -> Result<(), MutationScopeViolation> {
        match (self, mutation) {
            (Self::FormatOnly, RepairMutationKind::SourceLogic) => {
                Err(MutationScopeViolation::SourceLogicOutsideFormatOnly)
            }
            _ => Ok(()),
        }
    }
}

impl RepairMutationKind {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FormattingOnly => "formatting_only",
            Self::SourceLogic => "source_logic",
        }
    }
}

fn parse_available_budget(count: u8) -> Result<RepairBudget, RepairBudgetError> {
    let Some(remaining) = NonZeroU8::new(count) else {
        return Err(RepairBudgetError::Zero);
    };
    Ok(RepairBudget::Available { remaining })
}

fn decrement_available_budget(count: u8) -> Result<RepairBudget, RepairBudgetError> {
    match count {
        0 => Err(RepairBudgetError::Exhausted),
        1 => Ok(RepairBudget::Exhausted),
        remaining => parse_available_budget(remaining.saturating_sub(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::types::BeadId;

    #[test]
    fn repair_budget_decrements_gate_and_bead_counts() {
        let gate_budget = gate_budget(2);
        let bead_budget = bead_budget(2);

        let gate_after_first = gate_budget.decrement();
        let bead_after_first = bead_budget.decrement();

        assert_eq!(gate_after_first.map(|budget| budget.budget.remaining()), Ok(1));
        assert_eq!(bead_after_first.map(|budget| budget.budget.remaining()), Ok(1));
    }

    #[test]
    fn repair_budget_exhausts_after_final_decrement() {
        let budget = repair_budget(1);
        let after_final = budget.decrement();

        assert_eq!(after_final.map(|budget| budget.is_exhausted()), Ok(true));
    }

    #[test]
    fn repair_budget_rejects_invalid_counts() {
        assert_eq!(RepairBudget::new(0), Err(RepairBudgetError::Zero));
        assert_eq!(
            RepairBudget::new(MAX_REPAIR_BUDGET.saturating_add(1)),
            Err(RepairBudgetError::TooLarge {
                count: MAX_REPAIR_BUDGET.saturating_add(1),
                max: MAX_REPAIR_BUDGET,
            })
        );
    }

    #[test]
    fn repair_budget_rejects_decrement_after_exhaustion() {
        let exhausted = repair_budget(1).decrement();
        let Ok(exhausted) = exhausted else {
            assert!(false, "budget should exhaust");
            return;
        };

        assert_eq!(exhausted.decrement(), Err(RepairBudgetError::Exhausted));
    }

    #[test]
    fn mutation_scope_violation_blocks_source_logic_for_format_repairs() {
        let scope = RepairMutationScope::from_failure_category("format");

        let violation = scope.ensure_allows(RepairMutationKind::SourceLogic);

        assert_eq!(scope.as_str(), "format_only");
        assert_eq!(scope.policy_text(), "formatting_only_no_source_logic");
        assert_eq!(violation, Err(MutationScopeViolation::SourceLogicOutsideFormatOnly));
        let Err(violation) = violation else {
            assert!(false, "source logic must violate format-only scope");
            return;
        };
        assert_eq!(
            violation.to_string(),
            "MutationScopeViolation: mutation 'source_logic' is outside repair scope 'format_only'"
        );
    }

    #[test]
    fn mutation_scope_allows_formatting_for_format_repairs() {
        let scope = RepairMutationScope::from_failure_category("format");

        assert_eq!(scope.ensure_allows(RepairMutationKind::FormattingOnly), Ok(()));
        assert_eq!(RepairMutationKind::FormattingOnly.as_str(), "formatting_only");
    }

    fn repair_budget(count: u8) -> RepairBudget {
        let Ok(budget) = RepairBudget::new(count) else {
            assert!(false, "repair budget should parse");
            return RepairBudget::Exhausted;
        };
        budget
    }

    fn gate_budget(count: u8) -> GateRepairBudget {
        let Ok(gate_id) = GateId::parse("fmt") else {
            assert!(false, "gate should parse");
            return GateRepairBudget { gate_id: GateId::Fmt, budget: RepairBudget::Exhausted };
        };
        let Ok(budget) = GateRepairBudget::new(gate_id, count) else {
            assert!(false, "gate budget should parse");
            return GateRepairBudget { gate_id, budget: RepairBudget::Exhausted };
        };
        budget
    }

    fn bead_budget(count: u8) -> BeadRepairBudget {
        let Ok(bead_id) = BeadId::parse("demo") else {
            assert!(false, "bead should parse");
            return BeadRepairBudget {
                bead_id: fallback_bead_id(),
                budget: RepairBudget::Exhausted,
            };
        };
        let Ok(budget) = BeadRepairBudget::new(bead_id, count) else {
            assert!(false, "bead budget should parse");
            return BeadRepairBudget {
                bead_id: fallback_bead_id(),
                budget: RepairBudget::Exhausted,
            };
        };
        budget
    }

    fn fallback_bead_id() -> BeadId {
        let Ok(bead_id) = BeadId::parse("fallback") else {
            std::process::abort();
        };
        bead_id
    }
}
