#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! OYA Zellij Plugin - Pipeline Orchestration Dashboard
//!
//! Real-time terminal UI for pipeline status, bead execution, and stage progress.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

// Plugin state
#[derive(Default)]
struct State {
    // Current view mode
    mode: ViewMode,

    // API connection
    server_url: String,
    api_connected: bool,
    last_error: Option<String>,

    // Bead data
    beads: Vec<BeadInfo>,
    selected_index: usize,

    // Pipeline data for selected bead
    pipeline_stages: Vec<StageInfo>,
}

#[derive(Default, Clone, Copy)]
enum ViewMode {
    #[default]
    BeadList,
    BeadDetail,
    PipelineView,
}

#[derive(Clone)]
struct BeadInfo {
    id: String,
    title: String,
    status: BeadStatus,
    current_stage: Option<String>,
    progress: f32, // 0.0 to 1.0
}

#[derive(Clone, Copy)]
enum BeadStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl BeadStatus {
    fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    fn color(&self) -> &str {
        match self {
            Self::Pending => "\x1b[90m",      // gray
            Self::InProgress => "\x1b[33m",   // yellow
            Self::Completed => "\x1b[32m",    // green
            Self::Failed => "\x1b[31m",       // red
        }
    }
}

#[derive(Clone)]
struct StageInfo {
    name: String,
    status: StageStatus,
    duration_ms: Option<u64>,
}

#[derive(Clone, Copy)]
enum StageStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

impl StageStatus {
    fn symbol(&self) -> &str {
        match self {
            Self::Pending => "○",
            Self::Running => "◐",
            Self::Passed => "●",
            Self::Failed => "✗",
            Self::Skipped => "⊘",
        }
    }

    fn color(&self) -> &str {
        match self {
            Self::Pending => "\x1b[90m",
            Self::Running => "\x1b[33m",
            Self::Passed => "\x1b[32m",
            Self::Failed => "\x1b[31m",
            Self::Skipped => "\x1b[90m",
        }
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Get server URL from config
        self.server_url = configuration
            .get("server_url")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "http://localhost:3000".to_string());

        // Request permissions
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        // Subscribe to events
        subscribe(&[EventType::Key, EventType::Timer]);

        // Set timer for auto-refresh (every 2 seconds)
        set_timeout(2.0);

        // Initial data load
        self.load_beads();
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key_with_mod) => {
                let bare_key = key_with_mod.bare_key;

                // Handle special keys and characters
                match bare_key {
                    // Quit
                    BareKey::Char('q') | BareKey::Esc => {
                        close_focus();
                        false
                    }

                    // Navigation
                    BareKey::Char('j') | BareKey::Down => {
                        if self.selected_index < self.beads.len().saturating_sub(1) {
                            self.selected_index += 1;
                            if self.mode == ViewMode::PipelineView {
                                self.load_pipeline_for_selected();
                            }
                        }
                        true
                    }
                    BareKey::Char('k') | BareKey::Up => {
                        self.selected_index = self.selected_index.saturating_sub(1);
                        if self.mode == ViewMode::PipelineView {
                            self.load_pipeline_for_selected();
                        }
                        true
                    }
                    BareKey::Char('g') => {
                        self.selected_index = 0;
                        true
                    }
                    BareKey::Char('G') => {
                        self.selected_index = self.beads.len().saturating_sub(1);
                        true
                    }

                    // View switching
                    BareKey::Char('1') => {
                        self.mode = ViewMode::BeadList;
                        true
                    }
                    BareKey::Char('2') => {
                        self.mode = ViewMode::BeadDetail;
                        true
                    }
                    BareKey::Char('3') => {
                        self.mode = ViewMode::PipelineView;
                        self.load_pipeline_for_selected();
                        true
                    }
                    BareKey::Enter => {
                        // Enter key cycles through views
                        self.mode = match self.mode {
                            ViewMode::BeadList => ViewMode::BeadDetail,
                            ViewMode::BeadDetail => ViewMode::PipelineView,
                            ViewMode::PipelineView => ViewMode::BeadList,
                        };
                        if self.mode == ViewMode::PipelineView {
                            self.load_pipeline_for_selected();
                        }
                        true
                    }

                    // Refresh
                    BareKey::Char('r') => {
                        self.load_beads();
                        if self.mode == ViewMode::PipelineView {
                            self.load_pipeline_for_selected();
                        }
                        true
                    }

                    _ => false,
                }
            }
            Event::Timer(_) => {
                // Auto-refresh
                self.load_beads();
                set_timeout(2.0);
                true
            }
            Event::PermissionRequestResult(_) => {
                self.load_beads();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // Clear and reset
        print!("\x1b[2J\x1b[H");

        // Render header
        self.render_header(cols);

        // Render main content
        let content_rows = rows.saturating_sub(4);
        match self.mode {
            ViewMode::BeadList => self.render_bead_list(content_rows, cols),
            ViewMode::BeadDetail => self.render_bead_detail(content_rows, cols),
            ViewMode::PipelineView => self.render_pipeline_view(content_rows, cols),
        }

        // Render footer
        self.render_footer(rows, cols);
    }
}

impl State {
    fn load_beads(&mut self) {
        // TODO: Replace with actual HTTP call to oya-web API
        // For now, use realistic mock data
        self.beads = vec![
            BeadInfo {
                id: "src-1234".to_string(),
                title: "Implement user authentication with JWT".to_string(),
                status: BeadStatus::InProgress,
                current_stage: Some("unit-test".to_string()),
                progress: 0.6,
            },
            BeadInfo {
                id: "src-5678".to_string(),
                title: "Add database migration for user schema".to_string(),
                status: BeadStatus::Completed,
                current_stage: Some("accept".to_string()),
                progress: 1.0,
            },
            BeadInfo {
                id: "src-9012".to_string(),
                title: "Fix memory leak in event processing".to_string(),
                status: BeadStatus::Failed,
                current_stage: Some("integration".to_string()),
                progress: 0.7,
            },
            BeadInfo {
                id: "src-3456".to_string(),
                title: "Refactor DAG traversal algorithm".to_string(),
                status: BeadStatus::Pending,
                current_stage: None,
                progress: 0.0,
            },
        ];

        self.api_connected = true;
        self.last_error = None;
    }

    fn load_pipeline_for_selected(&mut self) {
        // TODO: Fetch pipeline stages for selected bead from API
        self.pipeline_stages = vec![
            StageInfo { name: "implement".to_string(), status: StageStatus::Passed, duration_ms: Some(1240) },
            StageInfo { name: "unit-test".to_string(), status: StageStatus::Passed, duration_ms: Some(3560) },
            StageInfo { name: "coverage".to_string(), status: StageStatus::Running, duration_ms: None },
            StageInfo { name: "lint".to_string(), status: StageStatus::Pending, duration_ms: None },
            StageInfo { name: "static".to_string(), status: StageStatus::Pending, duration_ms: None },
            StageInfo { name: "integration".to_string(), status: StageStatus::Pending, duration_ms: None },
            StageInfo { name: "security".to_string(), status: StageStatus::Pending, duration_ms: None },
            StageInfo { name: "review".to_string(), status: StageStatus::Pending, duration_ms: None },
            StageInfo { name: "accept".to_string(), status: StageStatus::Pending, duration_ms: None },
        ];
    }

    fn render_header(&self, cols: usize) {
        let title = "OYA Pipeline Dashboard";
        let status_symbol = if self.api_connected { "●" } else { "○" };
        let status_color = if self.api_connected { "\x1b[32m" } else { "\x1b[31m" };

        println!("\x1b[1m{}\x1b[0m{}{}{}\x1b[0m",
            title,
            " ".repeat(cols.saturating_sub(title.len() + 3)),
            status_color,
            status_symbol
        );
        println!("{}", "─".repeat(cols));
    }

    fn render_bead_list(&self, rows: usize, cols: usize) {
        if self.beads.is_empty() {
            println!("\n  \x1b[2mNo beads found. Create one with: oya new -s <slug>\x1b[0m");
            return;
        }

        // Header row
        println!("\n  \x1b[1m{:<12} {:<45} {:<12} {:<15} {}\x1b[0m",
            "ID", "Title", "Status", "Stage", "Progress"
        );
        println!("  {}", "─".repeat(cols.saturating_sub(2)));

        // Bead rows
        for (idx, bead) in self.beads.iter().take(rows.saturating_sub(3)).enumerate() {
            let selected = idx == self.selected_index;
            let prefix = if selected { "\x1b[7m> " } else { "  " };
            let suffix = if selected { "\x1b[0m" } else { "" };

            let title = truncate(&bead.title, 45);
            let stage = bead.current_stage.as_deref().unwrap_or("-");
            let progress_bar = render_progress_bar(bead.progress, 15);

            println!("{}{:<12} {:<45} {}{:<12}\x1b[0m {:<15} {}{}",
                prefix,
                bead.id,
                title,
                bead.status.color(),
                bead.status.as_str(),
                stage,
                progress_bar,
                suffix
            );
        }

        // Summary line
        let total = self.beads.len();
        let completed = self.beads.iter().filter(|b| matches!(b.status, BeadStatus::Completed)).count();
        let in_progress = self.beads.iter().filter(|b| matches!(b.status, BeadStatus::InProgress)).count();
        let failed = self.beads.iter().filter(|b| matches!(b.status, BeadStatus::Failed)).count();

        println!("\n  \x1b[2m{} total | {} completed | {} in progress | {} failed\x1b[0m",
            total, completed, in_progress, failed
        );
    }

    fn render_bead_detail(&self, _rows: usize, cols: usize) {
        if let Some(bead) = self.beads.get(self.selected_index) {
            println!("\n  \x1b[1mBead Details\x1b[0m");
            println!("  {}", "─".repeat(cols.saturating_sub(2)));
            println!();
            println!("  \x1b[1mID:\x1b[0m       {}", bead.id);
            println!("  \x1b[1mTitle:\x1b[0m    {}", bead.title);
            println!("  \x1b[1mStatus:\x1b[0m   {}{}\x1b[0m", bead.status.color(), bead.status.as_str());

            if let Some(ref stage) = bead.current_stage {
                println!("  \x1b[1mStage:\x1b[0m    {}", stage);
            }

            println!("  \x1b[1mProgress:\x1b[0m {}", render_progress_bar(bead.progress, 30));

            // Show workspace info
            println!();
            println!("  \x1b[1mWorkspace:\x1b[0m");
            println!("    Path:   ~/.local/share/jj/repos/oya/{}", bead.id);
            println!("    Branch: {}", bead.id);

            // Quick actions
            println!();
            println!("  \x1b[1mQuick Actions:\x1b[0m");
            println!("    \x1b[2mzjj spawn {}  # Open in isolated workspace\x1b[0m", bead.id);
            println!("    \x1b[2moya stage -s {} --stage <name>  # Run stage\x1b[0m", bead.id);
        } else {
            println!("\n  \x1b[2mNo bead selected\x1b[0m");
        }
    }

    fn render_pipeline_view(&self, rows: usize, cols: usize) {
        if let Some(bead) = self.beads.get(self.selected_index) {
            println!("\n  \x1b[1mPipeline Stages: {}\x1b[0m", bead.id);
            println!("  {}", "─".repeat(cols.saturating_sub(2)));
            println!();

            if self.pipeline_stages.is_empty() {
                println!("  \x1b[2mNo pipeline stages yet\x1b[0m");
                return;
            }

            // Visual pipeline flow
            println!("  Pipeline Flow:");
            for (idx, stage) in self.pipeline_stages.iter().take(rows.saturating_sub(8)).enumerate() {
                let symbol = stage.status.symbol();
                let color = stage.status.color();
                let connector = if idx < self.pipeline_stages.len() - 1 { "│" } else { " " };

                let duration_str = if let Some(ms) = stage.duration_ms {
                    format!("({:.1}s)", ms as f64 / 1000.0)
                } else {
                    "".to_string()
                };

                println!("  {} {}{}\x1b[0m {:<15} {}",
                    connector,
                    color,
                    symbol,
                    stage.name,
                    duration_str
                );
            }

            // Overall status
            let passed = self.pipeline_stages.iter().filter(|s| matches!(s.status, StageStatus::Passed)).count();
            let total = self.pipeline_stages.len();
            let progress = if total > 0 { passed as f32 / total as f32 } else { 0.0 };

            println!();
            println!("  Overall: {}/{} stages passed {}", passed, total, render_progress_bar(progress, 20));
        } else {
            println!("\n  \x1b[2mNo bead selected\x1b[0m");
        }
    }

    fn render_footer(&self, rows: usize, cols: usize) {
        // Position at bottom
        print!("\x1b[{};1H", rows - 1);

        let view_mode = match self.mode {
            ViewMode::BeadList => "List",
            ViewMode::BeadDetail => "Detail",
            ViewMode::PipelineView => "Pipeline",
        };

        println!("{}", "─".repeat(cols));

        let help = format!(
            "\x1b[2m[{}] 1:List 2:Detail 3:Pipeline | j/k:Navigate g/G:Top/Bottom Enter:Cycle r:Refresh q:Quit\x1b[0m",
            view_mode
        );

        // Show error if present
        if let Some(ref err) = self.last_error {
            println!("\x1b[31mError: {}\x1b[0m", truncate(err, cols.saturating_sub(7)));
        } else {
            println!("{}", help);
        }
    }
}

// Helper to check if we're in a specific view mode
impl PartialEq for ViewMode {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (ViewMode::BeadList, ViewMode::BeadList)
                | (ViewMode::BeadDetail, ViewMode::BeadDetail)
                | (ViewMode::PipelineView, ViewMode::PipelineView)
        )
    }
}

// Helper functions
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn render_progress_bar(progress: f32, width: usize) -> String {
    let filled = (progress * width as f32) as usize;
    let empty = width.saturating_sub(filled);

    format!("\x1b[32m{}\x1b[90m{}\x1b[0m {}%",
        "█".repeat(filled),
        "░".repeat(empty),
        (progress * 100.0) as u8
    )
}
