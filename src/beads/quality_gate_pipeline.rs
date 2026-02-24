//! Quality Gates: Full Pipeline Orchestration
//!
//! Orchestrates the complete quality gate workflow for a stage.
//! Pure function: no I/O, stable pipeline composition.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::StageName;
use im::Vector;
use thiserror::Error;

// Import other bead modules
use crate::beads::gate_aggregation;
use crate::beads::gate_decision;
use crate::beads::gate_execution;
use crate::beads::gate_report;
use crate::beads::gate_selection;
use crate::beads::moon_command;

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

/// Orchestrate the full quality gate pipeline for a stage.
///
/// The `executor` closure is a capability injection: it receives a command string
/// and returns the exit code. This keeps the pipeline pure while allowing the
/// shell layer to provide real subprocess execution.
///
/// # Errors
/// Returns [`QualityGatePipelineError`] if aggregation fails
pub fn run_quality_gate_pipeline(
    stage: StageName,
    executor: impl Fn(&str) -> i32,
) -> Result<PipelineResult, QualityGatePipelineError> {
    // Step 1: Select typed gates for stage
    let gates = gate_selection::select_gates(&stage);

    if gates.is_empty() {
        return Ok(PipelineResult {
            stage,
            passed: true,
            decisions: Vec::new(),
            total_gates: 0,
            passed_gates: 0,
        });
    }

    // Step 2: Execute each gate using generated moon command via the injected executor
    let gate_results: Vec<_> = gates
        .iter()
        .map(|gate| {
            let command = moon_command::generate_moon_command(gate);
            gate_execution::execute_gate(gate.as_str(), &command.command, &executor)
        })
        .collect();

    // Step 4: Aggregate results
    let gate_results_vector: Vector<_> = gate_results.into();
    let aggregated = gate_aggregation::aggregate_gate_results(stage.clone(), &gate_results_vector)
        .map_err(|e| QualityGatePipelineError::Aggregation(e.to_string()))?;

    // Step 5: Build report
    let report = gate_report::build_gate_report(stage.clone(), &aggregated);

    // Step 6: Make decision
    let decision = gate_decision::make_gate_decision(&report);

    let stats = report.stats();
    Ok(PipelineResult {
        stage,
        passed: decision.is_passed(),
        decisions: vec![decision],
        total_gates: stats.total,
        passed_gates: stats.passed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test executor: always returns exit code 0 (all gates pass).
    // Tests here verify pipeline orchestration logic, not real command execution.
    fn passing_executor(_cmd: &str) -> i32 {
        0
    }

    // Test executor: always returns exit code 1 (all gates fail).
    fn failing_executor(_cmd: &str) -> i32 {
        1
    }

    #[test]
    fn pipeline_explore_stage() {
        let result = run_quality_gate_pipeline(StageName::JjWorkspace, passing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 0);
        assert_eq!(result.passed_gates, 0);
    }

    #[test]
    fn pipeline_contract_stage() {
        let result =
            run_quality_gate_pipeline(StageName::Implementation, passing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 2);
        assert_eq!(result.passed_gates, 2);
    }

    #[test]
    fn pipeline_red_stage() {
        let result =
            run_quality_gate_pipeline(StageName::Implementation, passing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 2);
        assert_eq!(result.passed_gates, 2);
    }

    #[test]
    fn pipeline_implementation_stage() {
        let result =
            run_quality_gate_pipeline(StageName::Implementation, passing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 2);
        assert_eq!(result.passed_gates, 2);
    }

    #[test]
    fn pipeline_witness_stage() {
        let result = run_quality_gate_pipeline(StageName::Main, passing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 1);
        assert_eq!(result.passed_gates, 1);
    }

    #[test]
    fn pipeline_ship_gate_stage() {
        let result = run_quality_gate_pipeline(StageName::Main, passing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 1);
        assert_eq!(result.passed_gates, 1);
    }

    #[test]
    fn pipeline_preserves_stage() {
        let result = run_quality_gate_pipeline(StageName::Main, passing_executor).unwrap();
        assert_eq!(result.stage, StageName::Main);
    }

    #[test]
    fn pipeline_stable() {
        let result1 =
            run_quality_gate_pipeline(StageName::Implementation, passing_executor).unwrap();
        let result2 =
            run_quality_gate_pipeline(StageName::Implementation, passing_executor).unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn pipeline_all_stages_pass_with_passing_executor() {
        let stages = [
            StageName::JjWorkspace,
            StageName::Implementation,
            StageName::Implementation,
            StageName::Implementation,
            StageName::Main,
            StageName::Main,
        ];

        for stage in stages {
            let result = run_quality_gate_pipeline(stage.clone(), passing_executor).unwrap();
            assert!(result.passed, "Stage {:?} should pass with passing executor", stage);
        }
    }

    #[test]
    fn pipeline_stages_with_gates_fail_when_executor_fails() {
        // Stages that have gates should fail when the executor returns non-zero
        let stages_with_gates = [
            StageName::Implementation,
            StageName::Implementation,
            StageName::Implementation,
            StageName::Main,
            StageName::Main,
        ];

        for stage in stages_with_gates {
            let result = run_quality_gate_pipeline(stage.clone(), failing_executor).unwrap();
            assert!(!result.passed, "Stage {:?} should fail with failing executor", stage);
        }
    }

    #[test]
    fn pipeline_explore_passes_regardless_of_executor() {
        // Explore has no gates — executor result is irrelevant
        let result = run_quality_gate_pipeline(StageName::JjWorkspace, failing_executor).unwrap();
        assert!(result.passed);
        assert_eq!(result.total_gates, 0);
    }
}
