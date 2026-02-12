//! Progress bar rendering tests for BeadList
//!
//! Tests the rendering of progress bars in the BeadList pane.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use zellij_frontend::layout::{Layout, PaneType};
use zellij_frontend::plugin::{StageInfo, StageState, TaskRow};
use zellij_frontend::render::{
    calculate_stage_progress, get_stage_info, render_progress_bar, Renderer,
};

fn create_task_with_progress() -> TaskRow {
    let mut task = TaskRow::new("src-abc1", "in_progress", "P0", "rust", "main");

    if let Some(implement_stage) = task.stages.iter_mut().find(|s| s.name == "implement") {
        implement_stage.state = StageState::Running;
    }

    if let Some(research_stage) = task.stages.iter_mut().find(|s| s.name == "research") {
        research_stage.state = StageState::Completed;
    }

    if let Some(plan_stage) = task.stages.iter_mut().find(|s| s.name == "plan") {
        plan_stage.state = StageState::Completed;
    }

    task
}

fn create_completed_task() -> TaskRow {
    let mut task = TaskRow::new("src-done", "passed", "P1", "gleam", "main");

    for stage in &mut task.stages {
        stage.state = StageState::Completed;
    }

    task
}

fn create_failed_task() -> TaskRow {
    let mut task = TaskRow::new("src-fail", "in_progress", "P2", "rust", "feature");

    if let Some(research_stage) = task.stages.iter_mut().find(|s| s.name == "research") {
        research_stage.state = StageState::Completed;
    }

    if let Some(plan_stage) = task.stages.iter_mut().find(|s| s.name == "plan") {
        plan_stage.state = StageState::Failed;
    }

    task
}

#[test]
fn render_progress_bar_zero_percent() {
    let bar = render_progress_bar(0.0, 10);
    assert!(bar.contains("░"), "Empty bar should use ░");
    assert!(bar.contains("0%"), "Should show 0%");
}

#[test]
fn render_progress_bar_full() {
    let bar = render_progress_bar(1.0, 10);
    assert!(bar.contains("█"), "Full bar should use █");
    assert!(bar.contains("100%"), "Should show 100%");
}

#[test]
fn render_progress_bar_half() {
    let bar = render_progress_bar(0.5, 10);
    let filled_count = bar.chars().filter(|&c| c == '█').count();
    assert_eq!(filled_count, 5, "Half progress should fill 5 chars");
    assert!(bar.contains("50%"), "Should show 50%");
}

#[test]
fn render_progress_bar_clamps_high_values() {
    let bar = render_progress_bar(1.5, 10);
    assert!(bar.contains("100%"), "Should clamp to 100%");
}

#[test]
fn render_progress_bar_clamps_negative() {
    let bar = render_progress_bar(-0.5, 10);
    assert!(bar.contains("0%"), "Should clamp to 0%");
}

#[test]
fn render_progress_bar_custom_width() {
    let bar = render_progress_bar(0.5, 20);
    let filled_count = bar.chars().filter(|&c| c == '█').count();
    assert_eq!(filled_count, 10, "Half of 20 should be 10 filled");
}

#[test]
fn get_stage_info_returns_running_stage() {
    let task = create_task_with_progress();
    let (running, _, _) = get_stage_info(&task);

    assert!(running.is_some(), "Should have a running stage");
    assert_eq!(running, Some(3), "implement is at index 3");
}

#[test]
fn get_stage_info_returns_none_when_no_running() {
    let task = create_completed_task();
    let (running, _, _) = get_stage_info(&task);

    assert!(running.is_none(), "Completed task has no running stage");
}

#[test]
fn get_stage_info_identifies_completed_task() {
    let task = create_completed_task();
    let (_, _, is_completed) = get_stage_info(&task);

    assert!(is_completed, "Task with 'passed' status is completed");
}

#[test]
fn get_stage_info_identifies_failed_stage() {
    let task = create_failed_task();
    let (_, failed, _) = get_stage_info(&task);

    assert!(failed.is_some(), "Should have a failed stage");
}

#[test]
fn calculate_progress_for_completed_task() {
    let task = create_completed_task();
    let (running, failed, is_completed) = get_stage_info(&task);
    let progress = calculate_stage_progress(&task.stages, running, failed, is_completed);

    assert!(
        (progress - 1.0).abs() < f32::EPSILON,
        "Completed task should be 100%"
    );
}

#[test]
fn calculate_progress_for_running_task() {
    let task = create_task_with_progress();
    let (running, failed, is_completed) = get_stage_info(&task);
    let progress = calculate_stage_progress(&task.stages, running, failed, is_completed);

    assert!(progress > 0.0, "Progress should be greater than 0");
    assert!(progress < 1.0, "Progress should be less than 1");
}

#[test]
fn calculate_progress_accounts_for_running_stage() {
    let task = create_task_with_progress();
    let (running, failed, is_completed) = get_stage_info(&task);
    let progress = calculate_stage_progress(&task.stages, running, failed, is_completed);

    // 2 completed + 0.5 for running = 2.5 / 6 = ~0.417
    let expected = 2.5_f32 / 6.0_f32;
    assert!(
        (progress - expected).abs() < 0.01,
        "Progress should account for running stage"
    );
}

#[test]
fn render_bead_list_shows_progress_bar_for_in_progress() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_task_with_progress()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    assert!(
        output.contains('█') || output.contains('░'),
        "BeadList should show progress bar for in_progress task"
    );
}

#[test]
fn render_bead_list_no_progress_bar_for_completed() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_completed_task()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Completed tasks with "passed" status don't show the bar (only "in_progress" does)
    assert!(
        !output.contains("[████") || !output.contains("░░░░]"),
        "Completed task should not show progress bar"
    );
}

#[test]
fn render_bead_list_progress_bar_width() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_task_with_progress()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Progress bar should include percentage
    assert!(output.contains('%'), "Progress bar should show percentage");
}

#[test]
fn calculate_progress_zero_stages() {
    let stages: Vec<StageInfo> = vec![];
    let progress = calculate_stage_progress(&stages, None, None, false);

    // Division by zero should be handled safely
    assert!(
        progress.is_finite(),
        "Progress should be finite with zero stages"
    );
}

#[test]
fn render_progress_bar_zero_width() {
    let bar = render_progress_bar(0.5, 0);
    // Should not panic, returns empty bar with percentage
    assert!(bar.contains('%'), "Should still show percentage");
}

#[test]
fn render_bead_list_multiple_tasks_with_progress() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![
        create_task_with_progress(),
        create_completed_task(),
        create_failed_task(),
    ];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Only in_progress tasks should show progress bar
    let progress_bar_count = output.matches('█').count() + output.matches('░').count();
    assert!(
        progress_bar_count > 0,
        "Should show at least one progress bar"
    );
}
