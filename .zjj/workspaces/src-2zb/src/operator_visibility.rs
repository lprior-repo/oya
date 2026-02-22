#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusSnapshot {
    pub run_id: String,
    pub stage: String,
    pub attempt: u64,
    pub reason: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoctorIssue {
    pub run_id: String,
    pub reason_code: String,
    pub stage: String,
    pub age_seconds: u64,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoctorReport {
    pub stuck_runs: usize,
    pub cleanup_pending_backlog: usize,
    pub issues: Vec<DoctorIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanupReconcilePlan {
    pub prune_run_ids: Vec<String>,
}

pub(crate) fn run_status_command(run_id: Option<String>) -> Result<()> {
    let events = read_bridge_events()?;
    let snapshot = status_snapshot_from_events(&events, run_id.as_deref());
    println!("run_id={}", snapshot.run_id);
    println!("stage={}", snapshot.stage);
    println!("attempt={}", snapshot.attempt);
    println!("reason={}", snapshot.reason);
    println!("next_action={}", snapshot.next_action);
    Ok(())
}

pub(crate) fn run_doctor_command(stuck_after_seconds: u64) -> Result<()> {
    let events = read_bridge_events()?;
    let now = Utc::now().to_rfc3339();
    let report = doctor_report_from_events(&events, &now, stuck_after_seconds);
    println!("stuck_runs={}", report.stuck_runs);
    println!("cleanup_pending_backlog={}", report.cleanup_pending_backlog);
    for issue in report.issues {
        println!(
            "run_id={} reason_code={} stage={} age_seconds={} next_action={}",
            issue.run_id, issue.reason_code, issue.stage, issue.age_seconds, issue.next_action
        );
    }
    Ok(())
}

pub(crate) fn run_cleanup_reconciler_command(keep_latest: u64) -> Result<()> {
    let events = read_bridge_events()?;
    let repo_root = std::env::current_dir().context("resolve current directory")?;
    let keep_latest = usize::try_from(keep_latest).map_or(usize::MAX, |value| value);
    let plan = build_cleanup_reconcile_plan(&events, keep_latest);
    let removed = deterministic_prune_run_artifacts(&repo_root, &plan.prune_run_ids)?;
    println!("prune_candidates={}", plan.prune_run_ids.len());
    println!("pruned={}", removed.len());
    for run_id in removed {
        println!("pruned_run_id={}", run_id);
    }
    Ok(())
}

pub(crate) fn run_tail_events_command(
    run_id: Option<String>,
    limit: u64,
    follow: bool,
    interval_seconds: u64,
) -> Result<()> {
    let path = bridge_events_path()?;
    print_tail_window(&path, run_id.as_deref(), limit)?;
    if !follow {
        return Ok(());
    }
    follow_tail(path, run_id, interval_seconds)
}

pub(crate) fn status_snapshot_from_events(
    events: &[Value],
    run_id: Option<&str>,
) -> StatusSnapshot {
    let latest = latest_event(events, run_id);
    let selected = latest.cloned().unwrap_or(Value::Null);
    let selected_run_id =
        json_string(&selected, "run_id").or(run_id.map(std::borrow::ToOwned::to_owned));
    StatusSnapshot {
        run_id: selected_run_id.unwrap_or_else(|| "unknown".to_string()),
        stage: json_string(&selected, "stage").unwrap_or_else(|| "unknown".to_string()),
        attempt: json_u64(&selected, "attempt").unwrap_or(0),
        reason: extract_reason(&selected),
        next_action: extract_next_action(&selected),
    }
}

pub(crate) fn bounded_tail_events(
    events: &[Value],
    run_id: Option<&str>,
    limit: usize,
) -> Vec<Value> {
    let filtered =
        events.iter().filter(|event| run_id_match(event, run_id)).cloned().collect::<Vec<_>>();
    let start = filtered.len().saturating_sub(limit);
    filtered[start..].to_vec()
}

pub(crate) fn doctor_report_from_events(
    events: &[Value],
    now_rfc3339: &str,
    stuck_after_seconds: u64,
) -> DoctorReport {
    let latest = latest_event_per_run(events);
    let now = parse_timestamp(now_rfc3339).unwrap_or_else(Utc::now);
    let issues = latest
        .iter()
        .filter_map(|event| classify_issue(event, now, stuck_after_seconds))
        .collect::<Vec<_>>();
    let stuck_runs = issues.iter().filter(|issue| issue.reason_code == "stuck_running").count();
    let cleanup_pending_backlog =
        issues.iter().filter(|issue| issue.reason_code == "cleanup_pending").count();
    DoctorReport { stuck_runs, cleanup_pending_backlog, issues }
}

pub(crate) fn build_cleanup_reconcile_plan(
    events: &[Value],
    keep_latest: usize,
) -> CleanupReconcilePlan {
    let ordered = latest_event_per_run(events)
        .into_iter()
        .filter(is_cleanup_pending)
        .filter_map(|event| {
            json_string(&event, "run_id").map(|run_id| (event_timestamp_key(&event), run_id))
        })
        .sorted_by(|left, right| left.cmp(right))
        .collect::<Vec<_>>();
    let prune_count = ordered.len().saturating_sub(keep_latest);
    let prune_run_ids = ordered.into_iter().take(prune_count).map(|(_, run_id)| run_id).collect();
    CleanupReconcilePlan { prune_run_ids }
}

pub(crate) fn deterministic_prune_run_artifacts(
    repo_root: &Path,
    prune_run_ids: &[String],
) -> Result<Vec<String>> {
    let ordered = prune_run_ids.iter().cloned().collect::<BTreeSet<_>>();
    ordered.into_iter().try_fold(Vec::new(), |removed, run_id| {
        let artifact_path = repo_root.join(".orchestrator").join("runs").join(&run_id);
        if !artifact_path.exists() {
            return Ok(removed);
        }
        fs::remove_dir_all(&artifact_path)
            .with_context(|| format!("remove artifact directory {}", artifact_path.display()))
            .map(|()| removed.into_iter().chain(std::iter::once(run_id)).collect::<Vec<_>>())
    })
}

fn classify_issue(
    event: &Value,
    now: DateTime<Utc>,
    stuck_after_seconds: u64,
) -> Option<DoctorIssue> {
    if is_cleanup_pending(event) {
        let age = age_seconds(event, now);
        if age >= stuck_after_seconds {
            return Some(build_issue(
                event,
                now,
                "cleanup_retry_exhausted",
                "escalate cleanup reconciler and inspect artifact retention",
            ));
        }
        return Some(build_issue(event, now, "cleanup_pending", "run cleanup reconciler"));
    }
    let is_running = json_string(event, "status").as_deref() == Some("running");
    let age_seconds = age_seconds(event, now);
    if is_running && age_seconds >= stuck_after_seconds {
        return Some(build_issue(event, now, "stuck_running", "inspect stage logs and retry"));
    }
    None
}

fn build_issue(
    event: &Value,
    now: DateTime<Utc>,
    reason_code: &str,
    next_action: &str,
) -> DoctorIssue {
    DoctorIssue {
        run_id: json_string(event, "run_id").unwrap_or_else(|| "unknown".to_string()),
        reason_code: reason_code.to_string(),
        stage: json_string(event, "stage").unwrap_or_else(|| "unknown".to_string()),
        age_seconds: age_seconds(event, now),
        next_action: next_action.to_string(),
    }
}

fn latest_event<'a>(events: &'a [Value], run_id: Option<&str>) -> Option<&'a Value> {
    events.iter().rev().find(|event| run_id_match(event, run_id))
}

fn latest_event_per_run(events: &[Value]) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    let mut latest = Vec::new();
    for event in events.iter().rev() {
        let Some(run_id) = json_string(event, "run_id") else {
            continue;
        };
        if seen.insert(run_id) {
            latest.push(event.clone());
        }
    }
    latest.reverse();
    latest
}

fn run_id_match(event: &Value, run_id: Option<&str>) -> bool {
    match run_id {
        None => true,
        Some(needle) => json_string(event, "run_id").as_deref() == Some(needle),
    }
}

fn extract_reason(event: &Value) -> String {
    json_string(event, "reason")
        .or_else(|| json_string(event, "failure_category"))
        .unwrap_or_else(|| "none".to_string())
}

fn extract_next_action(event: &Value) -> String {
    json_string(event, "next_action").unwrap_or_else(|| {
        if is_cleanup_pending(event) {
            "run cleanup reconciler".to_string()
        } else {
            "inspect latest event".to_string()
        }
    })
}

fn is_cleanup_pending(event: &Value) -> bool {
    if event.get("cleanup_pending").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    json_string(event, "status").as_deref() == Some("cleanup_pending")
}

fn age_seconds(event: &Value, now: DateTime<Utc>) -> u64 {
    let timestamp = json_string(event, "at")
        .or_else(|| json_string(event, "updated_at"))
        .unwrap_or_else(|| now.to_rfc3339());
    parse_timestamp(&timestamp).map_or(0, |parsed| (now - parsed).num_seconds().max(0) as u64)
}

fn event_timestamp_key(event: &Value) -> String {
    json_string(event, "at")
        .or_else(|| json_string(event, "updated_at"))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value).ok().map(|ts| ts.with_timezone(&Utc))
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(std::borrow::ToOwned::to_owned)
}

fn json_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn bridge_events_path() -> Result<PathBuf> {
    let repo_root = std::env::current_dir().context("resolve current directory")?;
    Ok(repo_root.join(".oya").join("bridge").join("events.jsonl"))
}

fn read_bridge_events() -> Result<Vec<Value>> {
    let path = bridge_events_path()?;
    read_event_file(&path)
}

fn read_event_file(path: &Path) -> Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("read bridge events file {}", path.display()))?;
    content.lines().filter(|line| !line.trim().is_empty()).map(parse_event_line).collect()
}

fn parse_event_line(line: &str) -> Result<Value> {
    serde_json::from_str::<Value>(line).with_context(|| format!("parse event json: {}", line))
}

fn print_tail_window(path: &Path, run_id: Option<&str>, limit: u64) -> Result<()> {
    let events = read_event_file(path)?;
    let bounded = bounded_tail_events(&events, run_id, limit as usize);
    for event in bounded {
        println!("{}", serde_json::to_string(&event)?);
    }
    Ok(())
}

fn follow_tail(path: PathBuf, run_id: Option<String>, interval_seconds: u64) -> Result<()> {
    let mut seen = read_event_file(&path)?.len();
    loop {
        std::thread::sleep(Duration::from_secs(interval_seconds));
        let events = read_event_file(&path)?;
        if events.len() <= seen {
            continue;
        }
        let new_events = events[seen..].to_vec();
        seen = events.len();
        for event in bounded_tail_events(&new_events, run_id.as_deref(), new_events.len()) {
            println!("{}", serde_json::to_string(&event)?);
        }
    }
}
