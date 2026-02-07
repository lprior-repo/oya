//! Bead detail view rendering with history tracking
//!
//! This module provides functionality for rendering detailed bead information
//! including state transition history with color-coded status badges.

use crate::BeadStatus;
use im::Vector;
use std::time::Instant;

/// History entry for tracking bead state transitions
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub timestamp: Instant,
    pub from_status: Option<BeadStatus>,
    pub to_status: BeadStatus,
    pub stage: Option<String>,
    pub note: Option<String>,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(
        timestamp: Instant,
        from_status: Option<BeadStatus>,
        to_status: BeadStatus,
    ) -> Self {
        Self {
            timestamp,
            from_status,
            to_status,
            stage: None,
            note: None,
        }
    }

    /// Add stage information to the history entry
    #[must_use]
    pub fn with_stage(mut self, stage: String) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Add a note to the history entry
    #[must_use]
    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    /// Format the history entry for display
    pub fn format(&self, cols: usize) -> String {
        let age = format_history_age(self.timestamp);
        let status_arrow = match &self.from_status {
            Some(from) => format!("{}{} → ", from.color(), from.as_str()),
            None => String::new(),
        };

        let status_badge = format!(
            "{}{}\x1b[0m",
            self.to_status.color(),
            self.to_status.as_str()
        );

        let stage_info = self
            .stage
            .as_ref()
            .map(|s| format!(" [@ {}]", truncate(s, 20)))
            .unwrap_or_default();

        let note_info = self
            .note
            .as_ref()
            .map(|n| format!(" - {}", truncate(n, cols.saturating_sub(50))))
            .unwrap_or_default();

        format!(
            "  {:>4} │ {}{}{}{}",
            age, status_arrow, status_badge, stage_info, note_info
        )
    }
}

/// Extended bead information with history
#[derive(Clone, Debug)]
pub struct BeadDetail {
    pub id: String,
    pub title: String,
    pub status: BeadStatus,
    pub current_stage: Option<String>,
    pub progress: f32,
    pub history: Vector<HistoryEntry>,
}

impl BeadDetail {
    /// Create a new bead detail
    pub fn new(
        id: String,
        title: String,
        status: BeadStatus,
        progress: f32,
    ) -> Self {
        Self {
            id,
            title,
            status,
            current_stage: None,
            progress,
            history: Vector::new(),
        }
    }

    /// Set the current stage
    #[must_use]
    pub fn with_stage(mut self, stage: String) -> Self {
        self.current_stage = Some(stage);
        self
    }

    /// Add a history entry
    #[must_use]
    pub fn with_history_entry(mut self, entry: HistoryEntry) -> Self {
        self.history.push_back(entry);
        self
    }

    /// Add initial history entry (creation)
    #[must_use]
    pub fn with_initial_history(mut self) -> Self {
        let entry = HistoryEntry::new(Instant::now(), None, self.status)
            .with_note("Created".to_string());
        self.history.push_back(entry);
        self
    }

    /// Record a status transition
    #[must_use]
    pub fn record_transition(mut self, new_status: BeadStatus, note: Option<String>) -> Self {
        let entry = HistoryEntry::new(Instant::now(), Some(self.status), new_status)
            .with_stage(
                self.current_stage
                    .clone()
                    .unwrap_or_else(|| String::from("unknown")),
            );
        let entry = if let Some(note_text) = note {
            entry.with_note(note_text)
        } else {
            entry
        };
        self.history.push_back(entry);
        self.status = new_status;
        self
    }

    /// Render the bead detail with history section
    pub fn render(&self, rows: usize, cols: usize) {
        // Render basic bead info
        self.render_header(cols);
        self.render_basic_info();
        self.render_workspace_info();
        self.render_progress();

        // Calculate available rows for history
        let used_rows = 8; // Header, basic info, workspace, progress, spacing
        let history_rows = rows.saturating_sub(used_rows).max(5);

        // Render history section
        self.render_history_section(history_rows, cols);

        // Render quick actions
        self.render_quick_actions();
    }

    fn render_header(&self, cols: usize) {
        println!("\n  \x1b[1mBead Details\x1b[0m");
        println!("  {}", "─".repeat(cols.saturating_sub(2)));
        println!();
    }

    fn render_basic_info(&self) {
        println!("  \x1b[1mID:\x1b[0m       {}", self.id);
        println!("  \x1b[1mTitle:\x1b[0m    {}", self.title);
        println!(
            "  \x1b[1mStatus:\x1b[0m   {}{}\x1b[0m",
            self.status.color(),
            self.status.as_str()
        );

        if let Some(stage) = self.current_stage.as_ref() {
            println!("  \x1b[1mStage:\x1b[0m    {}", stage);
        }
    }

    fn render_workspace_info(&self) {
        println!();
        println!("  \x1b[1mWorkspace:\x1b[0m");
        println!("    Path:   ~/.local/share/jj/repos/oya/{}", self.id);
        println!("    Branch: {}", self.id);
    }

    fn render_progress(&self) {
        println!();
        println!(
            "  \x1b[1mProgress:\x1b[0m {}",
            render_progress_bar(self.progress, 30)
        );
    }

    fn render_history_section(&self, rows: usize, cols: usize) {
        println!();
        println!("  \x1b[1mHistory:\x1b[0m");
        println!("  {}", "─".repeat(cols.saturating_sub(2)));
        println!();

        if self.history.is_empty() {
            println!("  \x1b[2mNo history available\x1b[0m");
            return;
        }

        // Show legend
        println!(
            "  \x1b[1mLegend:\x1b[0m {}○\x1b[0m Pending {}◐\x1b[0m In Progress {}●\x1b[0m Completed {}✗\x1b[0m Failed",
            BeadStatus::Pending.color(),
            BeadStatus::InProgress.color(),
            BeadStatus::Completed.color(),
            BeadStatus::Failed.color(),
        );
        println!();

        // Render history entries (most recent first)
        println!("  Age  │ Status Transition");
        println!("  {}", "─".repeat(cols.saturating_sub(4)));

        self.history
            .iter()
            .rev()
            .take(rows.saturating_sub(3))
            .for_each(|entry| {
                println!("{}", entry.format(cols));
            });

        if self.history.len() > rows.saturating_sub(3) {
            println!(
                "  \x1b[2m... and {} more entries\x1b[0m",
                self.history.len().saturating_sub(rows.saturating_sub(3))
            );
        }
    }

    fn render_quick_actions(&self) {
        println!();
        println!("  \x1b[1mQuick Actions:\x1b[0m");
        println!(
            "    \x1b[2mzjj spawn {}  # Open in isolated workspace\x1b[0m",
            self.id
        );
        println!(
            "    \x1b[2moya stage -s {} --stage <name>  # Run stage\x1b[0m",
            self.id
        );
    }
}

/// Format the age of a history entry
fn format_history_age(timestamp: Instant) -> String {
    let elapsed = timestamp.elapsed();
    let secs = elapsed.as_secs();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs.saturating_div(60))
    } else if secs < 86400 {
        format!("{}h", secs.saturating_div(3600))
    } else {
        format!("{}d", secs.saturating_div(86400))
    }
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Render a progress bar
fn render_progress_bar(progress: f32, width: usize) -> String {
    let clamped = progress.clamp(0.0, 1.0);
    let filled = (clamped * width as f32).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);
    let percent = (clamped * 100.0).round() as u8;

    format!(
        "\x1b[32m{}\x1b[90m{}\x1b[0m {}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percent
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_bead_detail() -> BeadDetail {
        let mut bead = BeadDetail::new(
            "test-bead".to_string(),
            "Test Bead".to_string(),
            BeadStatus::Pending,
            0.0,
        )
        .with_initial_history()
        .with_stage("implement".to_string());

        // Add some history entries
        bead = bead.record_transition(
            BeadStatus::InProgress,
            Some("Started implementation".to_string()),
        );
        bead = bead.record_transition(
            BeadStatus::Completed,
            Some("All stages passed".to_string()),
        );

        bead
    }

    #[test]
    fn test_bead_detail_creation() {
        let bead = BeadDetail::new(
            "test-id".to_string(),
            "Test Title".to_string(),
            BeadStatus::Pending,
            0.5,
        );

        assert_eq!(bead.id, "test-id");
        assert_eq!(bead.title, "Test Title");
        assert_eq!(bead.status, BeadStatus::Pending);
        assert_eq!(bead.progress, 0.5);
        assert!(bead.history.is_empty());
    }

    #[test]
    fn test_bead_detail_with_stage() {
        let bead = BeadDetail::new(
            "test-id".to_string(),
            "Test".to_string(),
            BeadStatus::Pending,
            0.0,
        )
        .with_stage("implement".to_string());

        assert_eq!(bead.current_stage, Some("implement".to_string()));
    }

    #[test]
    fn test_bead_detail_with_initial_history() {
        let bead = BeadDetail::new(
            "test-id".to_string(),
            "Test".to_string(),
            BeadStatus::Pending,
            0.0,
        )
        .with_initial_history();

        assert_eq!(bead.history.len(), 1);
        let entry = bead.history.front().unwrap();
        assert!(entry.note.as_ref().is_some_and(|n| n.contains("Created")));
    }

    #[test]
    fn test_bead_detail_record_transition() {
        let bead = BeadDetail::new(
            "test-id".to_string(),
            "Test".to_string(),
            BeadStatus::Pending,
            0.0,
        )
        .with_initial_history();

        let bead = bead.record_transition(
            BeadStatus::InProgress,
            Some("Started work".to_string()),
        );

        assert_eq!(bead.status, BeadStatus::InProgress);
        assert_eq!(bead.history.len(), 2);

        let latest_entry = bead.history.back().unwrap();
        assert_eq!(latest_entry.to_status, BeadStatus::InProgress);
        assert!(latest_entry.note.as_ref().is_some_and(|n| n.contains("Started work")));
    }

    #[test]
    fn test_history_entry_formatting() {
        let entry = HistoryEntry::new(
            Instant::now(),
            Some(BeadStatus::Pending),
            BeadStatus::InProgress,
        )
        .with_stage("implement".to_string())
        .with_note("Test note".to_string());

        let formatted = entry.format(80);

        assert!(formatted.contains("→"));
        assert!(formatted.contains("in_progress"));
        assert!(formatted.contains("implement"));
        assert!(formatted.contains("Test note"));
    }

    #[test]
    fn test_history_entry_without_previous_status() {
        let entry = HistoryEntry::new(Instant::now(), None, BeadStatus::Pending)
            .with_note("Initial state".to_string());

        let formatted = entry.format(80);

        // Should not contain arrow since there's no previous status
        assert!(!formatted.contains("→"));
        assert!(formatted.contains("pending"));
        assert!(formatted.contains("Initial state"));
    }

    #[test]
    fn test_bead_status_colors() {
        assert_eq!(BeadStatus::Pending.as_str(), "pending");
        assert_eq!(BeadStatus::InProgress.as_str(), "in_progress");
        assert_eq!(BeadStatus::Completed.as_str(), "completed");
        assert_eq!(BeadStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_bead_status_symbols() {
        assert_eq!(BeadStatus::Pending.symbol(), "○");
        assert_eq!(BeadStatus::InProgress.symbol(), "◐");
        assert_eq!(BeadStatus::Completed.symbol(), "●");
        assert_eq!(BeadStatus::Failed.symbol(), "✗");
    }

    #[test]
    fn test_format_history_age() {
        // Test seconds
        let age = format_history_age(Instant::now());
        assert!(age.ends_with('s') || age.ends_with('m'));

        // The actual value depends on timing, so we just check it's not empty
        assert!(!age.is_empty());
    }

    #[test]
    fn test_truncate_function() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("test", 4), "test");
        assert_eq!(truncate("testing", 4), "t...");
    }

    #[test]
    fn test_render_progress_bar() {
        let bar = render_progress_bar(0.5, 10);
        assert!(bar.contains("%"));
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));

        // Test clamping
        let bar_full = render_progress_bar(1.5, 10);
        assert!(bar_full.contains("100%"));

        let bar_empty = render_progress_bar(-0.5, 10);
        assert!(bar_empty.contains("0%"));
    }

    #[test]
    fn test_bead_detail_with_multiple_transitions() {
        let bead = build_bead_detail();

        assert_eq!(bead.history.len(), 3); // Initial + 2 transitions

        // Verify history order (most recent should be last)
        let entries: Vec<_> = bead.history.iter().collect();
        assert_eq!(entries[0].to_status, BeadStatus::Pending); // Initial
        assert_eq!(entries[1].to_status, BeadStatus::InProgress);
        assert_eq!(entries[2].to_status, BeadStatus::Completed); // Most recent
    }

    #[test]
    fn test_history_entry_with_stage_and_note() {
        let entry = HistoryEntry::new(
            Instant::now(),
            Some(BeadStatus::InProgress),
            BeadStatus::Completed,
        )
        .with_stage("lint".to_string())
        .with_note("All checks passed".to_string());

        assert_eq!(entry.stage, Some("lint".to_string()));
        assert_eq!(entry.note, Some("All checks passed".to_string()));
        assert_eq!(entry.from_status, Some(BeadStatus::InProgress));
        assert_eq!(entry.to_status, BeadStatus::Completed);
    }

    #[test]
    fn test_bead_detail_complete_lifecycle() {
        let bead = BeadDetail::new(
            "lifecycle-test".to_string(),
            "Lifecycle Test".to_string(),
            BeadStatus::Pending,
            0.0,
        )
        .with_initial_history()
        .with_stage("implement".to_string());

        assert_eq!(bead.status, BeadStatus::Pending);

        let bead = bead.record_transition(BeadStatus::InProgress, None);
        assert_eq!(bead.status, BeadStatus::InProgress);

        let bead = bead.record_transition(BeadStatus::Completed, Some("Done".to_string()));
        assert_eq!(bead.status, BeadStatus::Completed);

        // Should have 3 history entries: initial + 2 transitions
        assert_eq!(bead.history.len(), 3);
    }

    #[test]
    fn test_history_entry_format_with_long_note() {
        let entry = HistoryEntry::new(
            Instant::now(),
            Some(BeadStatus::Pending),
            BeadStatus::InProgress,
        )
        .with_note("This is a very long note that should be truncated when displayed".to_string());

        let formatted = entry.format(60);
        // Note should be truncated with "..."
        assert!(formatted.contains("..."));
    }

    #[test]
    fn test_bead_status_color_codes() {
        // Verify color codes are ANSI escape sequences
        let pending_color = BeadStatus::Pending.color();
        assert!(pending_color.starts_with("\x1b["));

        let in_progress_color = BeadStatus::InProgress.color();
        assert!(in_progress_color.starts_with("\x1b["));

        let completed_color = BeadStatus::Completed.color();
        assert!(completed_color.starts_with("\x1b["));

        let failed_color = BeadStatus::Failed.color();
        assert!(failed_color.starts_with("\x1b["));
    }
}
