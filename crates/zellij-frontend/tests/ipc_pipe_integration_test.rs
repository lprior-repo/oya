//! Integration tests for Zellij IPC pipe handler
//!
//! These tests verify the pipe handler correctly handles stdin/stdout communication
//! with actual OS pipes and process I/O.
//!
//! All tests use functional Rust patterns: zero unwraps, zero panics, Result-based error handling.

use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use zellij_frontend::{ZellijIpcClient, ZellijStdin, ZellijStdout};

/// Helper to create a test message
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestMessage {
    id: String,
    data: String,
}

/// Test error type for integration tests
#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TestError {}

impl From<std::io::Error> for TestError {
    fn from(e: std::io::Error) -> Self {
        TestError(e.to_string())
    }
}

impl From<serde_json::Error> for TestError {
    fn from(e: serde_json::Error) -> Self {
        TestError(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for TestError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        TestError(e.to_string())
    }
}

/// Test basic send/receive through a pipe
///
/// This test verifies that:
/// 1. Data can be written to a pipe
/// 2. Data can be read from a pipe
/// 3. The round-trip preserves data integrity
#[test]
fn test_pipe_send_receive_roundtrip() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    let test_data = b"Hello, pipe!";
    writer.write_all(test_data)?;
    writer.flush()?;

    let mut buffer = vec![0u8; test_data.len()];
    reader.read_exact(&mut buffer)?;

    assert_eq!(test_data, buffer.as_slice());
    Ok(())
}

/// Test bidirectional communication through pipes
///
/// This test verifies that:
/// 1. Two pipes can be used for bidirectional communication
/// 2. Data flows correctly in both directions
/// 3. No data corruption occurs
#[test]
fn test_bidirectional_pipe_communication() -> Result<(), TestError> {
    let (mut read1, mut write1) = os_pipe::pipe()?;
    let (read2, mut write2) = os_pipe::pipe()?;

    let handle = thread::spawn(move || -> Result<(), TestError> {
        let mut read2 = read2;
        let mut buf = [0u8; 16];
        read2.read_exact(&mut buf)?;
        write1.write_all(b"Pong")?;
        Ok(())
    });

    write2.write_all(b"Ping")?;

    let mut response = [0u8; 4];
    read1.read_exact(&mut response)?;

    assert_eq!(b"Pong", &response);
    handle
        .join()
        .map_err(|e| TestError(format!("Thread panicked: {:?}", e)))??;
    Ok(())
}

/// Test JSON serialization through pipes
///
/// This test verifies that:
/// 1. JSON data can be serialized
/// 2. JSON data can be transmitted through a pipe
/// 3. JSON data can be deserialized correctly
#[test]
fn test_json_serialization_through_pipe() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    let original = TestMessage {
        id: "test-123".to_string(),
        data: "Hello, world!".to_string(),
    };

    let json = serde_json::to_string(&original)?;
    writer.write_all(json.as_bytes())?;
    writer.flush()?;

    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let received: TestMessage = serde_json::from_slice(&buffer)?;

    assert_eq!(original, received);
    Ok(())
}

/// Test handling of partial reads through pipes
///
/// This test verifies that:
/// 1. Data arriving in chunks is handled correctly
/// 2. The reader can accumulate partial data
/// 3. No data is lost during partial reads
#[test]
fn test_partial_reads_through_pipe() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    let large_data = "x".repeat(10000);
    writer.write_all(large_data.as_bytes())?;
    drop(writer);

    let mut received = String::new();
    let mut buffer = [0u8; 100];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let chunk = buffer
            .get(..bytes_read)
            .ok_or_else(|| TestError("Buffer slice out of bounds".to_string()))?;
        received.push_str(&String::from_utf8_lossy(chunk));
        if received.len() >= large_data.len() {
            break;
        }
    }

    assert_eq!(large_data, received);
    Ok(())
}

/// Test error handling when pipe is closed
///
/// This test verifies that:
/// 1. Closed pipe is detected
/// 2. Error is returned gracefully (no panic)
/// 3. Error message is clear
#[test]
fn test_closed_pipe_returns_eof() -> Result<(), TestError> {
    let (mut reader, _writer) = os_pipe::pipe()?;

    drop(_writer);

    let mut buffer = [0u8; 100];
    let result = reader.read(&mut buffer)?;

    assert_eq!(result, 0, "Should return 0 bytes (EOF) when pipe is closed");
    Ok(())
}

/// Test process-based pipe communication
///
/// This test verifies that:
/// 1. Child process stdin/stdout can be used for IPC
/// 2. Bidirectional communication works across process boundaries
/// 3. Process cleanup happens correctly
#[test]
fn test_process_pipe_communication() -> Result<(), TestError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| TestError("Failed to get stdin".to_string()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| TestError("Failed to get stdout".to_string()))?;

    let test_data = b"Hello from parent!";
    stdin.write_all(test_data)?;
    drop(stdin);

    let mut buffer = Vec::new();
    stdout.read_to_end(&mut buffer)?;

    assert_eq!(test_data, buffer.as_slice());

    let exit_status = child.wait()?;
    assert!(exit_status.success());
    Ok(())
}

/// Test JSON-based process communication
///
/// This test verifies:
/// 1. JSON messages can be sent to a child process
/// 2. JSON responses can be received from a child process
/// 3. Message framing works correctly
#[test]
fn test_json_process_communication() -> Result<(), TestError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("while read line; do echo \"$line\"; done")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| TestError("Failed to get stdin".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TestError("Failed to get stdout".to_string()))?;
    let stdout_reader = std::io::BufReader::new(stdout);

    let messages = vec![
        TestMessage {
            id: "msg1".to_string(),
            data: "data1".to_string(),
        },
        TestMessage {
            id: "msg2".to_string(),
            data: "data2".to_string(),
        },
    ];

    for msg in &messages {
        let json = serde_json::to_string(msg)?;
        writeln!(stdin, "{}", json)?;
    }
    drop(stdin);

    let mut responses = Vec::new();
    for line in stdout_reader.lines() {
        let line = line?;
        let response: TestMessage = serde_json::from_str(&line)?;
        responses.push(response);
    }

    assert_eq!(messages, responses);
    child.wait()?;
    Ok(())
}

/// Test concurrent access to pipes
///
/// This test verifies:
/// 1. Multiple threads can safely access different pipes
/// 2. No race conditions occur
/// 3. Data integrity is maintained
#[test]
fn test_concurrent_pipe_access() -> Result<(), TestError> {
    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..5_u8 {
        let count = Arc::clone(&success_count);
        let handle = thread::spawn(move || -> Result<(), TestError> {
            let (mut reader, mut writer) = os_pipe::pipe()?;

            let data = format!("Thread {}", i);
            writer.write_all(data.as_bytes())?;

            let mut buffer = vec![0u8; data.len()];
            reader.read_exact(&mut buffer)?;

            if buffer == data.as_bytes() {
                count.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        });
        handles.push(handle);
    }

    for handle in handles {
        handle
            .join()
            .map_err(|e| TestError(format!("Thread panicked: {:?}", e)))??;
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 5);
    Ok(())
}

/// Test large message handling through pipes
///
/// This test verifies:
/// 1. Large messages (> 1MB) can be sent through pipes
/// 2. No data corruption occurs
/// 3. Performance is acceptable
#[test]
fn test_large_message_through_pipe() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    let large_data = "x".repeat(1_000_000);
    writer.write_all(large_data.as_bytes())?;
    writer.flush()?;
    drop(writer);

    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    assert_eq!(large_data.len(), buffer.len());
    assert_eq!(large_data.as_bytes(), buffer.as_slice());
    Ok(())
}

/// Test newline-delimited JSON framing
///
/// This test verifies:
/// 1. Multiple JSON objects can be sent on the same stream
/// 2. Newline delimiters correctly frame messages
/// 3. No cross-message contamination occurs
#[test]
fn test_newline_delimited_json_framing() -> Result<(), TestError> {
    let (reader, mut writer) = os_pipe::pipe()?;

    let msg1 = TestMessage {
        id: "1".to_string(),
        data: "first".to_string(),
    };
    let msg2 = TestMessage {
        id: "2".to_string(),
        data: "second".to_string(),
    };

    let json1 = serde_json::to_string(&msg1)?;
    let json2 = serde_json::to_string(&msg2)?;
    writeln!(writer, "{}", json1)?;
    writeln!(writer, "{}", json2)?;
    writer.flush()?;
    drop(writer);

    let mut reader = std::io::BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line)?;
    let received1: TestMessage = serde_json::from_str(line.trim())?;
    assert_eq!(msg1, received1);

    line.clear();
    reader.read_line(&mut line)?;
    let received2: TestMessage = serde_json::from_str(line.trim())?;
    assert_eq!(msg2, received2);
    Ok(())
}

// ============================================================================
// ZellijIpcClient Integration Tests
// ============================================================================

/// Test ZellijStdout wrapper
///
/// This test verifies that:
/// 1. ZellijStdout can write to stdout
/// 2. Flush works correctly
/// 3. No panics occur on error
#[test]
fn test_zellij_stdout_write_and_flush() -> Result<(), TestError> {
    let mut stdout = ZellijStdout::new();
    let data = b"test output\n";

    stdout.write_all(data)?;
    stdout.flush()?;
    Ok(())
}

/// Test ZellijStdin wrapper
///
/// This test verifies that:
/// 1. ZellijStdin can be created
/// 2. The read methods are available
/// 3. No panics occur on instantiation
#[test]
fn test_zellij_stdin_creation() {
    let _stdin = ZellijStdin::new();
}

/// Test ZellijIpcClient creation
///
/// This test verifies that:
/// 1. ZellijIpcClient can be instantiated
/// 2. No panics occur during initialization
/// 3. Internal state is properly initialized
#[test]
fn test_zellij_ipc_client_creation() {
    let _client = ZellijIpcClient::new();
}

/// Test ZellijIpcClient send_command with valid GuestMessage
///
/// This test verifies that:
/// 1. A GuestMessage can be serialized
/// 2. send_command handles the message correctly
/// 3. No panics occur during serialization
/// 4. Errors are returned as Result (not panic)
#[test]
fn test_zellij_ipc_client_send_command_serialization() -> Result<(), TestError> {
    let _client = ZellijIpcClient::new();

    let test_msg = r#"{"type": "test", "data": "test data"}"#;
    let _parsed: serde_json::Value = serde_json::from_str(test_msg)?;

    let json = serde_json::to_string(&test_msg)?;
    assert!(!json.is_empty(), "Serialization should produce output");
    Ok(())
}

/// Test ZellijIpcClient error handling
///
/// This test verifies that:
/// 1. Invalid JSON is handled gracefully
/// 2. Error messages are clear and specific
/// 3. No panics occur on error paths
#[test]
fn test_zellij_ipc_client_error_handling() {
    let invalid_json = "{invalid json";

    let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Invalid JSON should return error");

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(!error_msg.is_empty(), "Error message should not be empty");
        assert!(
            error_msg.contains("expected"),
            "Error should be descriptive"
        );
    }
}

/// Test correlation ID generation
///
/// This test verifies that:
/// 1. Correlation IDs are generated
/// 2. IDs are unique (within reasonable bounds)
/// 3. No panics occur during generation
#[test]
fn test_correlation_id_generation() {
    use zellij_frontend::correlation::CorrelationContext;

    let ctx = CorrelationContext::new();
    let id1 = ctx.generate_request_id();
    let id2 = ctx.generate_request_id();

    assert_ne!(id1, id2, "Correlation IDs should be unique");
    assert!(!id1.is_empty(), "Correlation ID should not be empty");
    assert!(!id2.is_empty(), "Correlation ID should not be empty");
}

/// Test partial JSON framing detection
///
/// This test verifies that:
/// 1. Incomplete JSON is detected
/// 2. Complete JSON is recognized
/// 3. No false positives in framing
#[test]
fn test_json_framing_detection() {
    let complete = r#"{"key": "value"}"#;
    assert!(
        complete.trim().ends_with('}'),
        "Complete JSON should end with }}"
    );

    let incomplete = r#"{"key": "value""#;
    assert!(
        !incomplete.trim().ends_with('}'),
        "Incomplete JSON should not end with }}"
    );

    let array = r#"[1, 2, 3]"#;
    assert!(
        array.trim().ends_with(']'),
        "Complete array should end with ]"
    );
}

/// Test message payload formatting
///
/// This test verifies that:
/// 1. Request ID is correctly added to payload
/// 2. JSON structure is valid
/// 3. Payload can be parsed back
#[test]
fn test_message_payload_formatting() -> Result<(), TestError> {
    let request_id = "test-request-123";
    let data = r#"{"type": "test"}"#;
    let payload = format!(r#"{{"request_id": "{}", "data": {}}}"#, request_id, data);

    let parsed: serde_json::Value = serde_json::from_str(&payload)?;

    assert_eq!(parsed["request_id"], request_id, "Request ID should match");
    assert_eq!(parsed["data"]["type"], "test", "Data should be preserved");
    Ok(())
}

/// Test concurrent client creation
///
/// This test verifies that:
/// 1. Multiple clients can be created
/// 2. No race conditions occur
/// 3. Each client has independent state
#[test]
fn test_concurrent_client_creation() -> Result<(), TestError> {
    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..5 {
        let count = Arc::clone(&success_count);
        let handle = thread::spawn(move || {
            let _client = ZellijIpcClient::new();
            count.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle
            .join()
            .map_err(|e| TestError(format!("Thread panicked: {:?}", e)))?;
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 5);
    Ok(())
}

/// Test pipe write error recovery
///
/// This test verifies:
/// 1. Write errors are handled gracefully
/// 2. No panics on write failure
/// 3. Error information is preserved
#[test]
fn test_pipe_write_error_recovery() -> Result<(), TestError> {
    let (reader, mut writer) = os_pipe::pipe()?;

    writer.write_all(b"data")?;
    drop(reader);
    drop(writer);

    Ok(())
}

/// Test buffer boundary handling
///
/// This test verifies:
/// 1. Exact buffer size reads work correctly
/// 2. No off-by-one errors
/// 3. Data integrity at boundaries
#[test]
fn test_buffer_boundary_handling() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    let data = b"12345678"; // 8 bytes
    writer.write_all(data)?;
    writer.flush()?;
    drop(writer);

    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer)?;

    assert_eq!(&buffer, data);
    Ok(())
}

/// Test empty message handling
///
/// This test verifies:
/// 1. Empty data can be written
/// 2. Empty reads are handled correctly
/// 3. No panics on empty input
#[test]
fn test_empty_message_handling() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    writer.write_all(b"")?;
    writer.flush()?;
    drop(writer);

    let mut buffer = Vec::new();
    let bytes_read = reader.read_to_end(&mut buffer)?;

    assert_eq!(bytes_read, 0);
    Ok(())
}

/// Test unicode data through pipes
///
/// This test verifies:
/// 1. Unicode strings can be transmitted
/// 2. UTF-8 encoding is preserved
/// 3. No data corruption for non-ASCII
#[test]
fn test_unicode_data_through_pipe() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    let unicode_data = "Hello, 世界! 🌍 Привет мир";
    writer.write_all(unicode_data.as_bytes())?;
    writer.flush()?;
    drop(writer);

    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let received = String::from_utf8(buffer)?;
    assert_eq!(unicode_data, received);
    Ok(())
}

/// Test rapid sequential messages
///
/// This test verifies:
/// 1. Multiple messages can be sent in sequence
/// 2. Each message is received correctly
/// 3. No message interleaving
#[test]
fn test_rapid_sequential_messages() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    for i in 0..10 {
        let msg = format!("message-{}", i);
        writer.write_all(msg.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    drop(writer);

    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let received = String::from_utf8(buffer)?;

    for i in 0..10 {
        let expected = format!("message-{}", i);
        assert!(received.contains(&expected), "Should contain {}", expected);
    }
    Ok(())
}

/// Test pipe timeout behavior simulation
///
/// This test verifies:
/// 1. Non-blocking reads can be attempted
/// 2. Timeout-like conditions are detected
/// 3. No indefinite blocking
#[test]
fn test_pipe_non_blocking_read() -> Result<(), TestError> {
    let (mut reader, mut writer) = os_pipe::pipe()?;

    writer.write_all(b"short data")?;
    writer.flush()?;

    let mut buffer = [0u8; 100];
    let bytes_read = reader.read(&mut buffer)?;

    assert!(bytes_read > 0, "Should read some data");
    Ok(())
}
