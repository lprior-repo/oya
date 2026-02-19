//! Quality Gates: Gate Selection
//!
//! Selects gates for a given stage based on stage configuration.
//! Pure function: no I/O, deterministic selection.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::StageName;
use im::Vector;
use thiserror::Error;

/// Error types for gate selection
#[derive(Debug, Error)]
pub enum GateSelectionError {
    #[error("Unknown stage: {0}")]
    UnknownStage(String),
}

/// Input: Stage name
/// Output: Vector of gates to execute for that stage
#[must_use]
pub fn select_gates(stage: &StageName) -> Vector<String> {
    stage.gates().iter().map(|gate| gate.as_str().to_string()).collect::<Vector<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_gates_plan_stage() {
        let gates = select_gates(&StageName::Plan);
        assert_eq!(gates.len(), 1);
        assert!(gates.contains(&"compiles".to_string()));
    }

    #[test]
    fn select_gates_tdd15_stage() {
        let gates = select_gates(&StageName::Tdd15);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&"compiles".to_string()));
        assert!(gates.contains(&"tests_pass".to_string()));
    }

    #[test]
    fn select_gates_ship_gate_stage() {
        let gates = select_gates(&StageName::ShipGate);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&"moon_ci".to_string()));
        assert!(gates.contains(&"zjj_merge_queue".to_string()));
    }
}
