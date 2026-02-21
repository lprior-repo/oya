//! Quality Gates: Gate Selection
//!
//! Selects gates for a given stage based on stage configuration.
//! Pure function: no I/O, stable selection.

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
    fn select_gates_explore_stage() {
        let gates = select_gates(&StageName::Explore);
        assert!(gates.is_empty());
    }

    #[test]
    fn select_gates_contract_stage() {
        let gates = select_gates(&StageName::Contract);
        assert_eq!(gates.len(), 1);
        assert!(gates.contains(&Gate::Compiles));
    }

    #[test]
    fn select_gates_red_stage() {
        let gates = select_gates(&StageName::Red);
        assert_eq!(gates.len(), 1);
        assert!(gates.contains(&Gate::Compiles));
    }

    #[test]
    fn select_gates_implementation_stage() {
        let gates = select_gates(&StageName::Implementation);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&Gate::Compiles));
        assert!(gates.contains(&Gate::TestsPass));
    }

    #[test]
    fn select_gates_witness_stage() {
        let gates = select_gates(&StageName::Witness);
        assert_eq!(gates.len(), 1);
        assert!(gates.contains(&Gate::HoldoutScenarios));
    }

    #[test]
    fn select_gates_ship_gate_stage() {
        let gates = select_gates(&StageName::ShipGate);
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&Gate::CueArtifactGenerated));
        assert!(gates.contains(&Gate::ZjjMergeQueue));
    }

    #[test]
    fn select_gates_all_stages_match_stage_gates() {
        let stages = vec![
            StageName::Explore,
            StageName::Contract,
            StageName::Red,
            StageName::Implementation,
            StageName::Witness,
            StageName::ShipGate,
        ];

        stages.into_iter().for_each(|stage| {
            let selected: Vec<_> = select_gates(&stage).into_iter().collect();
            assert_eq!(selected, stage.gates());
        });
    }

    #[test]
    fn select_gates_stable() {
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
