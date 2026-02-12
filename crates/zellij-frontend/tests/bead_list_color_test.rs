use zellij_frontend::layout::{Layout, Pane, PaneType};
use zellij_frontend::plugin::{StageInfo, StageState, TaskRow};
use zellij_frontend::render::Renderer;

fn create_test_task(slug: &str, status: &str) -> TaskRow {
    TaskRow::new(slug, status, "P1", "rust", "main")
}

fn create_test_layout() -> Layout {
    let mut layout = Layout::new(80, 24).expect("Failed to create layout");
    let pane = Pane::new(PaneType::BeadList, 1, 1, 20, 40).expect("Failed to create pane");
    layout.add_pane(pane);
    layout
}

fn get_bead_list_output(
    renderer: &Renderer,
    layout: &Layout,
    tasks: &[TaskRow],
    selected_index: usize,
) -> String {
    renderer.render_layout(layout, tasks, selected_index, PaneType::BeadList, None)
}

fn extract_bead_list_content(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.contains("src-"))
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn test_bead_list_in_progress_row_has_blue_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "in_progress")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let bead_lines = extract_bead_list_content(&output);

    assert_eq!(bead_lines.len(), 1, "Should have one task line");
    assert!(
        bead_lines[0].contains("\x1b[34m") || bead_lines[0].contains("\x1b[94m"),
        "In-progress row should have blue color, got: {:?}",
        bead_lines[0]
    );
}

#[test]
fn test_bead_list_passed_row_has_green_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "passed")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let bead_lines = extract_bead_list_content(&output);

    assert_eq!(bead_lines.len(), 1, "Should have one task line");
    assert!(
        bead_lines[0].contains("\x1b[32m") || bead_lines[0].contains("\x1b[92m"),
        "Passed row should have green color, got: {:?}",
        bead_lines[0]
    );
}

#[test]
fn test_bead_list_failed_row_has_red_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "failed")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let bead_lines = extract_bead_list_content(&output);

    assert_eq!(bead_lines.len(), 1, "Should have one task line");
    assert!(
        bead_lines[0].contains("\x1b[31m") || bead_lines[0].contains("\x1b[91m"),
        "Failed row should have red color, got: {:?}",
        bead_lines[0]
    );
}

#[test]
fn test_bead_list_open_row_has_default_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let bead_lines = extract_bead_list_content(&output);

    assert_eq!(bead_lines.len(), 1, "Should have one task line");
    assert!(
        bead_lines[0].contains("\x1b[37m")
            || bead_lines[0].contains("\x1b[0m")
            || !bead_lines[0].contains("\x1b[3"),
        "Open row should have white/default color, got: {:?}",
        bead_lines[0]
    );
}

#[test]
fn test_bead_list_blocked_row_has_yellow_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "blocked")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let bead_lines = extract_bead_list_content(&output);

    assert_eq!(bead_lines.len(), 1, "Should have one task line");
    assert!(
        bead_lines[0].contains("\x1b[33m") || bead_lines[0].contains("\x1b[93m"),
        "Blocked row should have yellow color, got: {:?}",
        bead_lines[0]
    );
}

#[test]
fn test_bead_list_integrated_row_has_green_color() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-abc", "integrated")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let bead_lines = extract_bead_list_content(&output);

    assert_eq!(bead_lines.len(), 1, "Should have one task line");
    assert!(
        bead_lines[0].contains("\x1b[32m") || bead_lines[0].contains("\x1b[92m"),
        "Integrated row should have green color, got: {:?}",
        bead_lines[0]
    );
}

#[test]
fn test_bead_list_multiple_rows_have_different_colors() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![
        create_test_task("src-aaa", "open"),
        create_test_task("src-bbb", "in_progress"),
        create_test_task("src-ccc", "passed"),
        create_test_task("src-ddd", "failed"),
    ];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 1);
    let lines: Vec<&str> = output.lines().collect();

    let open_line = lines.iter().find(|l| l.contains("src-aaa"));
    let in_progress_line = lines.iter().find(|l| l.contains("src-bbb"));
    let passed_line = lines.iter().find(|l| l.contains("src-ccc"));
    let failed_line = lines.iter().find(|l| l.contains("src-ddd"));

    assert!(
        in_progress_line.map_or(false, |l| l.contains("\x1b[34m") || l.contains("\x1b[94m")),
        "In-progress should be blue"
    );
    assert!(
        passed_line.map_or(false, |l| l.contains("\x1b[32m") || l.contains("\x1b[92m")),
        "Passed should be green"
    );
    assert!(
        failed_line.map_or(false, |l| l.contains("\x1b[31m") || l.contains("\x1b[91m")),
        "Failed should be red"
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
