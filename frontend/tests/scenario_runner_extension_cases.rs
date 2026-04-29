#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
use anyhow::Result;
use oya_frontend::scenario_runner::run_validation;
use std::collections::HashMap;
use std::path::Path;

#[tokio::test]
async fn extension_behavior_cases_are_counted() -> Result<()> {
    let scenario_dir = Path::new("specs/scenarios/flow_extender");
    let report = run_validation(scenario_dir, "http://127.0.0.1:9", HashMap::new()).await?;

    assert_eq!(report.spec_id, "flow-wasm-v1");
    assert_eq!(report.total_scenarios, 7);
    assert_eq!(report.passed_scenarios, 6);
    assert_eq!(report.failed_scenarios, 1);
    assert!(report.results.iter().any(|result| result.scenario_id == "ext-accept-apply"));
    assert!(report.results.iter().any(|result| result.scenario_id == "ext-preview-hints"));
    assert!(report
        .results
        .iter()
        .any(|result| result.scenario_id == "ext-reject-clear" && !result.passed));
    assert!(report.results.iter().any(|result| result.scenario_id == "ext-bundle-semantics"));
    assert!(report
        .results
        .iter()
        .any(|result| result.scenario_id == "ext-bundle-preview-apply-consistency"));
    assert!(report.results.iter().any(|result| result.scenario_id == "ext-apply-idempotent"));
    assert!(report
        .results
        .iter()
        .any(|result| result.scenario_id == "ext-awakeable-signal-resolution"));

    let category = report
        .category_breakdown
        .get("extension-behavior")
        .ok_or_else(|| anyhow::anyhow!("missing extension-behavior category"))?;
    assert_eq!(category.total, 7);
    assert_eq!(category.passed, 6);
    assert_eq!(category.failed, 1);

    Ok(())
}
