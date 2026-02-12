use zellij_frontend::layout::{Layout, Pane, PaneType};
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
    let full_output =
        renderer.render_layout(layout, tasks, selected_index, PaneType::BeadList, None);
    full_output
}

#[test]
fn test_bead_list_selected_row_has_highlight() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![
        create_test_task("src-aaa", "open"),
        create_test_task("src-bbb", "in_progress"),
        create_test_task("src-ccc", "passed"),
    ];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 1);

    assert!(
        output.contains("\x1b[7m"),
        "Selected row should have reverse video ANSI code"
    );
}

#[test]
fn test_bead_list_selected_row_starts_with_selection_style() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[7m"),
        "Selected row should start with selection style"
    );
}

#[test]
fn test_bead_list_non_selected_row_has_no_highlight() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![
        create_test_task("src-aaa", "open"),
        create_test_task("src-bbb", "in_progress"),
    ];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 1);

    let lines: Vec<&str> = output.lines().collect();
    let first_task_line = lines.iter().find(|line| line.contains("src-aaa"));

    if let Some(line) = first_task_line {
        assert!(
            !line.starts_with("\x1b[7m"),
            "Non-selected row should not start with selection style"
        );
    }
}

#[test]
fn test_bead_list_selection_resets_after_row() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[0m"),
        "Should have reset code after styled content"
    );
}

#[test]
fn test_bead_list_selected_row_displays_indicator() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains('►'),
        "Selected row should have selection indicator"
    );
}

#[test]
fn test_bead_list_multiple_rows_only_one_selected() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![
        create_test_task("src-aaa", "open"),
        create_test_task("src-bbb", "in_progress"),
        create_test_task("src-ccc", "passed"),
    ];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 1);

    let lines: Vec<&str> = output.lines().collect();
    let task_lines_with_highlight: Vec<&&str> = lines
        .iter()
        .filter(|line| line.contains("\x1b[7m"))
        .collect();

    assert_eq!(
        task_lines_with_highlight.len(),
        1,
        "Exactly one row should be highlighted"
    );
}
