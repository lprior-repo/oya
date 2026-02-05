//!
//! # ACP (Agent Client Protocol) Schemas for OpenCode Integration
//!
//! Type-safe Rust port of OpenCode's TypeScript schemas from message-v2.ts
//!
//! See: vendor/opencode-sme/opencode-main/packages/opencode/src/session/message-v2.ts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

/// Base identifier for all message parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartBase {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
}

/// Text message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct TextPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<TimeRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// AI reasoning message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct ReasoningPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub time: TimeRange,
}

/// Time range for message parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
}

/// Snapshot message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct SnapshotPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub snapshot: String,
}

/// Patch message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct PatchPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub hash: String,
    pub files: Vec<String>,
}

/// File source for file parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FilePartSource {
    #[serde(rename_all = "camelCase")]
    File { path: String },
    #[serde(rename_all = "camelCase")]
    Symbol {
        path: String,
        range: LspRange,
        name: String,
        kind: i32,
    },
}

/// LSP range for symbol sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP position
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// File attachment message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct FilePart {
    #[serde(flatten)]
    pub base: PartBase,
    pub mime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSource>,
}

/// Agent message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct AgentPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<TextPosition>,
}

/// Text position with start/end
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPosition {
    pub value: String,
    pub start: i32,
    pub end: i32,
}

/// Tool state variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ToolState {
    #[serde(rename_all = "camelCase")]
    Pending {
        input: HashMap<String, serde_json::Value>,
        raw: String,
    },
    #[serde(rename_all = "camelCase")]
    Running {
        input: HashMap<String, serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
        time: TimeRange,
    },
    #[serde(rename_all = "camelCase")]
    Completed {
        input: HashMap<String, serde_json::Value>,
        output: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
        time: ToolTimeRange,
        #[serde(skip_serializing_if = "Option::is_none")]
        attachments: Option<Vec<FilePart>>,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        input: HashMap<String, serde_json::Value>,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<HashMap<String, serde_json::Value>>,
        time: TimeRange,
    },
}

/// Time range for tool states (includes compacted time)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolTimeRange {
    pub start: i64,
    pub end: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted: Option<i64>,
}

/// Tool execution message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct ToolPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub call_id: String,
    pub tool: String,
    pub state: ToolState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Step start message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct StepStartPart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

/// Step finish message part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct StepFinishPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub reason: String,
    pub snapshot: Option<String>,
    pub cost: f64,
    pub tokens: TokenUsage,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache: CacheStats,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub read: u64,
    pub write: u64,
}

/// All message part types (discriminated union)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MessagePart {
    Text(TextPart),
    Reasoning(ReasoningPart),
    Snapshot(SnapshotPart),
    Patch(PatchPart),
    File(FilePart),
    Agent(AgentPart),
    Tool(ToolPart),
    StepStart(StepStartPart),
    StepFinish(StepFinishPart),
}

/// ACP protocol error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_retryable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Complete ACP message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AcpMessage {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "reasoning")]
    Reasoning(ReasoningPart),
    #[serde(rename = "step-start")]
    StepStart(StepStartPart),
    #[serde(rename = "step-finish")]
    StepFinish(StepFinishPart),
    #[serde(rename = "tool")]
    Tool(ToolPart),
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotPart),
}

impl AcpMessage {
    /// Create a text message
    pub fn text(line: impl Into<String>, _sequence: u64) -> Self {
        Self::Text(TextPart {
            base: PartBase {
                id: Ulid::new().to_string(),
                session_id: Ulid::new().to_string(),
                message_id: Ulid::new().to_string(),
            },
            text: line.into(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        })
    }

    /// Create a final chunk
    pub fn final_chunk(content: impl Into<String>, _sequence: u64) -> Self {
        Self::Text(TextPart {
            base: PartBase {
                id: Ulid::new().to_string(),
                session_id: Ulid::new().to_string(),
                message_id: Ulid::new().to_string(),
            },
            text: content.into(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        })
    }

    /// Create an error chunk
    pub fn error(error: impl Into<String>, _sequence: u64) -> Self {
        Self::Text(TextPart {
            base: PartBase {
                id: Ulid::new().to_string(),
                session_id: Ulid::new().to_string(),
                message_id: Ulid::new().to_string(),
            },
            text: error.into(),
            synthetic: None,
            ignored: None,
            time: None,
            metadata: None,
        })
    }
}

/// Backward-compatible execution result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub tokens_used: Option<TokenUsage>,
    pub duration: Option<std::time::Duration>,
}

impl ExecutionResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            tokens_used: None,
            duration: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            tokens_used: None,
            duration: None,
        }
    }

    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = Some(duration);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_part_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let part = TextPart {
            base: PartBase {
                id: "test-id".to_string(),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            text: "Hello, World!".to_string(),
            synthetic: None,
            ignored: None,
            time: Some(TimeRange {
                start: 1234567890,
                end: Some(1234567891),
            }),
            metadata: None,
        };

        let json = serde_json::to_string(&part)?;
        let parsed: TextPart = serde_json::from_str(&json)?;

        assert_eq!(parsed.text, part.text);
        assert_eq!(parsed.base.id, part.base.id);
        Ok(())
    }

    #[test]
    fn test_tool_state_completed() -> Result<(), Box<dyn std::error::Error>> {
        let state = ToolState::Completed {
            input: HashMap::new(),
            output: "Success!".to_string(),
            title: "Test Tool".to_string(),
            metadata: None,
            time: ToolTimeRange {
                start: 1234567890,
                end: 1234567891,
                compacted: None,
            },
            attachments: None,
        };

        let json = serde_json::to_string(&state)?;
        let parsed: ToolState = serde_json::from_str(&json)?;

        assert!(matches!(parsed, ToolState::Completed { .. }));
        Ok(())
    }

    #[test]
    fn test_token_usage() -> Result<(), Box<dyn std::error::Error>> {
        let tokens = TokenUsage {
            input: 1000,
            output: 500,
            reasoning: 100,
            cache: CacheStats {
                read: 8000,
                write: 2000,
            },
        };

        let json = serde_json::to_string(&tokens)?;
        let parsed: TokenUsage = serde_json::from_str(&json)?;

        assert_eq!(parsed.input, 1000);
        assert_eq!(parsed.cache.read, 8000);
        Ok(())
    }
}
