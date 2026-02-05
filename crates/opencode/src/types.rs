//! Types for opencode execution results and streaming.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Result of executing a prompt via opencode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution was successful.
    pub success: bool,
    /// The output text from the AI.
    pub output: String,
    /// Any files that were modified.
    pub modified_files: Vec<ModifiedFile>,
    /// Commands that were executed.
    pub commands_executed: Vec<CommandExecution>,
    /// Total tokens used.
    pub tokens_used: TokenUsage,
    /// Duration of the execution.
    #[serde(with = "duration_millis")]
    pub duration: Duration,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExecutionResult {
    /// Create a new successful execution result.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            modified_files: Vec::new(),
            commands_executed: Vec::new(),
            tokens_used: TokenUsage::default(),
            duration: Duration::ZERO,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed execution result.
    pub fn failure(output: impl Into<String>) -> Self {
        Self {
            success: false,
            output: output.into(),
            modified_files: Vec::new(),
            commands_executed: Vec::new(),
            tokens_used: TokenUsage::default(),
            duration: Duration::ZERO,
            metadata: HashMap::new(),
        }
    }

    /// Add a modified file to the result.
    #[must_use]
    pub fn with_modified_file(mut self, file: ModifiedFile) -> Self {
        self.modified_files.push(file);
        self
    }

    /// Add a command execution to the result.
    #[must_use]
    pub fn with_command(mut self, cmd: CommandExecution) -> Self {
        self.commands_executed.push(cmd);
        self
    }

    /// Set the duration.
    #[must_use]
    pub const fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set the token usage.
    #[must_use]
    pub const fn with_tokens(mut self, tokens: TokenUsage) -> Self {
        self.tokens_used = tokens;
        self
    }
}

/// Information about a file that was modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedFile {
    /// Path to the file.
    pub path: String,
    /// Type of modification.
    pub modification: ModificationType,
    /// Diff of the changes (if available).
    pub diff: Option<String>,
}

/// Type of file modification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModificationType {
    /// File was created.
    Created,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed.
    Renamed,
}

/// Information about a command that was executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    /// The command that was run.
    pub command: String,
    /// Exit code of the command.
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Duration of the command.
    #[serde(with = "duration_millis")]
    pub duration: Duration,
}

impl CommandExecution {
    /// Check if the command succeeded.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,
    /// Tokens in the completion.
    pub completion_tokens: u32,
    /// Total tokens.
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create new token usage.
    #[must_use]
    pub const fn new(prompt: u32, completion: u32) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }
}

/// A chunk of streamed output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// The type of chunk.
    pub chunk_type: ChunkType,
    /// The content of the chunk.
    pub content: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Sequence number for ordering.
    pub sequence: u64,
}

impl StreamChunk {
    /// Create a text chunk.
    pub fn text(content: impl Into<String>, sequence: u64) -> Self {
        Self {
            chunk_type: ChunkType::Text,
            content: content.into(),
            is_final: false,
            sequence,
        }
    }

    /// Create a tool use chunk.
    pub fn tool_use(content: impl Into<String>, sequence: u64) -> Self {
        Self {
            chunk_type: ChunkType::ToolUse,
            content: content.into(),
            is_final: false,
            sequence,
        }
    }

    /// Create a thinking/reasoning chunk.
    pub fn thinking(content: impl Into<String>, sequence: u64) -> Self {
        Self {
            chunk_type: ChunkType::Thinking,
            content: content.into(),
            is_final: false,
            sequence,
        }
    }

    /// Create a final chunk.
    pub fn final_chunk(content: impl Into<String>, sequence: u64) -> Self {
        Self {
            chunk_type: ChunkType::Text,
            content: content.into(),
            is_final: true,
            sequence,
        }
    }

    /// Create an error chunk.
    pub fn error(content: impl Into<String>, sequence: u64) -> Self {
        Self {
            chunk_type: ChunkType::Error,
            content: content.into(),
            is_final: true,
            sequence,
        }
    }
}

/// Type of stream chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    /// Regular text output.
    Text,
    /// Tool use (command execution, file edit, etc.).
    ToolUse,
    /// Thinking/reasoning output.
    Thinking,
    /// Error message.
    Error,
    /// Status update.
    Status,
}

/// Serialization helper for Duration as milliseconds.
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success("Hello, world!");
        assert!(result.success);
        assert_eq!(result.output, "Hello, world!");
    }

    #[test]
    fn test_execution_result_with_file() {
        let result = ExecutionResult::success("Done").with_modified_file(ModifiedFile {
            path: "src/main.rs".into(),
            modification: ModificationType::Modified,
            diff: Some("+fn main() {}".into()),
        });
        assert_eq!(result.modified_files.len(), 1);
        assert_eq!(result.modified_files[0].path, "src/main.rs");
    }

    #[test]
    fn test_stream_chunk_text() {
        let chunk = StreamChunk::text("Hello", 1);
        assert_eq!(chunk.chunk_type, ChunkType::Text);
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_stream_chunk_final() {
        let chunk = StreamChunk::final_chunk("Done", 10);
        assert!(chunk.is_final);
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
