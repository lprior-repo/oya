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
fn test_bead_list_has_header_row() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("Slug"),
        "Header should contain 'Slug' column"
    );
}

#[test]
fn test_bead_list_header_has_stage_column() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("Stage"),
        "Header should contain 'Stage' column"
    );
}

#[test]
fn test_bead_list_header_appears_before_data_rows() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![
        create_test_task("src-aaa", "open"),
        create_test_task("src-bbb", "in_progress"),
    ];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);
    let lines: Vec<&str> = output.lines().collect();

    let header_line_idx = lines.iter().position(|line| line.contains("Slug"));
    let first_task_idx = lines.iter().position(|line| line.contains("src-aaa"));

    match (header_line_idx, first_task_idx) {
        (Some(header_idx), Some(task_idx)) => {
            assert!(
                header_idx < task_idx,
                "Header should appear before data rows"
            );
        }
        _ => panic!("Could not find header or task in output"),
    }
}

#[test]
fn test_bead_list_header_uses_header_style() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("\x1b[1;33m"),
        "Header should use bold yellow style"
    );
}

#[test]
fn test_bead_list_header_only_once() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![
        create_test_task("src-aaa", "open"),
        create_test_task("src-bbb", "in_progress"),
        create_test_task("src-ccc", "passed"),
    ];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    let slug_count = output.matches("Slug").count();
    assert_eq!(
        slug_count, 1,
        "Header 'Slug' should appear exactly once, found {} times",
        slug_count
    );
}

#[test]
fn test_bead_list_empty_list_shows_header() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks: Vec<TaskRow> = vec![];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("Slug"),
        "Header should be rendered even with empty task list"
    );
}

#[test]
fn test_bead_list_header_has_separator_line() {
    let renderer = Renderer::new();
    let layout = create_test_layout();
    let tasks = vec![create_test_task("src-aaa", "open")];

    let output = get_bead_list_output(&renderer, &layout, &tasks, 0);

    assert!(
        output.contains("─"),
        "Header should have separator line with box-drawing characters"
    );
}
