use super::{run_lifecycle_with_progress, LifecycleProgressUpdate, LifecycleRunRequest};
use crate::lifecycle::effects::{
    CommandExecutor, CommandFailure, CommandResult, Effect, EffectJournalEntry,
};
use crate::lifecycle::types::FailureCategory;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug)]
struct ExpectedCall {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    result: Result<CommandResult, CommandFailure>,
}

#[derive(Debug)]
struct ScriptedExecutor {
    calls: Mutex<VecDeque<ExpectedCall>>,
}

impl ScriptedExecutor {
    fn new(calls: Vec<ExpectedCall>) -> Self {
        Self { calls: Mutex::new(calls.into_iter().collect()) }
    }

    fn assert_empty(&self) {
        let remaining = self.calls.lock().map_or_else(|_| usize::MAX, |calls| calls.len());
        assert_eq!(remaining, 0, "expected no remaining calls");
    }
}

#[async_trait]
impl CommandExecutor for ScriptedExecutor {
    async fn run(
        &self,
        program: &str,
        args: &[String],
        _timeout: Duration,
        cwd: Option<&str>,
    ) -> Result<CommandResult, CommandFailure> {
        let mut guard = self
            .calls
            .lock()
            .map_err(|_| CommandFailure::Spawn { message: "test mutex poisoned".to_owned() })?;
        let next = guard.pop_front().ok_or_else(|| CommandFailure::Spawn {
            message: "unexpected extra command".to_owned(),
        })?;
        assert_eq!(program, next.program);
        assert_eq!(args, next.args);
        assert_eq!(cwd, next.cwd.as_deref());
        next.result
    }
}

fn ok(stdout: &str) -> Result<CommandResult, CommandFailure> {
    Ok(CommandResult { status_code: Some(0), stdout: stdout.to_owned(), stderr: String::new() })
}

fn non_zero(stderr: &str) -> Result<CommandResult, CommandFailure> {
    Ok(CommandResult { status_code: Some(1), stdout: String::new(), stderr: stderr.to_owned() })
}

fn call(
    program: &str,
    args: &[&str],
    cwd: Option<&str>,
    result: Result<CommandResult, CommandFailure>,
) -> ExpectedCall {
    ExpectedCall {
        program: program.to_owned(),
        args: args.iter().map(|value| (*value).to_owned()).collect(),
        cwd: cwd.map(std::borrow::ToOwned::to_owned),
        result,
    }
}

#[tokio::test]
async fn run_lifecycle_success_path_executes_jj_only_git_bridge() {
    let bead = "edge-test-001";
    let workspace = "oya-edge-test-001";
    let workspace_path = "/home/lewis/src/oya-edge-test-001";
    let prompt =
        "Lifecycle smoke run for bead edge-test-001. Reply with a short JSON status and exit.";
    let pr_body = "## Summary\n- Implements bead `edge-test-001` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling";
    let executor = ScriptedExecutor::new(vec![
        call("br", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
        call("jj", &["workspace", "add", workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt],
            Some(workspace_path),
            ok("{\"status\":\"ok\"}"),
        ),
        call("moon", &["run", ":ci"], Some(workspace_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(workspace_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(workspace_path), ok("")),
        call("jj", &["file", "track", "."], Some(workspace_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-001 via lifecycle"],
            Some(workspace_path),
            ok(""),
        ),
        call("jj", &["bookmark", "set", bead, "-r", "@"], Some(workspace_path), ok("")),
        call(
            "jj",
            &["git", "push", "--remote", "origin", "--bookmark", bead],
            Some(workspace_path),
            ok(""),
        ),
        call(
            "gh",
            &[
                "pr",
                "create",
                "--head",
                bead,
                "--repo",
                "lprior-repo/oya",
                "--base",
                "main",
                "--title",
                "Lifecycle edge-test-001",
                "--body",
                pr_body,
            ],
            Some(workspace_path),
            ok("https://github.com/lprior-repo/oya/pull/321\n"),
        ),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
    ]);

    let mut progress = Vec::<LifecycleProgressUpdate>::new();
    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest {
            bead_id: Some(bead.to_owned()),
            model: None,
            repo: Some("lprior-repo/oya".to_owned()),
        },
        |update| progress.push(update),
    )
    .await;

    executor.assert_empty();
    assert!(result.is_ok());
    let outcome = result.expect("success outcome");
    assert_eq!(outcome.compensation_journal.len(), 1);
    assert!(
        progress
            .iter()
            .any(|event| matches!(event, LifecycleProgressUpdate::Finished { success: true, pr_url: Some(url), .. } if url.ends_with("/pull/321")))
    );
    assert!(progress.iter().any(|event| {
        matches!(
            event,
            LifecycleProgressUpdate::Step {
                step,
                status: super::LifecycleStepStatus::Succeeded,
                details: Some(details),
                ..
            } if step == "opencode" && details.get("events").is_some()
        )
    }));
}

#[tokio::test]
async fn run_lifecycle_pr_output_without_url_triggers_terminal_compensations() {
    let bead = "edge-test-002";
    let workspace = "oya-edge-test-002";
    let workspace_path = "/home/lewis/src/oya-edge-test-002";
    let prompt =
        "Lifecycle smoke run for bead edge-test-002. Reply with a short JSON status and exit.";
    let executor = ScriptedExecutor::new(vec![
        call("br", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
        call("jj", &["workspace", "add", workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt],
            Some(workspace_path),
            ok("{\"status\":\"ok\"}"),
        ),
        call("moon", &["run", ":ci"], Some(workspace_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(workspace_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(workspace_path), ok("")),
        call("jj", &["file", "track", "."], Some(workspace_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-002 via lifecycle"],
            Some(workspace_path),
            ok(""),
        ),
        call("jj", &["bookmark", "set", bead, "-r", "@"], Some(workspace_path), ok("")),
        call(
            "jj",
            &["git", "push", "--remote", "origin", "--bookmark", bead],
            Some(workspace_path),
            ok(""),
        ),
        call(
            "gh",
            &[
                "pr",
                "create",
                "--head",
                bead,
                "--base",
                "main",
                "--title",
                "Lifecycle edge-test-002",
                "--body",
                "## Summary\n- Implements bead `edge-test-002` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling",
            ],
            Some(workspace_path),
            ok("created but no url in output\n"),
        ),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
        call(
            "br",
            &[
                "update",
                bead,
                "--status",
                "blocked",
                "--notes",
                "lifecycle failed after terminal error",
            ],
            None,
            ok(""),
        ),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
    ]);

    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected terminal PR failure");
    assert_eq!(failure.error.category(), FailureCategory::PullRequest);
    assert!(failure.error.is_terminal());
    assert_eq!(failure.compensation_journal.len(), 3);
}

#[tokio::test]
async fn run_lifecycle_transient_failure_skips_terminal_compensations() {
    let bead = "edge-test-003";
    let workspace = "oya-edge-test-003";
    let workspace_path = "/home/lewis/src/oya-edge-test-003";
    let prompt =
        "Lifecycle smoke run for bead edge-test-003. Reply with a short JSON status and exit.";
    let executor = ScriptedExecutor::new(vec![
        call("br", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
        call("jj", &["workspace", "add", workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt],
            Some(workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
    ]);

    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected transient opencode failure");
    assert_eq!(failure.error.category(), FailureCategory::Command);
    assert!(!failure.error.is_terminal());
    assert_eq!(failure.compensation_journal.len(), 1);
}

#[tokio::test]
async fn run_lifecycle_rejects_invalid_model_before_effects() {
    let executor = ScriptedExecutor::new(Vec::new());
    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest {
            bead_id: Some("edge-test-004".to_owned()),
            model: Some(" ".to_owned()),
            repo: None,
        },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected validation failure");
    assert_eq!(failure.error.category(), FailureCategory::Validation);
    assert!(failure.error.message().contains("invalid model"));
}

#[tokio::test]
async fn run_lifecycle_rejects_invalid_repo_before_effects() {
    let executor = ScriptedExecutor::new(Vec::new());
    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest {
            bead_id: Some("edge-test-005".to_owned()),
            model: None,
            repo: Some("owner/repo/extra".to_owned()),
        },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected validation failure");
    assert_eq!(failure.error.category(), FailureCategory::Validation);
    assert!(failure.error.message().contains("invalid repo slug"));
}

#[test]
fn step_details_keeps_stderr_when_opencode_stdout_has_no_json() {
    let details = super::step_details(&EffectJournalEntry {
        effect: Effect::Opencode { prompt: "x".to_owned(), model: "m".to_owned(), cwd: None },
        timeout_secs: 1200,
        success: true,
        stdout: "plain text output".to_owned(),
        stderr: "warning on stderr".to_owned(),
    });
    assert_eq!(
        details,
        Some(serde_json::json!({
            "events": [],
            "stderr": "warning on stderr"
        }))
    );
}
