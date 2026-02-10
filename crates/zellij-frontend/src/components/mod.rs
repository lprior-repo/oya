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
