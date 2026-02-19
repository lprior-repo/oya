//! Quality Gates: Full Pipeline Orchestration
//!
//! Orchestrates the complete quality gate workflow for a stage.
//! Pure function: no I/O, deterministic pipeline composition.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::StageName;
use thiserror::Error;

// Import other bead modules
use crate::beads::gate_aggregation;
use crate::beads::gate_decision;
use crate::beads::gate_execution;
use crate::beads::gate_report;
use crate::beads::gate_selection;

/// Error types for pipeline orchestration
#[derive(Debug, Error)]
pub enum QualityGatePipelineError {
    #[error("Selection error: {0}")]
    Selection(String),
    #[error("Aggregation error: {0}")]
    Aggregation(String),
    #[error("Report error: {0}")]
    Report(String),
}

/// Pipeline execution result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineResult {
    pub stage: StageName,
    pub passed: bool,
    pub decisions: Vec<gate_decision::GateDecision>,
    pub total_gates: usize,
    pub passed_gates: usize,
}

/// Orchestrate the full quality gate pipeline for a stage
/// Pure function: composes all bead functions in sequence
/// # Errors
/// Returns [`QualityGatePipelineError`] if aggregation fails
#[must_use]
pub fn run_quality_gate_pipeline(
    stage: StageName,
) -> Result<PipelineResult, QualityGatePipelineError> {
    // Step 1: Select gates for stage
    let gate_names = gate_selection::select_gates(&stage);

    // Step 2: Execute each gate (in real implementation, would run commands)
    let gate_results: Vec<_> = gate_names
        .iter()
        .map(|gate_name| {
            gate_execution::execute_gate(gate_name, &format!("command for {gate_name}"))
        })
        .collect();

    // Step 3: Aggregate results
    let aggregated = gate_aggregation::aggregate_gate_results(stage.clone(), &gate_results.into())
        .map_err(|e| QualityGatePipelineError::Aggregation(e.to_string()))?;

    // Step 4: Build report
    let report = gate_report::build_gate_report(stage.clone(), &aggregated);

    // Step 5: Make decision
    let decision = gate_decision::make_gate_decision(&report);

    Ok(PipelineResult {
        stage,
        passed: decision.passed,
        decisions: vec![decision],
        total_gates: report.total_gates,
        passed_gates: report.passed_gates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_plan_stage() {
        let result = run_quality_gate_pipeline(StageName::Plan).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 1);
        assert_eq!(result.passed_gates, 1);
    }

    #[test]
    fn pipeline_tdd15_stage() {
        let result = run_quality_gate_pipeline(StageName::Tdd15).unwrap();
        assert!(result.passed); // In pure function, defaults to success
        assert_eq!(result.total_gates, 2);
        assert_eq!(result.passed_gates, 2);
    }

    #[test]
    fn pipeline_ship_gate_stage() {
        let result = run_quality_gate_pipeline(StageName::ShipGate).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 2);
        assert_eq!(result.passed_gates, 2);
    }
}
