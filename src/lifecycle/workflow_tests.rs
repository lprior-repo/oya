use super::{run_lifecycle_with_progress, LifecycleProgressUpdate, LifecycleRunRequest};
use crate::lifecycle::effects::{
    CommandExecutor, CommandFailure, CommandResult, Effect, EffectJournalEntry,
};
use crate::lifecycle::types::FailureCategory;
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
    let prompt = "Implement bead edge-test-001 in this workspace with real code changes. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return short JSON summary with changed_files and ci_status.";
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
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(workspace_path),
            ok("src/main.rs\nREADME.md\n"),
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
    let prompt = "Implement bead edge-test-002 in this workspace with real code changes. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return short JSON summary with changed_files and ci_status.";
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
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(workspace_path),
            ok("src/main.rs\nREADME.md\n"),
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
    let prompt = "Implement bead edge-test-003 in this workspace with real code changes. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return short JSON summary with changed_files and ci_status.";
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
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt],
            Some(workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt],
            Some(workspace_path),
            non_zero("simulated opencode transient failure"),
        ),
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
    assert!(failure.error.message().contains("after 4 attempts (3 retries)"));
    assert_eq!(failure.compensation_journal.len(), 1);
}

#[tokio::test]
async fn run_lifecycle_transient_opencode_recovers_after_retry() {
    let bead = "edge-test-003b";
    let workspace = "oya-edge-test-003b";
    let workspace_path = "/home/lewis/src/oya-edge-test-003b";
    let prompt = "Implement bead edge-test-003b in this workspace with real code changes. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return short JSON summary with changed_files and ci_status.";
    let pr_body = "## Summary\n- Implements bead `edge-test-003b` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling";
    let executor = ScriptedExecutor::new(vec![
        call("br", &["update", bead, "--status", "in_progress"], None, ok("")),
        call("jj", &["workspace", "forget", workspace], None, ok("")),
        call("jj", &["workspace", "add", workspace_path, "--name", workspace], None, ok("")),
        call(
            "opencode",
            &["run", "--format", "json", "--model", "zai-coding-plan/glm-5", prompt],
            Some(workspace_path),
            non_zero("transient opencode failure"),
        ),
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
            &["describe", "-m", "chore: implement edge-test-003b via lifecycle"],
            Some(workspace_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(workspace_path),
            ok("src/main.rs\n"),
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
                "Lifecycle edge-test-003b",
                "--body",
                pr_body,
            ],
            Some(workspace_path),
            ok("https://github.com/lprior-repo/oya/pull/333\n"),
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
    assert!(result.is_ok());
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
    let workspace_path = "/home/lewis/src/oya-edge-test-006";
    let prompt = "Implement bead edge-test-006 in this workspace with real code changes. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return short JSON summary with changed_files and ci_status.";
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
            &["describe", "-m", "chore: implement edge-test-006 via lifecycle"],
            Some(workspace_path),
            ok(""),
        ),
        call(
            "jj",
            &["diff", "--name-only", "--from", "main@origin", "--to", "@"],
            Some(workspace_path),
            ok(".beads/beads.db\n"),
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
    let failure = result.expect_err("expected no-change failure");
    assert!(failure.error.is_terminal());
    assert_eq!(failure.error.category(), FailureCategory::Command);
    assert!(failure.error.message().contains("no non-.beads changes"));
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

#[test]
fn strip_diff_prefix_handles_modified() {
    assert_eq!(super::strip_diff_prefix("M src/main.rs"), "src/main.rs");
}

#[test]
fn strip_diff_prefix_handles_added() {
    assert_eq!(super::strip_diff_prefix("A src/new_file.rs"), "src/new_file.rs");
}

#[test]
fn strip_diff_prefix_handles_renamed() {
    assert_eq!(super::strip_diff_prefix("R src/old.rs"), "src/old.rs");
}

#[test]
fn strip_diff_prefix_handles_deleted() {
    assert_eq!(super::strip_diff_prefix("D src/dead.rs"), "src/dead.rs");
}

#[test]
fn strip_diff_prefix_passthrough_unknown() {
    assert_eq!(super::strip_diff_prefix("src/plain.rs"), "src/plain.rs");
    assert_eq!(super::strip_diff_prefix("? src/untracked.rs"), "? src/untracked.rs");
}

#[test]
fn validate_workspace_changes_rejects_empty() {
    let result = super::validate_workspace_changes("");
    assert!(result.is_err());
    assert!(result.expect_err("expected empty workspace failure").is_terminal());
}

#[test]
fn validate_workspace_changes_rejects_only_beads() {
    let result = super::validate_workspace_changes(".beads/beads.db\n.beads/config.yaml\n");
    assert!(result.is_err());
    assert!(result
        .expect_err("expected .beads-only failure")
        .message()
        .contains("no non-.beads changes"));
}

#[test]
fn validate_workspace_changes_accepts_mixed() {
    let result = super::validate_workspace_changes("src/main.rs\n.beads/beads.db\n");
    assert!(result.is_ok());
}

#[test]
fn validate_workspace_changes_accepts_source_only() {
    let result = super::validate_workspace_changes("src/main.rs\nsrc/lib.rs\n");
    assert!(result.is_ok());
}

#[test]
fn validate_workspace_changes_handles_prefixed_output() {
    let result = super::validate_workspace_changes("M src/main.rs\nA src/new.rs\n");
    assert!(result.is_ok());
}

#[test]
fn validate_workspace_changes_ignores_whitespace() {
    let result = super::validate_workspace_changes("  \n  src/main.rs  \n  \n");
    assert!(result.is_ok());
}

#[test]
fn timestamp_now_emits_rfc3339() {
    let timestamp = super::timestamp_now();
    assert!(DateTime::parse_from_rfc3339(&timestamp).is_ok());
}

#[test]
fn validate_dag_accepts_empty_step_list() {
    let result = super::validate_dag(&[]);
    assert!(result.is_ok());
}

#[test]
fn validate_dag_accepts_steps_with_no_dependencies() {
    let steps = vec![
        super::LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec![],
        },
        super::LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec![],
        },
    ];
    let result = super::validate_dag(&steps);
    assert!(result.is_ok());
}

#[test]
fn validate_dag_accepts_valid_dependency_chain() {
    let steps = vec![
        super::LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec![],
        },
        super::LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
    ];
    let result = super::validate_dag(&steps);
    assert!(result.is_ok());
}

#[test]
fn validate_dag_rejects_missing_dependency() {
    let steps = vec![super::LifecycleStep {
        name: "step_a".to_owned(),
        effect: Effect::Br { args: vec![], cwd: None },
        compensation: None,
        transition: super::StepTransition::None,
        dependencies: vec!["nonexistent_step".to_owned()],
    }];
    let result = super::validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected missing dependency error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("unknown dependency"));
    assert!(error.message().contains("nonexistent_step"));
}

#[test]
fn validate_dag_rejects_direct_cycle() {
    let steps = vec![
        super::LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_b".to_owned()],
        },
        super::LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
    ];
    let result = super::validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected cycle error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("cycle"));
}

#[test]
fn validate_dag_rejects_indirect_cycle() {
    let steps = vec![
        super::LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_c".to_owned()],
        },
        super::LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
        super::LifecycleStep {
            name: "step_c".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_b".to_owned()],
        },
    ];
    let result = super::validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected indirect cycle error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("cycle"));
}

#[test]
fn validate_dag_rejects_self_dependency() {
    let steps = vec![super::LifecycleStep {
        name: "step_a".to_owned(),
        effect: Effect::Br { args: vec![], cwd: None },
        compensation: None,
        transition: super::StepTransition::None,
        dependencies: vec!["step_a".to_owned()],
    }];
    let result = super::validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected self-dependency error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("cycle"));
}

#[test]
fn validate_dag_rejects_dependency_after_step() {
    let steps = vec![
        super::LifecycleStep {
            name: "step_b".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec!["step_a".to_owned()],
        },
        super::LifecycleStep {
            name: "step_a".to_owned(),
            effect: Effect::Br { args: vec![], cwd: None },
            compensation: None,
            transition: super::StepTransition::None,
            dependencies: vec![],
        },
    ];
    let result = super::validate_dag(&steps);
    assert!(result.is_err());
    let error = result.expect_err("expected order validation error");
    assert_eq!(error.category(), FailureCategory::Validation);
    assert!(error.message().contains("appears later"));
}
