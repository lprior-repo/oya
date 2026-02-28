#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{Effect, EffectJournalEntry};
use serde_json::{json, Value};

pub fn step_details(entry: &EffectJournalEntry) -> Option<Value> {
    match &entry.effect {
        Effect::Opencode { .. } | Effect::OpencodeQa { .. } => Some(json!({
            "events": parse_json_lines(&entry.stdout),
            "receipt": parse_first_object(&entry.stdout),
            "timeout_secs": entry.timeout_secs,
            "success": entry.success,
            "stderr": entry.stderr,
        })),
        _ => None,
    }
}

fn parse_json_lines(raw: &str) -> Vec<Value> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn parse_first_object(raw: &str) -> Option<Value> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(Value::is_object)
}
