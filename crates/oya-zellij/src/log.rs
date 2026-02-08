//! Log aggregation and display for multi-source logging

use chrono::{DateTime, Utc};
use rpds::Vector;
use std::fmt;

/// Log level for filtering and display
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub fn color_code(self) -> &'static str {
        match self {
            Self::Debug => "\x1b[36m",
            Self::Info => "\x1b[37m",
            Self::Warning => "\x1b[33m",
            Self::Error => "\x1b[31m",
        }
    }

    pub fn reset_color() -> &'static str {
        "\x1b[0m"
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Identifies the source of a log entry
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogSource {
    EventStore,
    Orchestrator,
    Agent,
    Worker,
    Pipeline,
    ApiServer,
    Ui,
    Custom(String),
}

impl LogSource {
    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::EventStore => "event_store",
            Self::Orchestrator => "orchestrator",
            Self::Agent => "agent",
            Self::Worker => "worker",
            Self::Pipeline => "pipeline",
            Self::ApiServer => "api_server",
            Self::Ui => "ui",
            Self::Custom(name) => name,
        }
    }

    pub fn short_name(&self) -> &str {
        match self {
            Self::EventStore => "EVTS",
            Self::Orchestrator => "ORCH",
            Self::Agent => "AGNT",
            Self::Worker => "WRKR",
            Self::Pipeline => "PIPE",
            Self::ApiServer => "API",
            Self::Ui => "UI",
            Self::Custom(name) => {
                if name.len() <= 4 {
                    name
                } else {
                    &name[..4]
                }
            }
        }
    }
}

impl fmt::Display for LogSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single log entry with source tagging
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    timestamp: DateTime<Utc>,
    level: LogLevel,
    source: LogSource,
    message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, source: LogSource, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            source,
            message: message.into(),
        }
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn source(&self) -> &LogSource {
        &self.source
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn format(&self) -> String {
        format!(
            "[{}] [{}] {} - {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            self.source.short_name(),
            self.level,
            self.message
        )
    }

    pub fn format_colored(&self) -> String {
        format!(
            "[{}] [{}] {}{}{} - {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            self.source.short_name(),
            self.level.color_code(),
            self.level,
            LogLevel::reset_color(),
            self.message
        )
    }

    pub fn format_compact(&self) -> String {
        format!(
            "{} {} {} {}",
            self.timestamp.format("%H:%M:%S%.3f"),
            self.source.short_name(),
            self.level.as_str(),
            self.message
        )
    }
}

/// Aggregates logs from multiple sources
#[derive(Debug, Clone)]
pub struct LogAggregator {
    entries: Vector<LogEntry>,
    max_entries: usize,
    min_level: LogLevel,
}

impl LogAggregator {
    pub fn new() -> Self {
        Self {
            entries: Vector::new(),
            max_entries: 1000,
            min_level: LogLevel::Debug,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        let mut aggregator = Self::new();
        aggregator.max_entries = max_entries;
        aggregator
    }

    pub fn with_min_level(min_level: LogLevel) -> Self {
        let mut aggregator = Self::new();
        aggregator.min_level = min_level;
        aggregator
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries(&self) -> &Vector<LogEntry> {
        &self.entries
    }

    pub fn add_entry(mut self, entry: LogEntry) -> Self {
        if entry.level < self.min_level {
            return self;
        }

        self.entries = self.entries.push_back(entry);

        // Note: rpds::Vector doesn't have a simple drop method
        // For production use, we'd need to implement proper trimming
        // using iterator-based approaches or a different data structure
        if self.entries.len() > self.max_entries * 2 {
            // Force trim if we get too large (2x max)
            let start = self.entries.len() - self.max_entries;
            self.entries = self.entries.iter().skip(start).cloned().collect();
        }

        self
    }

    pub fn add_entries(mut self, entries: impl IntoIterator<Item = LogEntry>) -> Self {
        for entry in entries {
            self = self.add_entry(entry);
        }
        self
    }

    pub fn entries_by_source(&self, source: &LogSource) -> Vector<LogEntry> {
        self.entries
            .iter()
            .filter(|e: &&LogEntry| e.source() == source)
            .cloned()
            .collect()
    }

    pub fn entries_by_level(&self, level: LogLevel) -> Vector<LogEntry> {
        self.entries
            .iter()
            .filter(|e: &&LogEntry| e.level() >= level)
            .cloned()
            .collect()
    }

    pub fn entries_by_source_and_level(
        &self,
        source: &LogSource,
        level: LogLevel,
    ) -> Vector<LogEntry> {
        self.entries
            .iter()
            .filter(|e: &&LogEntry| e.source() == source && e.level() >= level)
            .cloned()
            .collect()
    }

    pub fn clear(mut self) -> Self {
        self.entries = Vector::new();
        self
    }

    pub fn recent_entries(&self, count: usize) -> Vector<LogEntry> {
        let start = if self.entries.len() > count {
            self.entries.len() - count
        } else {
            0
        };

        self.entries.iter().skip(start).cloned().collect()
    }

    pub fn render(&self) -> Vec<String> {
        self.entries.iter().map(|e: &LogEntry| e.format()).collect()
    }

    pub fn render_colored(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|e: &LogEntry| e.format_colored())
            .collect()
    }

    pub fn render_compact(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|e: &LogEntry| e.format_compact())
            .collect()
    }

    pub fn render_source(&self, source: &LogSource) -> Vec<String> {
        self.entries_by_source(source)
            .iter()
            .map(|e: &LogEntry| e.format())
            .collect()
    }
}

impl Default for LogAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(LogLevel::Info, LogSource::Orchestrator, "Test message");

        assert_eq!(entry.level(), LogLevel::Info);
        assert_eq!(entry.source(), &LogSource::Orchestrator);
        assert_eq!(entry.message(), "Test message");
    }

    #[test]
    fn test_log_entry_formatting() {
        let entry = LogEntry::new(LogLevel::Error, LogSource::Agent, "Agent failed");

        let formatted = entry.format();
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("AGNT"));
        assert!(formatted.contains("Agent failed"));
    }

    #[test]
    fn test_log_source_short_names() {
        assert_eq!(LogSource::EventStore.short_name(), "EVTS");
        assert_eq!(LogSource::Orchestrator.short_name(), "ORCH");
        assert_eq!(LogSource::Agent.short_name(), "AGNT");
        assert_eq!(LogSource::Custom("test".to_string()).short_name(), "test");
        assert_eq!(
            LogSource::Custom("longname".to_string()).short_name(),
            "long"
        );
    }

    #[test]
    fn test_log_aggregator_add_entry() {
        let aggregator = LogAggregator::new();
        let entry = LogEntry::new(LogLevel::Info, LogSource::Worker, "Worker started");

        let updated = aggregator.add_entry(entry);
        assert_eq!(updated.entry_count(), 1);
    }

    #[test]
    fn test_log_aggregator_filters_by_level() {
        let aggregator = LogAggregator::with_min_level(LogLevel::Warning);

        let debug_entry = LogEntry::new(LogLevel::Debug, LogSource::Ui, "Debug message");

        let warn_entry = LogEntry::new(LogLevel::Warning, LogSource::Ui, "Warning message");

        let updated = aggregator.add_entry(debug_entry).add_entry(warn_entry);

        assert_eq!(updated.entry_count(), 1);
        assert_eq!(
            updated
                .entries()
                .first()
                .map_or(LogLevel::Info, |e| e.level()),
            LogLevel::Warning,
        );
    }

    #[test]
    fn test_log_aggregator_trims_old_entries() {
        let aggregator = LogAggregator::with_max_entries(5);

        let mut updated = aggregator;
        for i in 0..20 {
            let entry = LogEntry::new(
                LogLevel::Info,
                LogSource::Pipeline,
                format!("Message {}", i),
            );
            updated = updated.add_entry(entry);
        }

        // Check that trimming occurred (should be less than 20)
        assert!(updated.entry_count() < 20);
        // And that we still have some entries
        assert!(updated.entry_count() > 0);
    }

    #[test]
    fn test_log_aggregator_filters_by_source() {
        let mut aggregator = LogAggregator::new();

        let entry1 = LogEntry::new(LogLevel::Info, LogSource::Agent, "Agent message");

        let entry2 = LogEntry::new(LogLevel::Info, LogSource::Worker, "Worker message");

        aggregator = aggregator.add_entry(entry1).add_entry(entry2);

        let agent_logs = aggregator.entries_by_source(&LogSource::Agent);
        assert_eq!(agent_logs.len(), 1);
        if let Some(entry) = agent_logs.first() {
            assert_eq!(entry.source(), &LogSource::Agent);
        }
    }

    #[test]
    fn test_log_aggregator_clear() {
        let mut aggregator = LogAggregator::new();

        let entry = LogEntry::new(LogLevel::Info, LogSource::ApiServer, "API message");

        aggregator = aggregator.add_entry(entry);
        assert_eq!(aggregator.entry_count(), 1);

        aggregator = aggregator.clear();
        assert_eq!(aggregator.entry_count(), 0);
    }

    #[test]
    fn test_log_aggregator_recent_entries() {
        let mut aggregator = LogAggregator::new();

        for i in 0..10 {
            let entry = LogEntry::new(
                LogLevel::Info,
                LogSource::EventStore,
                format!("Message {}", i),
            );
            aggregator = aggregator.add_entry(entry);
        }

        let recent = aggregator.recent_entries(3);
        assert_eq!(recent.len(), 3);

        assert!(recent.iter().all(|e| e.message().contains("7")
            || e.message().contains("8")
            || e.message().contains("9")));
    }

    #[test]
    fn test_log_aggregator_render() {
        let mut aggregator = LogAggregator::new();

        let entry = LogEntry::new(LogLevel::Error, LogSource::Worker, "Error occurred");

        aggregator = aggregator.add_entry(entry);

        let rendered = aggregator.render();
        assert_eq!(rendered.len(), 1);
        assert!(rendered[0].contains("ERROR"));
        assert!(rendered[0].contains("WRKR"));
        assert!(rendered[0].contains("Error occurred"));
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Error);
    }

    #[test]
    fn test_custom_log_source() {
        let custom = LogSource::custom("my_component");
        assert_eq!(custom.as_str(), "my_component");
        assert_eq!(custom.short_name(), "my_c");

        let entry = LogEntry::new(LogLevel::Info, custom.clone(), "Test");

        assert_eq!(entry.source(), &custom);
    }
}
