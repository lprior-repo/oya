//! UI component styling module
//!
//! Provides ANSI color codes and styling for terminal UI.

/// Normal border color (dim)
#[must_use]
pub const fn border_normal() -> &'static str {
    "\x1b[36m" // Cyan
}

/// Focused border color (bright)
#[must_use]
pub const fn border_focused() -> &'static str {
    "\x1b[96m" // Bright cyan
}

/// Reset color to default
#[must_use]
pub const fn reset() -> &'static str {
    "\x1b[0m"
}

/// Selected item color (highlighted)
#[must_use]
pub const fn selected() -> &'static str {
    "\x1b[7m" // Reverse video
}

/// Header text color
#[must_use]
pub const fn header() -> &'static str {
    "\x1b[1;33m" // Bold yellow
}

/// Label text color
#[must_use]
pub const fn label() -> &'static str {
    "\x1b[33m" // Yellow
}

/// Normal text color
#[must_use]
pub const fn text() -> &'static str {
    "\x1b[0m" // Reset
}

/// Overlay background color
#[must_use]
pub const fn overlay() -> &'static str {
    "\x1b[48;5;236m" // Dark gray background
}

/// Status color for open/ready items
#[must_use]
pub const fn status_open() -> &'static str {
    "\x1b[37m" // White
}

/// Status color for in-progress items
#[must_use]
pub const fn status_in_progress() -> &'static str {
    "\x1b[34m" // Blue
}

/// Status color for passed/completed items
#[must_use]
pub const fn status_passed() -> &'static str {
    "\x1b[32m" // Green
}

/// Status color for failed items
#[must_use]
pub const fn status_failed() -> &'static str {
    "\x1b[31m" // Red
}

/// Status color for blocked items
#[must_use]
pub const fn status_blocked() -> &'static str {
    "\x1b[33m" // Yellow
}

/// Get status color based on status string
#[must_use]
pub fn status_color(status: &str) -> &'static str {
    match status {
        "in_progress" | "running" => status_in_progress(),
        "passed" | "completed" | "integrated" => status_passed(),
        "failed" | "error" => status_failed(),
        "blocked" => status_blocked(),
        _ => status_open(),
    }
}
