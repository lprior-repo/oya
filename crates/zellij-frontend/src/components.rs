// Components module - UI components for OYA UI

/// Common styling constants
pub mod style {
    /// ANSI color codes
    pub const COLOR_RESET: &str = "\x1b[0m";
    pub const COLOR_RED: &str = "\x1b[31m";
    pub const COLOR_GREEN: &str = "\x1b[32m";

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

    /// Helper to colorize text
    #[must_use]
    pub fn colorize(text: &str, color: &str) -> String {
        format!("{}{}{}", color, text, COLOR_RESET)
    }
}
