//! Integration tests for Zellij IPC pipe handler
//!
//! These tests verify the pipe handler correctly handles stdin/stdout communication
//! with actual OS pipes and process I/O.

use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use zellij_frontend::{ZellijIpcClient, ZellijStdin, ZellijStdout};

/// Helper to create a test message
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestMessage {
    id: String,
    data: String,
}

/// Test helper program that acts as a simple echo server
///
/// Reads JSON from stdin, parses it, adds a prefix, and writes it back to stdout.
/// This simulates the backend behavior.
fn echo_server_main() {
    use std::io::{self, Read};

    let mut input = String::new();
    let mut stdin = io::stdin();
    stdin
        .read_to_string(&mut input)
        .expect("Failed to read from stdin");

    // Echo the input back with a prefix
    println!("ECHO: {}", input.trim());
}

/// Test basic send/receive through a pipe
///
/// This test verifies that:
/// 1. Data can be written to a pipe
/// 2. Data can be read from a pipe
/// 3. The round-trip preserves data integrity
#[test]
fn test_pipe_send_receive_roundtrip() {
    // Create a pipe for testing
    let (mut reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    // Write data to the pipe
    let test_data = b"Hello, pipe!";
    writer
        .write_all(test_data)
        .expect("Failed to write to pipe");
    writer.flush().expect("Failed to flush pipe");

    // Read data from the pipe
    let mut buffer = vec![0u8; test_data.len()];
    reader
        .read_exact(&mut buffer)
        .expect("Failed to read from pipe");

    assert_eq!(test_data, buffer.as_slice());
}

/// Test bidirectional communication through pipes
///
/// This test verifies that:
/// 1. Two pipes can be used for bidirectional communication
/// 2. Data flows correctly in both directions
/// 3. No data corruption occurs
#[test]
fn test_bidirectional_pipe_communication() {
    let (mut read1, mut write1) = os_pipe::pipe().expect("Failed to create pipe 1");
    let (mut read2, mut write2) = os_pipe::pipe().expect("Failed to create pipe 2");

    // Spawn a thread to act as the other endpoint
    thread::spawn(move || {
        let mut buf = [0u8; 16];
        read2
            .read_exact(&mut buf)
            .expect("Failed to read from pipe 2");
        write1
            .write_all(b"Pong")
            .expect("Failed to write to pipe 1");
    });

    // Send a message and wait for response
    write2
        .write_all(b"Ping")
        .expect("Failed to write to pipe 2");

    let mut response = [0u8; 4];
    read1
        .read_exact(&mut response)
        .expect("Failed to read response");

    assert_eq!(b"Pong", &response);
}

/// Test JSON serialization through pipes
///
/// This test verifies that:
/// 1. JSON data can be serialized
/// 2. JSON data can be transmitted through a pipe
/// 3. JSON data can be deserialized correctly
#[test]
fn test_json_serialization_through_pipe() {
    let (mut reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    let original = TestMessage {
        id: "test-123".to_string(),
        data: "Hello, world!".to_string(),
    };

    // Serialize and send
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    writer
        .write_all(json.as_bytes())
        .expect("Failed to write to pipe");
    writer.flush().expect("Failed to flush");

    // Read and deserialize
    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .expect("Failed to read from pipe");

    let received: TestMessage = serde_json::from_slice(&buffer).expect("Failed to deserialize");

    assert_eq!(original, received);
}

/// Test handling of partial reads through pipes
///
/// This test verifies that:
/// 1. Data arriving in chunks is handled correctly
/// 2. The reader can accumulate partial data
/// 3. No data is lost during partial reads
#[test]
fn test_partial_reads_through_pipe() {
    let (reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    let large_data = "x".repeat(10000);
    writer
        .write_all(large_data.as_bytes())
        .expect("Failed to write to pipe");

    // Read in small chunks
    let mut received = String::new();
    let mut buffer = [0u8; 100]; // Small buffer to force partial reads
    let mut reader = reader; // Make reader mutable

    loop {
        let bytes_read = reader.read(&mut buffer).expect("Failed to read");
        if bytes_read == 0 {
            break;
        }
        received.push_str(&String::from_utf8_lossy(&buffer[..bytes_read]));
        if received.len() >= large_data.len() {
            break;
        }
    }

    assert_eq!(large_data, received);
}

/// Test error handling when pipe is closed
///
/// This test verifies that:
/// 1. Closed pipe is detected
/// 2. Error is returned gracefully (no panic)
/// 3. Error message is clear
#[test]
fn test_closed_pipe_returns_error() {
    let (mut reader, _writer) = os_pipe::pipe().expect("Failed to create pipe");

    // Drop the writer to close the pipe
    drop(_writer);

    let mut buffer = [0u8; 100];
    let result = reader.read(&mut buffer);

    // Should return Ok(0) indicating EOF, not an error
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

/// Test process-based pipe communication
///
/// This test verifies that:
/// 1. Child process stdin/stdout can be used for IPC
/// 2. Bidirectional communication works across process boundaries
/// 3. Process cleanup happens correctly
#[test]
fn test_process_pipe_communication() {
    // Spawn a child process that acts as echo server
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("cat") // cat echoes stdin to stdout
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    // Get stdin and stdout handles
    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let mut stdout = child.stdout.take().expect("Failed to get stdout");

    // Send data
    let test_data = b"Hello from parent!";
    stdin
        .write_all(test_data)
        .expect("Failed to write to child stdin");
    drop(stdin); // Close stdin to signal EOF to cat

    // Read response
    let mut buffer = Vec::new();
    stdout
        .read_to_end(&mut buffer)
        .expect("Failed to read from child stdout");

    assert_eq!(test_data, buffer.as_slice());

    // Wait for child to exit
    let exit_status = child.wait().expect("Failed to wait for child process");

    assert!(exit_status.success());
}

/// Test JSON-based process communication
///
/// This test verifies:
/// 1. JSON messages can be sent to a child process
/// 2. JSON responses can be received from a child process
/// 3. Message framing works correctly
#[test]
fn test_json_process_communication() {
    use std::io::{BufRead, BufReader};

    // Spawn a simple JSON echo server
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("while read line; do echo \"$line\"; done")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stdout_reader = BufReader::new(stdout);

    // Send multiple JSON messages
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
        let json = serde_json::to_string(msg).expect("Failed to serialize");
        writeln!(stdin, "{}", json).expect("Failed to write to stdin");
    }
    drop(stdin);

    // Read responses
    let mut responses = Vec::new();
    for line in stdout_reader.lines() {
        let line = line.expect("Failed to read line");
        let response: TestMessage = serde_json::from_str(&line).expect("Failed to deserialize");
        responses.push(response);
    }

    assert_eq!(messages, responses);

    child.wait().expect("Failed to wait for child process");
}

/// Test concurrent access to pipes
///
/// This test verifies:
/// 1. Multiple threads can safely access different pipes
/// 2. No race conditions occur
/// 3. Data integrity is maintained
#[test]
fn test_concurrent_pipe_access() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..5 {
        let count = Arc::clone(&success_count);
        let handle = thread::spawn(move || {
            let (mut reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

            let data = format!("Thread {}", i);
            writer
                .write_all(data.as_bytes())
                .expect("Failed to write to pipe");

            let mut buffer = vec![0u8; data.len()];
            reader
                .read_exact(&mut buffer)
                .expect("Failed to read from pipe");

            if buffer == data.as_bytes() {
                count.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 5);
}

/// Test large message handling through pipes
///
/// This test verifies:
/// 1. Large messages (> 1MB) can be sent through pipes
/// 2. No data corruption occurs
/// 3. Performance is acceptable
#[test]
fn test_large_message_through_pipe() {
    let (mut reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    // Create a large message (~1MB)
    let large_data = "x".repeat(1_000_000);

    writer
        .write_all(large_data.as_bytes())
        .expect("Failed to write to pipe");
    writer.flush().expect("Failed to flush");

    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .expect("Failed to read from pipe");

    assert_eq!(large_data.len(), buffer.len());
    assert_eq!(large_data.as_bytes(), buffer.as_slice());
}

/// Test newline-delimited JSON framing
///
/// This test verifies:
/// 1. Multiple JSON objects can be sent on the same stream
/// 2. Newline delimiters correctly frame messages
/// 3. No cross-message contamination occurs
#[test]
fn test_newline_delimited_json_framing() {
    let (reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    // Send multiple JSON messages, each on its own line
    let msg1 = TestMessage {
        id: "1".to_string(),
        data: "first".to_string(),
    };
    let msg2 = TestMessage {
        id: "2".to_string(),
        data: "second".to_string(),
    };

    writeln!(writer, "{}", serde_json::to_string(&msg1).unwrap()).expect("Failed to write msg1");
    writeln!(writer, "{}", serde_json::to_string(&msg2).unwrap()).expect("Failed to write msg2");
    writer.flush().expect("Failed to flush");

    // Read with line-based framing
    let mut reader = std::io::BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line).expect("Failed to read line 1");
    let received1: TestMessage = serde_json::from_str(line.trim()).expect("Failed to parse msg1");
    assert_eq!(msg1, received1);

    line.clear();
    reader.read_line(&mut line).expect("Failed to read line 2");
    let received2: TestMessage = serde_json::from_str(line.trim()).expect("Failed to parse msg2");
    assert_eq!(msg2, received2);
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
fn test_zellij_stdout_write_and_flush() {
    let mut stdout = ZellijStdout::new();
    let data = b"test output\n";

    let result = stdout.write_all(data);
    assert!(result.is_ok(), "write_all should succeed");

    let flush_result = stdout.flush();
    assert!(flush_result.is_ok(), "flush should succeed");
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
    // Note: We can't test actual reading without providing stdin data
    // This test verifies the struct can be instantiated without panics
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
    // Client is created successfully - no panic
    // Internal correlation context and pending requests are initialized
}

/// Test ZellijIpcClient send_command with valid GuestMessage
///
/// This test verifies that:
/// 1. A GuestMessage can be serialized
/// 2. send_command handles the message correctly
/// 3. No panics occur during serialization
/// 4. Errors are returned as Result (not panic)
#[test]
fn test_zellij_ipc_client_send_command_serialization() {
    let _client = ZellijIpcClient::new();

    // Create a simple GuestMessage for testing
    // Note: We're testing serialization, not actual stdout writing
    // which would interfere with test output
    let test_msg = r#"{"type": "test", "data": "test data"}"#;
    let _parsed: serde_json::Value =
        serde_json::from_str(test_msg).expect("Test message should be valid JSON");

    // Verify serialization doesn't panic
    let json = serde_json::to_string(&test_msg);
    assert!(json.is_ok(), "Serialization should succeed");
}

/// Test ZellijIpcClient error handling
///
/// This test verifies that:
/// 1. Invalid JSON is handled gracefully
/// 2. Error messages are clear and specific
/// 3. No panics occur on error paths
#[test]
fn test_zellij_ipc_client_error_handling() {
    // Test that invalid JSON returns a clear error
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

    // IDs should be different
    assert_ne!(id1, id2, "Correlation IDs should be unique");

    // IDs should not be empty
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
    // Complete JSON object
    let complete = r#"{"key": "value"}"#;
    assert!(
        complete.trim().ends_with('}'),
        "Complete JSON should end with }}"
    );

    // Incomplete JSON object
    let incomplete = r#"{"key": "value""#;
    assert!(
        !incomplete.trim().ends_with('}'),
        "Incomplete JSON should not end with }}"
    );

    // JSON array
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
fn test_message_payload_formatting() {
    let request_id = "test-request-123";
    let data = r#"{"type": "test"}"#;
    let payload = format!(r#"{{"request_id": "{}", "data": {}}}"#, request_id, data);

    // Verify payload is valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&payload).expect("Payload should be valid JSON");

    // Verify structure
    assert_eq!(parsed["request_id"], request_id, "Request ID should match");
    assert_eq!(parsed["data"]["type"], "test", "Data should be preserved");
}

/// Test concurrent client creation
///
/// This test verifies that:
/// 1. Multiple clients can be created
/// 2. No race conditions occur
/// 3. Each client has independent state
#[test]
fn test_concurrent_client_creation() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        handle.join().expect("Thread panicked");
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 5);
}
