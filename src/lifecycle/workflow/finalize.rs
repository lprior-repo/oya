#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{
    run_compensation, CommandExecutor, Compensation, EffectJournalEntry,
};
use crate::lifecycle::transitions::LifecycleEvent;
use crate::lifecycle::types::{CompensationDiagnostic, WorkspaceName};
use futures_util::stream::{self, StreamExt};

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
    let (cleanup, cleanup_diagnostics) = workspace_cleanup(executor, workspace).await;
    let pr_url = pr_url_from_state(&acc.state);
    on_progress(LifecycleProgressUpdate::Finished {
        success: true,
        pr_url,
        message: None,
        compensation_diagnostics: cleanup_diagnostics.clone(),
    });
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
    let (mut compensation_journal, mut compensation_diagnostics): (
        Vec<EffectJournalEntry>,
        Vec<CompensationDiagnostic>,
    ) = if failure.error.is_terminal() {
        run_compensations(executor, failure.completed_compensations).await
    } else {
        (Vec::new(), Vec::new())
    };
    let (cleanup, cleanup_diagnostics) = workspace_cleanup(executor, workspace).await;
    compensation_journal.extend(cleanup);
    compensation_diagnostics.extend(cleanup_diagnostics);
    on_progress(LifecycleProgressUpdate::Finished {
        success: false,
        pr_url: pr_url_from_state(&failure.state),
        message: Some(failure.error.to_string()),
        compensation_diagnostics: compensation_diagnostics.clone(),
    });
    Err(LifecycleRunFailure {
        error: failure.error,
        state: Some(failure.state),
        journal: failure.journal,
        compensation_journal,
        compensation_diagnostics,
    })
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
    let attempts = stream::iter(reversed.into_iter())
        .then(|compensation| async move {
            run_compensation_with_diagnostic(executor, compensation).await
        })
        .collect::<Vec<(Option<EffectJournalEntry>, CompensationDiagnostic)>>()
        .await;
    let mut journal = Vec::new();
    let mut diagnostics = Vec::new();
    for (entry, diagnostic) in attempts {
        if let Some(item) = entry {
            journal.push(item);
        }
        diagnostics.push(diagnostic);
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
