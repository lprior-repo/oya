use zellij_frontend::layout::{Layout, PaneType};
use zellij_frontend::plugin::TaskRow;
use zellij_frontend::render::Renderer;

fn create_test_task(slug: &str, status: &str) -> TaskRow {
    TaskRow::new(slug, status, "P1", "rust", "main")
}

fn create_test_layout() -> Layout {
    Layout::new_3_pane()
}

fn get_bead_list_output(
    renderer: &Renderer,
    layout: &Layout,
    tasks: &[TaskRow],
    selected_index: usize,
) -> String {
    renderer.render_layout(layout, tasks, selected_index, PaneType::BeadList, None)
}

#[test]
fn test_bead_list_in_progress_row_has_blue_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "in_progress")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    eprintln!("OUTPUT: {output:?}");

    assert!(
        output.contains("\x1b[34m"),
        "In-progress row should have blue color"
    );
}

#[test]
fn test_bead_list_passed_row_has_green_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "passed")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[32m"),
        "Passed row should have green color"
    );
}

#[test]
fn test_bead_list_failed_row_has_red_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "failed")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[31m"),
        "Failed row should have red color"
    );
}

#[test]
fn test_bead_list_open_row_has_white_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[37m"),
        "Open row should have white color"
    );
}

#[test]
fn test_bead_list_blocked_row_has_yellow_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "blocked")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[33m"),
        "Blocked row should have yellow color"
    );
}

#[test]
fn test_bead_list_integrated_row_has_green_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "integrated")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[32m"),
        "Integrated row should have green color"
    );
}

#[test]
fn test_bead_list_running_row_has_blue_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "running")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[34m"),
        "Running row should have blue color"
    );
}

#[test]
fn test_bead_list_completed_row_has_green_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "completed")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[32m"),
        "Completed row should have green color"
    );
}

#[test]
fn test_bead_list_colors_reset_after_row() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "in_progress")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[0m"),
        "Colors should be reset after row content"
    );
}

#[test]
fn test_bead_list_unknown_status_uses_white_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "unknown_status")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[37m"),
        "Unknown status should default to white color"
    );
}
