//! Health Status Indicators for Agent Monitoring
//!
//! This module provides visual health status indicators for agents in the Zellij UI.
//! Health states include: Healthy, Unhealthy, and Unknown.
//!
//! # Design Principles
//!
//! - **Zero panics**: All functions return safe defaults instead of panicking
//! - **Functional patterns**: Pure functions with no hidden mutations
//! - **Clear visual indicators**: Color-coded symbols for terminal UI
//! - **Real-time updates**: Support for dynamic health state changes

use std::fmt;

/// Health status of an agent or system component
///
/// Three-state health model with clear visual indicators:
/// - **Healthy**: Component operating normally
/// - **Unhealthy**: Component experiencing issues
/// - **Unknown**: Health status cannot be determined
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthStatus {
    /// Component is operating normally
    Healthy,
    /// Component is experiencing issues or degraded performance
    Unhealthy,
    /// Health status cannot be determined
    Unknown,
}

impl HealthStatus {
    /// Get the color code for terminal display
    ///
    /// Returns ANSI escape codes for:
    /// - Healthy: Green
    /// - Unhealthy: Red
    /// - Unknown: Gray
    pub fn color(&self) -> &str {
        match self {
            Self::Healthy => "\x1b[32m",    // green
            Self::Unhealthy => "\x1b[31m",  // red
            Self::Unknown => "\x1b[90m",    // gray
        }
    }

    /// Get the symbol character for visual indicator
    ///
    /// Returns:
    /// - Healthy: ● (filled circle)
    /// - Unhealthy: ✗ (cross mark)
    /// - Unknown: ? (question mark)
    pub fn symbol(&self) -> &str {
        match self {
            Self::Healthy => "●",
            Self::Unhealthy => "✗",
            Self::Unknown => "?",
        }
    }

    /// Get the text label for the health status
    pub fn label(&self) -> &str {
        match self {
            Self::Healthy => "healthy",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }

    /// Create a health status from a health score (0.0 to 1.0)
    ///
    /// # Arguments
    ///
    /// * `score` - Health score between 0.0 and 1.0
    ///
    /// # Returns
    ///
    /// - `Healthy` if score >= 0.8
    /// - `Unhealthy` if score < 0.5
    /// - `Unknown` if score is NaN or outside valid range
    pub fn from_score(score: f64) -> Self {
        if score.is_nan() || !(0.0..=1.0).contains(&score) {
            return Self::Unknown;
        }

        if score >= 0.8 {
            Self::Healthy
        } else if score < 0.5 {
            Self::Unhealthy
        } else {
            Self::Unknown
        }
    }

    /// Format health status for display with color and symbol
    ///
    /// # Example
    ///
    /// ```
    /// # use oya_zellij::ui::health_indicators::HealthStatus;
    /// let status = HealthStatus::Healthy;
    /// assert_eq!(
    ///     status.format(),
    ///     "\x1b[32m●\x1b[0m \x1b[32mhealthy\x1b[0m"
    /// );
    /// ```
    pub fn format(&self) -> String {
        format!("{}{}\x1b[0m {}{}\x1b[0m", self.color(), self.symbol(), self.color(), self.label())
    }

    /// Format health status as a compact indicator (symbol only)
    pub fn format_compact(&self) -> String {
        format!("{}{}\x1b[0m", self.color(), self.symbol())
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Health indicator configuration for rendering
#[derive(Clone, Debug)]
pub struct HealthIndicatorConfig {
    /// Show color in output
    pub show_color: bool,
    /// Show symbols in output
    pub show_symbol: bool,
    /// Show text label in output
    pub show_label: bool,
    /// Custom width for text labels (0 = no truncation)
    pub label_width: usize,
}

impl Default for HealthIndicatorConfig {
    fn default() -> Self {
        Self {
            show_color: true,
            show_symbol: true,
            show_label: true,
            label_width: 0,
        }
    }
}

impl HealthIndicatorConfig {
    /// Create a compact configuration (symbol only, no label)
    pub fn compact() -> Self {
        Self {
            show_color: true,
            show_symbol: true,
            show_label: false,
            label_width: 0,
        }
    }

    /// Create a detailed configuration (color, symbol, and full label)
    pub fn detailed() -> Self {
        Self {
            show_color: true,
            show_symbol: true,
            show_label: true,
            label_width: 0,
        }
    }

    /// Create a minimal configuration (no color, text only)
    pub fn minimal() -> Self {
        Self {
            show_color: false,
            show_symbol: false,
            show_label: true,
            label_width: 0,
        }
    }

    /// Create a fixed-width configuration for table display
    pub fn fixed_width(width: usize) -> Self {
        Self {
            show_color: true,
            show_symbol: true,
            show_label: true,
            label_width: width,
        }
    }
}

/// Format a health status with custom configuration
///
/// # Arguments
///
/// * `status` - The health status to format
/// * `config` - Configuration for display options
///
/// # Example
///
/// ```
/// # use oya_zellij::ui::health_indicators::{HealthStatus, format_health};
/// let status = HealthStatus::Healthy;
/// let output = format_health(status, &HealthIndicatorConfig::compact());
/// assert!(output.contains("●"));
/// ```
pub fn format_health(status: HealthStatus, config: &HealthIndicatorConfig) -> String {
    let color = if config.show_color { status.color() } else { "" };
    let symbol = if config.show_symbol { status.symbol() } else { "" };
    let reset = if config.show_color { "\x1b[0m" } else { "" };

    let label = if config.show_label {
        let label_text = status.label();
        if config.label_width > 0 && label_text.len() > config.label_width {
            format!("{}...", &label_text[..config.label_width.saturating_sub(3)])
        } else {
            label_text.to_string()
        }
    } else {
        String::new()
    };

    let separator = if config.show_symbol && config.show_label && !symbol.is_empty() {
        " "
    } else {
        ""
    };

    format!(
        "{}{}{}{}{}{}",
        color, symbol, reset, separator, color, label
    )
}

/// Calculate overall health from multiple health scores
///
/// Returns the average health, or `Unknown` if the list is empty
/// or contains invalid values.
///
/// # Arguments
///
/// * `scores` - Slice of health scores (0.0 to 1.0)
///
/// # Returns
///
/// `HealthStatus` based on the average score
pub fn overall_health(scores: &[f64]) -> HealthStatus {
    if scores.is_empty() {
        return HealthStatus::Unknown;
    }

    let valid_scores: Vec<f64> = scores
        .iter()
        .filter(|&&s| !s.is_nan() && (0.0..=1.0).contains(&s))
        .copied()
        .collect();

    if valid_scores.is_empty() {
        return HealthStatus::Unknown;
    }

    let average = valid_scores.iter().sum::<f64>() / valid_scores.len() as f64;
    HealthStatus::from_score(average)
}

/// Health change event for tracking state transitions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthChangeEvent {
    /// Previous health status
    pub previous: HealthStatus,
    /// New health status
    pub current: HealthStatus,
    /// Timestamp of the change
    pub changed_at: std::time::Instant,
}

impl HealthChangeEvent {
    /// Create a new health change event
    pub fn new(previous: HealthStatus, current: HealthStatus) -> Self {
        Self {
            previous,
            current,
            changed_at: std::time::Instant::now(),
        }
    }

    /// Check if this is a degradation (healthy -> unhealthy)
    pub fn is_degradation(&self) -> bool {
        matches!(
            (self.previous, self.current),
            (HealthStatus::Healthy, HealthStatus::Unhealthy)
        )
    }

    /// Check if this is an improvement (unhealthy -> healthy)
    pub fn is_improvement(&self) -> bool {
        matches!(
            (self.previous, self.current),
            (HealthStatus::Unhealthy, HealthStatus::Healthy)
        )
    }

    /// Check if this is a status change to/from unknown
    pub fn involves_unknown(&self) -> bool {
        matches!(self.previous, HealthStatus::Unknown) || matches!(self.current, HealthStatus::Unknown)
    }
}

/// Track health changes and generate events
#[derive(Clone, Debug)]
pub struct HealthTracker {
    current: HealthStatus,
    history: Vec<HealthChangeEvent>,
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self {
            current: HealthStatus::Unknown,
            history: Vec::new(),
        }
    }
}

impl HealthTracker {
    /// Create a new health tracker with an initial status
    pub fn new(initial: HealthStatus) -> Self {
        Self {
            current: initial,
            history: Vec::new(),
        }
    }

    /// Update the health status and record change if different
    ///
    /// Returns `Some(HealthChangeEvent)` if the status changed, `None` otherwise
    pub fn update(&mut self, new_status: HealthStatus) -> Option<HealthChangeEvent> {
        if self.current == new_status {
            return None;
        }

        let event = HealthChangeEvent::new(self.current, new_status);
        self.current = new_status;
        self.history.push(event.clone());

        // Limit history to last 100 events
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        Some(event)
    }

    /// Get the current health status
    pub fn current(&self) -> HealthStatus {
        self.current
    }

    /// Get the history of health changes
    pub fn history(&self) -> &[HealthChangeEvent] {
        &self.history
    }

    /// Count how many times health has degraded
    pub fn degradation_count(&self) -> usize {
        self.history.iter().filter(|e| e.is_degradation()).count()
    }

    /// Count how many times health has improved
    pub fn improvement_count(&self) -> usize {
        self.history.iter().filter(|e| e.is_improvement()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to strip ANSI codes for comparison
    fn strip_ansi(s: &str) -> String {
        s.replace("\x1b[32m", "")
            .replace("\x1b[31m", "")
            .replace("\x1b[90m", "")
            .replace("\x1b[0m", "")
    }

    #[test]
    fn test_health_status_colors() {
        assert_eq!(HealthStatus::Healthy.color(), "\x1b[32m");
        assert_eq!(HealthStatus::Unhealthy.color(), "\x1b[31m");
        assert_eq!(HealthStatus::Unknown.color(), "\x1b[90m");
    }

    #[test]
    fn test_health_status_symbols() {
        assert_eq!(HealthStatus::Healthy.symbol(), "●");
        assert_eq!(HealthStatus::Unhealthy.symbol(), "✗");
        assert_eq!(HealthStatus::Unknown.symbol(), "?");
    }

    #[test]
    fn test_health_status_labels() {
        assert_eq!(HealthStatus::Healthy.label(), "healthy");
        assert_eq!(HealthStatus::Unhealthy.label(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.label(), "unknown");
    }

    #[test]
    fn test_health_status_from_score() {
        // High scores should be healthy
        assert_eq!(HealthStatus::from_score(1.0), HealthStatus::Healthy);
        assert_eq!(HealthStatus::from_score(0.9), HealthStatus::Healthy);
        assert_eq!(HealthStatus::from_score(0.8), HealthStatus::Healthy);

        // Medium scores should be unknown
        assert_eq!(HealthStatus::from_score(0.7), HealthStatus::Unknown);
        assert_eq!(HealthStatus::from_score(0.5), HealthStatus::Unknown);

        // Low scores should be unhealthy
        assert_eq!(HealthStatus::from_score(0.4), HealthStatus::Unhealthy);
        assert_eq!(HealthStatus::from_score(0.0), HealthStatus::Unhealthy);

        // NaN and out-of-range should be unknown
        assert_eq!(HealthStatus::from_score(f64::NAN), HealthStatus::Unknown);
        assert_eq!(HealthStatus::from_score(-0.1), HealthStatus::Unknown);
        assert_eq!(HealthStatus::from_score(1.1), HealthStatus::Unknown);
    }

    #[test]
    fn test_health_status_format() {
        let healthy = HealthStatus::Healthy;
        let formatted = healthy.format();
        assert!(formatted.contains("●"));
        assert!(formatted.contains("healthy"));

        let stripped = strip_ansi(&formatted);
        assert_eq!(stripped, "● healthy");
    }

    #[test]
    fn test_health_status_format_compact() {
        let healthy = HealthStatus::Healthy;
        let formatted = healthy.format_compact();
        assert!(formatted.contains("●"));

        let stripped = strip_ansi(&formatted);
        assert_eq!(stripped, "●");
    }

    #[test]
    fn test_health_status_default() {
        let status = HealthStatus::default();
        assert_eq!(status, HealthStatus::Unknown);
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
        assert_eq!(format!("{}", HealthStatus::Unknown), "unknown");
    }

    #[test]
    fn test_health_indicator_config_default() {
        let config = HealthIndicatorConfig::default();
        assert!(config.show_color);
        assert!(config.show_symbol);
        assert!(config.show_label);
        assert_eq!(config.label_width, 0);
    }

    #[test]
    fn test_health_indicator_config_compact() {
        let config = HealthIndicatorConfig::compact();
        assert!(config.show_color);
        assert!(config.show_symbol);
        assert!(!config.show_label);
    }

    #[test]
    fn test_health_indicator_config_minimal() {
        let config = HealthIndicatorConfig::minimal();
        assert!(!config.show_color);
        assert!(!config.show_symbol);
        assert!(config.show_label);
    }

    #[test]
    fn test_format_health_compact() {
        let config = HealthIndicatorConfig::compact();
        let output = format_health(HealthStatus::Healthy, &config);

        let stripped = strip_ansi(&output);
        assert_eq!(stripped, "●");
    }

    #[test]
    fn test_format_health_detailed() {
        let config = HealthIndicatorConfig::detailed();
        let output = format_health(HealthStatus::Healthy, &config);

        let stripped = strip_ansi(&output);
        assert_eq!(stripped, "● healthy");
    }

    #[test]
    fn test_format_health_minimal() {
        let config = HealthIndicatorConfig::minimal();
        let output = format_health(HealthStatus::Healthy, &config);

        let stripped = strip_ansi(&output);
        assert_eq!(stripped, "healthy");
    }

    #[test]
    fn test_format_health_fixed_width() {
        let config = HealthIndicatorConfig::fixed_width(5);
        let output = format_health(HealthStatus::Unhealthy, &config);

        let stripped = strip_ansi(&output);
        // "unhealthy" is 8 chars, should be truncated to "unhe..."
        assert_eq!(stripped, "✗ unhe...");
    }

    #[test]
    fn test_overall_health_empty() {
        let scores: Vec<f64> = vec![];
        assert_eq!(overall_health(&scores), HealthStatus::Unknown);
    }

    #[test]
    fn test_overall_health_all_healthy() {
        let scores = vec![0.9, 0.95, 0.85];
        assert_eq!(overall_health(&scores), HealthStatus::Healthy);
    }

    #[test]
    fn test_overall_health_all_unhealthy() {
        let scores = vec![0.1, 0.2, 0.3];
        assert_eq!(overall_health(&scores), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_overall_health_mixed() {
        let scores = vec![0.9, 0.6, 0.4];
        // Average is 0.63, which is unknown territory (0.5-0.8)
        assert_eq!(overall_health(&scores), HealthStatus::Unknown);
    }

    #[test]
    fn test_overall_health_with_nan() {
        let scores = vec![0.9, f64::NAN, 0.8];
        // Should ignore NaN and calculate average of 0.9 and 0.8 = 0.85
        assert_eq!(overall_health(&scores), HealthStatus::Healthy);
    }

    #[test]
    fn test_overall_health_all_nan() {
        let scores = vec![f64::NAN, f64::NAN];
        assert_eq!(overall_health(&scores), HealthStatus::Unknown);
    }

    #[test]
    fn test_health_change_event_degradation() {
        let event = HealthChangeEvent::new(HealthStatus::Healthy, HealthStatus::Unhealthy);
        assert!(event.is_degradation());
        assert!(!event.is_improvement());
    }

    #[test]
    fn test_health_change_event_improvement() {
        let event = HealthChangeEvent::new(HealthStatus::Unhealthy, HealthStatus::Healthy);
        assert!(event.is_improvement());
        assert!(!event.is_degradation());
    }

    #[test]
    fn test_health_change_event_involves_unknown() {
        let event1 = HealthChangeEvent::new(HealthStatus::Unknown, HealthStatus::Healthy);
        assert!(event1.involves_unknown());

        let event2 = HealthChangeEvent::new(HealthStatus::Healthy, HealthStatus::Unknown);
        assert!(event2.involves_unknown());

        let event3 = HealthChangeEvent::new(HealthStatus::Healthy, HealthStatus::Unhealthy);
        assert!(!event3.involves_unknown());
    }

    #[test]
    fn test_health_tracker_default() {
        let tracker = HealthTracker::default();
        assert_eq!(tracker.current(), HealthStatus::Unknown);
        assert!(tracker.history().is_empty());
    }

    #[test]
    fn test_health_tracker_with_initial() {
        let tracker = HealthTracker::new(HealthStatus::Healthy);
        assert_eq!(tracker.current(), HealthStatus::Healthy);
        assert!(tracker.history().is_empty());
    }

    #[test]
    fn test_health_tracker_update_same_status() {
        let mut tracker = HealthTracker::new(HealthStatus::Healthy);
        let event = tracker.update(HealthStatus::Healthy);

        assert!(event.is_none());
        assert_eq!(tracker.current(), HealthStatus::Healthy);
        assert!(tracker.history().is_empty());
    }

    #[test]
    fn test_health_tracker_update_different_status() {
        let mut tracker = HealthTracker::new(HealthStatus::Healthy);
        let event = tracker.update(HealthStatus::Unhealthy);

        assert!(event.is_some());
        assert_eq!(tracker.current(), HealthStatus::Unhealthy);
        assert_eq!(tracker.history().len(), 1);
    }

    #[test]
    fn test_health_tracker_degradation_count() {
        let mut tracker = HealthTracker::new(HealthStatus::Healthy);

        tracker.update(HealthStatus::Unhealthy);
        tracker.update(HealthStatus::Healthy);
        tracker.update(HealthStatus::Unhealthy);

        assert_eq!(tracker.degradation_count(), 2);
        assert_eq!(tracker.improvement_count(), 1);
    }

    #[test]
    fn test_health_tracker_history_limit() {
        let mut tracker = HealthTracker::new(HealthStatus::Healthy);

        // Add 150 changes (should be limited to 100)
        for i in 0..150 {
            let status = if i % 2 == 0 {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            };
            tracker.update(status);
        }

        assert!(tracker.history().len() <= 100);
    }

    #[test]
    fn test_health_tracker_multiple_transitions() {
        let mut tracker = HealthTracker::new(HealthStatus::Unknown);

        tracker.update(HealthStatus::Healthy);
        tracker.update(HealthStatus::Unhealthy);
        tracker.update(HealthStatus::Healthy);

        assert_eq!(tracker.current(), HealthStatus::Healthy);
        assert_eq!(tracker.history().len(), 3);
    }

    #[test]
    fn test_health_zero_panics_from_score() {
        // Test edge cases that could cause issues
        let results = vec![
            HealthStatus::from_score(f64::INFINITY),
            HealthStatus::from_score(f64::NEG_INFINITY),
            HealthStatus::from_score(f64::NAN),
            HealthStatus::from_score(-100.0),
            HealthStatus::from_score(100.0),
        ];

        // All should return Unknown without panicking
        for result in results {
            assert_eq!(result, HealthStatus::Unknown);
        }
    }

    #[test]
    fn test_health_zero_panics_overall_health() {
        // Test with invalid inputs
        assert_eq!(overall_health(&[f64::INFINITY, f64::NEG_INFINITY]), HealthStatus::Unknown);
        assert_eq!(overall_health(&[-1.0, 2.0]), HealthStatus::Unknown);
        assert_eq!(overall_health(&[f64::NAN]), HealthStatus::Unknown);
    }
}
