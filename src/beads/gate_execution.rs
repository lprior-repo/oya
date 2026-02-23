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

/// Execute a gate command via an injected executor.
///
/// The executor is a capability-based injection that separates the pure gate
/// decision logic from the impure subprocess execution. The caller (shell layer)
/// provides an executor that runs the command and returns the exit code.
///
/// # Arguments
/// * `gate_name` - The name of the gate being executed (preserved in result)
/// * `command` - The command string (passed through to the executor)
/// * `executor` - A closure that runs `command` and returns its exit code
pub fn execute_gate(
    gate_name: &str,
    command: &str,
    executor: impl Fn(&str) -> i32,
) -> GateExecutionResult {
    let exit_code = executor(command);
    if exit_code == 0 {
        GateExecutionResult::Passed { gate_name: gate_name.to_string() }
    } else {
        GateExecutionResult::Failed { gate_name: gate_name.to_string(), exit_code }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(_cmd: &str) -> i32 {
        0
    }
    fn fail(_cmd: &str) -> i32 {
        1
    }
    fn exit_code_42(_cmd: &str) -> i32 {
        42
    }

    #[test]
    fn execute_gate_success_with_passing_executor() {
        let result = execute_gate("compiles", "moon run :check", pass);
        assert!(result.is_passed());
        assert!(!result.is_failed());
        assert_eq!(result.gate_name(), "compiles");
        assert!(result.exit_code().is_none());

        match result {
            GateExecutionResult::Passed { gate_name } => {
                assert_eq!(gate_name, "compiles");
            }
            GateExecutionResult::Failed { .. } => unreachable!("expected Passed variant"),
        }
    }

    #[test]
    fn execute_gate_failure_with_failing_executor() {
        let result = execute_gate("clippy", "moon run :clippy", fail);
        assert!(!result.is_passed());
        assert!(result.is_failed());
        assert_eq!(result.gate_name(), "clippy");
        assert_eq!(result.exit_code(), Some(1));

        match result {
            GateExecutionResult::Failed { gate_name, exit_code } => {
                assert_eq!(gate_name, "clippy");
                assert_eq!(exit_code, 1);
            }
            GateExecutionResult::Passed { .. } => unreachable!("expected Failed variant"),
        }
    }

    #[test]
    fn execute_gate_preserves_exit_code_from_executor() {
        let result = execute_gate("tests", "moon run :test", exit_code_42);
        assert_eq!(result.exit_code(), Some(42));
    }

    #[test]
    fn execute_gate_stable_with_same_executor() {
        let result1 = execute_gate("compiles", "moon run :check", pass);
        let result2 = execute_gate("compiles", "moon run :check", pass);
        assert_eq!(result1, result2);
    }

    #[test]
    fn execute_gate_preserves_gate_name() {
        let gate_names = ["compiles", "tests_pass", "clippy_clean", "moon_ci"];
        for gate_name in gate_names {
            let result = execute_gate(gate_name, "command", pass);
            assert_eq!(result.gate_name(), gate_name);
        }
    }

    #[test]
    fn execute_gate_result_cloneable() {
        let result = execute_gate("compiles", "moon run :check", pass);
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn passed_cannot_have_exit_code() {
        let result = execute_gate("compiles", "moon run :check", pass);
        assert!(result.exit_code().is_none());
    }

    #[test]
    fn failed_must_have_exit_code() {
        let result = GateExecutionResult::Failed { gate_name: "tests".to_string(), exit_code: 42 };

        match result {
            GateExecutionResult::Failed { exit_code, .. } => {
                assert_eq!(exit_code, 42);
                assert_eq!(result.exit_code(), Some(42));
            }
            GateExecutionResult::Passed { .. } => unreachable!("expected Failed variant"),
        }
    }

    #[test]
    fn gate_name_accessible_from_both_variants() {
        let passed = execute_gate("gate1", "cmd", pass);
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
