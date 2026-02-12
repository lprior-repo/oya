use std::fmt::Write;

use thiserror::Error;

use crate::layout::{Layout, Pane, PaneType};
use crate::plugin::TaskRow;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("invalid render dimensions: {0}")]
    InvalidDimensions(String),
}

pub type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug, Clone, Copy, Default)]
pub struct Renderer;

impl Renderer {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[allow(clippy::unused_self)]
    pub fn render_layout(
        &self,
        layout: &Layout,
        tasks: &[TaskRow],
        selected_index: usize,
        focused_pane: PaneType,
        status_message: Option<&str>,
    ) -> String {
        let rows = layout.panes().iter().fold(24usize, |acc, pane| {
            acc.max(pane.row.saturating_sub(1).saturating_add(pane.height))
        });
        let cols = layout.panes().iter().fold(80usize, |acc, pane| {
            acc.max(pane.col.saturating_sub(1).saturating_add(pane.width))
        });

        let mut canvas = Canvas::new(rows, cols);

        for pane in layout.panes() {
            let focused = pane.pane_type == focused_pane;
            let title = if focused {
                format!("{} *", pane.title)
            } else {
                pane.title.clone()
            };
            canvas.draw_box(pane, &title);
            self.render_pane_contents(&mut canvas, pane, tasks, selected_index, status_message);
        }

        canvas.finish()
    }

    fn render_pane_contents(
        &self,
        canvas: &mut Canvas,
        pane: &Pane,
        tasks: &[TaskRow],
        selected_index: usize,
        status_message: Option<&str>,
    ) {
        match pane.pane_type {
            PaneType::BeadList => {
                let mut lines = vec![
                    "Task                  Status      Stage        Pri".to_string(),
                    "--------------------------------------------------".to_string(),
                ];
                for (index, task) in tasks.iter().enumerate() {
                    let marker = if index == selected_index { ">" } else { " " };
                    let stage = task.stage.as_deref().unwrap_or("-");
                    lines.push(format!(
                        "{} {:<20} {:<11} {:<11} {:<3}",
                        marker, task.slug, task.status, stage, task.priority
                    ));
                }
                canvas.write_lines(pane, &lines);
            }
            PaneType::BeadDetail => {
                let mut lines = Vec::new();
                match tasks.get(selected_index) {
                    Some(task) => {
                        lines.push(format!("Slug: {}", task.slug));
                        lines.push(format!("Status: {}", task.status));
                        lines.push(format!("Priority: {}", task.priority));
                        lines.push(format!("Language: {}", task.language));
                        lines.push(format!("Branch: {}", task.branch));
                        lines.push(format!(
                            "Current Stage: {}",
                            task.stage.as_deref().unwrap_or("-")
                        ));
                    }
                    None => lines.push("No task selected".to_string()),
                }
                canvas.write_lines(pane, &lines);
            }
            PaneType::PipelineView => {
                let mut lines = Vec::new();
                match tasks.get(selected_index) {
                    Some(task) => {
                        lines.push("Lifecycle:".to_string());
                        lines.push(task.stage_display());
                    }
                    None => lines.push("No pipeline data".to_string()),
                }
                if let Some(status) = status_message {
                    lines.push(String::new());
                    lines.push(format!("Status: {status}"));
                }
                canvas.write_lines(pane, &lines);
            }
            PaneType::WorkflowGraph => {
                let in_progress = tasks.iter().filter(|t| t.status == "in_progress").count();
                let created = tasks.iter().filter(|t| t.status == "created").count();
                let done = tasks.iter().filter(|t| t.status == "done").count();
                let lines = vec![
                    "Flow: [Created] -> [In Progress] -> [Done]".to_string(),
                    format!("Counts: created={created}  in_progress={in_progress}  done={done}"),
                    "Hint: Tab switches pane, j/k moves, Enter opens details".to_string(),
                ];
                canvas.write_lines(pane, &lines);
            }
        }
    }

    #[allow(clippy::unused_self)]
    pub fn render_help_overlay(
        &self,
        rows: usize,
        cols: usize,
        keybindings: &[(char, &'static str)],
        focused_pane: PaneType,
    ) -> RenderResult<String> {
        if rows == 0 || cols == 0 {
            return Err(RenderError::InvalidDimensions(
                "rows and cols must be > 0".to_string(),
            ));
        }

        let mut output = String::new();
        let _ = writeln!(output, "Help ({focused_pane})");
        let _ = writeln!(output, "");
        for (key, description) in keybindings {
            let _ = writeln!(output, "  {key:<2} {description}");
        }

        Ok(output)
    }
}

struct Canvas {
    width: usize,
    cells: Vec<Vec<char>>,
}

impl Canvas {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            width: cols,
            cells: vec![vec![' '; cols]; rows],
        }
    }

    fn put(&mut self, row: usize, col: usize, ch: char) {
        if row < self.cells.len() && col < self.width {
            self.cells[row][col] = ch;
        }
    }

    fn write_text(&mut self, row: usize, col: usize, text: &str, max_width: usize) {
        if max_width == 0 {
            return;
        }
        for (offset, ch) in text.chars().take(max_width).enumerate() {
            self.put(row, col.saturating_add(offset), ch);
        }
    }

    fn draw_box(&mut self, pane: &Pane, title: &str) {
        if pane.height < 2 || pane.width < 2 {
            return;
        }

        let top = pane.row.saturating_sub(1);
        let left = pane.col.saturating_sub(1);
        let bottom = top.saturating_add(pane.height.saturating_sub(1));
        let right = left.saturating_add(pane.width.saturating_sub(1));

        self.put(top, left, '+');
        self.put(top, right, '+');
        self.put(bottom, left, '+');
        self.put(bottom, right, '+');

        for col in left.saturating_add(1)..right {
            self.put(top, col, '-');
            self.put(bottom, col, '-');
        }

        for row in top.saturating_add(1)..bottom {
            self.put(row, left, '|');
            self.put(row, right, '|');
        }

        if pane.width > 4 {
            let title_text = format!(" {} ", title);
            self.write_text(
                top,
                left.saturating_add(2),
                &title_text,
                pane.width.saturating_sub(4),
            );
        }
    }

    fn write_lines(&mut self, pane: &Pane, lines: &[String]) {
        if pane.height < 3 || pane.width < 3 {
            return;
        }
        let content_top = pane.row;
        let content_left = pane.col;
        let content_height = pane.height.saturating_sub(2);
        let content_width = pane.width.saturating_sub(2);

        for (row_idx, line) in lines.iter().take(content_height).enumerate() {
            self.write_text(
                content_top.saturating_add(row_idx),
                content_left,
                line,
                content_width,
            );
        }
    }

    fn finish(self) -> String {
        let mut output = String::new();
        for row in self.cells {
            let line: String = row.into_iter().collect();
            let trimmed = line.trim_end();
            let _ = writeln!(output, "{trimmed}");
        }
        output
    }
}
