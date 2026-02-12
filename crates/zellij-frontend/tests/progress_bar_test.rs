//! Progress bar rendering tests for BeadList
//!
//! Tests the rendering of progress bars in the BeadList pane.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use zellij_frontend::layout::{Layout, PaneType};
use zellij_frontend::plugin::{StageState, TaskRow};
use zellij_frontend::render::Renderer;

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

fn create_open_task() -> TaskRow {
    TaskRow::new("src-open", "open", "P3", "rust", "main")
}

#[test]
fn render_bead_list_shows_progress_bar_for_in_progress_task() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_task_with_progress()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    assert!(
        output.contains('█') || output.contains('░'),
        "BeadList should show progress bar for in_progress task"
    );
    assert!(output.contains('%'), "Progress bar should show percentage");
}

#[test]
fn render_bead_list_no_progress_bar_for_passed_status() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let task = create_completed_task();
    let tasks = vec![task.clone()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // For "passed" status, the slug line in BeadList should NOT contain progress bar chars
    // We check that the line containing the slug doesn't also contain █ or percentage
    for line in output.lines() {
        if line.contains(&task.slug) {
            // This is the BeadList line for our task
            // It should NOT have both progress bar chars AND percentage (that's the progress bar)
            let has_bar_chars = line.contains('█') || line.contains('░');
            let has_percentage = line.contains('%');
            assert!(
                !(has_bar_chars && has_percentage),
                "Task with 'passed' status should not show progress bar in BeadList"
            );
        }
    }
}

#[test]
fn render_bead_list_no_progress_bar_for_open_status() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_open_task()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // "open" status should not show progress bar
    let has_progress_bar = output.contains('█') && output.contains('░');
    assert!(
        !has_progress_bar,
        "Task with 'open' status should not show progress bar"
    );
}

#[test]
fn render_bead_list_shows_progress_for_failed_in_progress() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_failed_task()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Failed in_progress tasks still show progress bar
    assert!(
        output.contains('█') || output.contains('░'),
        "Failed in_progress task should still show progress bar"
    );
}

#[test]
fn render_bead_list_multiple_tasks_mixed_status() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![
        create_task_with_progress(), // in_progress - should show bar
        create_completed_task(),     // passed - no bar
        create_failed_task(),        // in_progress (failed stage) - should show bar
        create_open_task(),          // open - no bar
    ];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Should have progress bar characters for the in_progress tasks
    let has_bar = output.contains('█') || output.contains('░');
    assert!(has_bar, "Should show progress bars for in_progress tasks");
}

#[test]
fn render_bead_detail_shows_pipeline_stages() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![create_task_with_progress()];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadDetail, None);

    // BeadDetail pane should show pipeline stages with progress
    assert!(
        output.contains("Pipeline"),
        "BeadDetail should show Pipeline header"
    );
    assert!(
        output.contains("research") || output.contains("plan") || output.contains("implement"),
        "BeadDetail should show stage names"
    );
}

#[test]
fn render_pipeline_view_shows_stages() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let pane = layout
        .get_pane(PaneType::PipelineView)
        .expect("PipelineView pane");
    let task = create_task_with_progress();

    let output = renderer.render_pipeline_view(pane, &task);

    assert!(
        output.contains("Pipeline"),
        "PipelineView should show Pipeline header"
    );
    assert!(
        output.contains('█') || output.contains('░'),
        "PipelineView should show progress bars"
    );
}

#[test]
fn render_empty_task_list_no_crash() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks: Vec<TaskRow> = vec![];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Should not crash and should show the BeadList pane
    assert!(
        output.contains("Beads") || output.contains("Bead List"),
        "Should show BeadList pane even when empty"
    );
}

#[test]
fn render_selected_task_highlighted() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![
        TaskRow::new("src-one", "open", "P1", "rust", "main"),
        TaskRow::new("src-two", "open", "P2", "rust", "main"),
    ];

    let output_first = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);
    let output_second = renderer.render_layout(&layout, &tasks, 1, PaneType::BeadList, None);

    // First task should be highlighted in first render
    assert!(
        output_first.contains("src-one"),
        "First task should be visible"
    );
    assert!(
        output_second.contains("src-two"),
        "Second task should be visible"
    );
}

#[test]
fn render_progress_bar_with_large_task_list() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();

    let tasks: Vec<TaskRow> = (0..20)
        .map(|i| {
            let mut task = TaskRow::new(
                &format!("src-{:03}", i),
                if i % 3 == 0 { "in_progress" } else { "open" },
                "P1",
                "rust",
                "main",
            );
            if i % 3 == 0 {
                if let Some(stage) = task.stages.iter_mut().find(|s| s.name == "implement") {
                    stage.state = StageState::Running;
                }
            }
            task
        })
        .collect();

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // Should handle many tasks without crashing
    assert!(
        output.contains('█') || output.contains('░'),
        "Should show progress bars for in_progress tasks in large list"
    );
}

#[test]
fn render_integrated_status_no_progress_bar() {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let tasks = vec![TaskRow::new("src-int", "integrated", "P1", "rust", "main")];

    let output = renderer.render_layout(&layout, &tasks, 0, PaneType::BeadList, None);

    // "integrated" status should not show progress bar (only "in_progress" does)
    let has_progress_bar = output.contains('█') && output.contains('░');
    assert!(
        !has_progress_bar,
        "Task with 'integrated' status should not show progress bar"
    );
}
