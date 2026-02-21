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

/// Given: Contract stage
/// When: Gates run
/// Then: Should only run Compiles gate
#[tokio::test]
async fn given_contract_stage_when_gates_run_then_only_compiles_required() {
    use oya::types::StageName;

    let gates = StageName::Contract.gates();

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

/// Given: Implementation stage
/// When: Gates run
/// Then: Should run Compiles and TestsPass
#[tokio::test]
async fn given_implementation_when_gates_run_then_runs_compile_and_test() {
    use oya::types::StageName;

    let gates = StageName::Implementation.gates();

    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::Compiles));
    assert!(gates.contains(&Gate::TestsPass));
}

/// Given: All 5 canonical stages
/// When: Gates are checked
/// Then: Each stage should have appropriate gates
#[tokio::test]
async fn given_all_stages_when_gates_checked_then_appropriate_for_stage() {
    use oya::types::StageName;

    // Contract stage: Just compile
    let contract_gates = StageName::Contract.gates();
    assert_eq!(contract_gates.len(), 1, "Contract should have 1 gate");
    assert_eq!(contract_gates[0], Gate::Compiles);

    // AcceptanceTest stage: Compile + tests must be red
    let acceptance_gates = StageName::AcceptanceTest.gates();
    assert!(acceptance_gates.contains(&Gate::Compiles));
    assert!(acceptance_gates.contains(&Gate::AcceptanceTestsAreRed));

    // Implementation stage: Compile + tests
    let impl_gates = StageName::Implementation.gates();
    assert!(impl_gates.contains(&Gate::Compiles));
    assert!(impl_gates.contains(&Gate::TestsPass));

    // Review stage: Consolidated quality gates
    let review_gates = StageName::Review.gates();
    assert!(review_gates.contains(&Gate::TestsPass));
    assert!(review_gates.contains(&Gate::EdgeCases));
    assert!(review_gates.contains(&Gate::NoVulnerabilities));
    assert!(review_gates.contains(&Gate::ClippyClean));
    assert!(review_gates.contains(&Gate::Security));

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
