#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{
    run_compensation, CommandExecutor, Compensation, EffectJournalEntry,
};
use crate::lifecycle::telemetry::{emit_step_telemetry, emit_unwind_signal};
use crate::lifecycle::transitions::LifecycleEvent;
use crate::lifecycle::types::{CompensationDiagnostic, WorkspaceName};
use futures_util::stream::{self, StreamExt};
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{
    ExecutionAcc, LifecycleProgressUpdate, LifecycleRunFailure, LifecycleRunOutcome, StepFailure,
};

pub async fn finalize_success<F>(
    executor: &dyn CommandExecutor,
    mut acc: ExecutionAcc,
    workspace: WorkspaceName,
    on_progress: &mut F,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let completed_state =
        crate::lifecycle::transitions::apply_event(&acc.state, LifecycleEvent::Completed).map_err(
            |error| LifecycleRunFailure {
                error,
                state: Some(acc.state.clone()),
                journal: acc.journal.clone(),
                compensation_journal: Vec::new(),
                compensation_diagnostics: Vec::new(),
            },
        )?;
    acc.state = completed_state;
    persist_lifecycle_artifacts(&acc.state, &acc.journal, &[], true, None);
    let (cleanup, cleanup_diagnostics) = workspace_cleanup(executor, workspace).await;
    for diagnostic in &cleanup_diagnostics {
        emit_unwind_signal(diagnostic);
    }
    let pr_url = pr_url_from_state(&acc.state);
    let finished_update = LifecycleProgressUpdate::Finished {
        success: true,
        pr_url: pr_url.clone(),
        message: None,
        compensation_diagnostics: cleanup_diagnostics.clone(),
    };
    on_progress(finished_update.clone());
    emit_step_telemetry(&finished_update);
    Ok(LifecycleRunOutcome {
        state: acc.state,
        journal: acc.journal,
        compensation_journal: cleanup,
        compensation_diagnostics: cleanup_diagnostics,
    })
}

pub async fn finalize_failure<F>(
    executor: &dyn CommandExecutor,
    failure: StepFailure,
    workspace: WorkspaceName,
    on_progress: &mut F,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let (mut compensation_journal, mut compensation_diagnostics) =
        collect_failure_compensations(executor, &failure).await;
    let (cleanup, cleanup_diagnostics) = workspace_cleanup(executor, workspace).await;
    for diagnostic in &cleanup_diagnostics {
        emit_unwind_signal(diagnostic);
    }
    compensation_journal.extend(cleanup);
    compensation_diagnostics.extend(cleanup_diagnostics);
    let finished_update = LifecycleProgressUpdate::Finished {
        success: false,
        pr_url: pr_url_from_state(&failure.state),
        message: Some(failure.error.to_string()),
        compensation_diagnostics: compensation_diagnostics.clone(),
    };
    on_progress(finished_update.clone());
    emit_step_telemetry(&finished_update);
    persist_lifecycle_artifacts(
        &failure.state,
        &failure.journal,
        &compensation_diagnostics,
        false,
        Some(failure.error.to_string().as_str()),
    );
    Err(LifecycleRunFailure {
        error: failure.error,
        state: Some(failure.state),
        journal: failure.journal,
        compensation_journal,
        compensation_diagnostics,
    })
}

async fn collect_failure_compensations(
    executor: &dyn CommandExecutor,
    failure: &StepFailure,
) -> (Vec<EffectJournalEntry>, Vec<CompensationDiagnostic>) {
    if failure.error.is_terminal() {
        run_compensations_with_telemetry(executor, failure.completed_compensations.clone()).await
    } else {
        (Vec::new(), Vec::new())
    }
}

fn persist_lifecycle_artifacts(
    state: &crate::lifecycle::types::LifecycleState,
    journal: &[EffectJournalEntry],
    diagnostics: &[CompensationDiagnostic],
    success: bool,
    error_message: Option<&str>,
) {
    if !artifacts_enabled() {
        return;
    }
    let (bead_id, dir) = artifact_dir(state);
    let writes = artifact_writes(&bead_id, journal, diagnostics, success, error_message);
    write_artifact_files(&dir, writes);
}

fn artifacts_enabled() -> bool {
    if cfg!(test) {
        return false;
    }
    std::env::var("OYA_DISABLE_REPORT_ARTIFACTS")
        .map(|value| value.trim().eq_ignore_ascii_case("1"))
        .map_or(true, |disabled| !disabled)
}

fn artifact_dir(state: &crate::lifecycle::types::LifecycleState) -> (String, PathBuf) {
    let bead_id = bead_id_from_state(state);
    let mut dir = PathBuf::from(".oya");
    dir.push("reports");
    dir.push(&bead_id);
    (bead_id, dir)
}

fn bead_id_from_state(state: &crate::lifecycle::types::LifecycleState) -> String {
    match &state.phase {
        crate::lifecycle::types::Phase::Planned(bead)
        | crate::lifecycle::types::Phase::WorkspaceReady(bead)
        | crate::lifecycle::types::Phase::Failed { bead, .. }
        | crate::lifecycle::types::Phase::PrOpen { bead, .. } => bead.bead_id.as_str().to_owned(),
        crate::lifecycle::types::Phase::Completed(result) => {
            result.bead.bead_id.as_str().to_owned()
        }
    }
}

fn artifact_writes(
    bead_id: &str,
    journal: &[EffectJournalEntry],
    diagnostics: &[CompensationDiagnostic],
    success: bool,
    error_message: Option<&str>,
) -> Vec<(String, String)> {
    let commands = summarize_commands(journal);
    let validation = summarize_validation(journal);
    let qa = summarize_qa(journal);
    let audit = summarize_audit(success, diagnostics, error_message);
    let mut files = vec![
        report("orchestrator-plan.md", bead_id, "orchestration evidence", &commands),
        report("contract-spec.md", bead_id, "contract evidence", &commands),
        report("martin-fowler-tests.md", bead_id, "test evidence", &validation),
        report("traceability-matrix.md", bead_id, "traceability evidence", &validation),
        report("implementation-report.md", bead_id, "implementation evidence", &commands),
        report("validation-report.md", bead_id, "validation evidence", &validation),
        report("qa-report.md", bead_id, "qa evidence", &qa),
        report("audit-report.md", bead_id, "audit evidence", &audit),
    ];
    if !success {
        files.push(report("defects.md", bead_id, "defect evidence", &audit));
    }
    files
}

fn report(name: &str, bead_id: &str, title: &str, body: &str) -> (String, String) {
    let content = format!("# {title}\n\nbead: `{bead_id}`\n\n{body}\n");
    (name.to_owned(), content)
}

fn summarize_commands(journal: &[EffectJournalEntry]) -> String {
    journal
        .iter()
        .map(|entry| format!("- effect: `{:?}` success: {}", entry.effect, entry.success))
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_validation(journal: &[EffectJournalEntry]) -> String {
    journal
        .iter()
        .filter(|entry| {
            matches!(entry.effect, crate::lifecycle::effects::Effect::MoonRun { .. })
                || matches!(entry.effect, crate::lifecycle::effects::Effect::MoonCi { .. })
        })
        .map(|entry| format!("- validation: `{:?}` success: {}", entry.effect, entry.success))
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_qa(journal: &[EffectJournalEntry]) -> String {
    journal
        .iter()
        .filter(|entry| {
            matches!(entry.effect, crate::lifecycle::effects::Effect::OpencodeQa { .. })
        })
        .map(|entry| format!("- qa: success: {} stderr: {}", entry.success, entry.stderr.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_audit(
    success: bool,
    diagnostics: &[CompensationDiagnostic],
    error_message: Option<&str>,
) -> String {
    let mut lines = vec![format!("- verdict: {}", if success { "pass" } else { "fail" })];
    if let Some(error) = error_message {
        lines.push(format!("- error: {error}"));
    }
    lines.extend(diagnostics.iter().map(|item| {
        format!(
            "- compensation: {} target={} success={} error={}",
            item.compensation_type,
            item.target,
            item.success,
            item.error.as_deref().unwrap_or("")
        )
    }));
    lines.join("\n")
}

fn write_artifact_files(dir: &Path, files: Vec<(String, String)>) {
    if let Err(error) = fs::create_dir_all(dir) {
        tracing::warn!(path = %dir.display(), error = %error, "failed to create artifact directory");
        return;
    }
    for (name, content) in files {
        let mut path = PathBuf::from(dir);
        path.push(name);
        if let Err(error) = fs::write(&path, content) {
            tracing::warn!(path = %path.display(), error = %error, "failed to write artifact file");
        }
    }
}

fn pr_url_from_state(state: &crate::lifecycle::types::LifecycleState) -> Option<String> {
    match &state.phase {
        crate::lifecycle::types::Phase::PrOpen { pr, .. } => Some(pr.url.clone()),
        crate::lifecycle::types::Phase::Completed(result) => {
            result.pr.as_ref().map(|pr| pr.url.clone())
        }
        crate::lifecycle::types::Phase::Planned(_)
        | crate::lifecycle::types::Phase::WorkspaceReady(_)
        | crate::lifecycle::types::Phase::Failed { .. } => None,
    }
}

async fn workspace_cleanup(
    executor: &dyn CommandExecutor,
    workspace: WorkspaceName,
) -> (Vec<EffectJournalEntry>, Vec<CompensationDiagnostic>) {
    let compensation = Compensation::ForgetWorkspace { workspace };
    let (entry, diagnostic) = run_compensation_with_diagnostic(executor, compensation).await;
    let journal = entry.into_iter().collect::<Vec<_>>();
    (journal, vec![diagnostic])
}

async fn run_compensations(
    executor: &dyn CommandExecutor,
    compensations: Vec<Compensation>,
) -> (Vec<EffectJournalEntry>, Vec<CompensationDiagnostic>) {
    let reversed = compensations.into_iter().rev().collect::<Vec<_>>();
    let attempts = stream::iter(reversed)
        .then(|compensation| async move {
            run_compensation_with_diagnostic(executor, compensation).await
        })
        .collect::<Vec<(Option<EffectJournalEntry>, CompensationDiagnostic)>>()
        .await;
    let journal = attempts.iter().filter_map(|(entry, _)| entry.clone()).collect::<Vec<_>>();
    let diagnostics = attempts.into_iter().map(|(_, diagnostic)| diagnostic).collect::<Vec<_>>();
    (journal, diagnostics)
}

async fn run_compensations_with_telemetry(
    executor: &dyn CommandExecutor,
    compensations: Vec<Compensation>,
) -> (Vec<EffectJournalEntry>, Vec<CompensationDiagnostic>) {
    let (journal, diagnostics) = run_compensations(executor, compensations).await;
    for diagnostic in &diagnostics {
        emit_unwind_signal(diagnostic);
    }
    (journal, diagnostics)
}

async fn run_compensation_with_diagnostic(
    executor: &dyn CommandExecutor,
    compensation: Compensation,
) -> (Option<EffectJournalEntry>, CompensationDiagnostic) {
    let (comp_type, target) = compensation_metadata(&compensation);
    match run_compensation(executor, compensation).await {
        Ok(entry) => (
            Some(entry),
            CompensationDiagnostic {
                compensation_type: comp_type,
                target,
                success: true,
                error: None,
            },
        ),
        Err(error) => (
            None,
            CompensationDiagnostic {
                compensation_type: comp_type,
                target,
                success: false,
                error: Some(error.to_string()),
            },
        ),
    }
}

fn compensation_metadata(compensation: &Compensation) -> (String, String) {
    match compensation {
        Compensation::ForgetWorkspace { workspace } => {
            ("forget_workspace".to_owned(), workspace.as_str().to_owned())
        }
        Compensation::MarkBeadBlocked { bead, .. } => {
            ("mark_bead_blocked".to_owned(), bead.bead_id.as_str().to_owned())
        }
    }
}
