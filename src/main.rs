#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use oya::application::{run_pipeline, PipelineConfig, PipelineOutcome, StageExecution};
use oya::orchestration::{BeadId, FailureCategory, StageName};
use oya::persistence::OyaDb;

#[derive(serde::Serialize)]
struct RestateInvocation<'a> {
    bead_id: &'a str,
    context: &'a str,
    run_id: &'a str,
    outcome: &'a str,
    reason: Option<&'a str>,
}

fn parse_arg(args: &[String], index: usize, fallback: &str) -> String {
    args.get(index)
        .map_or_else(|| fallback.to_string(), std::clone::Clone::clone)
}

fn parse_failure_category(name: &str) -> FailureCategory {
    match name {
        "test_failed" => FailureCategory::TestFailed,
        "compile_failed" => FailureCategory::CompileFailed,
        "lint_failed" => FailureCategory::LintFailed,
        "provider_unavailable" => FailureCategory::ProviderUnavailable,
        _ => FailureCategory::OutputParseFailure,
    }
}

fn stage_should_fail(stage: &StageName, failing_stage: &Option<StageName>) -> bool {
    failing_stage
        .as_ref()
        .is_some_and(|configured| configured == stage)
}

async fn invoke_restate(endpoint: &str, payload: &RestateInvocation<'_>) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("restate request failed: {e}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("restate returned status {}", response.status()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let bead = parse_arg(&args, 1, "bead-local");
    let context = parse_arg(&args, 2, "default context");

    let db = OyaDb::connect("memory://")
        .await
        .map_err(|e| format!("db connect failed: {e}"))?;
    db.init_schema()
        .await
        .map_err(|e| format!("db init failed: {e}"))?;

    let fail_stage = std::env::var("OYA_FAIL_STAGE")
        .ok()
        .and_then(|value| StageName::try_from(value.as_str()).ok());
    let fail_category = std::env::var("OYA_FAIL_CATEGORY")
        .ok()
        .map(|value| parse_failure_category(value.as_str()))
        .unwrap_or(FailureCategory::OutputParseFailure);

    let outcome = run_pipeline(
        &db,
        BeadId::new(bead.clone()),
        context.as_str(),
        PipelineConfig::default(),
        |stage, attempt, execution_context, previous| {
            let retry_note = previous
                .map(|result| format!("retry_after={:?}", result.failure_category))
                .unwrap_or_else(|| "first_attempt".to_string());

            if stage_should_fail(&stage, &fail_stage) {
                StageExecution::fail(
                    serde_json::json!({
                        "stage": stage.as_str(),
                        "attempt": attempt,
                        "context": execution_context,
                        "note": retry_note,
                    }),
                    fail_category.clone(),
                )
            } else {
                StageExecution::pass(serde_json::json!({
                    "stage": stage.as_str(),
                    "attempt": attempt,
                    "context": execution_context,
                    "note": retry_note,
                    "status": "ok",
                }))
            }
        },
    )
    .await
    .map_err(|e| format!("pipeline execution failed: {e}"))?;

    let restate_endpoint = std::env::var("OYA_RESTATE_ENDPOINT").ok();

    match outcome {
        PipelineOutcome::Shipped { run_id } => {
            println!("shipped run={run_id}");
            if let Some(endpoint) = restate_endpoint {
                let payload = RestateInvocation {
                    bead_id: bead.as_str(),
                    context: context.as_str(),
                    run_id: run_id.as_str(),
                    outcome: "shipped",
                    reason: None,
                };
                invoke_restate(endpoint.as_str(), &payload).await?;
            }
            Ok(())
        }
        PipelineOutcome::Failed { run_id, reason } => {
            println!("failed run={run_id} reason={reason}");
            if let Some(endpoint) = restate_endpoint {
                let payload = RestateInvocation {
                    bead_id: bead.as_str(),
                    context: context.as_str(),
                    run_id: run_id.as_str(),
                    outcome: "failed",
                    reason: Some(reason.as_str()),
                };
                invoke_restate(endpoint.as_str(), &payload).await?;
            }
            Ok(())
        }
    }
}
