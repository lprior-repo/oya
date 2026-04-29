use super::dag::validate_dag;
use super::execution::{
    run_lifecycle_with_progress, step_details, strip_diff_prefix, validate_workspace_changes,
};
use super::progress::timestamp_now;
use super::steps::{build_steps, LifecycleStep, StepTransition};
use super::types::{LifecycleProgressUpdate, LifecycleRunRequest, LifecycleStepStatus};
use crate::lifecycle::effects::{
    CommandExecutor, CommandFailure, CommandResult, Effect, EffectJournalEntry,
};
use crate::lifecycle::types::{BeadData, BeadId, FailureCategory, Model};
use async_trait::async_trait;
use chrono::DateTime;
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

fn workspace_path(workspace: &str) -> String {
    std::env::current_dir().map_or_else(
        |_| format!("/home/lewis/src/{}", workspace),
        |p| format!("{}/{}", p.to_string_lossy(), workspace),
    )
}

fn opencode_prompt(bead: &str) -> String {
    format!(
        "Implement bead {bead} in this workspace using functional-rust approach and tests derived from contract. Do not call `oya` or `br`. Use Moon for build/test/lint, OpenCode for agent execution, and Git/GitHub for version-control and PR flow. Do not require any non-Git version-control tool. Return one JSON receipt object with required keys: objective, allowed_scope, files_touched, commands, exit_codes, key_stdout_stderr, diff_summary, risks_unknowns, pass_fail_recommendation.",
    )
}

fn qa_prompt(bead: &str) -> String {
    format!(
        "Run qa-enforcer verification for bead {bead} against implemented contract and tests. Execute adversarial and regression checks. Return one JSON receipt object with required keys: objective, allowed_scope, files_touched, commands, exit_codes, key_stdout_stderr, diff_summary, risks_unknowns, pass_fail_recommendation. Exit non-zero when verdict is fail.",
    )
}

#[test]
fn git_only_vcs_proof_routes_git_operations_through_jj() {
    let Ok(bead_id) = BeadId::parse("git-only-vcs") else {
        assert!(false, "test bead id should parse");
        return;
    };
    let bead = BeadData::from_bead_id(bead_id);
    let steps = build_steps(&bead, &Model::default_model(), Some("priorlewis43/oya"));

    assert_jj_args(&steps, "jj_sync_main", &["git", "fetch", "--remote", "origin"]);
    assert_jj_args(
        &steps,
        "bookmark_push",
        &["git", "push", "--remote", "origin", "--bookmark", "git-only-vcs"],
    );
    assert_eq!(git_subcommand_steps(&steps), vec!["jj_sync_main", "bookmark_push"]);
}

#[test]
fn workflow_prompt_guides_agents_to_git_only_version_control() {
    let Ok(bead_id) = BeadId::parse("git-only-prompt") else {
        assert!(false, "test bead id should parse");
        return;
    };
    let bead = BeadData::from_bead_id(bead_id);
    let steps = build_steps(&bead, &Model::default_model(), Some("priorlewis43/oya"));
    let Some(prompt) = opencode_prompt_for_step(&steps) else {
        assert!(false, "opencode step should have an agent prompt");
        return;
    };

    assert!(prompt.contains("Use Moon for build/test/lint"));
    assert!(prompt.contains("OpenCode for agent execution"));
    assert!(prompt.contains("Git/GitHub for version-control and PR flow"));
    assert!(prompt.contains("Do not require any non-Git version-control tool"));
    assert!(!prompt.contains("Use moon/jj/gh"));
    assert!(!prompt.contains("jj"));
    assert!(!prompt.contains("Jujutsu"));
}

fn opencode_prompt_for_step(steps: &[LifecycleStep]) -> Option<&str> {
    steps.iter().find(|step| step.name == "opencode").and_then(|step| match &step.effect {
        Effect::Opencode { prompt, .. } => Some(prompt.as_str()),
        _ => None,
    })
}

fn assert_jj_args(steps: &[LifecycleStep], name: &str, expected: &[&str]) {
    let actual = jj_args_for_step(steps, name);
    assert_eq!(actual.as_deref(), Some(expected));
}

fn jj_args_for_step<'a>(steps: &'a [LifecycleStep], name: &str) -> Option<Vec<&'a str>> {
    steps.iter().find(|step| step.name == name).and_then(|step| match &step.effect {
        Effect::Jj { args, .. } => Some(args.iter().map(String::as_str).collect()),
        _ => None,
    })
}

fn git_subcommand_steps(steps: &[LifecycleStep]) -> Vec<&str> {
    steps
        .iter()
        .filter(|step| jj_args_for_step(steps, &step.name).is_some_and(args_start_with_git))
        .map(|step| step.name.as_str())
        .collect()
}

fn args_start_with_git(args: Vec<&str>) -> bool {
    args.first().copied() == Some("git")
}

fn valid_receipt_json() -> &'static str {
    "{\"objective\":\"implement bead\",\"allowed_scope\":[\"src\"],\"files_touched\":[\"src/main.rs\"],\"commands\":[\"moon run :quick\"],\"exit_codes\":[0],\"key_stdout_stderr\":[\"ok\"],\"diff_summary\":\"code updated\",\"risks_unknowns\":[\"none\"],\"pass_fail_recommendation\":\"pass\"}"
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
    let ws_path = workspace_path(workspace);
    let prompt = opencode_prompt(bead);
    let qa = qa_prompt(bead);
    let pr_body = "## Summary\n- Implements bead `edge-test-001` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling";
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &ws_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&ws_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&ws_path),
            ok(valid_receipt_json()),
        ),
        call("moon", &["run", ":quick"], Some(&ws_path), ok("")),
        call("moon", &["run", ":test"], Some(&ws_path), ok("")),
        call("moon", &["run", ":test"], Some(&ws_path), ok("")),
        call("moon", &["run", ":ci"], Some(&ws_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(&ws_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(&ws_path), ok("")),
        call("jj", &["file", "track", "."], Some(&ws_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-001 via lifecycle"],
            Some(&ws_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(&ws_path),
            ok("src/main.rs\nREADME.md\n"),
        ),
        call("jj", &["bookmark", "set", bead, "-r", "@"], Some(&ws_path), ok("")),
        call(
            "jj",
            &["git", "push", "--remote", "origin", "--bookmark", bead],
            Some(&ws_path),
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
            Some(&ws_path),
            ok("https://github.com/lprior-repo/oya/pull/321\n"),
        ),
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
    assert_filesystem_workspace_cleanup(&outcome.compensation_journal, 1);
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
                status: LifecycleStepStatus::Succeeded,
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
    let ws_path = workspace_path(workspace);
    let prompt = opencode_prompt(bead);
    let qa = qa_prompt(bead);
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &ws_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&ws_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&ws_path),
            ok(valid_receipt_json()),
        ),
        call("moon", &["run", ":quick"], Some(&ws_path), ok("")),
        call("moon", &["run", ":test"], Some(&ws_path), ok("")),
        call("moon", &["run", ":test"], Some(&ws_path), ok("")),
        call("moon", &["run", ":ci"], Some(&ws_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(&ws_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(&ws_path), ok("")),
        call("jj", &["file", "track", "."], Some(&ws_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-002 via lifecycle"],
            Some(&ws_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(&ws_path),
            ok("src/main.rs\nREADME.md\n"),
        ),
        call("jj", &["bookmark", "set", bead, "-r", "@"], Some(&ws_path), ok("")),
        call(
            "jj",
            &["git", "push", "--remote", "origin", "--bookmark", bead],
            Some(&ws_path),
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
            Some(&ws_path),
            ok("created but no url in output\n"),
        ),
        call(
            "bd",
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
    assert_filesystem_workspace_cleanup(&failure.compensation_journal, 2);
}

#[tokio::test]
async fn run_lifecycle_existing_pr_in_stderr_is_treated_as_success() {
    let bead = "edge-test-002b";
    let workspace = "oya-edge-test-002b";
    let workspace_path = workspace_path("oya-edge-test-002b");
    let prompt = opencode_prompt(bead);
    let qa = qa_prompt(bead);
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call("moon", &["run", ":quick"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":test"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":test"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":ci"], Some(&workspace_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(&workspace_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(&workspace_path), ok("")),
        call("jj", &["file", "track", "."], Some(&workspace_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-002b via lifecycle"],
            Some(&workspace_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(&workspace_path),
            ok("src/main.rs\n"),
        ),
        call("jj", &["bookmark", "set", bead, "-r", "@"], Some(&workspace_path), ok("")),
        call(
            "jj",
            &["git", "push", "--remote", "origin", "--bookmark", bead],
            Some(&workspace_path),
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
                "Lifecycle edge-test-002b",
                "--body",
                "## Summary\n- Implements bead `edge-test-002b` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling",
            ],
            Some(&workspace_path),
            non_zero("a pull request for branch \"edge-test-002b\" into branch \"main\" already exists:\nhttps://github.com/lprior-repo/oya/pull/4242\n"),
        ),
    ]);

    let mut progress = Vec::<LifecycleProgressUpdate>::new();
    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |update| progress.push(update),
    )
    .await;

    executor.assert_empty();
    assert!(result.is_ok());
    assert!(progress.iter().any(|event| {
        matches!(
            event,
            LifecycleProgressUpdate::Finished { success: true, pr_url: Some(url), .. }
                if url.ends_with("/pull/4242")
        )
    }));
}

#[tokio::test]
async fn run_lifecycle_transient_failure_skips_terminal_compensations() {
    let bead = "edge-test-003";
    let workspace = "oya-edge-test-003";
    let workspace_path = workspace_path("oya-edge-test-003");
    let prompt = opencode_prompt(bead);
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
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
    assert!(failure.error.message().contains("after 4 attempts (3 retries)"));
    assert_eq!(failure.compensation_journal.len(), 1);
    assert_filesystem_workspace_cleanup(&failure.compensation_journal, 1);
}

#[tokio::test]
async fn run_lifecycle_transient_opencode_recovers_after_retry() {
    let bead = "edge-test-003b";
    let workspace = "oya-edge-test-003b";
    let workspace_path = workspace_path("oya-edge-test-003b");
    let prompt = opencode_prompt(bead);
    let qa = qa_prompt(bead);
    let pr_body = "## Summary\n- Implements bead `edge-test-003b` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling";
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            non_zero("transient opencode failure"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call("moon", &["run", ":quick"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":test"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":test"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":ci"], Some(&workspace_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(&workspace_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(&workspace_path), ok("")),
        call("jj", &["file", "track", "."], Some(&workspace_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-003b via lifecycle"],
            Some(&workspace_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(&workspace_path),
            ok("src/main.rs\n"),
        ),
        call("jj", &["bookmark", "set", bead, "-r", "@"], Some(&workspace_path), ok("")),
        call(
            "jj",
            &["git", "push", "--remote", "origin", "--bookmark", bead],
            Some(&workspace_path),
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
                "Lifecycle edge-test-003b",
                "--body",
                pr_body,
            ],
            Some(&workspace_path),
            ok("https://github.com/lprior-repo/oya/pull/333\n"),
        ),
    ]);

    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_ok());
}

#[tokio::test]
async fn run_lifecycle_rejects_invalid_bead_id_before_effects() {
    let executor = ScriptedExecutor::new(Vec::new());
    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some("bad/../id".to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected validation failure");
    assert_eq!(failure.error.category(), FailureCategory::Validation);
    assert!(failure.error.message().contains("invalid chars"));
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

#[tokio::test]
async fn run_lifecycle_fails_when_only_bead_files_changed() {
    let bead = "edge-test-006";
    let workspace = "oya-edge-test-006";
    let workspace_path = workspace_path("oya-edge-test-006");
    let prompt = opencode_prompt(bead);
    let qa = qa_prompt(bead);
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call("moon", &["run", ":quick"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":test"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":test"], Some(&workspace_path), ok("")),
        call("moon", &["run", ":ci"], Some(&workspace_path), ok("")),
        call("jj", &["git", "fetch", "--remote", "origin"], Some(&workspace_path), ok("")),
        call("jj", &["rebase", "-d", "main@origin"], Some(&workspace_path), ok("")),
        call("jj", &["file", "track", "."], Some(&workspace_path), ok("")),
        call(
            "jj",
            &["describe", "-m", "chore: implement edge-test-006 via lifecycle"],
            Some(&workspace_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(&workspace_path),
            ok(".beads/beads.db\n"),
        ),
        call(
            "bd",
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
    ]);

    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected no-change failure");
    assert!(failure.error.is_terminal());
    assert_eq!(failure.error.category(), FailureCategory::Command);
    assert!(failure.error.message().contains("no non-.beads changes"));
}

#[tokio::test]
async fn run_lifecycle_fails_when_opencode_receipt_is_missing_fields() {
    let bead = "edge-test-007";
    let workspace = "oya-edge-test-007";
    let workspace_path = workspace_path("oya-edge-test-007");
    let prompt = opencode_prompt(bead);
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok("{\"status\":\"ok\"}"),
        ),
        call(
            "bd",
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
    ]);

    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected missing receipt fields failure");
    assert!(failure.error.is_terminal());
    assert_eq!(failure.error.category(), FailureCategory::Command);
    assert!(failure.error.message().contains("opencode receipt missing required fields"));
}

#[tokio::test]
async fn run_lifecycle_qa_failure_retries_three_times_then_blocks() {
    let bead = "edge-test-007b";
    let workspace = "oya-edge-test-007b";
    let workspace_path = workspace_path("oya-edge-test-007b");
    let prompt = opencode_prompt(bead);
    let qa = qa_prompt(bead);
    let executor = ScriptedExecutor::new(vec![
        call("bd", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "add", &workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            non_zero("qa failed"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            non_zero("qa failed"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            non_zero("qa failed"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt.as_str()],
            Some(&workspace_path),
            ok(valid_receipt_json()),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", qa.as_str()],
            Some(&workspace_path),
            non_zero("qa failed"),
        ),
        call(
            "bd",
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
    ]);

    let result = run_lifecycle_with_progress(
        &executor,
        LifecycleRunRequest { bead_id: Some(bead.to_owned()), model: None, repo: None },
        |_| {},
    )
    .await;

    executor.assert_empty();
    assert!(result.is_err());
    let failure = result.expect_err("expected qa retry exhaustion failure");
    assert!(failure.error.is_terminal());
    assert_eq!(failure.error.category(), FailureCategory::Command);
    assert_eq!(failure.compensation_journal.len(), 3);
    assert_filesystem_workspace_cleanup(&failure.compensation_journal, 2);
}

fn assert_filesystem_workspace_cleanup(entries: &[EffectJournalEntry], expected_count: usize) {
    assert_eq!(workspace_cleanup_count(entries), expected_count);
    assert!(!entries.iter().any(is_jj_workspace_forget));
}

fn workspace_cleanup_count(entries: &[EffectJournalEntry]) -> usize {
    entries.iter().filter(|entry| matches!(entry.effect, Effect::WorkspacePrepare { .. })).count()
}

fn is_jj_workspace_forget(entry: &EffectJournalEntry) -> bool {
    match &entry.effect {
        Effect::Jj { args, .. } => {
            args.first().is_some_and(|arg| arg == "workspace")
                && args.get(1).is_some_and(|arg| arg == "forget")
        }
        _ => false,
    }
}

#[test]
fn step_details_keeps_stderr_when_opencode_stdout_has_no_json() {
    let details = step_details(&EffectJournalEntry {
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
            "receipt": null,
            "timeout_secs": 1200,
            "success": true,
            "stderr": "warning on stderr"
        }))
    );
}

#[test]
fn strip_diff_prefix_handles_modified() {
    assert_eq!(strip_diff_prefix("M src/main.rs"), "src/main.rs");
}

#[test]
fn strip_diff_prefix_handles_added() {
    assert_eq!(strip_diff_prefix("A src/new_file.rs"), "src/new_file.rs");
}

#[test]
fn strip_diff_prefix_handles_renamed() {
    assert_eq!(strip_diff_prefix("R src/old.rs"), "src/old.rs");
}

#[test]
fn strip_diff_prefix_handles_deleted() {
    assert_eq!(strip_diff_prefix("D src/dead.rs"), "src/dead.rs");
}

#[test]
fn strip_diff_prefix_passthrough_unknown() {
    assert_eq!(strip_diff_prefix("src/plain.rs"), "src/plain.rs");
    assert_eq!(strip_diff_prefix("? src/untracked.rs"), "? src/untracked.rs");
}

#[test]
fn validate_workspace_changes_rejects_empty() {
    let result = validate_workspace_changes("");
    assert!(result.is_err());
    assert!(result.expect_err("expected empty workspace failure").is_terminal());
}

#[test]
fn validate_workspace_changes_rejects_only_beads() {
    let result = validate_workspace_changes(".beads/beads.db\n.beads/config.yaml\n");
    assert!(result.is_err());
    assert!(result
        .expect_err("expected .beads-only failure")
        .message()
        .contains("no non-.beads changes"));
}

#[test]
fn validate_workspace_changes_accepts_mixed() {
    let result = validate_workspace_changes("src/main.rs\n.beads/beads.db\n");
    assert!(result.is_ok());
}

#[test]
fn validate_workspace_changes_accepts_source_only() {
    let result = validate_workspace_changes("src/main.rs\nsrc/lib.rs\n");
    assert!(result.is_ok());
}

#[test]
fn validate_workspace_changes_handles_prefixed_output() {
    let result = validate_workspace_changes("M src/main.rs\nA src/new.rs\n");
    assert!(result.is_ok());
}

#[test]
fn validate_workspace_changes_ignores_whitespace() {
    let result = validate_workspace_changes("  \n  src/main.rs  \n  \n");
    assert!(result.is_ok());
}

#[test]
fn timestamp_now_emits_rfc3339() {
    let timestamp = timestamp_now();
    assert!(DateTime::parse_from_rfc3339(&timestamp).is_ok());
}

#[test]
fn validate_dag_accepts_empty_step_list() {
    let result = validate_dag(&[]);
    assert!(result.is_ok());
}

#[test]
fn validate_dag_accepts_steps_with_no_dependencies() {
    let steps = vec![
        LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec![],
        },
        LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec![],
        },
    ];
    let result = validate_dag(&steps);
    assert!(result.is_ok());
}

#[test]
fn validate_dag_accepts_valid_dependency_chain() {
    let steps = vec![
        LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec![],
        },
        LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
    ];
    let result = validate_dag(&steps);
    assert!(result.is_ok());
}

#[test]
fn validate_dag_rejects_missing_dependency() {
    let steps = vec![LifecycleStep {
        name: "step_a".to_owned(),
        effect: Effect::Br { args: vec![], cwd: None },
        compensation: None,
        transition: StepTransition::None,
        dependencies: vec!["nonexistent_step".to_owned()],
    }];
    let result = validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected missing dependency error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("unknown dependency"));
    assert!(error.message().contains("nonexistent_step"));
}

#[test]
fn validate_dag_rejects_direct_cycle() {
    let steps = vec![
        LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_b".to_owned()],
        },
        LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
    ];
    let result = validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected cycle error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("cycle"));
}

#[test]
fn validate_dag_rejects_indirect_cycle() {
    let steps = vec![
        LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_c".to_owned()],
        },
        LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
        LifecycleStep {
            name: "step_c".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_b".to_owned()],
        },
    ];
    let result = validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected indirect cycle error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("cycle"));
}

#[test]
fn validate_dag_rejects_self_dependency() {
    let steps = vec![LifecycleStep {
        name: "step_a".to_owned(),
        effect: Effect::Br { args: vec![], cwd: None },
        compensation: None,
        transition: StepTransition::None,
        dependencies: vec!["step_a".to_owned()],
    }];
    let result = validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected self-dependency error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("cycle"));
}

#[test]
fn validate_dag_rejects_dependency_after_step() {
    let steps = vec![
        LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
        LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: StepTransition::None,
            dependencies: vec![],
        },
    ];
    let result = validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected order validation error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("appears later"));
}
