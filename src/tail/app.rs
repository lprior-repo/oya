//! TUI application state and event loop.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::parser::parse_invocation;
use super::restate::fetch_invocations_blocking;
use super::types::InvocationView;
use super::ui::{render_invocation, render_invocation_list};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::blocking::Client as BlockingClient;
use std::io::IsTerminal;
use std::io::Stdout;
use std::time::{Duration, Instant};

/// Application state for the tail TUI.
pub struct App {
    pub invocations: Vec<InvocationView>,
    pub selected: usize,
    pub focused: bool,
    pub running: bool,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
    pub run_id_filter: Option<String>,
    pub error: Option<String>,
}

impl App {
    pub fn new(refresh_interval_secs: u64, run_id: Option<String>) -> Self {
        Self {
            invocations: Vec::new(),
            selected: 0,
            focused: false,
            running: true,
            last_refresh: Instant::now() - Duration::from_secs(100), // Force immediate refresh
            refresh_interval: Duration::from_secs(refresh_interval_secs),
            run_id_filter: run_id,
            error: None,
        }
    }

    pub fn refresh(&mut self, client: &BlockingClient) {
        self.last_refresh = Instant::now();

        match fetch_invocations_blocking(client, 20) {
            Ok(rows) => {
                self.invocations = rows.iter().map(parse_invocation).collect();
                self.error = None;

                // Apply filter if set
                if let Some(ref run_id) = self.run_id_filter {
                    self.invocations.retain(|inv| inv.run_id.as_str().contains(run_id));
                }

                // Ensure selection is valid
                if self.selected >= self.invocations.len() && !self.invocations.is_empty() {
                    self.selected = self.invocations.len() - 1;
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.focused {
                    self.focused = false;
                } else {
                    self.running = false;
                }
            }
            KeyCode::Char('r') => {
                self.last_refresh = Instant::now() - Duration::from_secs(100);
            }
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected < self.invocations.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                if !self.invocations.is_empty() {
                    self.focused = true;
                }
            }
            _ => {}
        }
    }
}

/// Run the tail TUI.
pub fn run_tail(refresh_interval: u64, run_id: Option<String>) -> Result<()> {
    if !std::io::stdout().is_terminal() {
        return Err(anyhow::anyhow!(
            "tail TUI requires an interactive terminal; run without piping or use a TTY"
        ));
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and client
    let mut app = App::new(refresh_interval, run_id);
    let client = BlockingClient::new();

    // Initial data load
    app.refresh(&client);

    // Main loop
    let res = run_main_loop(&mut terminal, &mut app, &client);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    res
}

fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    client: &BlockingClient,
) -> Result<()> {
    loop {
        terminal.draw(|frame| draw_frame(frame, app))?;

        // Handle events
        let timeout = app.refresh_interval.saturating_sub(app.last_refresh.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key.code);
            }
        }

        // Auto-refresh
        if app.last_refresh.elapsed() >= app.refresh_interval {
            app.refresh(client);
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}

fn draw_frame(frame: &mut ratatui::Frame, app: &App) {
    let area = frame.area();
    if app.focused {
        if let Some(invocation) = app.invocations.get(app.selected) {
            render_invocation(frame, area, invocation);
        }
        return;
    }

    draw_list_view(frame, app, area);
}

fn draw_list_view(frame: &mut ratatui::Frame, app: &App, area: ratatui::layout::Rect) {
    render_invocation_list(frame, area, &app.invocations, app.selected);

    if let Some(error) = app.error.as_deref() {
        render_error_line(frame, area, error);
    }

    render_hint_line(frame, area);
}

fn render_error_line(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, error: &str) {
    let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
        format!("Error: {}", error),
        ratatui::style::Style::default().fg(ratatui::style::Color::Red),
    )]);
    let widget =
        ratatui::widgets::Paragraph::new(line).alignment(ratatui::layout::Alignment::Center);
    let error_area =
        ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(2), area.width, 1);
    frame.render_widget(widget, error_area);
}

fn render_hint_line(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
        "[enter] focus  [up/down] navigate  [r] refresh  [q] quit",
        ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
    )]);
    let widget =
        ratatui::widgets::Paragraph::new(line).alignment(ratatui::layout::Alignment::Center);
    let hint_area =
        ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    frame.render_widget(widget, hint_area);
}
