//! Gate execution behavior tests
//!
//! These verify the quality gate system works correctly.

use oya::orchestrator::{FakeOrchestrator, FakeOrchestratorConfig, GateResult, Orchestrator};
use oya::types::Gate;

mod util;

// =============================================================================
// GATE BEHAVIOR: Quality gates
// =============================================================================

/// Given: Compiles gate
/// When: It runs
/// Then: Should check compilation
#[tokio::test]
async fn given_compiles_gate_when_it_runs_then_checks_compilation() {
    let orch = util::passing_orchestrator();

    let result = orch.run_gate(Gate::Compiles).unwrap();

    assert!(result.passed);
    assert_eq!(result.gate_name, "compiles");
}

/// Given: TestsPass gate
/// When: It runs
/// Then: Should check tests
#[tokio::test]
async fn given_testspass_gate_when_it_runs_then_checks_tests() {
    let orch = util::passing_orchestrator();

    let result = orch.run_gate(Gate::TestsPass).unwrap();

    assert!(result.passed);
    assert_eq!(result.gate_name, "tests_pass");
}

/// Given: Gate configured to fail
/// When: It runs
/// Then: Should report failure
#[tokio::test]
async fn given_gate_configured_to_fail_when_it_runs_then_reports_failure() {
    let mut config = FakeOrchestratorConfig::default();
    config.gate_results.insert(
        "compiles".to_string(),
        GateResult {
            gate_name: "compiles".to_string(),
            command: "moon run :check".to_string(),
            passed: false,
            exit_code: 1,
            output: "compilation error".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());
    let result = orch.run_gate(Gate::Compiles).unwrap();

    assert!(!result.passed);
    assert_eq!(result.exit_code, 1);
}

/// Given: Research stage
/// When: Gates run
/// Then: Should only run Compiles gate
#[tokio::test]
async fn given_research_stage_when_gates_run_then_only_compiles_required() {
    use oya::types::StageName;

    let gates = StageName::Research.gates();

    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0], Gate::Compiles);
}

/// Given: ShipGate stage
/// When: Gates run
/// Then: Should run MoonCi and ZjjMergeQueue
#[tokio::test]
async fn given_shipgate_when_gates_run_then_runs_ci_and_merge_checks() {
    use oya::types::StageName;

    let gates = StageName::ShipGate.gates();

    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::MoonCi));
    assert!(gates.contains(&Gate::ZjjMergeQueue));
}

/// Given: Tdd15 stage
/// When: Gates run
/// Then: Should run Compiles and TestsPass
#[tokio::test]
async fn given_tdd15_when_gates_run_then_runs_compile_and_test() {
    use oya::types::StageName;

    let gates = StageName::Tdd15.gates();

    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::Compiles));
    assert!(gates.contains(&Gate::TestsPass));
}

/// Given: All 8 stages
/// When: Gates are checked
/// Then: Each stage should have appropriate gates
#[tokio::test]
async fn given_all_stages_when_gates_checked_then_appropriate_for_stage() {
    use oya::types::StageName;

    // Early stages: Just compile
    for stage in [StageName::Research, StageName::Plan, StageName::Contract] {
        let gates = stage.gates();
        assert_eq!(gates.len(), 1, "{:?} should have 1 gate", stage);
        assert_eq!(gates[0], Gate::Compiles);
    }

    // Implementation stages: Compile + tests
    let tdd15_gates = StageName::Tdd15.gates();
    assert!(tdd15_gates.contains(&Gate::Compiles));
    assert!(tdd15_gates.contains(&Gate::TestsPass));

    // QA stage: Tests + edge cases
    let qa_gates = StageName::Qa.gates();
    assert!(qa_gates.contains(&Gate::TestsPass));
    assert!(qa_gates.contains(&Gate::EdgeCases));

    // Security stage: Vulnerability check
    let redqueen_gates = StageName::RedQueen.gates();
    assert!(redqueen_gates.contains(&Gate::NoVulnerabilities));

    // Review stage: Lint + security
    let gptreview_gates = StageName::GptReview.gates();
    assert!(gptreview_gates.contains(&Gate::ClippyClean));
    assert!(gptreview_gates.contains(&Gate::Security));

    // Ship stage: Full CI + merge check
    let shipgate_gates = StageName::ShipGate.gates();
    assert!(shipgate_gates.contains(&Gate::MoonCi));
    assert!(shipgate_gates.contains(&Gate::ZjjMergeQueue));
}

/// Given: Gate fails
/// When: Stage checks gates
/// Then: Stage should fail
#[tokio::test]
async fn given_gate_fails_when_stage_checks_it_then_stage_fails() {
    let mut config = FakeOrchestratorConfig::default();
    config.gate_results.insert(
        "tests_pass".to_string(),
        GateResult {
            gate_name: "tests_pass".to_string(),
            command: "moon run :test".to_string(),
            passed: false,
            exit_code: 1,
            output: "test failures".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());
    let result = orch.run_gate(Gate::TestsPass).unwrap();

    assert!(!result.passed, "Gate failure should cause stage failure");
}
