//! Happy path tests for IPC transport
//!
//! Tests verify the transport layer correctly handles normal operation.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use oya_ipc::{IpcTransport, TransportError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum HostMessage {
    BeadList(Vec<BeadSummary>),
    Error(String),
    BeadDetail(Option<BeadDetail>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BeadSummary {
    id: String,
    title: String,
    status: BeadStatus,
    priority: Priority,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum BeadStatus {
    Open,
    InProgress,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Priority {
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BeadDetail {
    id: String,
    description: String,
}

fn create_transport_pair() -> (
    IpcTransport<DuplexReader, DuplexWriter>,
    IpcTransport<DuplexReader, DuplexWriter>,
) {
    IpcTransport::transport_pair()
}

#[test]
fn test_send_recv_1kb_message_roundtrip_succeeds() {
    let (mut client, mut server) = create_transport_pair();
    let original = HostMessage::BeadList(vec![BeadSummary {
        id: "src-123".to_string(),
        title: "Test bead".to_string(),
        status: BeadStatus::Open,
        priority: Priority::P1,
    }]);

    client.send(&original).expect("send should succeed");
    let received = server.recv::<HostMessage>().expect("recv should succeed");

    assert_eq!(original, received);
}

#[test]
fn test_length_prefix_encoded_as_big_endian() {
    use std::io::{BufReader, BufWriter, Cursor};

    struct CaptureWriter {
        buffer: Vec<u8>,
    }

    impl std::io::Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let reader = std::io::empty();
    let writer = CaptureWriter { buffer: Vec::new() };

    let mut transport = IpcTransport::new(reader, writer);
    let msg = HostMessage::BeadList(vec![]);

    transport.send(&msg).unwrap();

    let written = transport.split().1.into_inner().unwrap().buffer;
    assert!(written.len() >= 4);

    let length_prefix = u32::from_be_bytes([written[0], written[1], written[2], written[3]]);

    assert_eq!(length_prefix as usize, written.len() - 4);
}

#[test]
fn test_flush_is_called_after_each_send() {
    use std::io::{BufWriter, Cursor};

    struct FlushCounter {
        buffer: Vec<u8>,
        flush_count: std::cell::RefCell<usize>,
    }

    impl std::io::Write for FlushCounter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            *self.flush_count.borrow_mut() += 1;
            Ok(())
        }
    }

    let reader = std::io::empty();
    let writer = FlushCounter {
        buffer: Vec::new(),
        flush_count: std::cell::RefCell::new(0),
    };

    let mut transport = IpcTransport::new(reader, writer);
    let msg = HostMessage::BeadList(vec![]);

    transport.send(&msg).unwrap();

    let flush_count = *transport.split().1.flush_count.borrow();
    assert!(flush_count > 0);
}

#[test]
fn test_multiple_messages_are_independently_framed() {
    let (mut client, mut server) = create_transport_pair();

    client.send(&HostMessage::BeadList(vec![])).unwrap();
    client
        .send(&HostMessage::Error("test".to_string()))
        .unwrap();
    client.send(&HostMessage::BeadDetail(None)).unwrap();

    let msg1 = server.recv::<HostMessage>().unwrap();
    let msg2 = server.recv::<HostMessage>().unwrap();
    let msg3 = server.recv::<HostMessage>().unwrap();

    assert!(matches!(msg1, HostMessage::BeadList(_)));
    assert!(matches!(msg2, HostMessage::Error(_)));
    assert!(matches!(msg3, HostMessage::BeadDetail(_)));
}

#[test]
fn test_message_at_exactly_1mb_limit_succeeds() {
    let (mut client, mut server) = create_transport_pair();

    // Create a message that serializes to ~1MB
    let large_string = "x".repeat(1_000_000);
    let large_msg = HostMessage::Error(large_string);

    client.send(&large_msg).expect("send should succeed");
    let received = server.recv::<HostMessage>().expect("recv should succeed");

    assert_eq!(large_msg, received);
}

#[test]
fn test_bidirectional_send_recv_in_both_directions() {
    let (mut client, mut server) = create_transport_pair();

    let client_msg = HostMessage::BeadList(vec![]);
    let server_msg = HostMessage::Error("ack".to_string());

    client.send(&client_msg).unwrap();
    server.send(&server_msg).unwrap();

    let server_received = server.recv::<HostMessage>().unwrap();
    let client_received = client.recv::<HostMessage>().unwrap();

    assert_eq!(server_received, client_msg);
    assert_eq!(client_received, server_msg);
}
