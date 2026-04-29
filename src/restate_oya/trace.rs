#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
use serde_json::Value;

use super::types::{OpenCodeTraceEvent, OpenCodeTraceSnapshot};

pub fn parse_jsonl_events(raw: &str) -> Result<Vec<Value>, serde_json::Error> {
    raw.lines().filter(|line| !line.trim().is_empty()).map(serde_json::from_str::<Value>).collect()
}

pub fn summarize_events(events: &[Value]) -> Value {
    let tool_calls = events
        .iter()
        .filter(|event| event.get("type") == Some(&Value::String("tool_use".to_owned())))
        .count();
    let final_text = events
        .iter()
        .rev()
        .find_map(|event| event.get("part")?.get("text")?.as_str())
        .map_or_else(String::new, |text| truncate_text(text, 500));
    serde_json::json!({
        "event_count": events.len(),
        "tool_calls": tool_calls,
        "final_text": final_text,
    })
}

pub fn fallback_summary(raw_output: &str) -> Value {
    serde_json::json!({
        "event_count": 0,
        "tool_calls": 0,
        "final_text": truncate_text(raw_output, 500),
        "parse_error": true,
    })
}

pub fn build_clean_trace(events: &[Value]) -> Value {
    let (_, entries) = events.iter().fold((0usize, Vec::new()), |(step, mut acc), event| {
        match trace_entry_pure(event, step) {
            Some((next_step, entry)) => {
                acc.push(entry);
                (next_step, acc)
            }
            None => (step, acc),
        }
    });
    Value::Array(entries)
}

pub fn empty_trace_snapshot(key: &str) -> OpenCodeTraceSnapshot {
    OpenCodeTraceSnapshot {
        bead_id: None,
        workflow_key: key.to_owned(),
        active_invocation_id: None,
        model: None,
        started_at: None,
        updated_at: None,
        finished_at: None,
        status: "not_started".to_owned(),
        current_event: None,
        events: Vec::new(),
        tool_call_count: 0,
        text_event_count: 0,
        last_error: None,
        summary: None,
    }
}

pub fn normalize_opencode_event(
    sequence: u64,
    received_at: String,
    raw: Value,
) -> OpenCodeTraceEvent {
    let kind = raw.get("type").and_then(Value::as_str).map_or("unknown", std::convert::identity);
    let part = raw.get("part").and_then(Value::as_object);
    let state = part.and_then(|value| value.get("state")).and_then(Value::as_object);
    let input = state.and_then(|value| value.get("input")).and_then(Value::as_object);
    OpenCodeTraceEvent {
        sequence,
        received_at,
        kind: kind.to_owned(),
        step: raw.get("step").and_then(Value::as_u64),
        tool: part.and_then(|value| value.get("tool")).and_then(Value::as_str).map(str::to_owned),
        description: input
            .and_then(|value| value.get("description"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        command: input
            .and_then(|value| value.get("command"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        query: input
            .and_then(|value| value.get("query"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        text: part
            .and_then(|value| value.get("text"))
            .and_then(Value::as_str)
            .map(|value| truncate_text(value, 1_000)),
        error: raw.get("error").and_then(Value::as_str).map(str::to_owned),
        raw,
    }
}

pub fn apply_trace_event(
    mut snapshot: OpenCodeTraceSnapshot,
    event: OpenCodeTraceEvent,
) -> OpenCodeTraceSnapshot {
    snapshot.updated_at = Some(event.received_at.clone());
    snapshot.current_event = Some(event.clone());
    snapshot.tool_call_count += u64::from(event.kind == "tool_use");
    snapshot.text_event_count += u64::from(event.kind == "text");
    snapshot.events.push(event);
    snapshot
}

pub fn finalize_trace(
    mut snapshot: OpenCodeTraceSnapshot,
    success: bool,
    finished_at: String,
    last_error: Option<String>,
    summary: Option<Value>,
) -> OpenCodeTraceSnapshot {
    snapshot.status = if success { "succeeded" } else { "failed" }.to_owned();
    snapshot.updated_at = Some(finished_at.clone());
    snapshot.finished_at = Some(finished_at);
    snapshot.last_error = last_error;
    snapshot.summary = summary;
    snapshot.active_invocation_id = None;
    snapshot
}

fn trace_entry_pure(event: &Value, step: usize) -> Option<(usize, Value)> {
    match event.get("type")?.as_str()? {
        "step_start" => {
            let next_step = step + 1;
            Some((
                next_step,
                serde_json::json!({
                    "step": next_step,
                    "kind": "step_start",
                    "timestamp": event.get("timestamp"),
                    "session_id": event.get("sessionID"),
                }),
            ))
        }
        "tool_use" => Some((step, tool_entry(event, step))),
        "text" => Some((step, text_entry(event, step))),
        "step_finish" => Some((step, finish_entry(event, step))),
        _ => None,
    }
}

fn tool_entry(event: &Value, step: usize) -> Value {
    let part = event.get("part").and_then(Value::as_object);
    let state = part.and_then(|value| value.get("state")).and_then(Value::as_object);
    let input = state.and_then(|value| value.get("input")).and_then(Value::as_object);
    let tool = part
        .and_then(|value| value.get("tool"))
        .and_then(Value::as_str)
        .map_or("unknown", std::convert::identity);
    serde_json::json!({
        "step": step,
        "kind": "tool_use",
        "tool": tool,
        "description": input.and_then(|value| value.get("description")).and_then(Value::as_str),
        "command": input.and_then(|value| value.get("command")).and_then(Value::as_str),
        "query": input.and_then(|value| value.get("query")).and_then(Value::as_str),
    })
}

fn text_entry(event: &Value, step: usize) -> Value {
    let text = event
        .get("part")
        .and_then(|value| value.get("text"))
        .and_then(Value::as_str)
        .map_or_else(String::new, |value| truncate_text(value, 500));
    serde_json::json!({
        "step": step,
        "kind": "text",
        "text": text,
    })
}

fn finish_entry(event: &Value, step: usize) -> Value {
    let part = event.get("part").and_then(Value::as_object);
    serde_json::json!({
        "step": step,
        "kind": "step_finish",
        "reason": part.and_then(|value| value.get("reason")).and_then(Value::as_str),
        "tokens": part.and_then(|value| value.get("tokens")).cloned(),
    })
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect::<String>()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        apply_trace_event, empty_trace_snapshot, finalize_trace, normalize_opencode_event,
    };

    #[test]
    fn empty_snapshot_marks_trace_not_started() {
        let snapshot = empty_trace_snapshot("bead-123");

        assert_eq!(snapshot.workflow_key, "bead-123");
        assert_eq!(snapshot.status, "not_started");
        assert!(snapshot.events.is_empty());
        assert_eq!(snapshot.tool_call_count, 0);
    }

    #[test]
    fn normalize_preserves_unknown_event_raw_json() {
        let raw = json!({ "message": "future opencode event" });

        let event = normalize_opencode_event(7, "2026-04-29T00:00:00Z".to_owned(), raw.clone());

        assert_eq!(event.sequence, 7);
        assert_eq!(event.kind, "unknown");
        assert_eq!(event.raw, raw);
    }

    #[test]
    fn normalize_extracts_tool_call_fields() {
        let raw = json!({
            "type": "tool_use",
            "step": 3,
            "part": {
                "tool": "bash",
                "state": {
                    "input": {
                        "description": "run test gate",
                        "command": "moon run :test",
                        "query": "unused"
                    }
                }
            }
        });

        let event = normalize_opencode_event(1, "now".to_owned(), raw);

        assert_eq!(event.kind, "tool_use");
        assert_eq!(event.step, Some(3));
        assert_eq!(event.tool.as_deref(), Some("bash"));
        assert_eq!(event.description.as_deref(), Some("run test gate"));
        assert_eq!(event.command.as_deref(), Some("moon run :test"));
        assert_eq!(event.query.as_deref(), Some("unused"));
    }

    #[test]
    fn apply_trace_event_updates_current_event_and_counts() {
        let snapshot = empty_trace_snapshot("bead-123");
        let tool = normalize_opencode_event(
            1,
            "2026-04-29T00:00:01Z".to_owned(),
            json!({ "type": "tool_use", "part": { "tool": "bash" } }),
        );
        let text = normalize_opencode_event(
            2,
            "2026-04-29T00:00:02Z".to_owned(),
            json!({ "type": "text", "part": { "text": "done" } }),
        );

        let snapshot = apply_trace_event(snapshot, tool);
        let snapshot = apply_trace_event(snapshot, text);

        assert_eq!(snapshot.events.len(), 2);
        assert_eq!(snapshot.tool_call_count, 1);
        assert_eq!(snapshot.text_event_count, 1);
        assert_eq!(snapshot.updated_at.as_deref(), Some("2026-04-29T00:00:02Z"));
        assert_eq!(snapshot.current_event.as_ref().map(|event| event.sequence), Some(2));
    }

    #[test]
    fn finalize_trace_records_terminal_state() {
        let mut snapshot = empty_trace_snapshot("bead-123");
        snapshot.active_invocation_id = Some("inv_123".to_owned());

        let snapshot = finalize_trace(
            snapshot,
            false,
            "2026-04-29T00:00:03Z".to_owned(),
            Some("model unavailable".to_owned()),
            Some(json!({ "tool_calls": 1 })),
        );

        assert_eq!(snapshot.status, "failed");
        assert_eq!(snapshot.finished_at.as_deref(), Some("2026-04-29T00:00:03Z"));
        assert_eq!(snapshot.last_error.as_deref(), Some("model unavailable"));
        assert!(snapshot.active_invocation_id.is_none());
        assert_eq!(snapshot.summary, Some(json!({ "tool_calls": 1 })));
    }
}
