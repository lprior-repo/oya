//! Error path tests for IPC transport
//!
//! Tests verify the transport layer correctly handles error conditions.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use oya_ipc::{IpcTransport, TransportError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum HostMessage {
    BeadList(Vec<BeadSummary>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BeadSummary {
    id: String,
    title: String,
}

fn create_transport_pair() -> (
    IpcTransport<DuplexReader, DuplexWriter>,
    IpcTransport<DuplexReader, DuplexWriter>,
) {
    IpcTransport::transport_pair()
}

#[test]
fn test_send_message_exceeding_1mb_returns_error() {
    let (mut client, _server) = create_transport_pair();

    // Create a message larger than 1MB
    let oversized_msg = HostMessage::Error("x".repeat(1_048_577));

    let result = client.send(&oversized_msg);

    assert!(matches!(
        result,
        Err(TransportError::MessageTooLarge { .. })
    ));

    if let Err(TransportError::MessageTooLarge {
        actual_size,
        max_size,
    }) = result
    {
        assert!(*actual_size > 1_048_576);
        assert_eq!(*max_size, 1_048_576);
    }
}

#[test]
fn test_recv_with_invalid_length_prefix_returns_error() {
    use std::io::Write;

    struct InvalidPrefixReader {
        sent_prefix: bool,
    }

    impl std::io::Read for InvalidPrefixReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if !self.sent_prefix && buf.len() >= 4 {
                // Write length prefix = 2MB
                let length = 2_097_152u32;
                buf[0..4].copy_from_slice(&length.to_be_bytes());
                self.sent_prefix = true;
                return Ok(4);
            }
            Ok(0) // EOF
        }
    }

    let reader = InvalidPrefixReader { sent_prefix: false };
    let writer = std::io::sink();

    let mut transport = IpcTransport::new(reader, writer);

    let result: Result<HostMessage, _> = transport.recv();

    assert!(matches!(result, Err(TransportError::InvalidLength { .. })));

    if let Err(TransportError::InvalidLength { length, reason }) = result {
        assert_eq!(length, 2_097_152);
        assert!(reason.contains("exceeds maximum"));
    }
}

#[test]
fn test_recv_with_zero_length_prefix_returns_error() {
    use std::io::Write;

    struct ZeroPrefixReader {
        sent_prefix: bool,
    }

    impl std::io::Read for ZeroPrefixReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if !self.sent_prefix && buf.len() >= 4 {
                // Write length prefix = 0
                buf[0..4].copy_from_slice(&0u32.to_be_bytes());
                self.sent_prefix = true;
                return Ok(4);
            }
            Ok(0) // EOF
        }
    }

    let reader = ZeroPrefixReader { sent_prefix: false };
    let writer = std::io::sink();

    let mut transport = IpcTransport::new(reader, writer);

    let result: Result<HostMessage, _> = transport.recv();

    assert!(matches!(result, Err(TransportError::InvalidLength { .. })));

    if let Err(TransportError::InvalidLength { length, reason }) = result {
        assert_eq!(length, 0);
        assert!(reason.contains("zero-length"));
    }
}

#[test]
fn test_recv_eof_during_length_prefix_returns_error() {
    struct EarlyEofReader {
        bytes_sent: usize,
    }

    impl std::io::Read for EarlyEofReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.bytes_sent < 2 {
                let to_send = std::cmp::min(2 - self.bytes_sent, buf.len());
                self.bytes_sent += to_send;
                return Ok(to_send);
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "EOF",
            ))
        }
    }

    let reader = EarlyEofReader { bytes_sent: 0 };
    let writer = std::io::sink();

    let mut transport = IpcTransport::new(reader, writer);

    let result: Result<HostMessage, _> = transport.recv();

    assert!(matches!(result, Err(TransportError::UnexpectedEof { .. })));

    if let Err(TransportError::UnexpectedEof {
        bytes_read,
        expected_bytes,
    }) = result
    {
        assert_eq!(*bytes_read, 0); // No complete prefix read
        assert_eq!(*expected_bytes, 4);
    }
}

#[test]
fn test_recv_eof_during_payload_returns_error() {
    struct TruncatedPayloadReader {
        state: usize,
    }

    impl std::io::Read for TruncatedPayloadReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.state {
                0 => {
                    // Send length prefix = 1000
                    if buf.len() >= 4 {
                        buf[0..4].copy_from_slice(&1000u32.to_be_bytes());
                        self.state = 1;
                        Ok(4)
                    } else {
                        Ok(0)
                    }
                }
                1 => {
                    // Send only 500 bytes of payload
                    let to_send = std::cmp::min(500, buf.len());
                    self.state = 2;
                    Ok(to_send)
                }
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "EOF",
                )),
            }
        }
    }

    let reader = TruncatedPayloadReader { state: 0 };
    let writer = std::io::sink();

    let mut transport = IpcTransport::new(reader, writer);

    let result: Result<HostMessage, _> = transport.recv();

    assert!(matches!(result, Err(TransportError::UnexpectedEof { .. })));

    if let Err(TransportError::UnexpectedEof {
        bytes_read,
        expected_bytes,
    }) = result
    {
        assert_eq!(*bytes_read, 504); // 4 bytes prefix + 500 bytes payload
        assert_eq!(*expected_bytes, 1004); // 4 bytes prefix + 1000 bytes payload
    }
}

#[test]
fn test_recv_with_corrupted_payload_returns_error() {
    struct CorruptedPayloadReader {
        state: usize,
    }

    impl std::io::Read for CorruptedPayloadReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.state {
                0 => {
                    // Send length prefix = 100
                    if buf.len() >= 4 {
                        buf[0..4].copy_from_slice(&100u32.to_be_bytes());
                        self.state = 1;
                        Ok(4)
                    } else {
                        Ok(0)
                    }
                }
                1 => {
                    // Send invalid bincode payload (all zeros)
                    let payload = vec![0u8; 100];
                    let to_send = std::cmp::min(payload.len(), buf.len());
                    buf[..to_send].copy_from_slice(&payload[..to_send]);
                    self.state = 2;
                    Ok(to_send)
                }
                _ => Ok(0),
            }
        }
    }

    let reader = CorruptedPayloadReader { state: 0 };
    let writer = std::io::sink();

    let mut transport = IpcTransport::new(reader, writer);

    let result: Result<HostMessage, _> = transport.recv();

    assert!(matches!(
        result,
        Err(TransportError::DeserializationFailed { .. })
    ));

    if let Err(TransportError::DeserializationFailed {
        cause,
        payload_bytes,
    }) = result
    {
        assert!(!cause.is_empty());
        assert_eq!(*payload_bytes, 100);
    }
}

#[test]
fn test_send_with_failing_writer_returns_error() {
    struct FailingWriter;

    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "broken pipe",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "broken pipe",
            ))
        }
    }

    let reader = std::io::empty();
    let writer = FailingWriter;

    let mut transport = IpcTransport::new(reader, writer);

    let msg = HostMessage::BeadList(vec![]);

    let result = transport.send(&msg);

    assert!(matches!(result, Err(TransportError::WriteFailed { .. })));
}
