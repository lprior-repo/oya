//! Timeline utilities - now using lean single-write model.
//!
//! Timeline is accumulated in-memory and written once at completion
//! instead of incremental appends via append_timeline().

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde_json::json;

/// Build a timeline entry for lean timeline construction.
///
/// This helper creates timeline entries that can be collected into
/// a single JSON array and written once at completion.
pub(super) fn build_timeline_entry(
    event: &str,
    stage: Option<&str>,
    attempt: Option<u32>,
    duration_ms: Option<u64>,
    at: &str,
) -> serde_json::Value {
    let mut entry = json!({
        "event": event,
        "at": at
    });

    if let Some(s) = stage {
        entry["stage"] = json!(s);
    }
    if let Some(a) = attempt {
        entry["attempt"] = json!(a);
    }
    if let Some(d) = duration_ms {
        entry["duration_ms"] = json!(d);
    }

    entry
}
