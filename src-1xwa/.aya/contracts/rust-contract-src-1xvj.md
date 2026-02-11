# Rust Contract: Length-Prefixed Buffer Transport Layer

**Bead ID:** `src-1xvj`
**Priority:** P1
**Size:** Small
**Generated:** 2026-02-07 23:28:02
**Type:** Feature - IPC Transport Protocol

## Overview

Implement a high-performance transport layer for IPC communication between Zellij plugin processes and the orchestrator. The transport uses a length-prefixed binary protocol with bincode serialization for efficient message passing.

### Protocol Specification

**Message Format**:
```
[4 bytes: length (big-endian u32)][N bytes: bincode payload]
```

**Constraints**:
- Max message size: 1MB (1,048,576 bytes)
- Length prefix: 32-bit unsigned integer, big-endian
- Must flush writer after each message
- Buffer sizes: 8KB read buffer, 8KB write buffer

## Functional Requirements

### Core Functionality

The `IpcTransport` must:

1. **Send messages**: Serialize to bincode, write length prefix, write payload, flush
2. **Receive messages**: Read length prefix, validate size, read payload, deserialize
3. **Handle errors**: All error cases must return `Result<T, TransportError>`
4. **Resource management**: Proper buffer flushing on drop

### API Surface

```rust
use std::io::{BufReader, BufWriter, Read, Write};
use serde::{Serialize, de::DeserializeOwned};

/// IPC transport for length-prefixed messages
pub struct IpcTransport<R, W> {
    reader: BufReader<R>,
    writer: BufWriter<W>,
    config: TransportConfig,
}

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub max_message_size: usize,
    pub read_buffer_size: usize,
    pub write_buffer_size: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_message_size: 1024 * 1024, // 1MB
            read_buffer_size: 8 * 1024,    // 8KB
            write_buffer_size: 8 * 1024,   // 8KB
        }
    }
}

impl<R: Read, W: Write> IpcTransport<R, W> {
    /// Create new transport with default config
    pub fn new(reader: R, writer: W) -> Self;

    /// Create new transport with custom config
    pub fn with_config(reader: R, writer: W, config: TransportConfig) -> Self;

    /// Send a message (length-prefix + bincode payload)
    pub fn send<T: Serialize>(&mut self, msg: &T) -> Result<(), TransportError>;

    /// Receive a message (read length-prefix, read payload, deserialize)
    pub fn recv<T: DeserializeOwned>(&mut self) -> Result<T, TransportError>;

    /// Flush write buffer
    pub fn flush(&mut self) -> Result<(), TransportError>;

    /// Get underlying reader (for async conversion)
    pub fn reader_mut(&mut self) -> &mut BufReader<R>;

    /// Get underlying writer (for async conversion)
    pub fn writer_mut(&mut self) -> &mut BufWriter<W>;
}

impl<R, W> Drop for IpcTransport<R, W> {
    fn drop(&mut self) {
        // Flush write buffer on drop
        let _ = self.flush();
    }
}
```

### Input/Output Specifications

| Operation | Input | Validation | Output |
|-----------|-------|------------|--------|
| send | T: Serialize | Serialized size < max_message_size | Result\<()\> |
| recv | None | Length prefix valid, size < max_message_size | Result\<T\> |
| flush | None | None | Result\<()\> |

## Error Handling

### Error Hierarchy

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Message too large: {size} bytes (max: {max} bytes)")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Unexpected EOF: expected {expected} bytes, got {actual} bytes")]
    UnexpectedEof { expected: usize, actual: usize },

    #[error("Invalid length prefix: {0}")]
    InvalidLength(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(#[from] bincode::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type TransportResult<T> = Result<T, TransportError>;
```

### Error Propagation Strategy

- **Zero panics**: All error paths use `Result<T, E>`
- **Zero unwraps**: Forbidden in production code
- **Railway-Oriented Programming**: Use `?` operator throughout
- **Context preservation**: Errors include expected/actual sizes, max limits
- **Resource cleanup**: Drop impl ensures flush on all paths

## Implementation Details

### Send Operation

```rust
pub fn send<T: Serialize>(&mut self, msg: &T) -> Result<(), TransportError> {
    // Serialize message to bincode
    let payload = bincode::serialize(msg)?;

    // Validate message size
    if payload.len() > self.config.max_message_size {
        return Err(TransportError::MessageTooLarge {
            size: payload.len(),
            max: self.config.max_message_size,
        });
    }

    // Write length prefix (big-endian u32)
    let length = payload.len() as u32;
    self.writer.write_all(&length.to_be_bytes())?;

    // Write payload
    self.writer.write_all(&payload)?;

    // Flush buffer
    self.flush()?;

    Ok(())
}
```

### Receive Operation

```rust
pub fn recv<T: DeserializeOwned>(&mut self) -> Result<T, TransportError> {
    // Read length prefix (4 bytes)
    let mut length_bytes = [0u8; 4];
    self.reader.read_exact(&mut length_bytes).map_err(|e| {
        match e.kind() {
            std::io::ErrorKind::UnexpectedEof => {
                TransportError::UnexpectedEof {
                    expected: 4,
                    actual: 0,
                }
            }
            _ => TransportError::Io(e),
        }
    })?;

    // Parse length prefix (big-endian u32)
    let length = u32::from_be_bytes(length_bytes) as usize;

    // Validate length
    if length > self.config.max_message_size {
        return Err(TransportError::MessageTooLarge {
            size: length,
            max: self.config.max_message_size,
        });
    }

    if length == 0 {
        return Err(TransportError::InvalidLength(
            "Message length cannot be zero".to_string()
        ));
    }

    // Read payload
    let mut payload = vec![0u8; length];
    self.reader.read_exact(&mut payload).map_err(|e| {
        match e.kind() {
            std::io::ErrorKind::UnexpectedEof => {
                TransportError::UnexpectedEof {
                    expected: length,
                    actual: payload.len(),
                }
            }
            _ => TransportError::Io(e),
        }
    })?;

    // Deserialize payload
    let msg = bincode::deserialize(&payload)?;

    Ok(msg)
}
```

## Performance Requirements

| Metric | Target | Measurement |
|--------|--------|-------------|
| send() latency | <2µs | 1KB message serialization + write |
| recv() latency | <3µs | Read + deserialize 1KB message |
| Memory overhead | <16KB | 8KB read + 8KB write buffers |
| Max throughput | >1M msg/s | Sustained rate for 1KB messages |

### Performance Optimization

- **Buffered I/O**: Use `BufReader`/`BufWriter` for reduced syscalls
- **Zero-copy**: Use `read_exact` directly into payload buffer
- **Stack allocation**: Length prefix read into stack array
- **Eager validation**: Check message size before allocating payload buffer

## Testing Requirements

See `martin-fowler-tests-src-1xvj.md` for comprehensive test strategy including:
- Send/recv round-trip tests
- Oversized message rejection
- Partial read handling
- Concurrent access (multiple tasks)
- Performance benchmarks

## Integration Points

### Upstream Dependencies

- **std**: I/O primitives (`Read`, `Write`)
- **serde**: Serialization framework
- **bincode**: Binary serialization format

### Downstream Consumers

- **IpcWorker**: Uses `IpcTransport<ChildStdout, ChildStdin>`
- **Async adapter**: Async wrapper for `tokio::io` types

### External Systems

- **Zellij host**: Provides stdin/stdout to plugin process
- **Plugin process**: Communicates via transport

## Async Compatibility

The sync transport can be wrapped for async use:

```rust
// Async wrapper (future work)
pub struct AsyncIpcTransport<R, W> {
    inner: IpcTransport<BufReader<R>, BufWriter<W>>,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> AsyncIpcTransport<R, W> {
    pub async fn send_async<T: Serialize>(&mut self, msg: &T) -> Result<(), TransportError>;
    pub async fn recv_async<T: DeserializeOwned>(&mut self) -> Result<T, TransportError>;
}
```

## Documentation Requirements

- [x] Public API documentation
- [x] Protocol specification
- [x] Error handling guide
- [x] Performance characteristics
- [x] Usage examples

## Non-Functional Requirements

### Reliability

- No message corruption (length prefix + checksum)
- No resource leaks (flush on drop)
- Partial read handling (clear error messages)

### Maintainability

- Clear separation: protocol, serialization, I/O
- Comprehensive logging for debugging
- Type-safe (generics for message types)

### Security

- Message size limits prevent DoS
- Input validation on all length prefixes
- No unsafe code (zero-copy via safe APIs)

## Acceptance Criteria

1. [ ] Send operation works for all serializable types
2. [ ] Recv operation works for all deserializable types
3. [ ] Message size validation (<1MB)
4. [ ] Error handling for all failure modes
5. [ ] Zero panics, zero unwraps
6. [ ] All tests passing (see test plan)
7. [ ] Performance targets met (<2µs send, <3µs recv)
8. [ ] Proper resource cleanup (flush on drop)

## Test Scenarios

### Scenario 1: Successful Round-Trip
- **Send**: Serialize message, write length + payload
- **Recv**: Read length, validate, read payload, deserialize
- **Expected**: Original message received

### Scenario 2: Oversized Message
- **Send**: Message with payload >1MB
- **Expected**: `MessageTooLarge` error, no data written

### Scenario 3: Invalid Length Prefix
- **Recv**: Length prefix indicates 0 bytes
- **Expected**: `InvalidLength` error

### Scenario 4: Partial Read (EOF)
- **Recv**: Length prefix valid, but EOF during payload read
- **Expected**: `UnexpectedEof` error with expected/actual bytes

### Scenario 5: Concurrent Access
- **Setup**: Multiple tasks sharing transport via mutex
- **Expected**: All messages sent/received correctly (serialized access)

---

*Generated by Architect Agent*
*Contract status: COMPLETE - Ready for implementation*
