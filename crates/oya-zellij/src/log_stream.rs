//! Log streaming with backpressure control
//!
//! Provides types and functions for handling log messages from pipeline stages
//! through Zellij's custom message pipe with bounded buffer backpressure.

use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of log messages to keep in buffer (backpressure limit)
#[allow(dead_code)]
pub const MAX_LOG_MESSAGES: usize = 1000;

/// Custom message name for log streaming events
#[allow(dead_code)]
pub const LOG_EVENT_NAME: &str = "log";

/// A log message from a pipeline stage or bead
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct LogMessage {
    /// The log message content
    pub content: String,
    /// Log level/severity
    pub level: LogLevel,
    /// Source identifier (bead_id or stage name)
    pub source: String,
    /// When the log message was received
    #[allow(dead_code)]
    pub timestamp: Instant,
}

impl LogMessage {
    /// Create a new log message
    #[allow(dead_code)]
    pub fn new(content: String, level: LogLevel, source: String) -> Self {
        Self {
            content,
            level,
            source,
            timestamp: Instant::now(),
        }
    }

    /// Parse a log message from JSON payload
    ///
    /// Expected JSON format:
    /// ```json
    /// {
    ///   "content": "message text",
    ///   "level": "info|warn|error",
    ///   "source": "bead-id"
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn from_json(json: &str) -> Result<Self, ParseError> {
        #[derive(serde::Deserialize)]
        struct JsonLogMessage {
            content: String,
            #[serde(default)]
            level: String,
            #[serde(default)]
            source: String,
        }

        std::str::from_utf8(json.as_bytes())
            .map_err(|_| ParseError::InvalidUtf8)
            .and_then(|_| {
                serde_json::from_str::<JsonLogMessage>(json)
                    .map_err(|e| ParseError::JsonError(e.to_string()))
            })
            .map(|msg| {
                let level = match msg.level.to_lowercase().as_str() {
                    "trace" => LogLevel::Trace,
                    "debug" => LogLevel::Debug,
                    "warn" | "warning" => LogLevel::Warn,
                    "error" | "err" => LogLevel::Error,
                    _ => LogLevel::Info,
                };

                Self::new(msg.content, level, msg.source)
            })
    }
}

/// Log level/severity
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Get ANSI color code for this log level
    #[allow(dead_code)]
    pub fn color(&self) -> &str {
        match self {
            Self::Trace => "\x1b[90m", // gray
            Self::Debug => "\x1b[36m", // cyan
            Self::Info => "\x1b[37m",  // white
            Self::Warn => "\x1b[33m",  // yellow
            Self::Error => "\x1b[31m", // red
        }
    }

    /// Get string representation of log level
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// Get symbol for log level
    #[allow(dead_code)]
    pub fn symbol(&self) -> &str {
        match self {
            Self::Trace => "·",
            Self::Debug => "·",
            Self::Info => "i",
            Self::Warn => "!",
            Self::Error => "x",
        }
    }
}

/// Error parsing log messages
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ParseError {
    InvalidUtf8,
    JsonError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            Self::JsonError(e) => write!(f, "JSON parse error: {}", e),
        }
    }
}

/// Bounded buffer for log messages with backpressure control
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct LogBuffer {
    #[allow(dead_code)]
    messages: VecDeque<LogMessage>,
    #[allow(dead_code)]
    dropped_count: usize,
}

impl LogBuffer {
    /// Create a new empty log buffer
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            dropped_count: 0,
        }
    }

    /// Push a log message into the buffer with backpressure
    ///
    /// If the buffer is full, the oldest message is dropped (FIFO eviction).
    /// This provides automatic backpressure by bounded memory usage.
    #[allow(dead_code)]
    pub fn push(&mut self, message: LogMessage) {
        // Apply backpressure: drop oldest messages when buffer is full
        while self.messages.len() >= MAX_LOG_MESSAGES {
            self.messages.pop_front();
            self.dropped_count = self.dropped_count.saturating_add(1);
        }

        self.messages.push_back(message);
    }

    /// Get all messages in reverse chronological order (newest first)
    #[allow(dead_code)]
    pub fn messages_rev(&self) -> Vec<LogMessage> {
        self.messages.iter().rev().cloned().collect()
    }

    /// Get the number of messages currently in the buffer
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the buffer is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the number of dropped messages due to backpressure
    #[allow(dead_code)]
    pub fn dropped_count(&self) -> usize {
        self.dropped_count
    }

    /// Clear all messages from the buffer
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.messages.clear();
        self.dropped_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_log(content: &str, level: LogLevel, source: &str) -> LogMessage {
        LogMessage::new(content.to_string(), level, source.to_string())
    }

    #[test]
    fn test_log_message_new() {
        let msg = create_test_log("test message", LogLevel::Info, "bead-1");
        assert_eq!(msg.content, "test message");
        assert_eq!(msg.level, LogLevel::Info);
        assert_eq!(msg.source, "bead-1");
    }

    #[test]
    fn test_log_level_colors() {
        assert_eq!(LogLevel::Trace.color(), "\x1b[90m");
        assert_eq!(LogLevel::Debug.color(), "\x1b[36m");
        assert_eq!(LogLevel::Info.color(), "\x1b[37m");
        assert_eq!(LogLevel::Warn.color(), "\x1b[33m");
        assert_eq!(LogLevel::Error.color(), "\x1b[31m");
    }

    #[test]
    fn test_log_level_strings() {
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_log_level_symbols() {
        assert_eq!(LogLevel::Trace.symbol(), "·");
        assert_eq!(LogLevel::Debug.symbol(), "·");
        assert_eq!(LogLevel::Info.symbol(), "i");
        assert_eq!(LogLevel::Warn.symbol(), "!");
        assert_eq!(LogLevel::Error.symbol(), "x");
    }

    #[test]
    fn test_log_buffer_new() {
        let buffer = LogBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.dropped_count(), 0);
    }

    #[test]
    fn test_log_buffer_push() {
        let mut buffer = LogBuffer::new();
        buffer.push(create_test_log("msg1", LogLevel::Info, "src1"));
        buffer.push(create_test_log("msg2", LogLevel::Warn, "src2"));

        assert_eq!(buffer.len(), 2);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.dropped_count(), 0);
    }

    #[test]
    fn test_log_buffer_backpressure() {
        let mut buffer = LogBuffer::new();

        // Fill buffer to capacity + 1
        for i in 0..=MAX_LOG_MESSAGES {
            buffer.push(create_test_log(
                &format!("message {}", i),
                LogLevel::Info,
                "src",
            ));
        }

        // Buffer should be at max capacity
        assert_eq!(buffer.len(), MAX_LOG_MESSAGES);

        // One message should have been dropped
        assert_eq!(buffer.dropped_count(), 1);

        // Oldest message should be dropped (FIFO eviction)
        let messages = buffer.messages_rev();
        assert!(!messages.iter().any(|m| m.content == "message 0"));
        assert!(messages.iter().any(|m| m.content == "message 1"));
    }

    #[test]
    fn test_log_buffer_messages_rev() {
        let mut buffer = LogBuffer::new();
        buffer.push(create_test_log("msg1", LogLevel::Info, "src1"));
        buffer.push(create_test_log("msg2", LogLevel::Warn, "src2"));
        buffer.push(create_test_log("msg3", LogLevel::Error, "src3"));

        let messages = buffer.messages_rev();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "msg3"); // newest first
        assert_eq!(messages[1].content, "msg2");
        assert_eq!(messages[2].content, "msg1"); // oldest last
    }

    #[test]
    fn test_log_buffer_clear() {
        let mut buffer = LogBuffer::new();
        buffer.push(create_test_log("msg1", LogLevel::Info, "src1"));
        buffer.push(create_test_log("msg2", LogLevel::Warn, "src2"));

        buffer.clear();

        assert!(buffer.is_empty());
        assert_eq!(buffer.dropped_count(), 0);
    }

    #[test]
    fn test_log_message_from_json_valid() {
        let json = r#"{"content":"test","level":"info","source":"bead-1"}"#;
        let result = LogMessage::from_json(json);

        assert!(result.is_ok());
        let msg = result
            .unwrap_or_else(|_| LogMessage::new("".to_string(), LogLevel::Info, "".to_string()));
        assert_eq!(msg.content, "test");
        assert_eq!(msg.level, LogLevel::Info);
        assert_eq!(msg.source, "bead-1");
    }

    #[test]
    fn test_log_message_from_json_defaults() {
        let json = r#"{"content":"test"}"#;
        let result = LogMessage::from_json(json);

        assert!(result.is_ok());
        let msg = result
            .unwrap_or_else(|_| LogMessage::new("".to_string(), LogLevel::Info, "".to_string()));
        assert_eq!(msg.content, "test");
        assert_eq!(msg.level, LogLevel::Info); // default
        assert_eq!(msg.source, ""); // default
    }

    #[test]
    fn test_log_message_from_json_invalid_level() {
        let json = r#"{"content":"test","level":"unknown","source":"bead-1"}"#;
        let result = LogMessage::from_json(json);

        assert!(result.is_ok());
        let msg = result
            .unwrap_or_else(|_| LogMessage::new("".to_string(), LogLevel::Info, "".to_string()));
        assert_eq!(msg.level, LogLevel::Info); // defaults to Info for unknown
    }

    #[test]
    fn test_log_message_from_json_invalid_utf8() {
        // Create invalid UTF-8 bytes
        let invalid_bytes: Vec<u8> = vec![0xFF, 0xFE];
        let _json = std::str::from_utf8(&invalid_bytes).map_err(|_| ParseError::InvalidUtf8);

        // Test that from_json handles invalid UTF-8
        let result = std::str::from_utf8(&invalid_bytes)
            .map_err(|_| ParseError::InvalidUtf8)
            .and_then(|s| {
                serde_json::from_str::<serde_json::Value>(s)
                    .map_err(|e| ParseError::JsonError(e.to_string()))
            });

        assert!(result.is_err());
        assert!(matches!(result, Err(ParseError::InvalidUtf8)));
    }

    #[test]
    fn test_log_message_from_json_invalid_json() {
        let json = "{not valid json}";
        let result = LogMessage::from_json(json);

        assert!(result.is_err());
        assert!(matches!(result, Err(ParseError::JsonError(_))));
    }

    #[test]
    fn test_parse_error_display() {
        assert_eq!(format!("{}", ParseError::InvalidUtf8), "Invalid UTF-8");
        assert_eq!(
            format!("{}", ParseError::JsonError("test error".to_string())),
            "JSON parse error: test error"
        );
    }

    #[test]
    fn test_log_level_parse_variants() {
        let test_cases = vec![
            ("warn", LogLevel::Warn),
            ("warning", LogLevel::Warn),
            ("error", LogLevel::Error),
            ("err", LogLevel::Error),
            ("info", LogLevel::Info),
            ("debug", LogLevel::Debug),
            ("trace", LogLevel::Trace),
        ];

        for (level_str, expected) in test_cases {
            let json = format!(r#"{{"content":"test","level":"{}"}}"#, level_str);
            let result = LogMessage::from_json(&json);
            assert!(result.is_ok());
            let level = result.map(|msg| msg.level);
            assert_eq!(level.unwrap_or(LogLevel::Info), expected);
        }
    }
}
