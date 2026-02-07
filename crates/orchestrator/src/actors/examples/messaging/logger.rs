//! Logger Actor - Demonstrates send() pattern
//!
//! This actor shows:
//! - **send()** pattern: Async message passing without waiting for response
//! - Message buffering and processing
//! - Thread-safe logging with persistent state

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use ractor::{Actor, ActorProcessingErr, ActorRef};
use rpds::{ArcK, Vector};
use std::fmt;

//==============================================================================
// Messages
//==============================================================================

/// Logger messages - all use send() pattern (fire-and-forget)
#[derive(Debug, Clone, PartialEq)]
pub enum LoggerMessage {
    /// Log a message
    Log { msg: String },

    /// Log a message with level
    LogWithLevel { level: LogLevel, msg: String },

    /// Clear all log entries
    Clear,

    /// Get all log entries (note: this demonstrates you CAN get replies with send,
    /// but typically you'd use call! for request-response)
    GetAll {
        /// Sender for the reply
        reply: tokio::sync::oneshot::Sender<Vector<LogEntry, ArcK>>,
    },
}

/// Log level for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug information
    Debug,
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

impl LogLevel {
    /// Get the log level as a string
    pub fn as_str(&self) -> &str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

//==============================================================================
// Log Entry
//==============================================================================

/// A single log entry
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    /// Timestamp when the log was created
    timestamp: chrono::DateTime<chrono::Utc>,
    /// Log level
    level: LogLevel,
    /// Log message
    message: String,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            level,
            message,
        }
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.timestamp
    }

    /// Get the log level
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Get the message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Format the log entry
    pub fn format(&self) -> String {
        format!(
            "[{}] {} - {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            self.level,
            self.message
        )
    }
}

//==============================================================================
// State
//==============================================================================

/// Logger state using persistent data structures
#[derive(Debug, Clone)]
pub struct LoggerState {
    /// Log entries (persistent vector for efficient appends)
    entries: Vector<LogEntry, ArcK>,
    /// Maximum number of entries to keep
    max_entries: usize,
}

impl LoggerState {
    /// Create a new logger state
    pub fn new() -> Self {
        Self {
            entries: Vector::new_with_ptr_kind(),
            max_entries: 1000,
        }
    }

    /// Create with custom max entries
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Vector::new_with_ptr_kind(),
            max_entries,
        }
    }

    /// Get the number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get all entries
    pub fn entries(&self) -> &Vector<LogEntry, ArcK> {
        &self.entries
    }

    /// Add a log entry
    pub fn add_entry(mut self, entry: LogEntry) -> Self {
        self.entries = self.entries.push_back(entry);

        // Trim if we exceed max entries
        if self.entries.len() > self.max_entries {
            let excess = self.entries.len() - self.max_entries;
            self.entries = self.entries.drop(excess);
        }

        self
    }

    /// Clear all entries
    pub fn clear(mut self) -> Self {
        self.entries = Vector::new_with_ptr_kind();
        self
    }
}

impl Default for LoggerState {
    fn default() -> Self {
        Self::new()
    }
}

//==============================================================================
// Actor Implementation
//==============================================================================

/// Logger actor demonstrating send() pattern
///
/// This actor uses async message passing and doesn't provide responses
/// (except for the GetAll message which uses a oneshot channel).
pub struct LoggerActor;

impl LoggerActor {
    /// Create a new logger actor
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggerActor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Actor for LoggerActor {
    type Msg = LoggerMessage;
    type State = LoggerState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(LoggerState::new())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            LoggerMessage::Log { msg } => {
                // Create info-level log entry
                let entry = LogEntry::new(LogLevel::Info, msg);
                *state = state.clone().add_entry(entry);
                Ok(())
            }

            LoggerMessage::LogWithLevel { level, msg } => {
                // Create log entry with specified level
                let entry = LogEntry::new(level, msg);
                *state = state.clone().add_entry(entry);
                Ok(())
            }

            LoggerMessage::Clear => {
                // Clear all entries
                *state = state.clone().clear();
                Ok(())
            }

            LoggerMessage::GetAll { reply } => {
                // Send all entries back via oneshot channel
                let _ = reply.send(state.entries().clone());
                Ok(())
            }
        }
    }
}

//==============================================================================
// Tests
//==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Helper to spawn a logger actor for testing
    async fn spawn_logger()
    -> Result<(ActorRef<LoggerMessage>, ractor::ActorHandle), ActorProcessingErr> {
        Actor::spawn(None, LoggerActor::new(), ()).await
    }

    #[tokio::test]
    async fn send_pattern_log_message() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A logger actor
        let (actor, handle) = spawn_logger().await?;

        // When: Sending a log message (async, no waiting)
        actor.send_message(LoggerMessage::Log {
            msg: "Test message".to_string(),
        })?;

        // Give actor time to process
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Then: Message should be logged
        let (tx, rx) = tokio::sync::oneshot::channel();
        actor.send_message(LoggerMessage::GetAll { reply: tx })?;
        let entries = rx.await?;

        assert_eq!(entries.len(), 1);
        let entry = entries.first().ok_or("No entry found")?;
        assert_eq!(entry.level(), LogLevel::Info);
        assert_eq!(entry.message(), "Test message");

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn send_pattern_log_with_level() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A logger actor
        let (actor, handle) = spawn_logger().await?;

        // When: Sending log messages with different levels
        actor.send_message(LoggerMessage::LogWithLevel {
            level: LogLevel::Debug,
            msg: "Debug message".to_string(),
        })?;

        actor.send_message(LoggerMessage::LogWithLevel {
            level: LogLevel::Warning,
            msg: "Warning message".to_string(),
        })?;

        actor.send_message(LoggerMessage::LogWithLevel {
            level: LogLevel::Error,
            msg: "Error message".to_string(),
        })?;

        // Give actor time to process
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Then: All messages should be logged with correct levels
        let (tx, rx) = tokio::sync::oneshot::channel::<Vector<LogEntry, ArcK>>();
        actor.send_message(LoggerMessage::GetAll { reply: tx })?;
        let entries = rx.await?;

        assert_eq!(entries.len(), 3);

        let levels: Vec<LogLevel> = entries.iter().map(|e| e.level()).collect();
        assert_eq!(
            levels,
            vec![LogLevel::Debug, LogLevel::Warning, LogLevel::Error]
        );

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn send_pattern_clear_logs() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A logger actor with some messages
        let (actor, handle) = spawn_logger().await?;

        actor.send_message(LoggerMessage::Log {
            msg: "Message 1".to_string(),
        })?;
        actor.send_message(LoggerMessage::Log {
            msg: "Message 2".to_string(),
        })?;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Verify messages are logged
        let (tx, rx) = tokio::sync::oneshot::channel::<Vector<LogEntry, ArcK>>();
        actor.send_message(LoggerMessage::GetAll { reply: tx.clone() })?;
        assert_eq!(rx.await?.len(), 2);

        // When: Clearing logs
        actor.send_message(LoggerMessage::Clear)?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Then: Logs should be empty
        let (tx, rx) = tokio::sync::oneshot::channel::<Vector<LogEntry, ArcK>>();
        actor.send_message(LoggerMessage::GetAll { reply: tx })?;
        let entries = rx.await?;

        assert_eq!(entries.len(), 0);

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn send_pattern_concurrent_logging() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A logger actor
        let (actor, handle) = spawn_logger().await?;

        // When: Sending many log messages concurrently
        let mut tasks = Vec::new();
        for i in 0..100 {
            let actor_clone = actor.clone();
            let task = tokio::spawn(async move {
                actor_clone.send_message(LoggerMessage::Log {
                    msg: format!("Message {}", i),
                })
            });
            tasks.push(task);
        }

        // Wait for all sends to complete
        for task in tasks {
            task.await??;
        }

        // Give actor time to process all messages
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Then: All messages should be logged
        let (tx, rx) = tokio::sync::oneshot::channel::<Vector<LogEntry, ArcK>>();
        actor.send_message(LoggerMessage::GetAll { reply: tx })?;
        let entries = rx.await?;

        assert_eq!(entries.len(), 100);

        // Verify message contents
        let messages: Vec<&str> = entries.iter().map(|e: &LogEntry| e.message()).collect();
        for i in 0..100 {
            assert!(messages.contains(&format!("Message {}", i)));
        }

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn send_pattern_max_entries_limit() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A logger actor with small max_entries
        let (actor, handle) = Actor::spawn(
            None,
            LoggerActor::new(),
            (), // Could pass max_entries config here
        )
        .await?;

        // When: Sending more messages than max_entries
        for i in 0..1500 {
            actor.send_message(LoggerMessage::Log {
                msg: format!("Message {}", i),
            })?;
        }

        // Give actor time to process
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Then: Should keep only the most recent entries (1000 by default)
        let (tx, rx) = tokio::sync::oneshot::channel::<Vector<LogEntry, ArcK>>();
        actor.send_message(LoggerMessage::GetAll { reply: tx })?;
        let entries = rx.await?;

        // Default max_entries is 1000
        assert!(entries.len() <= 1000);

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[test]
    fn log_entry_formatting() {
        let entry = LogEntry::new(LogLevel::Info, "Test message".to_string());
        let formatted = entry.format();

        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("Test message"));
        assert!(formatted.contains("UTC"));
    }

    #[test]
    fn log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Warning.to_string(), "WARN");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn logger_state_functional_updates() {
        let state = LoggerState::new();
        assert_eq!(state.entry_count(), 0);

        let entry = LogEntry::new(LogLevel::Info, "Test".to_string());
        let state = state.add_entry(entry);
        assert_eq!(state.entry_count(), 1);

        let state = state.clear();
        assert_eq!(state.entry_count(), 0);
    }
}
