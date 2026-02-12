//! Substep rendering tests for PipelineView
//!
//! Tests the rendering of substeps within pipeline stages.

use zellij_frontend::layout::{Layout, PaneType};
use zellij_frontend::plugin::{StageInfo, StageState, SubStep, SubStepState, TaskRow};
use zellij_frontend::render::Renderer;

fn create_test_task_with_substeps() -> TaskRow {
    let mut task = TaskRow::new("src-abc1", "in_progress", "P0", "rust", "main");

    if let Some(implement_stage) = task.stages.iter_mut().find(|s| s.name == "implement") {
        implement_stage.state = StageState::Running;
        implement_stage.substeps = vec![
            SubStep::new("write_tests", SubStepState::Completed),
            SubStep::new("write_code", SubStepState::Running),
            SubStep::new("run_tests", SubStepState::NotStarted),
        ];
    }

    if let Some(research_stage) = task.stages.iter_mut().find(|s| s.name == "research") {
        research_stage.state = StageState::Completed;
    }

    if let Some(plan_stage) = task.stages.iter_mut().find(|s| s.name == "plan") {
        plan_stage.state = StageState::Completed;
    }

    task
}

#[test]
fn substep_new_creates_with_defaults() {
    let substep = SubStep::new("test_substep", SubStepState::NotStarted);
    assert_eq!(substep.name, "test_substep");
    assert_eq!(substep.state, SubStepState::NotStarted);
}

#[test]
fn substep_symbol_returns_correct_symbols() {
    assert_eq!(SubStep::new("a", SubStepState::NotStarted).symbol(), "○");
    assert_eq!(SubStep::new("a", SubStepState::Running).symbol(), "●");
    assert_eq!(SubStep::new("a", SubStepState::Completed).symbol(), "✓");
    assert_eq!(SubStep::new("a", SubStepState::Failed).symbol(), "✗");
}

#[test]
fn substep_display_formats_correctly() {
    let substep = SubStep::new("test_step", SubStepState::Completed);
    assert_eq!(substep.display(), "✓ test_step");
}

#[test]
fn stage_info_can_have_substeps() {
    let mut stage = StageInfo::new("implement");
    stage.substeps = vec![
        SubStep::new("write_tests", SubStepState::Completed),
        SubStep::new("write_code", SubStepState::Running),
    ];

    assert_eq!(stage.substeps.len(), 2);
}

#[test]
fn render_pipeline_view_includes_substeps() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let pane = layout
        .get_pane(PaneType::PipelineView)
        .ok_or("PipelineView pane missing")?;
    let task = create_test_task_with_substeps();

    let output = renderer.render_pipeline_view(pane, &task);

    assert!(output.contains("implement"), "Should contain stage name");
    assert!(
        output.contains("write_tests"),
        "Should contain substep name"
    );
    assert!(output.contains("write_code"), "Should contain substep name");
    Ok(())
}

#[test]
fn render_pipeline_view_shows_substep_states() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let pane = layout
        .get_pane(PaneType::PipelineView)
        .ok_or("PipelineView pane missing")?;
    let task = create_test_task_with_substeps();

    let output = renderer.render_pipeline_view(pane, &task);

    assert!(output.contains("✓"), "Should show completed substep symbol");
    assert!(output.contains("●"), "Should show running substep symbol");
    Ok(())
}

#[test]
fn render_pipeline_view_indents_substeps() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let pane = layout
        .get_pane(PaneType::PipelineView)
        .ok_or("PipelineView pane missing")?;
    let task = create_test_task_with_substeps();

    let output = renderer.render_pipeline_view(pane, &task);

    assert!(
        output.contains("  "),
        "Should have indentation for substeps"
    );
    Ok(())
}

#[test]
fn render_pipeline_view_hides_empty_substeps() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let pane = layout
        .get_pane(PaneType::PipelineView)
        .ok_or("PipelineView pane missing")?;

    let mut task = TaskRow::new("src-xyz", "open", "P1", "gleam", "feature");
    task.stages[0].state = StageState::Completed;

    let output = renderer.render_pipeline_view(pane, &task);

    assert!(
        !output.contains("  ○ ")
            && !output.contains("  ● ")
            && !output.contains("  ✓ ")
            && !output.contains("  ✗ "),
        "Should not show indented substep lines when none exist"
    );
    Ok(())
}

#[test]
fn substep_state_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let substep = SubStep::new("test_step", SubStepState::Running);
    let json = serde_json::to_string(&substep)?;
    let decoded: SubStep = serde_json::from_str(&json)?;

    assert_eq!(substep.name, decoded.name);
    assert_eq!(substep.state, decoded.state);
    Ok(())
}

#[test]
fn render_pipeline_view_shows_failed_substep() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new();
    let layout = Layout::new_3_pane();
    let pane = layout
        .get_pane(PaneType::PipelineView)
        .ok_or("PipelineView pane missing")?;

    let mut task = TaskRow::new("src-fail", "in_progress", "P0", "rust", "main");
    if let Some(stage) = task.stages.iter_mut().find(|s| s.name == "implement") {
        stage.state = StageState::Failed;
        stage.substeps = vec![
            SubStep::new("write_tests", SubStepState::Completed),
            SubStep::new("run_tests", SubStepState::Failed),
        ];
    }

    let output = renderer.render_pipeline_view(pane, &task);
    assert!(output.contains("✗"), "Should show failed substep symbol");
    Ok(())
}
