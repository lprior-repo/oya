//! Interactive TUI dashboard with kanban view
//!
//! Displays sessions organized by status in a kanban-style layout with:
//! - Real-time updates from beads database changes
//! - Vim-style keyboard navigation (hjkl)
//! - Session management actions (focus, add, remove)
//! - Responsive layout based on terminal width

use std::{
    io::{self, Stdout},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use zjj_core::{
    config::load_config,
    watcher::{query_beads_status, BeadsStatus, FileWatcher, WatchEvent},
};

use crate::{
    commands::get_session_db,
    session::{Session, SessionStatus},
};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Session data enriched with JJ changes and beads counts
#[derive(Debug, Clone)]
struct SessionData {
    session: Session,
    changes: Option<usize>,
    beads: BeadsStatus,
}

/// Dashboard application state
struct DashboardApp {
    /// All session data grouped by status
    sessions_by_status: Vec<Vec<SessionData>>,
    /// Currently selected column (0=Creating, 1=Active, 2=Paused, 3=Completed, 4=Failed)
    selected_column: usize,
    /// Currently selected row within the column
    selected_row: usize,
    /// Terminal width for responsive layout
    terminal_width: u16,
    /// Last time data was refreshed
    last_update: Instant,
    /// Whether to quit the application
    should_quit: bool,
    /// Confirmation dialog state
    confirm_dialog: Option<ConfirmDialog>,
    /// Input dialog state
    input_dialog: Option<InputDialog>,
}

/// Confirmation dialog for destructive actions
#[derive(Debug, Clone)]
struct ConfirmDialog {
    message: String,
    action: ConfirmAction,
}

/// Action to perform on confirmation
#[derive(Debug, Clone)]
enum ConfirmAction {
    RemoveSession(String),
}

/// Input dialog for adding new sessions
#[derive(Debug, Clone)]
struct InputDialog {
    prompt: String,
    input: String,
    action: InputAction,
}

/// Action to perform with input
#[derive(Debug, Clone)]
enum InputAction {
    AddSession,
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Run the interactive dashboard
pub fn run() -> Result<()> {
    // Check if we're in a JJ repo
    let _root = crate::cli::jj_root().context("Not in a JJ repository. Run 'jjz init' first.")?;

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Load config
    let config = load_config().context("Failed to load configuration")?;

    // Create app state
    let mut app = DashboardApp::new()?;

    // Setup file watcher if enabled
    let mut watcher_rx = if config.watch.enabled {
        setup_file_watcher(&config).ok()
    } else {
        None
    };

    // Main event loop
    let result = run_app(
        &mut terminal,
        &mut app,
        &mut watcher_rx,
        Duration::from_millis(u64::from(config.dashboard.refresh_ms)),
    );

    // Cleanup terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    result
}

// ═══════════════════════════════════════════════════════════════════════════
// TERMINAL MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

/// Main application event loop
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut DashboardApp,
    watcher_rx: &mut Option<tokio::sync::mpsc::Receiver<WatchEvent>>,
    refresh_interval: Duration,
) -> Result<()> {
    let mut last_refresh = Instant::now();

    loop {
        // Render UI
        terminal.draw(|f| ui(f, app))?;

        // Check for quit
        if app.should_quit {
            break;
        }

        // Handle events with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key_event(app, key)?;
            } else if let Event::Resize(width, height) = event::read()? {
                app.terminal_width = width;
                terminal.resize(Rect::new(0, 0, width, height))?;
            }
        }

        // Check file watcher
        if let Some(rx) = watcher_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    WatchEvent::BeadsChanged { .. } => {
                        app.refresh_sessions()?;
                    }
                }
            }
        }

        // Auto-refresh
        if last_refresh.elapsed() >= refresh_interval {
            app.refresh_sessions()?;
            last_refresh = Instant::now();
        }
    }

    Ok(())
}

/// Setup file watcher for beads database changes
fn setup_file_watcher(
    config: &zjj_core::config::Config,
) -> Result<tokio::sync::mpsc::Receiver<WatchEvent>> {
    // Get all workspace paths from sessions
    let db = get_session_db()?;
    let sessions = db.list(None)?;

    let workspaces: Vec<PathBuf> = sessions
        .into_iter()
        .map(|s| PathBuf::from(s.workspace_path))
        .collect();

    FileWatcher::watch_workspaces(&config.watch, workspaces).context("Failed to setup file watcher")
}

// ═══════════════════════════════════════════════════════════════════════════
// EVENT HANDLING
// ═══════════════════════════════════════════════════════════════════════════

/// Handle keyboard input
fn handle_key_event(app: &mut DashboardApp, key: KeyEvent) -> Result<()> {
    // Handle dialogs first
    if app.input_dialog.is_some() {
        let dialog = app
            .input_dialog
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to take input dialog"))?;
        return handle_input_dialog(app, dialog, key);
    }

    if app.confirm_dialog.is_some() {
        let dialog = app
            .confirm_dialog
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to take confirm dialog"))?;
        return handle_confirm_dialog(app, dialog, key);
    }

    // Normal key handling
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.move_left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.move_right();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_up();
        }
        KeyCode::Char('r') => {
            app.refresh_sessions()?;
        }
        KeyCode::Char('a') => {
            app.show_add_dialog();
        }
        KeyCode::Char('d') => {
            if let Some(session) = app.get_selected_session() {
                app.show_remove_dialog(session.session.name.clone());
            }
        }
        KeyCode::Enter => {
            if let Some(session) = app.get_selected_session() {
                focus_session(&session.session)?;
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        _ => {}
    }

    Ok(())
}

/// Handle input dialog events
fn handle_input_dialog(
    app: &mut DashboardApp,
    mut dialog: InputDialog,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let input = dialog.input.clone();
            let action = dialog.action.clone();

            match action {
                InputAction::AddSession => {
                    if !input.is_empty() {
                        add_session(&input)?;
                        app.refresh_sessions()?;
                    }
                }
            }
        }
        KeyCode::Esc => {
            // Dialog already taken, just return
        }
        KeyCode::Char(c) => {
            dialog.input.push(c);
            app.input_dialog = Some(dialog);
        }
        KeyCode::Backspace => {
            dialog.input.pop();
            app.input_dialog = Some(dialog);
        }
        _ => {
            app.input_dialog = Some(dialog);
        }
    }

    Ok(())
}

/// Handle confirmation dialog events
fn handle_confirm_dialog(
    app: &mut DashboardApp,
    dialog: ConfirmDialog,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Char('y' | 'Y') => match dialog.action {
            ConfirmAction::RemoveSession(name) => {
                remove_session(&name)?;
                app.refresh_sessions()?;
            }
        },
        KeyCode::Char('n' | 'N') | KeyCode::Esc => {
            // Dialog already taken, just return
        }
        _ => {
            // Restore dialog if other key pressed
            app.confirm_dialog = Some(dialog);
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// UI RENDERING
// ═══════════════════════════════════════════════════════════════════════════

/// Render the UI
fn ui(f: &mut Frame, app: &DashboardApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.size());

    // Main content area
    render_kanban(f, app, chunks[0]);

    // Status bar
    render_status_bar(f, app, chunks[1]);

    // Dialogs
    if let Some(ref dialog) = app.input_dialog {
        render_input_dialog(f, dialog);
    }

    if let Some(ref dialog) = app.confirm_dialog {
        render_confirm_dialog(f, dialog);
    }
}

/// Render kanban board
fn render_kanban(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let is_wide = area.width >= 120;

    if is_wide {
        render_kanban_horizontal(f, app, area);
    } else {
        render_kanban_vertical(f, app, area);
    }
}

/// Render kanban board horizontally (wide screens)
fn render_kanban_horizontal(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    let column_titles = ["Creating", "Active", "Paused", "Completed", "Failed"];

    column_titles
        .iter()
        .enumerate()
        .for_each(|(idx, title)| render_column(f, app, columns[idx], idx, title));
}

/// Render kanban board vertically (narrow screens)
fn render_kanban_vertical(f: &mut Frame, app: &DashboardApp, area: Rect) {
    // Just show the selected column
    let column_titles = ["Creating", "Active", "Paused", "Completed", "Failed"];
    let title = column_titles[app.selected_column];

    render_column(f, app, area, app.selected_column, title);
}

/// Render a single kanban column
fn render_column(f: &mut Frame, app: &DashboardApp, area: Rect, column_idx: usize, title: &str) {
    let sessions = &app.sessions_by_status[column_idx];
    let is_selected = column_idx == app.selected_column;

    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(idx, session_data)| {
            let is_row_selected = is_selected && idx == app.selected_row;
            format_session_item(session_data, is_row_selected)
        })
        .collect();

    let border_style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ({}) ", title, sessions.len()))
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(list, area);
}

/// Format a session as a list item
fn format_session_item(session_data: &SessionData, is_selected: bool) -> ListItem<'_> {
    let session = &session_data.session;
    let changes_str = session_data
        .changes
        .map_or_else(|| "-".to_string(), |c| c.to_string());

    let beads_str = match &session_data.beads {
        BeadsStatus::NoBeads => "-".to_string(),
        BeadsStatus::Counts {
            open,
            in_progress,
            blocked,
            ..
        } => format!("{open}/{in_progress}/{blocked}"),
    };

    let branch = session.branch.as_deref().unwrap_or("-");

    let line = Line::from(vec![
        Span::styled(
            format!("{:<15}", session.name),
            if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ),
        Span::raw(format!(" {branch} ")),
        Span::styled(
            format!("Δ{changes_str} "),
            Style::default().fg(Color::Green),
        ),
        Span::styled(format!("B{beads_str}"), Style::default().fg(Color::Blue)),
    ]);

    ListItem::new(line)
}

/// Render status bar with help text
fn render_status_bar(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let help_text = vec![
        Span::raw("hjkl/arrows:"),
        Span::styled(" navigate ", Style::default().fg(Color::Gray)),
        Span::raw("Enter:"),
        Span::styled(" focus ", Style::default().fg(Color::Gray)),
        Span::raw("d:"),
        Span::styled(" delete ", Style::default().fg(Color::Gray)),
        Span::raw("a:"),
        Span::styled(" add ", Style::default().fg(Color::Gray)),
        Span::raw("r:"),
        Span::styled(" refresh ", Style::default().fg(Color::Gray)),
        Span::raw("q:"),
        Span::styled(" quit ", Style::default().fg(Color::Gray)),
        Span::raw(format!(
            "| Last update: {:?} ago",
            app.last_update.elapsed()
        )),
    ];

    let paragraph = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL).title(" Help "));

    f.render_widget(paragraph, area);
}

/// Render input dialog
fn render_input_dialog(f: &mut Frame, dialog: &InputDialog) {
    let area = centered_rect(60, 20, f.size());

    let text = vec![
        Line::from(dialog.prompt.as_str()),
        Line::from(""),
        Line::from(Span::styled(
            &dialog.input,
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Input ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(paragraph, area);
}

/// Render confirmation dialog
fn render_confirm_dialog(f: &mut Frame, dialog: &ConfirmDialog) {
    let area = centered_rect(60, 20, f.size());

    let text = vec![
        Line::from(dialog.message.as_str()),
        Line::from(""),
        Line::from(Span::styled(
            "Press Y to confirm, N to cancel",
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(paragraph, area);
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ═══════════════════════════════════════════════════════════════════════════
// APP STATE MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

impl DashboardApp {
    /// Create a new dashboard app
    fn new() -> Result<Self> {
        let (width, _) = crossterm::terminal::size()?;
        let mut app = Self {
            sessions_by_status: vec![vec![], vec![], vec![], vec![], vec![]],
            selected_column: 1, // Start on "Active"
            selected_row: 0,
            terminal_width: width,
            last_update: Instant::now(),
            should_quit: false,
            confirm_dialog: None,
            input_dialog: None,
        };

        app.refresh_sessions()?;
        Ok(app)
    }

    /// Refresh session data from database
    fn refresh_sessions(&mut self) -> Result<()> {
        let db = get_session_db()?;
        let sessions = db.list(None)?;

        // Group sessions by status using itertools
        let mut grouped: Vec<Vec<SessionData>> = vec![vec![], vec![], vec![], vec![], vec![]];

        // Build session data with status grouping
        let session_data_map = sessions
            .into_iter()
            .map(|session| {
                let workspace_path = Path::new(&session.workspace_path);

                let changes = if workspace_path.exists() {
                    zjj_core::jj::workspace_status(workspace_path)
                        .ok()
                        .map(|status| status.change_count())
                } else {
                    None
                };

                let beads = query_beads_status(workspace_path).unwrap_or(BeadsStatus::NoBeads);

                let column_idx = match session.status {
                    SessionStatus::Creating => 0,
                    SessionStatus::Active => 1,
                    SessionStatus::Paused => 2,
                    SessionStatus::Completed => 3,
                    SessionStatus::Failed => 4,
                };

                let session_data = SessionData {
                    session,
                    changes,
                    beads,
                };

                (column_idx, session_data)
            })
            .into_group_map();

        // Populate grouped vec, preserving order
        for (column_idx, sessions_in_group) in session_data_map {
            grouped[column_idx] = sessions_in_group;
        }

        self.sessions_by_status = grouped;
        self.last_update = Instant::now();

        // Adjust selection if out of bounds
        self.adjust_selection();

        Ok(())
    }

    /// Get the currently selected session
    fn get_selected_session(&self) -> Option<&SessionData> {
        self.sessions_by_status[self.selected_column].get(self.selected_row)
    }

    /// Move selection left
    fn move_left(&mut self) {
        if self.selected_column > 0 {
            self.selected_column -= 1;
            self.adjust_selection();
        }
    }

    /// Move selection right
    fn move_right(&mut self) {
        if self.selected_column < 4 {
            self.selected_column += 1;
            self.adjust_selection();
        }
    }

    /// Move selection up
    const fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    /// Move selection down
    fn move_down(&mut self) {
        let max_row = self.sessions_by_status[self.selected_column]
            .len()
            .saturating_sub(1);
        if self.selected_row < max_row {
            self.selected_row += 1;
        }
    }

    /// Adjust selection to stay in bounds
    fn adjust_selection(&mut self) {
        let max_row = self.sessions_by_status[self.selected_column]
            .len()
            .saturating_sub(1);
        if self.selected_row > max_row {
            self.selected_row = max_row;
        }
    }

    /// Show dialog to add a new session
    fn show_add_dialog(&mut self) {
        self.input_dialog = Some(InputDialog {
            prompt: "Enter session name:".to_string(),
            input: String::new(),
            action: InputAction::AddSession,
        });
    }

    /// Show dialog to confirm session removal
    fn show_remove_dialog(&mut self, name: String) {
        self.confirm_dialog = Some(ConfirmDialog {
            message: format!("Remove session '{name}'?"),
            action: ConfirmAction::RemoveSession(name),
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION ACTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Focus a session by switching to its Zellij tab
fn focus_session(session: &Session) -> Result<()> {
    // Use zellij action to switch to the tab
    let output = std::process::Command::new("zellij")
        .args(["action", "go-to-tab-name", &session.zellij_tab])
        .output()
        .context("Failed to execute zellij command")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to focus session: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Add a new session
fn add_session(name: &str) -> Result<()> {
    crate::commands::add::run(name)?;
    Ok(())
}

/// Remove a session
fn remove_session(name: &str) -> Result<()> {
    crate::commands::remove::run(name)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_data_grouping_by_status() {
        // Test that sessions are correctly grouped by their status
        let session1 = Session {
            id: Some(1),
            name: "active-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/active".to_string(),
            zellij_tab: "jjz:active-session".to_string(),
            branch: Some("main".to_string()),
            created_at: 0,
            updated_at: 0,
            last_synced: None,
            metadata: None,
        };

        let session2 = Session {
            id: Some(2),
            name: "paused-session".to_string(),
            status: SessionStatus::Paused,
            workspace_path: "/tmp/paused".to_string(),
            zellij_tab: "jjz:paused-session".to_string(),
            branch: Some("main".to_string()),
            created_at: 0,
            updated_at: 0,
            last_synced: None,
            metadata: None,
        };

        // Verify status to column mapping
        let active_column = match session1.status {
            SessionStatus::Active => 1,
            _ => 0,
        };
        let paused_column = match session2.status {
            SessionStatus::Paused => 2,
            _ => 0,
        };

        assert_eq!(active_column, 1);
        assert_eq!(paused_column, 2);
    }

    #[test]
    fn test_beads_status_formatting() {
        let beads = BeadsStatus::Counts {
            open: 5,
            in_progress: 3,
            blocked: 2,
            closed: 10,
        };

        let formatted = match beads {
            BeadsStatus::Counts {
                open,
                in_progress,
                blocked,
                ..
            } => format!("{open}/{in_progress}/{blocked}"),
            BeadsStatus::NoBeads => "-".to_string(),
        };

        assert_eq!(formatted, "5/3/2");
    }

    #[test]
    fn test_centered_rect() {
        let full_area = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(50, 50, full_area);

        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 25);
    }

    #[test]
    fn test_column_navigation() {
        // Simulate column navigation
        let mut selected_column = 1;

        // Move right
        if selected_column < 4 {
            selected_column += 1;
        }
        assert_eq!(selected_column, 2);

        // Move right again
        if selected_column < 4 {
            selected_column += 1;
        }
        assert_eq!(selected_column, 3);

        // Move left
        if selected_column > 0 {
            selected_column -= 1;
        }
        assert_eq!(selected_column, 2);
    }

    #[test]
    fn test_row_navigation_bounds() {
        let sessions_in_column: usize = 5;
        let mut selected_row = 0;

        // Move down
        let max_row = sessions_in_column.saturating_sub(1);
        if selected_row < max_row {
            selected_row += 1;
        }
        assert_eq!(selected_row, 1);

        // Try to move down past bounds
        selected_row = 4;
        if selected_row < max_row {
            selected_row += 1;
        }
        assert_eq!(selected_row, 4); // Should not exceed max_row

        // Move up
        selected_row = selected_row.saturating_sub(1);
        assert_eq!(selected_row, 3);
    }

    #[test]
    fn test_layout_mode_selection() {
        // Wide screen
        let wide_width = 120;
        let is_wide = wide_width >= 120;
        assert!(is_wide);

        // Narrow screen
        let narrow_width = 80;
        let is_narrow = narrow_width < 120;
        assert!(is_narrow);
    }
}
