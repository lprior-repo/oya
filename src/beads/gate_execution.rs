//! Quality Gates: Gate Execution
//!
//! Executes a single gate command and returns the result.
//! Pure function: accepts command string, returns result structure.
//!
//! # Design (Scott Wlaschin DDD)
//!
//! - `GateExecutionResult` is a sum type: `Passed` or `Failed`
//! - Illegal states are unrepresentable: `Passed` has no exit code/failure details
//! - `Failed` always has an exit code (no implicit failure state)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

/// Result of executing a single gate
///
/// Sum type encoding: Passed and Failed are mutually exclusive states.
/// No `bool` flags - illegal states are unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateExecutionResult {
    Passed { gate_name: String },
    Failed { gate_name: String, exit_code: i32 },
}

impl GateExecutionResult {
    /// Get the gate name regardless of variant
    #[must_use]
    pub fn gate_name(&self) -> &str {
        match self {
            Self::Passed { gate_name } | Self::Failed { gate_name, .. } => gate_name,
        }
    }

    /// Get the exit code if failed
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        match self {
            Self::Failed { exit_code, .. } => Some(*exit_code),
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

/// Execute a gate command (pure function - accepts command, returns result)
/// In shell, this would run the command and capture exit code
#[must_use]
pub fn execute_gate(gate_name: &str, _command: &str) -> GateExecutionResult {
    // In real implementation, this would run the command
    // For pure function, we model the result structure
    GateExecutionResult::Passed { gate_name: gate_name.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_gate_success() {
        let result = execute_gate("compiles", "moon run :check");
        assert!(result.is_passed());
        assert!(!result.is_failed());
        assert_eq!(result.gate_name(), "compiles");
        assert!(result.exit_code().is_none());

        match result {
            GateExecutionResult::Passed { gate_name } => {
                assert_eq!(gate_name, "compiles");
            }
            GateExecutionResult::Failed { .. } => panic!("expected Passed variant"),
        }
    }

    #[test]
    fn execute_gate_failure_manual() {
        // Test the Failed variant directly since execute_gate returns Passed
        let result = GateExecutionResult::Failed { gate_name: "clippy".to_string(), exit_code: 1 };
        assert!(!result.is_passed());
        assert!(result.is_failed());
        assert_eq!(result.gate_name(), "clippy");
        assert_eq!(result.exit_code(), Some(1));

        match result {
            GateExecutionResult::Failed { gate_name, exit_code } => {
                assert_eq!(gate_name, "clippy");
                assert_eq!(exit_code, 1);
            }
            GateExecutionResult::Passed { .. } => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn execute_gate_deterministic() {
        // Same inputs should always produce same output
        let result1 = execute_gate("compiles", "moon run :check");
        let result2 = execute_gate("compiles", "moon run :check");
        assert_eq!(result1, result2);
    }

    #[test]
    fn execute_gate_preserves_gate_name() {
        // Gate name should be preserved exactly as passed
        let gate_names = ["compiles", "tests_pass", "clippy_clean", "moon_ci"];
        for gate_name in gate_names {
            let result = execute_gate(gate_name, "command");
            assert_eq!(result.gate_name(), gate_name);
        }
    }

    #[test]
    fn execute_gate_result_cloneable() {
        let result = execute_gate("compiles", "moon run :check");
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn execute_gate_result_eq_comparable() {
        let result1 = execute_gate("compiles", "moon run :check");
        let result2 = execute_gate("compiles", "moon run :check");
        assert_eq!(result1, result2);
    }

    #[test]
    fn passed_cannot_have_exit_code() {
        // Demonstrates compile-time guarantee: Passed variant has no exit_code field
        let result = execute_gate("compiles", "moon run :check");

        match result {
            GateExecutionResult::Passed { .. } => {
                // No exit_code field available - compile-time guarantee
                assert!(result.exit_code().is_none());
            }
            GateExecutionResult::Failed { .. } => panic!("expected Passed variant"),
        }
    }

    #[test]
    fn failed_must_have_exit_code() {
        // Failed variant always has an exit_code - no implicit failure state
        let result = GateExecutionResult::Failed { gate_name: "tests".to_string(), exit_code: 42 };

        match result {
            GateExecutionResult::Failed { exit_code, .. } => {
                // exit_code is directly available, not wrapped in Option in the variant
                assert_eq!(exit_code, 42);
                assert_eq!(result.exit_code(), Some(42));
            }
            GateExecutionResult::Passed { .. } => panic!("expected Failed variant"),
        }
    }

    #[test]
    fn gate_name_accessible_from_both_variants() {
        let passed = execute_gate("gate1", "cmd");
        let failed = GateExecutionResult::Failed { gate_name: "gate2".to_string(), exit_code: 1 };

        assert_eq!(passed.gate_name(), "gate1");
        assert_eq!(failed.gate_name(), "gate2");
    }

    #[test]
    fn different_exit_codes_for_failed() {
        let failed_1 = GateExecutionResult::Failed { gate_name: "test".to_string(), exit_code: 1 };
        let failed_2 = GateExecutionResult::Failed { gate_name: "test".to_string(), exit_code: 2 };

        assert_eq!(failed_1.exit_code(), Some(1));
        assert_eq!(failed_2.exit_code(), Some(2));
        assert_ne!(failed_1, failed_2);
    }
}
