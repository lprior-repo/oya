//! Quality Gates: Gate Selection
//!
//! Selects gates for a given stage based on stage configuration.
//! Pure function: no I/O, deterministic selection.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::{Gate, StageName};
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
pub fn select_gates(stage: &StageName) -> Vector<Gate> {
    stage.gates().into_iter().collect::<Vector<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_gates_contract_stage() {
        let gates = select_gates(&StageName::Contract);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&Gate::Compiles));
        assert!(gates.contains(&Gate::CueArtifactGenerated));
    }

    #[test]
    fn select_gates_implementation_stage() {
        let gates = select_gates(&StageName::Implementation);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&Gate::Compiles));
        assert!(gates.contains(&Gate::TestsPass));
    }

    #[test]
    fn select_gates_all_stages() {
        // Verify all three canonical stages have at least one gate
        let stages = vec![StageName::Contract, StageName::Implementation, StageName::ShipGate];

        for stage in stages {
            let gates = select_gates(&stage);
            assert!(!gates.is_empty(), "Stage {:?} should have at least one gate", stage);
        }
    }

    #[test]
    fn select_gates_ship_gate_stage() {
        let gates = select_gates(&StageName::ShipGate);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&Gate::MoonCi));
        assert!(gates.contains(&Gate::ZjjMergeQueue));
    }

    #[test]
    fn select_gates_deterministic() {
        // Same input should always produce same output
        let gates1 = select_gates(&StageName::Implementation);
        let gates2 = select_gates(&StageName::Implementation);
        assert_eq!(gates1, gates2);
    }

    #[test]
    fn select_gates_returns_vector() {
        let gates = select_gates(&StageName::Contract);
        // Verify it's an im::Vector
        let _: im::Vector<Gate> = gates;
    }
}
