use super::super::*;

fn first_non_empty_line_after_marker<'a>(message: &'a str, marker: &str) -> Option<&'a str> {
    let mut marker_seen = false;
    for line in message.lines() {
        if marker_seen {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        if line.trim_start().starts_with(marker) {
            marker_seen = true;
        }
    }
    None
}

pub(crate) fn summarize_failure_output(category: &FailureCategory, message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "No error output captured.".to_string();
    }
    if matches!(category, FailureCategory::OutputParseFailure) {
        if trimmed.contains("Command timed out after") {
            let timeout_line = trimmed
                .lines()
                .find(|line| line.contains("Command timed out after"))
                .map(str::trim)
                .unwrap_or("Command timed out after unknown duration");
            let stderr_preview = first_non_empty_line_after_marker(trimmed, "stderr:")
                .map(|line| truncate_clean(line, 180));
            let stdout_preview = first_non_empty_line_after_marker(trimmed, "stdout:")
                .map(|line| truncate_clean(line, 180));
            let details = match stderr_preview {
                Some(line) => format!("stderr: {}", line),
                None => match stdout_preview {
                    Some(line) => format!("stdout: {}", line),
                    None => "No stdout/stderr preview available.".to_string(),
                },
            };
            return format!(
                "{}\n{}\nKeep fixes narrowly scoped so the next run completes within timeout.",
                timeout_line, details
            );
        }

        return "Previous attempt failed with output_parse_failure. Emit concise, deterministic output and avoid raw tool-event streams or partial JSON payloads.".to_string();
    }
    truncate_clean(trimmed, 1200)
}
