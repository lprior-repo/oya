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
