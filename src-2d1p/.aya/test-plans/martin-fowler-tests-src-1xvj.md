# Martin Fowler Test Plan: Length-Prefixed Buffer Transport Layer

**Bead ID:** `src-1xvj`
**Generated:** 2026-02-07 23:28:02
**Reference:** [Martin Fowler's Test Patterns](https://martinfowler.com/bliki/TestPyramid.html)
**Type:** Feature - IPC Transport Protocol

## Test Strategy Overview

This test plan follows Martin Fowler's testing philosophy with a balanced test pyramid:
- **Unit tests**: Fast, isolated, numerous (send/recv, error handling)
- **Integration tests**: Slower, realistic interactions (real I/O, pipes)
- **End-to-end tests**: Slowest, critical paths only (full protocol)

## Test Categories

### 1. Unit Tests (70% of tests)

#### 1.1 Send/Recv Round-Trip Tests

```rust
#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_send_recv_roundtrip_string() {
        let mut buffer = Vec::new();
        {
            let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
            transport.send(&"Hello, World!".to_string()).unwrap();
        }

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let received: String = transport.recv().unwrap();
        assert_eq!(received, "Hello, World!");
    }

    #[test]
    fn test_send_recv_roundtrip_struct() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestMessage {
            id: String,
            value: i32,
        }

        let original = TestMessage {
            id: "test-123".to_string(),
            value: 42,
        };

        let mut buffer = Vec::new();
        {
            let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
            transport.send(&original).unwrap();
        }

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let received: TestMessage = transport.recv().unwrap();
        assert_eq!(received, original);
    }

    #[test]
    fn test_send_recv_multiple_messages() {
        let messages = vec
![1, 2, 3, 4, 5];

        let mut buffer = Vec::new();
        {
            let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
            for msg in &messages {
                transport.send(msg).unwrap();
            }
        }

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let mut received = Vec::new();
        for _ in 0..messages.len() {
            received.push(transport.recv::<i32>().unwrap());
        }
        assert_eq!(received, messages);
    }

    #[test]
    fn test_send_recv_empty_string() {
        let mut buffer = Vec::new();
        {
            let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
            transport.send(&"".to_string()).unwrap();
        }

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let received: String = transport.recv().unwrap();
        assert_eq!(received, "");
    }

    #[test]
    fn test_send_recv_large_message() {
        let large_data = vec
![42u8; 100_000]; // 100KB

        let mut buffer = Vec::new();
        {
            let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
            transport.send(&large_data).unwrap();
        }

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let received: Vec<u8> = transport.recv().unwrap();
        assert_eq!(received, large_data);
    }
}
```

**Coverage goal:** >90% of send/recv code

#### 1.2 Message Size Validation Tests

```rust
#[cfg(test)]
mod size_validation_tests {
    use super::*;

    #[test]
    fn test_send_oversized_message_fails() {
        let oversized = vec
![0u8; 2_000_000]; // 2MB > 1MB limit

        let mut buffer = Vec::new();
        let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);

        let result = transport.send(&oversized);
        assert!(result.is_err());

        match result.unwrap_err() {
            TransportError::MessageTooLarge { size, max } => {
                assert_eq!(size, 2_000_000);
                assert_eq!(max, 1_048_576);
            }
            _ => panic!("Expected MessageTooLarge error"),
        }
    }

    #[test]
    fn test_recv_oversized_message_fails() {
        let mut buffer = Vec::new();
        // Write length prefix for 2MB message
        let length = 2_000_000u32;
        buffer.extend_from_slice(&length.to_be_bytes());
        // Write partial payload
        buffer.extend_from_slice(&[0u8; 1000]);

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let result: Result<Vec<u8>, _> = transport.recv();

        assert!(result.is_err());
        match result.unwrap_err() {
            TransportError::MessageTooLarge { .. } => {
                // Expected
            }
            _ => panic!("Expected MessageTooLarge error"),
        }
    }

    #[test]
    fn test_recv_zero_length_message_fails() {
        let mut buffer = Vec::new();
        // Write zero length prefix
        buffer.extend_from_slice(&0u32.to_be_bytes());

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let result: Result<String, _> = transport.recv();

        assert!(result.is_err());
        match result.unwrap_err() {
            TransportError::InvalidLength(msg) => {
                assert!(msg.contains("zero"));
            }
            _ => panic!("Expected InvalidLength error"),
        }
    }

    #[test]
    fn test_send_exact_max_size_succeeds() {
        let max_size = 1_048_576; // Exactly 1MB
        let data = vec
![42u8; max_size];

        let mut buffer = Vec::new();
        let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);

        assert!(transport.send(&data).is_ok());
    }

    #[test]
    fn test_send_one_byte_over_max_fails() {
        let one_over = 1_048_577; // 1MB + 1 byte
        let data = vec
![42u8; one_over];

        let mut buffer = Vec::new();
        let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);

        assert!(transport.send(&data).is_err());
    }
}
```

#### 1.3 Partial Read / EOF Tests

```rust
#[cfg(test)]
mod partial_read_tests {
    use super::*;

    #[test]
    fn test_recv_eof_during_length_prefix() {
        // Only 2 bytes of length prefix (need 4)
        let buffer = vec
![0u8, 1u8];

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let result: Result<String, _> = transport.recv();

        assert!(result.is_err());
        match result.unwrap_err() {
            TransportError::UnexpectedEof { expected, actual } => {
                assert_eq!(expected, 4);
                assert_eq!(actual, 0);
            }
            _ => panic!("Expected UnexpectedEof error"),
        }
    }

    #[test]
    fn test_recv_eof_during_payload() {
        let mut buffer = Vec::new();
        // Write valid length prefix
        buffer.extend_from_slice(&1000u32.to_be_bytes());
        // Write only 500 bytes of payload (need 1000)
        buffer.extend_from_slice(&[0u8; 500]);

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let result: Result<Vec<u8>, _> = transport.recv();

        assert!(result.is_err());
        match result.unwrap_err() {
            TransportError::UnexpectedEof { expected, actual } => {
                assert_eq!(expected, 1000);
                assert_eq!(actual, 500);
            }
            _ => panic!("Expected UnexpectedEof error"),
        }
    }

    #[test]
    fn test_recv_empty_buffer() {
        let buffer = Vec::new();

        let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
        let result: Result<String, _> = transport.recv();

        assert!(result.is_err());
        match result.unwrap_err() {
            TransportError::UnexpectedEof { .. } => {
                // Expected
            }
            _ => panic!("Expected UnexpectedEof error"),
        }
    }
}
```

#### 1.4 Flush Behavior Tests

```rust
#[cfg(test)]
mod flush_tests {
    use super::*;

    #[test]
    fn test_send_without_flush_does_not_write() {
        let buffer = RefCell::new(Vec::new());

        {
            let mut transport = IpcTransport::new(
                Cursor::new(&[]),
                &buffer,
            );

            // Send without accessing buffer (should be buffered)
            transport.send(&"test".to_string()).unwrap();

            // Buffer should not be written yet (still in BufWriter)
            assert_eq!(buffer.borrow().len(), 0);
        }

        // Drop triggers flush
        assert!(buffer.borrow().len() > 0);
    }

    #[test]
    fn test_explicit_flush_writes_buffer() {
        let buffer = RefCell::new(Vec::new());
        let mut transport = IpcTransport::new(
            Cursor::new(&[]),
            &buffer,
        );

        transport.send(&"test".to_string()).unwrap();
        transport.flush().unwrap();

        assert!(buffer.borrow().len() > 0);
    }
}
```

#### 1.5 Property-Based Tests

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_roundtrip_preserves_data(
            data in prop::collection::vec(any::<u8>(), 0..1_000_000)
        ) {
            let mut buffer = Vec::new();
            {
                let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
                transport.send(&data).unwrap();
            }

            let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
            let received: Vec<u8> = transport.recv().unwrap();
            assert_eq!(received, data);
        }

        #[test]
        fn prop_multiple_messages_roundtrip(
            messages in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..10000),
                1..100
            )
        ) {
            let mut buffer = Vec::new();
            {
                let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
                for msg in &messages {
                    transport.send(msg).unwrap();
                }
            }

            let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
            for expected_msg in &messages {
                let received: Vec<u8> = transport.recv().unwrap();
                assert_eq!(received, *expected_msg);
            }
        }

        #[test]
        fn prop_string_roundtrip(s in "[a-zA-Z0-9]{0,10000}") {
            let mut buffer = Vec::new();
            {
                let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
                transport.send(&s).unwrap();
            }

            let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
            let received: String = transport.recv().unwrap();
            assert_eq!(received, s);
        }
    }
}
```

### 2. Integration Tests (20% of tests)

#### 2.1 Pipe I/O Tests

```rust
#[cfg(test)]
mod pipe_tests {
    use super::*;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_send_recv_over_pipe() {
        let (mut reader, mut writer) = os_pipe::pipe().unwrap();

        let send_handle = std::thread::spawn(move || {
            let mut transport = IpcTransport::new(std::io::empty(), writer);
            transport.send(&"Hello via pipe!".to_string())
        });

        let recv_handle = std::thread::spawn(move || {
            let mut transport = IpcTransport::new(reader, Vec::new());
            transport.recv::<String>()
        });

        assert!(send_handle.join().unwrap().is_ok());
        let received = recv_handle.join().unwrap().unwrap();
        assert_eq!(received, "Hello via pipe!");
    }

    #[test]
    fn test_bidirectional_communication() {
        let (reader1, writer1) = os_pipe::pipe().unwrap();
        let (reader2, writer2) = os_pipe::pipe().unwrap();

        let handle1 = std::thread::spawn(move || {
            let mut transport = IpcTransport::new(reader1, writer2);
            transport.send(&"ping".to_string()).unwrap();
            transport.recv::<String>().unwrap()
        });

        let handle2 = std::thread::spawn(move || {
            let mut transport = IpcTransport::new(reader2, writer1);
            let msg = transport.recv::<String>().unwrap();
            transport.send(&"pong".to_string()).unwrap();
            msg
        });

        assert_eq!(handle1.join().unwrap(), "pong");
        assert_eq!(handle2.join().unwrap(), "ping");
    }
}
```

#### 2.2 Concurrent Access Tests

```rust
#[tokio::test]
async fn test_concurrent_send_recv() {
    let (reader, writer) = os_pipe::pipe().unwrap();
    let transport = Arc::new(Mutex::new(IpcTransport::new(reader, writer)));

    let mut handles = vec
![];

    // Spawn multiple tasks sending messages
    for i in 0..10 {
        let transport_clone = Arc::clone(&transport);
        let handle = tokio::spawn(async move {
            let mut t = transport_clone.lock().unwrap();
            t.send(&i).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all sends to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all messages were sent
    let mut t = transport.lock().unwrap();
    for i in 0..10 {
        let received: i32 = t.recv().unwrap();
        assert_eq!(received, i);
    }
}
```

### 3. End-to-End Tests (10% of tests)

#### 3.1 Full Protocol Tests

```rust
#[test]
fn e2e_full_protocol_with_serialization_errors() {
    // Create message with invalid serialization (e.g., map with non-string keys)
    use std::collections::HashMap;

    let mut bad_map = HashMap::new();
    bad_map.insert(123, "value"); // Non-string key

    let mut buffer = Vec::new();
    let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);

    let result = transport.send(&bad_map);
    assert!(result.is_err());

    match result.unwrap_err() {
        TransportError::SerializationFailed(_) => {
            // Expected
        }
        _ => panic!("Expected SerializationFailed error"),
    }
}
```

#### 3.2 Stress Tests

```rust
#[test]
fn e2e_stress_test_many_messages() {
    let message_count = 10_000;
    let messages: Vec<i32> = (0..message_count).collect();

    let mut buffer = Vec::new();
    {
        let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
        for msg in &messages {
            transport.send(msg).unwrap();
        }
    }

    let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
    for expected in &messages {
        let received: i32 = transport.recv().unwrap();
        assert_eq!(received, *expected);
    }
}
```

### 4. Performance Tests

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

    fn benchmark_send(c: &mut Criterion) {
        let mut group = c.benchmark_group("send");

        for size in [64, 1024, 10_240].iter() {
            let data = vec
![42u8; *size];

            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, _| {
                    let mut buffer = Vec::new();
                    let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);

                    b.iter(|| {
                        transport.send(black_box(&data)).unwrap();
                        buffer.clear();
                    })
                },
            );
        }

        group.finish();
    }

    fn benchmark_recv(c: &mut Criterion) {
        let mut group = c.benchmark_group("recv");

        for size in [64, 1024, 10_240].iter() {
            let data = vec
![42u8; *size];
            let mut buffer = Vec::new();
            {
                let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
                transport.send(&data).unwrap();
            }

            group.bench_with_input(
                BenchmarkId::from_parameter(size),
                size,
                |b, _| {
                    let mut transport = IpcTransport::new(Cursor::new(buffer.clone()), Vec::new());

                    b.iter(|| {
                        black_box(transport.recv::<Vec<u8>>().unwrap())
                    })
                },
            );
        }

        group.finish();
    }

    fn benchmark_roundtrip(c: &mut Criterion) {
        let data = vec
![42u8; 1024]; // 1KB

        c.bench_function("roundtrip_1kb", |b| {
            b.iter(|| {
                let mut buffer = Vec::new();
                {
                    let mut transport = IpcTransport::new(Cursor::new(&[]), &mut buffer);
                    transport.send(black_box(&data)).unwrap();
                }

                let mut transport = IpcTransport::new(Cursor::new(buffer), Vec::new());
                black_box(transport.recv::<Vec<u8>>().unwrap())
            })
        });
    }

    criterion_group!(benches, benchmark_send, benchmark_recv, benchmark_roundtrip);
    criterion_main!(benches);
}
```

## Test Organization

```
crates/oya-ipc/
├── tests/
│   ├── unit/                         # Pure logic tests
│   │   ├── roundtrip.rs              # Send/recv round-trip
│   │   ├── size_validation.rs        # Message size limits
│   │   ├── partial_read.rs           # EOF handling
│   │   ├── flush_behavior.rs         # Buffer flushing
│   │   └── properties.rs             # Property-based tests
│   ├── integration/                  # Real I/O
│   │   ├── pipe_tests.rs             # Pipe communication
│   │   └── concurrent_access.rs      # Multi-threaded access
│   └── e2e/                          # Full protocol tests
│       ├── serialization_errors.rs   # Error handling
│       └── stress_tests.rs           # Large message counts
├── benches/
│   └── transport.rs                  # Performance benchmarks
└── src/
    └── transport.rs                  # Includes unit tests as mod tests
```

## Test Data Management

### Fixtures

```rust
#[fixture]
fn test_transport() -> (Cursor<Vec<u8>>, Vec<u8>, IpcTransport<Cursor<Vec<u8>>, Vec<u8>>) {
    let reader = Cursor::new(Vec::new());
    let writer = Vec::new();
    let transport = IpcTransport::new(reader, writer);
    (reader, writer, transport)
}
```

### Test Factories

```rust
fn create_message(size: usize) -> Vec<u8> {
    vec
![42u8; size]
}

fn create_test_messages(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| format!("message-{}", i))
        .collect()
}
```

## Mock Strategy

- **Use in-memory buffers** for unit tests (fast, deterministic)
- **Use real pipes** for integration tests (realistic, still fast)
- **No mocks for serialization** (use real bincode)
- **No mocks for I/O** (use Cursor or real pipes)

## Test Execution

```bash
# Unit tests only (fast feedback)
moon run :test-unit --package oya-ipc

# Integration tests
moon run :test-integration --package oya-ipc

# Full test suite
moon run :test --package oya-ipc

# With coverage
moon run :test-coverage --package oya-ipc

# Performance benchmarks
moon run :bench --package oya-ipc
```

## Acceptance Criteria

1. [ ] All unit tests passing (>90% coverage)
2. [ ] All integration tests passing
3. [ ] All E2E tests passing
4. [ ] Property tests finding no counterexamples
5. [ ] Performance benchmarks meet targets:
   - send() <2µs for 1KB message
   - recv() <3µs for 1KB message
   - roundtrip <5µs for 1KB message
6. [ ] Zero flaky tests (100% deterministic)
7. [ ] Message size validation works
8. [ ] Partial read handling works
9. [ ] Flush on drop works
10. [ ] Concurrent access works

## Test Metrics

| Metric | Target | Actual |
|--------|--------|--------|
| Unit test coverage | >90% | TBD |
| Integration test count | 20% of unit tests | TBD |
| E2E test count | 10% of unit tests | TBD |
| Test execution time | <30s (unit) | TBD |
| Flaky test rate | 0% | TBD |
| Property test cases | 1000+ | TBD |
| Message sizes tested | 1B - 1MB | ✓ |

## Test Checklist

### Unit Tests
- [ ] Round-trip (string, struct, multiple messages)
- [ ] Empty message handling
- [ ] Large message (100KB)
- [ ] Oversized message rejection (>1MB)
- [ ] Zero-length message rejection
- [ ] Exact max size (1MB)
- [ ] One byte over max rejection
- [ ] EOF during length prefix
- [ ] EOF during payload
- [ ] Empty buffer handling
- [ ] Flush behavior (implicit and explicit)
- [ ] Property tests (roundtrip, multiple messages, string)

### Integration Tests
- [ ] Pipe I/O (unidirectional)
- [ ] Bidirectional communication
- [ ] Concurrent send/recv (multiple tasks)

### E2E Tests
- [ ] Serialization error handling
- [ ] Stress test (10,000 messages)

### Performance Tests
- [ ] send() (64B, 1KB, 10KB)
- [ ] recv() (64B, 1KB, 10KB)
- [ ] roundtrip (1KB)

---

*Generated by Architect Agent*
*Test plan status: COMPLETE - Ready for implementation*
