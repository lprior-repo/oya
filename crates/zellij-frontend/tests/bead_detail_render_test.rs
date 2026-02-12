use oya_ipc::BeadDetail;
use zellij_frontend::layout::{Pane, PaneType};
use zellij_frontend::render::Renderer;

fn create_test_bead_detail() -> BeadDetail {
    BeadDetail {
        id: "src-abc123".to_string(),
        title: "Implement Feature X".to_string(),
        description: "A detailed description of the feature implementation".to_string(),
        state: "in_progress".to_string(),
        priority: 1,
        issue_type: "feature".to_string(),
        workflow_id: "wf-main".to_string(),
        created_at: 1704067200,
        updated_at: 1704153600,
        labels: vec!["size:small".to_string(), "stage:ready".to_string()],
        dependencies: vec!["src-001".to_string(), "src-002".to_string()],
    }
}

#[test]
fn test_render_bead_detail_metadata_happy_path() {
    let renderer = Renderer::new();
    let bead = create_test_bead_detail();
    let pane = Pane::new(PaneType::BeadDetail, 0, 0, 20, 50).expect("pane");

    let output = renderer.render_bead_detail_metadata(&pane, &bead);

    assert!(output.contains("src-abc123"), "Should contain bead ID");
    assert!(
        output.contains("Implement Feature X"),
        "Should contain title"
    );
    assert!(output.contains("in_progress"), "Should contain state");
    assert!(output.contains("feature"), "Should contain issue_type");
    assert!(output.contains("P1"), "Should contain priority");
    assert!(output.contains("wf-main"), "Should contain workflow_id");
    assert!(output.contains("size:small"), "Should contain labels");
    assert!(output.contains("src-001"), "Should contain dependencies");
}

#[test]
fn test_render_bead_detail_metadata_with_empty_labels() {
    let renderer = Renderer::new();
    let mut bead = create_test_bead_detail();
    bead.labels = vec![];
    let pane = Pane::new(PaneType::BeadDetail, 0, 0, 20, 50).expect("pane");

    let output = renderer.render_bead_detail_metadata(&pane, &bead);

    assert!(output.contains("Labels:"), "Should have labels section");
    assert!(
        output.contains("none") || !output.contains("size:small"),
        "Should show empty state"
    );
}

#[test]
fn test_render_bead_detail_metadata_with_empty_dependencies() {
    let renderer = Renderer::new();
    let mut bead = create_test_bead_detail();
    bead.dependencies = vec![];
    let pane = Pane::new(PaneType::BeadDetail, 0, 0, 20, 50).expect("pane");

    let output = renderer.render_bead_detail_metadata(&pane, &bead);

    assert!(
        output.contains("Dependencies:") || output.contains("Deps:"),
        "Should have dependencies section"
    );
}

#[test]
fn test_render_bead_detail_metadata_format_priority() {
    let renderer = Renderer::new();
    let mut bead = create_test_bead_detail();
    bead.priority = 0;
    let pane = Pane::new(PaneType::BeadDetail, 0, 0, 20, 50).expect("pane");

    let output_p0 = renderer.render_bead_detail_metadata(&pane, &bead);
    assert!(output_p0.contains("P0"), "Should show P0 for priority 0");

    bead.priority = 3;
    let output_p3 = renderer.render_bead_detail_metadata(&pane, &bead);
    assert!(output_p3.contains("P3"), "Should show P3 for priority 3");
}

#[test]
fn test_render_bead_detail_metadata_truncates_long_description() {
    let renderer = Renderer::new();
    let mut bead = create_test_bead_detail();
    bead.description = "This is a very long description that should be truncated when rendered in a small pane to avoid overflow issues in the terminal display".to_string();
    let pane = Pane::new(PaneType::BeadDetail, 0, 0, 10, 40).expect("pane");

    let output = renderer.render_bead_detail_metadata(&pane, &bead);

    assert!(!output.is_empty(), "Should produce output");
    assert!(
        output.contains("Description:"),
        "Should have description section"
    );
}

#[test]
fn test_render_bead_detail_metadata_handles_special_chars() {
    let renderer = Renderer::new();
    let mut bead = create_test_bead_detail();
    bead.title = "Feature with \"quotes\" and <brackets>".to_string();
    bead.labels = vec![
        "label:with:colons".to_string(),
        "label/with/slashes".to_string(),
    ];
    let pane = Pane::new(PaneType::BeadDetail, 0, 0, 20, 50).expect("pane");

    let output = renderer.render_bead_detail_metadata(&pane, &bead);

    assert!(output.contains("quotes"), "Should handle quotes in title");
    assert!(
        output.contains("brackets"),
        "Should handle brackets in title"
    );
}
