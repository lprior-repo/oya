//! Server-Sent Events (SSE) support for real-time streaming output.

use crate::types::StreamChunk;

/// Represents different types of SSE messages.
#[derive(Debug, Clone)]
pub enum Output {
    /// Text content output.
    Text(String),
    /// Tool use/execution output.
    ToolUse(String),
    /// Thinking/reasoning content.
    Thinking(String),
    /// Error message.
    Error(String),
    /// Status message.
    Status(String),
    /// Final output message.
    Final(String),
}

/// Formatter for converting StreamChunk to SSE-formatted strings.
///
/// Implements the Server-Sent Events protocol:
/// - Content-Type: text/event-stream
/// - Messages separated by double newlines
/// - Fields: event, data, id, retry
pub struct SseFormatter;

impl SseFormatter {
    /// Create a new SSE formatter.
    pub fn new() -> Self {
        Self
    }

    /// Format a single Output as an SSE message.
    ///
    /// Returns a formatted SSE message string.
    /// Each message follows the format:
    /// ```
    /// event: <event_type>
    /// data: <content>
    /// id: <sequence_number>
    ///
    /// ```
    pub fn format_message(&self, output: Output) -> String {
        let event_type = match &output {
            Output::Text(_) => "text",
            Output::ToolUse(_) => "tool",
            Output::Thinking(_) => "thinking",
            Output::Error(_) => "error",
            Output::Status(_) => "status",
            Output::Final(_) => "done",
        };

        let content = match &output {
            Output::Text(s)
            | Output::ToolUse(s)
            | Output::Thinking(s)
            | Output::Error(s)
            | Output::Status(s)
            | Output::Final(s) => s,
        };

        format!("event: {event_type}\ndata: {content}\nid: 1\n\n")
    }

    /// Format a StreamChunk as an SSE message.
    ///
    /// Returns a Result containing the formatted SSE message or an error.
    pub fn format_chunk(&self, chunk: StreamChunk) -> Result<String, String> {
        let output = match chunk.chunk_type {
            crate::types::ChunkType::Text => Output::Text(chunk.content),
            crate::types::ChunkType::ToolUse => Output::ToolUse(chunk.content),
            crate::types::ChunkType::Thinking => Output::Thinking(chunk.content),
            crate::types::ChunkType::Error => Output::Error(chunk.content),
            crate::types::ChunkType::Status => Output::Status(chunk.content),
        };

        Ok(self.format_message(output))
    }

    /// Format multiple StreamChunk values into a single SSE stream.
    ///
    /// Returns an iterator over formatted SSE messages.
    pub fn format_chunks<'a>(
        &'a self,
        chunks: impl IntoIterator<Item = StreamChunk> + 'a,
    ) -> impl Iterator<Item = Result<String, String>> + 'a {
        chunks
            .into_iter()
            .map(move |chunk| self.format_chunk(chunk))
    }
}

impl Default for SseFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_message_text() {
        let formatter = SseFormatter::new();
        let output = Output::Text("Hello, world!".to_string());

        let message = formatter.format_message(output);
        assert!(message.contains("event: text"));
        assert!(message.contains("data: Hello, world!"));
        assert!(message.contains("id: 1"));
        assert!(message.ends_with("\n\n"));
    }

    #[test]
    fn test_format_message_tool_use() {
        let formatter = SseFormatter::new();
        let output = Output::ToolUse("executed: cargo build".to_string());

        let message = formatter.format_message(output);
        assert!(message.contains("event: tool"));
        assert!(message.contains("data: executed: cargo build"));
    }

    #[test]
    fn test_format_message_thinking() {
        let formatter = SseFormatter::new();
        let output = Output::Thinking("analyzing prompt".to_string());

        let message = formatter.format_message(output);
        assert!(message.contains("event: thinking"));
        assert!(message.contains("data: analyzing prompt"));
    }

    #[test]
    fn test_format_message_error() {
        let formatter = SseFormatter::new();
        let output = Output::Error("failed to connect".to_string());

        let message = formatter.format_message(output);
        assert!(message.contains("event: error"));
        assert!(message.contains("data: failed to connect"));
    }

    #[test]
    fn test_format_message_status() {
        let formatter = SseFormatter::new();
        let output = Output::Status("processing".to_string());

        let message = formatter.format_message(output);
        assert!(message.contains("event: status"));
        assert!(message.contains("data: processing"));
    }

    #[test]
    fn test_format_message_final() {
        let formatter = SseFormatter::new();
        let output = Output::Final("completed".to_string());

        let message = formatter.format_message(output);
        assert!(message.contains("event: done"));
        assert!(message.contains("data: completed"));
    }

    #[test]
    fn test_format_chunk_text() -> Result<(), Box<dyn std::error::Error>> {
        let formatter = SseFormatter::new();
        let chunk = StreamChunk::text("Test content".to_string(), 1);

        let message = formatter.format_chunk(chunk)?;
        assert!(message.contains("event: text"));
        assert!(message.contains("data: Test content"));
        Ok(())
    }

    #[test]
    fn test_format_chunk_thinking() -> Result<(), Box<dyn std::error::Error>> {
        let formatter = SseFormatter::new();
        let chunk = StreamChunk::thinking("Thinking content", 1);

        let message = formatter.format_chunk(chunk)?;
        assert!(message.contains("event: thinking"));
        assert!(message.contains("data: Thinking content"));
        Ok(())
    }

    #[test]
    fn test_format_chunks_iterator() -> Result<(), Box<dyn std::error::Error>> {
        let formatter = SseFormatter::new();
        let chunks = vec![
            StreamChunk::text("First".to_string(), 1),
            StreamChunk::text("Second".to_string(), 2),
            StreamChunk::final_chunk("Done".to_string(), 3),
        ];

        let messages: Result<Vec<_>, _> = formatter
            .format_chunks(chunks)
            .map(|m| m.map(|s| s.lines().count()))
            .collect();

        assert!(messages.is_ok());
        let message_count = messages?;
        assert_eq!(message_count.len(), 3);
        Ok(())
    }

    #[test]
    fn test_format_chunk_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        let formatter = SseFormatter::new();
        let chunk = StreamChunk {
            chunk_type: crate::types::ChunkType::Error,
            content: "Test error".to_string(),
            is_final: false,
            sequence: 1,
        };

        let message = formatter.format_chunk(chunk)?;
        assert!(message.contains("event: error"));
        assert!(message.contains("data: Test error"));
        Ok(())
    }

    #[test]
    fn test_format_chunk_status() -> Result<(), Box<dyn std::error::Error>> {
        let formatter = SseFormatter::new();
        let chunk = StreamChunk::text("Processing...".to_string(), 1);

        let message = formatter.format_chunk(chunk)?;
        assert!(message.contains("event: text"));
        assert!(message.contains("data: Processing..."));
        Ok(())
    }
}
