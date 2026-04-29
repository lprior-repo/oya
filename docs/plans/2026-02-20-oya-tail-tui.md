# Oya Tail TUI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Ratatui-based TUI that displays live Restate pipeline invocations with enriched, human-friendly output.

**Architecture:** New `oya tail` CLI subcommand with its own module (`src/tail/`). Polls Restate SQL endpoint, parses nested JSON from invocations, enriches with stage/gate info, renders via Ratatui in a content-first, Apple-style layout.

**Tech Stack:** ratatui, crossterm, tokio, reqwest, serde_json, chrono

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml:1-54`

**Step 1: Add ratatui and crossterm to Cargo.toml**

```toml
# Add to [dependencies] section after "url = "2""
crossterm = "0.28"
ratatui = "0.29"
```

**Step 2: Verify dependencies compile**

Run: `moon run :check`
Expected: PASS (no compile errors)

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat(tail): add ratatui and crossterm dependencies"
```

---

## Task 2: Create Tail Module Structure

**Files:**
- Create: `src/tail/mod.rs`
- Create: `src/tail/types.rs`
- Modify: `src/main.rs:18-24`

**Step 1: Create src/tail/mod.rs**

```rust
//! Tail TUI module - live pipeline monitoring via Ratatui.

mod types;

pub use types::*;
```

**Step 2: Create src/tail/types.rs with core data types**

```rust
//! Data types for tail TUI.

use serde::{Deserialize, Serialize};

/// Enriched invocation data for display.
#[derive(Debug, Clone)]
pub struct InvocationView {
    pub run_id: String,
    pub target: String,
    pub status: InvocationStatus,
    pub result: Option<InvocationResult>,
    pub stage: Option<String>,
    pub attempt: Option<u32>,
    pub max_attempts: Option<u32>,
    pub gates: Vec<GateView>,
    pub last_output_lines: Vec<String>,
    pub error_summary: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub age_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationStatus {
    Running,
    Completed,
    Suspended,
    Cancelled,
    Unknown,
}

impl InvocationStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "completed" => Self::Completed,
            "suspended" => Self::Suspended,
            "cancelled" => Self::Cancelled,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationResult {
    Success,
    Failure,
}

impl InvocationResult {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "failure" => Some(Self::Failure),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateView {
    pub name: String,
    pub status: GateStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    Passed,
    Failed,
    Running,
    Pending,
}

/// Raw Restate sys_invocation row from SQL query.
#[derive(Debug, Clone, Deserialize)]
pub struct RestateInvocationRow {
    pub id: String,
    pub target: String,
    pub target_service_name: String,
    pub target_service_key: String,
    pub target_handler_name: String,
    pub status: String,
    pub completion_result: Option<String>,
    pub completion_failure: Option<String>,
    pub journal_size: Option<i64>,
    pub created_at: String,
    pub modified_at: String,
}

/// Response from Restate SQL query endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct RestateQueryResponse {
    pub rows: Vec<RestateInvocationRow>,
}
```

**Step 3: Add tail module to src/main.rs**

Find the module declarations (lines 18-24) and add `mod tail;`:

```rust
mod ops_poller;
mod orchestrator_types;
mod pipeline;
mod runtime_tools;
mod stage_executor;
mod stage_runtime;
mod tail;  // Add this line
mod workflow_runner;
```

**Step 4: Verify compilation**

Run: `moon run :check`
Expected: PASS

**Step 5: Commit**

```bash
git add src/tail/mod.rs src/tail/types.rs src/main.rs
git commit -m "feat(tail): add tail module with core data types"
```

---

## Task 3: Create Restate Query Client

**Files:**
- Create: `src/tail/restate.rs`
- Modify: `src/tail/mod.rs:1-4`

**Step 1: Create src/tail/restate.rs**

```rust
//! Restate SQL query client for fetching invocation data.

use super::types::{RestateInvocationRow, RestateQueryResponse};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

const DEFAULT_RESTATE_ADMIN_URL: &str = "http://127.0.0.1:9070";

/// Fetch OyaOrchestrator invocations from Restate.
pub async fn fetch_invocations(client: &Client, limit: usize) -> Result<Vec<RestateInvocationRow>> {
    let url = std::env::var("OYA_RESTATE_ADMIN_URL")
        .unwrap_or_else(|_| DEFAULT_RESTATE_ADMIN_URL.to_string());

    let query = format!(
        "SELECT id, target, target_service_name, target_service_key, target_handler_name, \
         status, completion_result, completion_failure, journal_size, created_at, modified_at \
         FROM sys_invocation \
         WHERE target_service_name = 'OyaOrchestrator' \
         ORDER BY modified_at DESC \
         LIMIT {};",
        limit
    );

    let response = client
        .post(format!("{}/query", url))
        .json(&json!({ "query": query }))
        .send()
        .await?
        .json::<RestateQueryResponse>()
        .await?;

    Ok(response.rows)
}

/// Fetch a specific invocation by run_id.
pub async fn fetch_invocation_by_id(
    client: &Client,
    run_id: &str,
) -> Result<Option<RestateInvocationRow>> {
    let url = std::env::var("OYA_RESTATE_ADMIN_URL")
        .unwrap_or_else(|_| DEFAULT_RESTATE_ADMIN_URL.to_string());

    let query = format!(
        "SELECT id, target, target_service_name, target_service_key, target_handler_name, \
         status, completion_result, completion_failure, journal_size, created_at, modified_at \
         FROM sys_invocation \
         WHERE target_service_name = 'OyaOrchestrator' \
         AND target_service_key = '{}' \
         ORDER BY modified_at DESC \
         LIMIT 1;",
        run_id
    );

    let response = client
        .post(format!("{}/query", url))
        .json(&json!({ "query": query }))
        .send()
        .await?
        .json::<RestateQueryResponse>()
        .await?;

    Ok(response.rows.into_iter().next())
}
```

**Step 2: Export restate module from mod.rs**

Update `src/tail/mod.rs`:

```rust
//! Tail TUI module - live pipeline monitoring via Ratatui.

mod restate;
mod types;

pub use restate::{fetch_invocation_by_id, fetch_invocations};
pub use types::*;
```

**Step 3: Verify compilation**

Run: `moon run :check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tail/restate.rs src/tail/mod.rs
git commit -m "feat(tail): add Restate SQL query client"
```

---

## Task 4: Create JSON Parser for Enriched Data

**Files:**
- Create: `src/tail/parser.rs`
- Modify: `src/tail/mod.rs`

**Step 1: Create src/tail/parser.rs**

```rust
//! Parse Restate invocation data into enriched view models.

use super::types::{
    GateStatus, GateView, InvocationResult, InvocationStatus, InvocationView, RestateInvocationRow,
};
use chrono::{DateTime, Utc};

/// Parse a Restate invocation row into an enriched view.
pub fn parse_invocation(row: &RestateInvocationRow) -> InvocationView {
    let status = InvocationStatus::from_str(&row.status);
    let result = row.completion_result.as_ref().and_then(InvocationResult::from_str);

    let (stage, attempt, gates, last_output, error_summary) =
        parse_completion_failure(row.completion_failure.as_deref());

    let age_seconds = calculate_age_seconds(&row.modified_at);

    InvocationView {
        run_id: row.target_service_key.clone(),
        target: row.target.clone(),
        status,
        result,
        stage,
        attempt,
        max_attempts: Some(2), // From StageName::max_attempts()
        gates,
        last_output_lines: last_output,
        error_summary,
        created_at: row.created_at.clone(),
        modified_at: row.modified_at.clone(),
        age_seconds,
    }
}

/// Parse the nested JSON from completion_failure field.
fn parse_completion_failure(
    failure: Option<&str>,
) -> (Option<String>, Option<u32>, Vec<GateView>, Vec<String>, Option<String>) {
    let Some(failure_str) = failure else {
        return (None, None, Vec::new(), Vec::new(), None);
    };

    // Try to parse as JSON
    let Ok(json) = serde_json::from_str::<serde_json::Value>(failure_str) else {
        // Not JSON, return as error summary
        return (None, None, Vec::new(), Vec::new(), Some(truncate_error(failure_str)));
    };

    // Extract stage from the mismatch message if present
    let stage = extract_stage_from_failure(&json);

    // Extract attempt (default to 1 if not found)
    let attempt = extract_attempt(&json);

    // Extract gates from the failure
    let gates = extract_gates(&json);

    // Extract last output lines
    let last_output = extract_output_lines(&json);

    // Extract error summary
    let error_summary = extract_error_summary(&json, failure_str);

    (stage, attempt, gates, last_output, error_summary)
}

fn extract_stage_from_failure(json: &serde_json::Value) -> Option<String> {
    // Look for stage in the failure message
    if let Some(obj) = json.as_object() {
        if let Some(stage) = obj.get("stage").and_then(|s| s.as_str()) {
            return Some(stage.to_string());
        }
    }
    None
}

fn extract_attempt(json: &serde_json::Value) -> Option<u32> {
    if let Some(obj) = json.as_object() {
        if let Some(attempt) = obj.get("attempt").and_then(|a| a.as_u64()) {
            return Some(attempt as u32);
        }
    }
    Some(1)
}

fn extract_gates(json: &serde_json::Value) -> Vec<GateView> {
    let mut gates = Vec::new();

    if let Some(obj) = json.as_object() {
        // Look for gates array
        if let Some(gates_arr) = obj.get("gates").and_then(|g| g.as_array()) {
            for gate in gates_arr {
                if let (Some(name), Some(passed)) = (
                    gate.get("gate").and_then(|g| g.as_str()),
                    gate.get("passed").and_then(|p| p.as_bool()),
                ) {
                    gates.push(GateView {
                        name: name.to_string(),
                        status: if passed { GateStatus::Passed } else { GateStatus::Failed },
                    });
                }
            }
        }

        // Also check for gate results in output parsing
        if let Some(output) = obj.get("output").and_then(|o| o.as_str()) {
            gates.extend(parse_gates_from_output(output));
        }
    }

    gates
}

fn parse_gates_from_output(output: &str) -> Vec<GateView> {
    let mut gates = Vec::new();

    // Parse gate results from output like "✅ check" or "❌ jj:sync"
    for line in output.lines() {
        let line = line.trim();
        if line.contains("✅") || line.contains("PASS") {
            if let Some(name) = extract_gate_name(line) {
                gates.push(GateView { name, status: GateStatus::Passed });
            }
        } else if line.contains("❌") || line.contains("FAIL") {
            if let Some(name) = extract_gate_name(line) {
                gates.push(GateView { name, status: GateStatus::Failed });
            }
        } else if line.contains("⏳") || line.contains("running") {
            if let Some(name) = extract_gate_name(line) {
                gates.push(GateView { name, status: GateStatus::Running });
            }
        }
    }

    gates
}

fn extract_gate_name(line: &str) -> Option<String> {
    // Extract gate name from lines like "✅ moon:check" or "oya:check (cached)"
    let cleaned = line
        .replace("✅", "")
        .replace("❌", "")
        .replace("⏳", "")
        .trim()
        .to_string();

    // Take first word/segment as gate name
    let name = cleaned.split_whitespace().next()?;
    Some(name.trim_end_matches(':').to_string())
}

fn extract_output_lines(json: &serde_json::Value) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(obj) = json.as_object() {
        if let Some(output) = obj.get("output").and_then(|o| o.as_str()) {
            // Take last 10 meaningful lines
            let output_lines: Vec<&str> = output.lines().collect();
            let start = output_lines.len().saturating_sub(10);
            for line in output_lines.iter().skip(start) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    lines.push(trimmed.to_string());
                }
            }
        }
    }

    lines
}

fn extract_error_summary(json: &serde_json::Value, raw: &str) -> Option<String> {
    // Check for explicit error message
    if let Some(obj) = json.as_object() {
        if let Some(error) = obj.get("error").and_then(|e| e.as_str()) {
            return Some(truncate_error(error));
        }
        if let Some(failure) = obj.get("failure").and_then(|f| f.as_str()) {
            return Some(truncate_error(failure));
        }
    }

    // Check for Restate error code pattern
    if raw.contains("[570]") {
        return Some("Non-deterministic execution mismatch".to_string());
    }

    None
}

fn truncate_error(s: &str) -> String {
    let max_len = 200;
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

fn calculate_age_seconds(modified_at: &str) -> u64 {
    let Ok(parsed) = DateTime::parse_from_rfc3339(modified_at) else {
        return 0;
    };
    let now = Utc::now();
    (now - parsed.with_timezone(&Utc)).num_seconds().max(0) as u64
}

/// Format age in human-readable form.
pub fn format_age(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h", seconds / 3600)
    }
}

/// Format duration in human-readable form.
pub fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
```

**Step 2: Export parser from mod.rs**

Update `src/tail/mod.rs`:

```rust
//! Tail TUI module - live pipeline monitoring via Ratatui.

mod parser;
mod restate;
mod types;

pub use parser::{format_age, format_duration, parse_invocation};
pub use restate::{fetch_invocation_by_id, fetch_invocations};
pub use types::*;
```

**Step 3: Verify compilation**

Run: `moon run :check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tail/parser.rs src/tail/mod.rs
git commit -m "feat(tail): add JSON parser for enriched invocation data"
```

---

## Task 5: Create Ratatui UI Renderer

**Files:**
- Create: `src/tail/ui.rs`
- Modify: `src/tail/mod.rs`

**Step 1: Create src/tail/ui.rs**

```rust
//! Ratatui UI rendering for tail TUI.

use super::types::{GateStatus, InvocationStatus, InvocationView};
use super::{format_age, format_duration};
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
            Constraint::Length(3),  // Run ID + status
            Constraint::Length(2),  // Stage + attempt
            Constraint::Length(1),  // Separator
            Constraint::Length(2),  // Gates
            Constraint::Length(1),  // Separator
            Constraint::Min(5),     // Output
            Constraint::Length(1),  // Separator
            Constraint::Length(2),  // Footer (time)
        ])
        .split(area);

    // Run ID centered
    let run_id_line = Line::from(vec![
        Span::styled(
            &invocation.run_id,
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);
    let run_id = Paragraph::new(run_id_line).alignment(Alignment::Center);
    frame.render_widget(run_id, chunks[0]);

    // Status badge
    let status_color = status_color(invocation.status, invocation.result);
    let status_text = status_text(invocation.status, invocation.result);
    let status_line = Line::from(vec![Span::styled(
        status_text,
        Style::default().fg(status_color).add_modifier(Modifier::BOLD),
    )]);
    let status_widget = Paragraph::new(status_line).alignment(Alignment::Center);
    frame.render_widget(status_widget, chunks[0].inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 0,
    }));

    // Stage + attempt
    if let (Some(stage), Some(attempt)) = (&invocation.stage, invocation.attempt) {
        let max_att = invocation.max_attempts.unwrap_or(2);
        let stage_line = Line::from(vec![Span::styled(
            format!("{} {}/{}", stage, attempt, max_att),
            Style::default().fg(Color::Yellow),
        )]);
        let stage_widget = Paragraph::new(stage_line).alignment(Alignment::Center);
        frame.render_widget(stage_widget, chunks[1]);
    }

    // Separator
    let sep = Paragraph::new(Line::from(Span::styled(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(sep, chunks[2]);

    // Gates
    let gates_line = build_gates_line(&invocation.gates);
    let gates_widget = Paragraph::new(gates_line).alignment(Alignment::Center);
    frame.render_widget(gates_widget, chunks[3]);

    // Separator
    let sep2 = Paragraph::new(Line::from(Span::styled(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(sep2, chunks[4]);

    // Output
    let output_lines: Vec<Line> = invocation
        .last_output_lines
        .iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Gray))))
        .collect();
    let output_widget = Paragraph::new(output_lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(output_widget, chunks[5]);

    // Separator
    let sep3 = Paragraph::new(Line::from(Span::styled(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(sep3, chunks[6]);

    // Footer: time info
    let age = format_age(invocation.age_seconds);
    let footer_text = match invocation.status {
        InvocationStatus::Running => format!("running for {}", format_duration(invocation.age_seconds)),
        InvocationStatus::Completed => format!("finished {} ago", age),
        _ => format!("modified {} ago", age),
    };
    let footer_line = Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    )]);
    let footer_widget = Paragraph::new(footer_line).alignment(Alignment::Center);
    frame.render_widget(footer_widget, chunks[7]);
}

/// Render a list of invocations (compact view).
pub fn render_invocation_list(frame: &mut Frame, area: Rect, invocations: &[InvocationView], selected: usize) {
    let lines: Vec<Line> = invocations
        .iter()
        .enumerate()
        .map(|(i, inv)| {
            let status_col = status_color(inv.status, inv.result);
            let status_text = short_status_text(inv.status, inv.result);
            let stage = inv.stage.as_deref().unwrap_or("-");
            let age = format_age(inv.age_seconds);

            let style = if i == selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Line::from(vec![
                Span::styled(format!("  {:<25} ", inv.run_id), style),
                Span::styled(format!("{:<10} ", status_text), Style::default().fg(status_col).bg(style.bg.unwrap_or(Color::Reset))),
                Span::styled(format!("{:<15} ", stage), style),
                Span::styled(format!("{:>6}", age), Style::default().fg(Color::DarkGray).bg(style.bg.unwrap_or(Color::Reset))),
            ])
        })
        .collect();

    let list_widget = Paragraph::new(lines);
    frame.render_widget(list_widget, area);
}

fn status_color(status: InvocationStatus, result: Option<super::types::InvocationResult>) -> Color {
    match status {
        InvocationStatus::Running => Color::Yellow,
        InvocationStatus::Completed => match result {
            Some(super::types::InvocationResult::Success) => Color::Green,
            Some(super::types::InvocationResult::Failure) => Color::Red,
            None => Color::Gray,
        },
        InvocationStatus::Suspended => Color::Blue,
        InvocationStatus::Cancelled => Color::DarkGray,
        InvocationStatus::Unknown => Color::Gray,
    }
}

fn status_text(status: InvocationStatus, result: Option<super::types::InvocationResult>) -> String {
    match status {
        InvocationStatus::Running => "RUNNING".to_string(),
        InvocationStatus::Completed => match result {
            Some(super::types::InvocationResult::Success) => "COMPLETED".to_string(),
            Some(super::types::InvocationResult::Failure) => "FAILED".to_string(),
            None => "COMPLETED".to_string(),
        },
        InvocationStatus::Suspended => "SUSPENDED".to_string(),
        InvocationStatus::Cancelled => "CANCELLED".to_string(),
        InvocationStatus::Unknown => "UNKNOWN".to_string(),
    }
}

fn short_status_text(status: InvocationStatus, result: Option<super::types::InvocationResult>) -> String {
    match status {
        InvocationStatus::Running => "running".to_string(),
        InvocationStatus::Completed => match result {
            Some(super::types::InvocationResult::Success) => "success".to_string(),
            Some(super::types::InvocationResult::Failure) => "failed".to_string(),
            None => "done".to_string(),
        },
        InvocationStatus::Suspended => "suspended".to_string(),
        InvocationStatus::Cancelled => "cancelled".to_string(),
        InvocationStatus::Unknown => "unknown".to_string(),
    }
}

fn build_gates_line(gates: &[super::types::GateView]) -> Line {
    let spans: Vec<Span> = gates
        .iter()
        .map(|gate| {
            let (icon, color) = match gate.status {
                GateStatus::Passed => ("✅", Color::Green),
                GateStatus::Failed => ("❌", Color::Red),
                GateStatus::Running => ("⏳", Color::Yellow),
                GateStatus::Pending => ("⏸️", Color::Gray),
            };
            Span::styled(format!(" {} {} ", icon, gate.name), Style::default().fg(color))
        })
        .collect();

    Line::from(spans)
}
```

**Step 2: Export ui from mod.rs**

Update `src/tail/mod.rs`:

```rust
//! Tail TUI module - live pipeline monitoring via Ratatui.

mod parser;
mod restate;
mod types;
mod ui;

pub use parser::{format_age, format_duration, parse_invocation};
pub use restate::{fetch_invocation_by_id, fetch_invocations};
pub use types::*;
pub use ui::{render_invocation, render_invocation_list};
```

**Step 3: Verify compilation**

Run: `moon run :check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tail/ui.rs src/tail/mod.rs
git commit -m "feat(tail): add Ratatui UI renderer"
```

---

## Task 6: Create TUI App with Event Loop

**Files:**
- Create: `src/tail/app.rs`
- Modify: `src/tail/mod.rs`

**Step 1: Create src/tail/app.rs**

```rust
//! TUI application state and event loop.

use super::types::InvocationView;
use super::{fetch_invocations, parse_invocation, render_invocation, render_invocation_list};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use reqwest::Client;
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

    pub fn tick(&mut self, client: &Client) {
        if self.last_refresh.elapsed() >= self.refresh_interval {
            self.refresh(client);
        }
    }

    pub fn refresh(&mut self, client: &Client) {
        self.last_refresh = Instant::now();

        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(async {
            let rows = fetch_invocations(client, 20).await?;
            Ok::<_, anyhow::Error>(rows)
        });

        match result {
            Ok(rows) => {
                self.invocations = rows.iter().map(parse_invocation).collect();
                self.error = None;

                // Apply filter if set
                if let Some(ref run_id) = self.run_id_filter {
                    self.invocations.retain(|inv| inv.run_id.contains(run_id));
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
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and client
    let mut app = App::new(refresh_interval, run_id);
    let client = Client::new();

    // Initial data load
    app.refresh(&client);

    // Main loop
    let res = run_main_loop(&mut terminal, &mut app, &client);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    client: &Client,
) -> Result<()> {
    loop {
        // Draw
        terminal.draw(|frame| {
            let area = frame.area();

            if app.focused {
                if let Some(invocation) = app.invocations.get(app.selected) {
                    render_invocation(frame, area, invocation);
                }
            } else {
                // List view
                render_invocation_list(frame, area, &app.invocations, app.selected);

                // Show error if any
                if let Some(ref error) = app.error {
                    let error_line = ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            format!("Error: {}", error),
                            ratatui::style::Style::default().fg(ratatui::style::Color::Red),
                        ),
                    ]);
                    let error_widget = ratatui::widgets::Paragraph::new(error_line)
                        .alignment(ratatui::layout::Alignment::Center);
                    let error_area = ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(2), area.width, 1);
                    frame.render_widget(error_widget, error_area);
                }

                // Keybinding hint
                let hint = ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        "[enter] focus  [↑↓] navigate  [r] refresh  [q] quit",
                        ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
                    ),
                ]);
                let hint_widget = ratatui::widgets::Paragraph::new(hint)
                    .alignment(ratatui::layout::Alignment::Center);
                let hint_area = ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
                frame.render_widget(hint_widget, hint_area);
            }
        })?;

        // Handle events
        let timeout = app.refresh_interval.saturating_sub(app.last_refresh.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key.code);
            }
        }

        // Auto-refresh
        app.tick(client);

        if !app.running {
            break;
        }
    }

    Ok(())
}
```

**Step 2: Export app from mod.rs**

Update `src/tail/mod.rs`:

```rust
//! Tail TUI module - live pipeline monitoring via Ratatui.

mod app;
mod parser;
mod restate;
mod types;
mod ui;

pub use app::{run_tail, App};
pub use parser::{format_age, format_duration, parse_invocation};
pub use restate::{fetch_invocation_by_id, fetch_invocations};
pub use types::*;
pub use ui::{render_invocation, render_invocation_list};
```

**Step 3: Verify compilation**

Run: `moon run :check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tail/app.rs src/tail/mod.rs
git commit -m "feat(tail): add TUI app with event loop"
```

---

## Task 7: Add Tail Subcommand to CLI

**Files:**
- Modify: `src/main.rs:357-406`

**Step 1: Add Tail subcommand enum variant**

Find the `CliCommand` enum (around line 364-372) and add `Tail`:

```rust
#[derive(Subcommand, Debug)]
enum CliCommand {
    #[command(about = "Run the Restate orchestrator server (default)")]
    Serve,
    #[command(about = "Continuously poll OpenCode status and stream to stdout")]
    OpsPoll,
    #[command(about = "Run a bead through the TDD15 pipeline via Restate")]
    Run(RunArgs),
    #[command(about = "Live TUI for monitoring pipeline invocations")]
    Tail(TailArgs),
}
```

**Step 2: Add TailArgs struct**

Add after `RunArgs` struct (around line 388):

```rust
#[derive(Parser, Debug, Clone, PartialEq)]
struct TailArgs {
    #[arg(long, default_value = "2", help = "Refresh interval in seconds")]
    interval: u64,
    #[arg(help = "Filter to specific run ID (optional)")]
    run_id: Option<String>,
}
```

**Step 3: Update CliMode enum**

Find the `CliMode` enum (around line 401-406) and add `Tail`:

```rust
#[derive(Debug, Clone, PartialEq)]
enum CliMode {
    Serve,
    OpsPoll,
    Run(RunArgs),
    Tail(TailArgs),
}
```

**Step 4: Update parse_cli_mode**

Find the `parse_cli_mode` function (around line 392-399) and add the `Tail` case:

```rust
fn parse_cli_mode() -> CliMode {
    let cli = Cli::parse();
    match cli.command {
        None | Some(CliCommand::Serve) => CliMode::Serve,
        Some(CliCommand::OpsPoll) => CliMode::OpsPoll,
        Some(CliCommand::Run(args)) => CliMode::Run(args),
        Some(CliCommand::Tail(args)) => CliMode::Tail(args),
    }
}
```

**Step 5: Update main match**

Find the `main` function's match on `mode` (around line 410-417) and add the `Tail` case:

```rust
match mode {
    CliMode::OpsPoll => ops_poller::run_ops_poller().await,
    CliMode::Serve => run_server().await,
    CliMode::Run(args) => workflow_runner::run_workflow(args).await,
    CliMode::Tail(args) => {
        // Need to enter tokio runtime for the tail app
        tail::run_tail(args.interval, args.run_id).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
```

**Step 6: Verify compilation**

Run: `moon run :check`
Expected: PASS

**Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat(tail): add tail subcommand to CLI"
```

---

## Task 8: Integration Test and Verification

**Files:**
- No new files

**Step 1: Build and verify**

Run: `moon run :build`
Expected: PASS

**Step 2: Run clippy**

Run: `moon run :check`
Expected: PASS (no clippy warnings)

**Step 3: Test CLI help**

Run: `cargo run -- tail --help`
Expected: Shows tail subcommand help with --interval and run_id options

**Step 4: Manual integration test**

Prerequisites: Restate running with at least one invocation

Run: `cargo run -- tail`
Expected: TUI opens showing invocations list

Press: `Enter` to focus on selected invocation
Press: `Esc` to return to list
Press: `q` to quit

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix(tail): address integration issues"
```

---

## Summary

This plan creates a complete `oya tail` TUI with:

1. **Dependencies** - ratatui + crossterm
2. **Types** - InvocationView, GateView, etc.
3. **Restate client** - SQL query for invocations
4. **Parser** - Enriches raw JSON into display models
5. **UI** - Apple-style content-first layout
6. **App** - Event loop with auto-refresh
7. **CLI** - `oya tail [--interval N] [run_id]`
8. **Integration** - Build, test, verify
