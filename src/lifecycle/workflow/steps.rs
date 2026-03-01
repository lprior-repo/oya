#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{Compensation, Effect};
use crate::lifecycle::transitions::LifecycleEvent;
use crate::lifecycle::types::{BeadData, Model};

#[derive(Debug, Clone)]
pub struct LifecycleStep {
    pub name: String,
    pub effect: Effect,
    pub compensation: Option<Compensation>,
    pub transition: StepTransition,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum StepTransition {
    None,
    Static(LifecycleEvent),
    ValidateReceipt { required_fields: Vec<String> },
    ValidateWorkspaceChanges,
    PullRequestOpened { bead: BeadData },
}

pub fn build_steps(
    bead: &BeadData,
    model: &Model,
    repo: Option<&str>,
    cwd: Option<&str>,
) -> Vec<LifecycleStep> {
    let mut steps = vec![
        br_in_progress_step(bead, cwd),
        workspace_prepare_step(bead),
        workspace_create_step(bead),
        opencode_step(bead, model),
        qa_enforcer_step(bead, model),
        ltc_quick_step(bead),
        ltc_targeted_step(bead),
        ltc_test_step(bead),
        moon_ci_step(bead),
        jj_sync_main_step(bead),
        jj_rebase_main_step(bead),
        jj_track_step(bead),
        jj_describe_step(bead),
        validate_changes_step(bead),
        bookmark_create_step(bead),
    ];
    steps.push(bookmark_push_step(bead));
    steps.push(pr_create_step(bead, repo));
    steps
}

fn deps(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn workspace_prepare_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "workspace_prepare".to_owned(),
        effect: Effect::WorkspacePrepare {
            workspace: bead.workspace.clone(),
            path: bead.workspace_path.clone(),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["mark_in_progress"]),
    }
}

fn br_in_progress_step(bead: &BeadData, cwd: Option<&str>) -> LifecycleStep {
    LifecycleStep {
        name: "mark_in_progress".to_owned(),
        effect: Effect::Br {
            args: vec![
                "update".to_owned(),
                bead.bead_id.as_str().to_owned(),
                "--status".to_owned(),
                "in_progress".to_owned(),
            ],
            cwd: cwd.map(String::from),
        },
        compensation: Some(Compensation::MarkBeadBlocked {
            bead: bead.clone(),
            reason: "lifecycle failed after terminal error".to_owned(),
        }),
        transition: StepTransition::None,
        dependencies: Vec::new(),
    }
}

fn workspace_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "workspace_add".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "workspace".to_owned(),
                "add".to_owned(),
                bead.workspace_path.clone(),
                "--name".to_owned(),
                bead.workspace.as_str().to_owned(),
            ],
            cwd: None,
        },
        compensation: Some(Compensation::ForgetWorkspace { workspace: bead.workspace.clone() }),
        transition: StepTransition::Static(LifecycleEvent::WorkspacePrepared),
        dependencies: deps(&["workspace_prepare"]),
    }
}

fn jj_sync_main_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "jj_sync_main".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "git".to_owned(),
                "fetch".to_owned(),
                "--remote".to_owned(),
                "origin".to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["moon_ci"]),
    }
}

fn jj_rebase_main_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "jj_rebase_main".to_owned(),
        effect: Effect::Jj {
            args: vec!["rebase".to_owned(), "-d".to_owned(), "main@origin".to_owned()],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["jj_sync_main"]),
    }
}

fn opencode_step(bead: &BeadData, model: &Model) -> LifecycleStep {
    let prompt = format!(
        "Implement bead {} in this workspace using functional-rust approach and tests derived from contract. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return one JSON receipt object with required keys: objective, allowed_scope, files_touched, commands, exit_codes, key_stdout_stderr, diff_summary, risks_unknowns, pass_fail_recommendation.",
        bead.bead_id.as_str()
    );
    LifecycleStep {
        name: "opencode".to_owned(),
        effect: Effect::Opencode {
            prompt,
            model: model.as_str().to_owned(),
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::ValidateReceipt { required_fields: receipt_required_fields() },
        dependencies: deps(&["workspace_add"]),
    }
}

fn qa_enforcer_step(bead: &BeadData, model: &Model) -> LifecycleStep {
    let prompt = format!(
        "Run qa-enforcer verification for bead {} against implemented contract and tests. Execute adversarial and regression checks. Return one JSON receipt object with required keys: objective, allowed_scope, files_touched, commands, exit_codes, key_stdout_stderr, diff_summary, risks_unknowns, pass_fail_recommendation. Exit non-zero when verdict is fail.",
        bead.bead_id.as_str()
    );
    LifecycleStep {
        name: "qa_enforcer".to_owned(),
        effect: Effect::OpencodeQa {
            prompt,
            model: model.as_str().to_owned(),
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::ValidateReceipt { required_fields: receipt_required_fields() },
        dependencies: deps(&["opencode"]),
    }
}

fn receipt_required_fields() -> Vec<String> {
    [
        "objective",
        "allowed_scope",
        "files_touched",
        "commands",
        "exit_codes",
        "key_stdout_stderr",
        "diff_summary",
        "risks_unknowns",
        "pass_fail_recommendation",
    ]
    .into_iter()
    .map(std::borrow::ToOwned::to_owned)
    .collect()
}

fn ltc_quick_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "ltc_quick".to_owned(),
        effect: Effect::MoonRun {
            task: ":quick".to_owned(),
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["qa_enforcer"]),
    }
}

fn ltc_targeted_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "ltc_targeted".to_owned(),
        effect: Effect::MoonRun {
            task: ":test".to_owned(),
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["ltc_quick"]),
    }
}

fn ltc_test_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "ltc_test".to_owned(),
        effect: Effect::MoonRun {
            task: ":test".to_owned(),
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["ltc_targeted"]),
    }
}

fn validate_changes_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "validate_changes".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "diff".to_owned(),
                "--name-only".to_owned(),
                "--from".to_owned(),
                "main@origin".to_owned(),
                "--to".to_owned(),
                "@".to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::ValidateWorkspaceChanges,
        dependencies: deps(&["jj_describe"]),
    }
}

fn moon_ci_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "moon_ci".to_owned(),
        effect: Effect::MoonCi { cwd: Some(bead.workspace_path.clone()) },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["ltc_test"]),
    }
}

fn jj_track_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "jj_track".to_owned(),
        effect: Effect::Jj {
            args: vec!["file".to_owned(), "track".to_owned(), ".".to_owned()],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["jj_rebase_main"]),
    }
}

fn jj_describe_step(bead: &BeadData) -> LifecycleStep {
    let message = format!("chore: implement {} via lifecycle", bead.bead_id.as_str());
    LifecycleStep {
        name: "jj_describe".to_owned(),
        effect: Effect::Jj {
            args: vec!["describe".to_owned(), "-m".to_owned(), message],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["jj_track"]),
    }
}

fn bookmark_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "bookmark_create".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "bookmark".to_owned(),
                "set".to_owned(),
                bead.bookmark.as_str().to_owned(),
                "-r".to_owned(),
                "@".to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["validate_changes"]),
    }
}

fn bookmark_push_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "bookmark_push".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "git".to_owned(),
                "push".to_owned(),
                "--remote".to_owned(),
                "origin".to_owned(),
                "--bookmark".to_owned(),
                bead.bookmark.as_str().to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["bookmark_create"]),
    }
}

fn pr_create_step(bead: &BeadData, repo: Option<&str>) -> LifecycleStep {
    let title = format!("Lifecycle {}", bead.bead_id.as_str());
    let body = format!(
        "## Summary\n- Implements bead `{}` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling",
        bead.bead_id.as_str()
    );
    let mut args = vec![
        "pr".to_owned(),
        "create".to_owned(),
        "--head".to_owned(),
        bead.bookmark.as_str().to_owned(),
    ];
    if let Some(value) = repo {
        args.push("--repo".to_owned());
        args.push(value.to_owned());
    }
    args.extend([
        "--base".to_owned(),
        "main".to_owned(),
        "--title".to_owned(),
        title,
        "--body".to_owned(),
        body,
    ]);
    LifecycleStep {
        name: "pr_create".to_owned(),
        effect: Effect::Gh { args, cwd: Some(bead.workspace_path.clone()) },
        compensation: None,
        transition: StepTransition::PullRequestOpened { bead: bead.clone() },
        dependencies: deps(&["bookmark_push"]),
    }
}
