// Components module - UI components for OYA UI

/// Common styling constants
pub mod style {
    /// ANSI color codes
    pub const COLOR_RESET: &str = "\x1b[0m";
    pub const COLOR_RED: &str = "\x1b[31m";
    pub const COLOR_GREEN: &str = "\x1b[32m";

    /// Helper to colorize text
    #[must_use]
    pub fn colorize(text: &str, color: &str) -> String {
        format!("{}{}{}", color, text, COLOR_RESET)
    }
}
