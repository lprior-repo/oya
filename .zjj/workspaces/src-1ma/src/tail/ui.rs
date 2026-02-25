//! Ratatui UI rendering for tail TUI.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::parser::{format_age, format_duration};
use super::types::{GateState, InvocationState, InvocationView, StageName};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render a single invocation in focused view.
pub fn render_invocation(frame: &mut Frame, area: Rect, invocation: &InvocationView) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Run ID + status
            Constraint::Length(2), // Stage + attempt
            Constraint::Length(1), // Separator
            Constraint::Length(2), // Gates
            Constraint::Length(1), // Separator
            Constraint::Min(5),    // Output
            Constraint::Length(1), // Separator
            Constraint::Length(2), // Footer (time)
        ])
        .split(area);

    render_header(frame, chunks[0], invocation);
    render_stage_attempt(frame, chunks[1], invocation);
    render_separator(frame, chunks[2], area.width);
    render_gates(frame, chunks[3], invocation);
    render_separator(frame, chunks[4], area.width);
    render_output(frame, chunks[5], invocation);
    render_separator(frame, chunks[6], area.width);
    render_footer(frame, chunks[7], invocation);
}

fn render_header(frame: &mut Frame, area: Rect, invocation: &InvocationView) {
    let run_id_line = Line::from(vec![Span::styled(
        invocation.run_id.as_str(),
        Style::default().add_modifier(Modifier::BOLD),
    )]);
    let run_id = Paragraph::new(run_id_line).alignment(Alignment::Center);
    frame.render_widget(run_id, area);

    let status_color = state_color(invocation.state);
    let status_text = state_text(invocation.state);
    let status_line = Line::from(vec![Span::styled(
        status_text,
        Style::default().fg(status_color).add_modifier(Modifier::BOLD),
    )]);
    let status_widget = Paragraph::new(status_line).alignment(Alignment::Center);
    frame.render_widget(
        status_widget,
        area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
    );
}

fn render_stage_attempt(frame: &mut Frame, area: Rect, invocation: &InvocationView) {
    if let (Some(ref stage), Some(attempt)) = (&invocation.stage, invocation.attempt) {
        let max_att = stage.max_attempts();
        let stage_line = Line::from(vec![Span::styled(
            format!("{} {}/{}", stage.as_str(), attempt.value(), max_att),
            Style::default().fg(Color::Yellow),
        )]);
        let stage_widget = Paragraph::new(stage_line).alignment(Alignment::Center);
        frame.render_widget(stage_widget, area);
    }
}

fn render_separator(frame: &mut Frame, area: Rect, width: u16) {
    let separator = Paragraph::new(Line::from(Span::styled(
        separator_line(width),
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(separator, area);
}

fn render_gates(frame: &mut Frame, area: Rect, invocation: &InvocationView) {
    let gates_line = build_gates_line(&invocation.gates);
    let gates_widget = Paragraph::new(gates_line).alignment(Alignment::Center);
    frame.render_widget(gates_widget, area);
}

fn render_output(frame: &mut Frame, area: Rect, invocation: &InvocationView) {
    let output_lines: Vec<Line> = invocation
        .last_output_lines
        .iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Gray))))
        .collect();
    let output_widget =
        Paragraph::new(output_lines).alignment(Alignment::Left).wrap(Wrap { trim: false });
    frame.render_widget(output_widget, area);
}

fn render_footer(frame: &mut Frame, area: Rect, invocation: &InvocationView) {
    let footer_text = if invocation.state.is_running() {
        format!("running for {}", format_duration(invocation.age_seconds))
    } else if invocation.state.is_terminal() {
        format!("finished {} ago", format_age(invocation.age_seconds))
    } else {
        format!("modified {} ago", format_age(invocation.age_seconds))
    };
    let footer_line =
        Line::from(vec![Span::styled(footer_text, Style::default().fg(Color::DarkGray))]);
    let footer_widget = Paragraph::new(footer_line).alignment(Alignment::Center);
    frame.render_widget(footer_widget, area);
}

/// Render a list of invocations (compact view).
pub fn render_invocation_list(
    frame: &mut Frame,
    area: Rect,
    invocations: &[InvocationView],
    selected: usize,
) {
    let lines: Vec<Line> = invocations
        .iter()
        .enumerate()
        .map(|(i, inv)| {
            let status_col = state_color(inv.state);
            let status_text = short_state_text(inv.state);
            let stage = inv.stage.as_ref().map_or("-", StageName::as_str);
            let age = format_age(inv.age_seconds);

            let style =
                if i == selected { Style::default().bg(Color::DarkGray) } else { Style::default() };

            let bg_color = style.bg.map_or(Color::Reset, |c| c);

            Line::from(vec![
                Span::styled(format!("  {:<25} ", inv.run_id.as_str()), style),
                Span::styled(
                    format!("{:<10} ", status_text),
                    Style::default().fg(status_col).bg(bg_color),
                ),
                Span::styled(format!("{:<15} ", stage), style),
                Span::styled(
                    format!("{:>6}", age),
                    Style::default().fg(Color::DarkGray).bg(bg_color),
                ),
            ])
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title(" Oya Pipeline Invocations ");
    let list_widget = Paragraph::new(lines).block(block);
    frame.render_widget(list_widget, area);
}

fn separator_line(width: u16) -> String {
    "\u{2501}".repeat(width.saturating_sub(4) as usize)
}

/// Color based on explicit InvocationState - no option confusion.
fn state_color(state: InvocationState) -> Color {
    match state {
        InvocationState::Running => Color::Yellow,
        InvocationState::CompletedSuccess => Color::Green,
        InvocationState::CompletedFailure => Color::Red,
        InvocationState::Suspended => Color::Blue,
        InvocationState::Cancelled => Color::DarkGray,
        InvocationState::Unknown => Color::Gray,
    }
}

/// Text based on explicit InvocationState - exhaustive match.
fn state_text(state: InvocationState) -> String {
    match state {
        InvocationState::Running => "RUNNING".to_string(),
        InvocationState::CompletedSuccess => "COMPLETED".to_string(),
        InvocationState::CompletedFailure => "FAILED".to_string(),
        InvocationState::Suspended => "SUSPENDED".to_string(),
        InvocationState::Cancelled => "CANCELLED".to_string(),
        InvocationState::Unknown => "UNKNOWN".to_string(),
    }
}

/// Short text for list view - exhaustive match.
fn short_state_text(state: InvocationState) -> String {
    match state {
        InvocationState::Running => "running".to_string(),
        InvocationState::CompletedSuccess => "success".to_string(),
        InvocationState::CompletedFailure => "failed".to_string(),
        InvocationState::Suspended => "suspended".to_string(),
        InvocationState::Cancelled => "cancelled".to_string(),
        InvocationState::Unknown => "unknown".to_string(),
    }
}

/// Build gates display line using GateState.icon() - single source of truth.
fn build_gates_line(gates: &[super::types::GateView]) -> Line<'_> {
    let spans: Vec<Span> = gates
        .iter()
        .map(|gate| {
            let icon = gate.state.icon();
            let color = match gate.state {
                GateState::Passed => Color::Green,
                GateState::Failed => Color::Red,
                GateState::Running => Color::Yellow,
            };
            Span::styled(format!(" {} {} ", icon, gate.name.as_str()), Style::default().fg(color))
        })
        .collect();

    Line::from(spans)
}
