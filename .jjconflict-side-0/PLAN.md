# SSE Implementation Plan

## Overview
Implement Server-Sent Events (SSE) streaming for the OpenCode client to provide real-time streaming output compatible with SSE protocol.

## Requirements
- Real-time streaming output via SSE protocol
- `text/event-stream` MIME type
- Proper SSE message formatting (event, data, id, retry fields)
- Backward compatibility with existing StreamChunk interface
- Zero panics, zero unwraps (Railway-Oriented Programming)

## Current Architecture
```
stream() → mpsc channel → stream_cli_output() → CLI stdout → Stream<StreamChunk>
```

## Proposed Architecture
```
stream_sse() → mpsc channel → stream_cli_output() → CLI stdout → Stream<StreamChunk>
                         ↓
                  SSE formatter → Stream<Output>
```

## Implementation Steps

### 1. Add SSE Formatter Module
**File**: `crates/opencode/src/sse.rs` (new file)

Create a module that transforms `Stream<StreamChunk>` into `Stream<Output>` with SSE formatting.

**Key Components**:
- `SseFormatter` struct with `StreamChunk` consumer
- `Output` enum for SSE message types:
  - `Text(String)` - Regular text chunks
  - `ToolUse(String)` - Tool use chunks
  - `Thinking(String)` - Thinking chunks
  - `Error(String)` - Error chunks
  - `Status(String)` - Status chunks
  - `Final(String)` - Final completion message
- `format_message()` function to convert Output to SSE format
- `format_chunk()` function to convert StreamChunk to Output
- Stream transformation using `tokio_stream::StreamExt` and `StreamExt::map`

**Functional Patterns**:
```rust
// Use and_then for error handling
chunk.and_then(|c| self.format_chunk(c))

// Use map for transformations
stream.map(|result| result.and_then(|chunk| self.format_chunk(chunk)))
```

### 2. Add stream_sse() Method
**File**: `crates/opencode/src/client.rs`

Add new method to `OpencodeClient`:

```rust
/// Execute a prompt and stream results as SSE-formatted output.
///
/// Returns a stream of SSE-formatted strings.
/// Each message is formatted according to SSE protocol:
/// - Content-Type: text/event-stream
/// - Messages separated by double newlines
/// - Fields: event, data, id, retry
pub async fn stream_sse(
    &self,
    prompt: &str,
) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
    // Implementation
}
```

**Implementation Approach**:
- Call existing `stream()` method to get Stream<StreamChunk>
- Pipe through SSE formatter
- Return new Stream<String>

**Railway-Oriented Programming**:
```rust
// Zero unwraps pattern
self.stream(&prompt)
    .await
    .map(|stream| self.format_sse_stream(stream))
    .and_then(|result| result.map_err(Error::from))
```

### 3. Add SSE Formatting Support
**File**: `crates/opencode/src/sse.rs`

Implement SSE message formatting:

```rust
fn format_message(&self, output: Output) -> String {
    match output {
        Output::Text(content) => format!("data: {}\n\n", content),
        Output::ToolUse(content) => format!("event: tool_use\ndata: {}\n\n", content),
        Output::Thinking(content) => format!("event: thinking\ndata: {}\n\n", content),
        Output::Error(content) => format!("event: error\ndata: {}\n\n", content),
        Output::Status(content) => format!("event: status\ndata: {}\n\n", content),
        Output::Final(content) => format!("event: completion\ndata: {}\n\n", content),
    }
}
```

### 4. Handle ChunkType Mapping
**File**: `crates/opencode/src/sse.rs`

Map StreamChunk::ChunkType to SSE event types:

```rust
fn map_chunk_type(&self, chunk_type: ChunkType) -> &'static str {
    match chunk_type {
        ChunkType::Text => "",
        ChunkType::ToolUse => "tool_use",
        ChunkType::Thinking => "thinking",
        ChunkType::Error => "error",
        ChunkType::Status => "status",
    }
}
```

### 5. Add HTTP Integration (Optional)
**File**: `crates/opencode/src/server.rs` (if server exists)

Add SSE endpoint handler:

```rust
pub async fn sse_handler(
    State(client): State<OpencodeClient>,
    Path(prompt): Path<String>,
) -> Response {
    let stream = client.stream_sse(&prompt)
        .await
        .unwrap_or_else(|e| {
            // Return error as SSE event
            stream_error("server_error", e.to_string())
        });

    Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(Body::from_stream(stream))
        .unwrap()
}
```

## Test Strategy

### Unit Tests (Phase 4 - RED)

**File**: `crates/opencode/src/sse.rs` (tests)

1. **SSE Message Formatting Tests**:
   - Test text formatting (no event field)
   - Test event field for ToolUse
   - Test event field for Thinking
   - Test error event formatting
   - Test status event formatting
   - Test final completion event

2. **ChunkType Mapping Tests**:
   - Test Text → empty event
   - Test ToolUse → "tool_use" event
   - Test Thinking → "thinking" event
   - Test Error → "error" event
   - Test Status → "status" event

3. **Stream Transformation Tests**:
   - Test StreamChunk to SSE output conversion
   - Test error propagation through stream
   - Test final chunk handling
   - Test multiple chunks in sequence

4. **Integration Tests**:
   - Test stream_sse() integration with stream()
   - Test SSE output with real CLI execution
   - Test error handling in SSE stream

### Integration Tests (Phase 5 - GREEN)

**File**: `crates/opencode/src/client.rs` (tests)

1. **Stream SSE Method Tests**:
   - Test stream_sse() returns proper stream type
   - Test stream_sse() handles empty prompts
   - Test stream_sse() propagates errors
   - Test stream_sse() with valid prompts

2. **SSE Protocol Compliance Tests**:
   - Test output has correct Content-Type header
   - Test messages separated by double newlines
   - Test SSE event field presence/absence
   - Test SSE data field format

## Error Handling

### Error Types
- **SseError**: SSE-specific errors (formatting, stream handling)
- **StreamError**: Stream-specific errors (channel, read errors)

### Error Propagation
- Use `Result<T, Error>` for all stream items
- Propagate errors from stream_cli_output()
- Format errors as SSE events

## Performance Considerations

1. **Streaming Efficiency**:
   - No buffering of entire response
   - Real-time streaming as chunks arrive
   - Minimal memory overhead

2. **Channel Size**:
   - Use reasonable channel size (100) for StreamChunk
   - Monitor for backpressure issues

3. **Error Recovery**:
   - Graceful error handling
   - Send error events to client
   - Clean shutdown on errors

## Backward Compatibility

1. **Existing Methods**:
   - Keep `stream()` unchanged
   - Add `stream_sse()` as new method
   - No changes to `StreamChunk` or `ChunkType`

2. **API Stability**:
   - New method doesn't modify existing API
   - Existing tests remain valid
   - No breaking changes

## Dependencies

**No New Dependencies Required**:
- `tokio` - Already available
- `tokio-stream` - Already available
- `futures` - Already available
- `serde` - Already available

**Optional**:
- `axum` - For HTTP server integration (if HTTP server exists)
- `hyper` - For HTTP server integration (if HTTP server exists)

## Success Criteria

1. **Functional**:
   - SSE streaming works correctly
   - All SSE event types formatted properly
   - Real-time streaming as chunks arrive
   - Errors handled gracefully

2. **Quality**:
   - Zero panics, zero unwraps
   - Railway-Oriented Programming patterns
   - Comprehensive test coverage
   - Clear documentation

3. **Performance**:
   - Real-time streaming (no buffering)
   - Minimal memory overhead
   - Efficient channel usage

4. **Compatibility**:
   - Backward compatible with existing API
   - No breaking changes
   - Works with existing tests

## Risk Assessment

**Low Risk**:
- New method doesn't modify existing code
- Reuses existing streaming infrastructure
- Well-defined interface

**Mitigation**:
- Comprehensive test coverage
- Error handling
- Backward compatibility

## Implementation Order

1. ✅ Phase 1: RESEARCH (Complete)
2. ⏳ Phase 2: PLAN (Create plan)
3. ⏳ Phase 3: VERIFY (Validate plan)
4. ⏳ Phase 4: RED (Write failing tests)
5. ⏳ Phase 5: GREEN (Implement)
6. ⏳ Phase 6: REFACTOR (Clean up)
7. ⏳ Phase 7: MF#1 (Code review)
8. ⏳ Phase 9: VERIFY-CRITERIA
9. ⏳ Phase 10: FP-GATES
10. ⏳ Phase 11: QA
11. ⏳ Phase 12: MF#2
12. ⏳ Phase 13: CONSISTENCY
13. ⏳ Phase 14: LIABILITY
14. ⏳ Phase 15: LANDING

## Notes

- Use functional programming patterns throughout
- Zero panics, zero unwraps
- Railway-Oriented Programming
- Comprehensive error handling
- Clear, idiomatic Rust code
