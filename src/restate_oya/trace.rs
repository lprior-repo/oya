use serde_json::Value;

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
        .map(|text| text.chars().take(500).collect::<String>())
        .map_or_else(String::new, std::convert::identity);
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
        .map(|value| truncate_text(value, 500))
        .map_or_else(String::new, std::convert::identity);
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
