use crate::domain::{Task, TaskStatus};
use crate::error::{Error, Result};
use crate::stages::Stage;

/// Resolve a stage range from CLI inputs.
///
/// # Errors
/// Returns an error when the stage labels are invalid or out of order.
pub fn resolve_stage_range(
    stage: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<Stage>> {
    let start = match from {
        Some(label) => Stage::parse(label)?,
        None => Stage::parse(stage)?,
    };
    let end = match from {
        Some(_) => Stage::parse(to.unwrap_or(stage))?,
        None => start,
    };

    stage_range(start, end)
}

/// Build a strictly increasing stage range.
///
/// # Errors
/// Returns an error when the start stage comes after the end stage.
pub fn stage_range(start: Stage, end: Stage) -> Result<Vec<Stage>> {
    if start.index() > end.index() {
        return Err(Error::InvalidStageSequence(format!(
            "start stage '{start}' must not come after '{end}'"
        )));
    }

    let stages = Stage::all()
        .iter()
        .skip(start.index())
        .take(end.index().saturating_sub(start.index()).saturating_add(1))
        .copied()
        .collect::<Vec<_>>();

    if stages.is_empty() {
        Err(Error::InvalidStageSequence(
            "stage range must contain at least one stage".to_string(),
        ))
    } else {
        Ok(stages)
    }
}

/// Apply a stage plan to a task.
///
/// # Errors
/// Returns an error when the task cannot transition through the stages.
pub fn apply_stage_plan(task: Task, stages: &[Stage]) -> Result<Task> {
    let last_stage = stages.last().copied().ok_or_else(|| {
        Error::InvalidStageSequence("stage plan must contain at least one stage".to_string())
    })?;

    let progressed = stages
        .iter()
        .try_fold(task, |current, stage| current.start_stage(*stage))?;

    if matches!(last_stage, Stage::Accept) {
        progressed.pass_pipeline()
    } else {
        Ok(progressed)
    }
}

/// Build a stage plan for a task up to the provided end stage.
///
/// # Errors
/// Returns an error when the task is already complete or the stage labels are invalid.
pub fn plan_task_stages(task: &Task, end: Stage) -> Result<Vec<Stage>> {
    let start = match &task.status {
        TaskStatus::Created => Stage::Implement,
        TaskStatus::InProgress { stage } => Stage::parse(stage)?,
        TaskStatus::FailedPipeline { stage, .. } => Stage::parse(stage)?,
        TaskStatus::PassedPipeline | TaskStatus::Integrated => {
            return Err(Error::InvalidTransition {
                from: task.status.to_string(),
                to: format!("run pipeline to {end}"),
            });
        }
    };

    stage_range(start, end)
}

/// Run a task through the pipeline up to the provided end stage.
///
/// # Errors
/// Returns an error when the task cannot progress through the pipeline.
pub fn run_task_pipeline(task: Task, end: Stage) -> Result<Task> {
    let stages = plan_task_stages(&task, end)?;
    apply_stage_plan(task, &stages)
}

/// Run a task through the full pipeline (implement → accept).
///
/// # Errors
/// Returns an error when the task cannot progress through the pipeline.
pub fn run_full_pipeline(task: Task) -> Result<Task> {
    run_task_pipeline(task, Stage::Accept)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStageStatus {
    Pending,
    Running,
    Failed,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReport {
    pub stage: Stage,
    pub status: PipelineStageStatus,
}

/// Build a pipeline status report for a task.
#[must_use]
pub fn pipeline_report(task: &Task) -> Vec<StageReport> {
    let stages = Stage::all();
    match &task.status {
        TaskStatus::Created => stages
            .iter()
            .map(|stage| StageReport {
                stage: *stage,
                status: PipelineStageStatus::Pending,
            })
            .collect(),
        TaskStatus::InProgress { stage } => report_with_marker(stages, stage, false),
        TaskStatus::FailedPipeline { stage, .. } => report_with_marker(stages, stage, true),
        TaskStatus::PassedPipeline | TaskStatus::Integrated => stages
            .iter()
            .map(|stage| StageReport {
                stage: *stage,
                status: PipelineStageStatus::Complete,
            })
            .collect(),
    }
}

fn report_with_marker(stages: &[Stage], marker: &str, failed: bool) -> Vec<StageReport> {
    let marker_stage = Stage::parse(marker).ok();
    stages
        .iter()
        .map(|stage| StageReport {
            stage: *stage,
            status: stage_status_for(*stage, marker_stage, failed),
        })
        .collect()
}

fn stage_status_for(stage: Stage, marker: Option<Stage>, failed: bool) -> PipelineStageStatus {
    match marker {
        Some(marker) if stage.index() < marker.index() => PipelineStageStatus::Complete,
        Some(marker) if stage.index() == marker.index() => {
            if failed {
                PipelineStageStatus::Failed
            } else {
                PipelineStageStatus::Running
            }
        }
        _ => PipelineStageStatus::Pending,
    }
}

/// Approve a task for integration.
///
/// # Errors
/// Returns an error when the task is not eligible for integration.
pub fn approve_task(task: Task, force: bool) -> Result<Task> {
    match task.status {
        TaskStatus::Integrated => Ok(task),
        TaskStatus::PassedPipeline => task.integrate(),
        _ if force => {
            let passed = task.start_stage(Stage::Accept)?.pass_pipeline()?;
            passed.integrate()
        }
        _ => Err(Error::InvalidTransition {
            from: task.status.to_string(),
            to: TaskStatus::Integrated.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Language, Slug};

    fn build_task(slug: &str) -> Option<Task> {
        match Slug::new(slug) {
            Ok(slug) => Some(Task::new(slug, Language::Rust)),
            Err(err) => {
                assert!(false, "slug should parse: {err}");
                None
            }
        }
    }

    #[test]
    fn resolve_stage_range_defaults_to_single_stage() {
        let stages = resolve_stage_range("lint", None, None);
        assert!(stages.is_ok());
        let stages = match stages {
            Ok(stages) => stages,
            Err(err) => {
                assert!(false, "expected stages: {err}");
                return;
            }
        };
        assert_eq!(stages, vec![Stage::Lint]);
    }

    #[test]
    fn resolve_stage_range_from_to_builds_sequence() {
        let stages = resolve_stage_range("lint", Some("unit-test"), Some("lint"));
        assert!(stages.is_ok());
        let stages = match stages {
            Ok(stages) => stages,
            Err(err) => {
                assert!(false, "expected stages: {err}");
                return;
            }
        };
        assert_eq!(stages, vec![Stage::UnitTest, Stage::Coverage, Stage::Lint]);
    }

    #[test]
    fn stage_range_rejects_reverse_order() {
        let result = stage_range(Stage::Review, Stage::Implement);
        assert!(result.is_err());
    }

    #[test]
    fn apply_stage_plan_updates_status_for_non_terminal_stage() {
        let task = match build_task("task-stage-1") {
            Some(task) => task,
            None => return,
        };
        let updated = apply_stage_plan(task, &[Stage::Implement]);
        assert!(updated.is_ok());
        let updated = match updated {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected update: {err}");
                return;
            }
        };
        assert_eq!(
            updated.status,
            TaskStatus::InProgress {
                stage: Stage::Implement.as_str().to_string()
            }
        );
    }

    #[test]
    fn apply_stage_plan_marks_pipeline_passed_after_accept() {
        let task = match build_task("task-stage-2") {
            Some(task) => task,
            None => return,
        };
        let stages = [Stage::Implement, Stage::UnitTest, Stage::Accept];
        let updated = apply_stage_plan(task, &stages);
        assert!(updated.is_ok());
        let updated = match updated {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected update: {err}");
                return;
            }
        };
        assert_eq!(updated.status, TaskStatus::PassedPipeline);
    }

    #[test]
    fn plan_task_stages_starts_from_created() {
        let task = match build_task("task-plan-1") {
            Some(task) => task,
            None => return,
        };
        let stages = plan_task_stages(&task, Stage::UnitTest);
        assert!(stages.is_ok());
        let stages = match stages {
            Ok(stages) => stages,
            Err(err) => {
                assert!(false, "expected stages: {err}");
                return;
            }
        };
        assert_eq!(stages, vec![Stage::Implement, Stage::UnitTest]);
    }

    #[test]
    fn plan_task_stages_resumes_from_in_progress() {
        let task = match build_task("task-plan-2") {
            Some(task) => task,
            None => return,
        };
        let task = match task.start_stage(Stage::Lint) {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected task: {err}");
                return;
            }
        };
        let stages = plan_task_stages(&task, Stage::Review);
        assert!(stages.is_ok());
        let stages = match stages {
            Ok(stages) => stages,
            Err(err) => {
                assert!(false, "expected stages: {err}");
                return;
            }
        };
        assert_eq!(
            stages,
            vec![
                Stage::Lint,
                Stage::Static,
                Stage::Integration,
                Stage::Security,
                Stage::Review
            ]
        );
    }

    #[test]
    fn plan_task_stages_resumes_from_failed_stage() {
        let task = match build_task("task-plan-3") {
            Some(task) => task,
            None => return,
        };
        let task = match task.fail_stage(Stage::Coverage, "failed") {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected failed task: {err}");
                return;
            }
        };
        let stages = plan_task_stages(&task, Stage::Lint);
        assert!(stages.is_ok());
        let stages = match stages {
            Ok(stages) => stages,
            Err(err) => {
                assert!(false, "expected stages: {err}");
                return;
            }
        };
        assert_eq!(stages, vec![Stage::Coverage, Stage::Lint]);
    }

    #[test]
    fn plan_task_stages_rejects_completed_task() {
        let task = match build_task("task-plan-4") {
            Some(task) => task,
            None => return,
        };
        let task = match task
            .start_stage(Stage::Accept)
            .and_then(|task| task.pass_pipeline())
        {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected passed pipeline: {err}");
                return;
            }
        };
        let result = plan_task_stages(&task, Stage::Accept);
        assert!(result.is_err());
    }

    #[test]
    fn approve_task_requires_passed_pipeline_without_force() {
        let task = match build_task("task-approve-1") {
            Some(task) => task,
            None => return,
        };
        let result = approve_task(task, false);
        assert!(result.is_err());
    }

    #[test]
    fn approve_task_integrates_after_passed_pipeline() {
        let task = match build_task("task-approve-2") {
            Some(task) => task,
            None => return,
        };
        let progressed = task
            .start_stage(Stage::Accept)
            .and_then(|task| task.pass_pipeline());
        assert!(progressed.is_ok());
        let progressed = match progressed {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected passed pipeline: {err}");
                return;
            }
        };
        let approved = approve_task(progressed, false);
        assert!(approved.is_ok());
        let approved = match approved {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected approved task: {err}");
                return;
            }
        };
        assert_eq!(approved.status, TaskStatus::Integrated);
    }

    #[test]
    fn approve_task_force_advances_to_integration() {
        let task = match build_task("task-approve-3") {
            Some(task) => task,
            None => return,
        };
        let approved = approve_task(task, true);
        assert!(approved.is_ok());
        let approved = match approved {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected approved task: {err}");
                return;
            }
        };
        assert_eq!(approved.status, TaskStatus::Integrated);
    }

    #[test]
    fn run_full_pipeline_from_created_passes_pipeline() {
        let task = match build_task("task-run-1") {
            Some(task) => task,
            None => return,
        };

        let updated = run_full_pipeline(task);
        assert!(updated.is_ok());
        let updated = match updated {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected pipeline completion: {err}");
                return;
            }
        };

        assert_eq!(updated.status, TaskStatus::PassedPipeline);
    }

    #[test]
    fn run_task_pipeline_halts_at_requested_stage() {
        let task = match build_task("task-run-2") {
            Some(task) => task,
            None => return,
        };

        let updated = run_task_pipeline(task, Stage::Review);
        assert!(updated.is_ok());
        let updated = match updated {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected pipeline progression: {err}");
                return;
            }
        };

        assert_eq!(
            updated.status,
            TaskStatus::InProgress {
                stage: Stage::Review.as_str().to_string()
            }
        );
    }

    #[test]
    fn pipeline_report_marks_running_stage() {
        let task = match build_task("task-report-1") {
            Some(task) => task,
            None => return,
        };
        let task = match task.start_stage(Stage::Lint) {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected stage start: {err}");
                return;
            }
        };

        let report = pipeline_report(&task);
        let lint = report
            .iter()
            .find(|entry| entry.stage == Stage::Lint)
            .map(|entry| entry.status);

        assert_eq!(lint, Some(PipelineStageStatus::Running));
    }

    #[test]
    fn pipeline_report_marks_failed_stage() {
        let task = match build_task("task-report-2") {
            Some(task) => task,
            None => return,
        };
        let task = match task.fail_stage(Stage::Integration, "failed") {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected failure: {err}");
                return;
            }
        };

        let report = pipeline_report(&task);
        let integration = report
            .iter()
            .find(|entry| entry.stage == Stage::Integration)
            .map(|entry| entry.status);

        assert_eq!(integration, Some(PipelineStageStatus::Failed));
    }

    #[test]
    fn pipeline_report_marks_all_complete_when_passed() {
        let task = match build_task("task-report-3") {
            Some(task) => task,
            None => return,
        };
        let task = match task
            .start_stage(Stage::Accept)
            .and_then(|task| task.pass_pipeline())
        {
            Ok(task) => task,
            Err(err) => {
                assert!(false, "expected passed pipeline: {err}");
                return;
            }
        };

        let report = pipeline_report(&task);
        let all_complete = report
            .iter()
            .all(|entry| entry.status == PipelineStageStatus::Complete);
        assert!(all_complete);
    }
}
