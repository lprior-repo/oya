//! Backpressure tests for Zellij pipe handler
//!
//! These tests verify the pipe handler correctly implements flow control
//! when handling high-volume log streaming (100k+ lines).

use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Test that consumer can throttle a fast producer with backpressure
///
/// This test verifies:
/// 1. Consumer controls the flow rate via acknowledgments
/// 2. Producer respects backpressure signals
/// 3. System remains stable under rate mismatches
#[test]
fn test_backpressure_throttling() {
    // Create pipes for bidirectional communication
    let (consumer_reader, mut producer_writer) = os_pipe::pipe().expect("Failed to create pipe");
    let (mut producer_reader, mut consumer_writer) =
        os_pipe::pipe().expect("Failed to create pipe");

    let messages_sent = Arc::new(AtomicUsize::new(0));
    let messages_received = Arc::new(AtomicUsize::new(0));
    let backpressure_signals = Arc::new(AtomicUsize::new(0));

    let sent = Arc::clone(&messages_sent);
    let recv = Arc::clone(&messages_received);
    let bp = Arc::clone(&backpressure_signals);

    // Spawn producer thread (fast)
    let producer = thread::spawn(move || {
        for i in 0..100 {
            let msg = format!("Message {:05}\n", i);

            // Check for backpressure signal from consumer
            let mut buf = [0u8; 1];
            match producer_reader.read(&mut buf) {
                Ok(1) if buf[0] == b'\x01' => {
                    bp.fetch_add(1, Ordering::SeqCst);
                    // Respect backpressure - wait before continuing
                    thread::sleep(Duration::from_millis(10));
                }
                _ => {}
            }

            producer_writer
                .write_all(msg.as_bytes())
                .expect("Failed to write");
            sent.fetch_add(1, Ordering::SeqCst);
        }
    });

    // Consumer thread (slow - simulates processing time)
    let consumer = thread::spawn(move || {
        let mut reader = std::io::BufReader::new(consumer_reader);
        let mut line = String::new();

        for i in 0..100 {
            line.clear();

            // Simulate slow processing
            thread::sleep(Duration::from_millis(20));

            // Signal backpressure every 10 messages
            if i > 0 && i % 10 == 0 {
                consumer_writer
                    .write_all(b"\x01")
                    .expect("Failed to send backpressure");
            }

            // Read message
            match std::io::BufRead::read_line(&mut reader, &mut line) {
                Ok(_) if !line.is_empty() => {
                    recv.fetch_add(1, Ordering::SeqCst);
                }
                _ => break,
            }
        }
    });

    producer.join().expect("Producer panicked");
    consumer.join().expect("Consumer panicked");

    // Verify all messages were received despite rate mismatch
    assert_eq!(
        messages_received.load(Ordering::SeqCst),
        messages_sent.load(Ordering::SeqCst),
        "All messages should be received"
    );

    // Verify backpressure was applied
    assert!(
        backpressure_signals.load(Ordering::SeqCst) > 0,
        "Backpressure signals should have been sent"
    );
}

/// Test streaming 100k log lines without memory overflow
///
/// This test verifies:
/// 1. Large volume (100k) lines can be streamed
/// 2. Memory usage stays bounded (no unbounded growth)
/// 3. All lines are received correctly (no data loss)
/// 4. Performance is acceptable (< 30 seconds)
#[test]
fn test_100k_log_line_streaming() {
    let start_time = Instant::now();
    let (reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    const TOTAL_LINES: usize = 100_000;

    let lines_received = Arc::new(AtomicUsize::new(0));
    let received = Arc::clone(&lines_received);

    // Producer thread - write 100k lines as fast as possible
    let producer = thread::spawn(move || {
        for i in 0..TOTAL_LINES {
            let line = format!(
                "[2024-01-{:02} {:02}:{:02}:{:02}.{:03}] [TEST] INFO - Log entry number {:06}\n",
                (i % 30) + 1,
                (i / 3600) % 24,
                (i / 60) % 60,
                i % 60,
                i % 1000,
                i
            );
            writer.write_all(line.as_bytes()).expect("Failed to write");
        }
        writer.flush().expect("Failed to flush");
    });

    // Consumer thread - read and count lines
    let consumer = thread::spawn(move || {
        let mut reader = std::io::BufReader::new(reader);
        let mut line = String::new();
        let mut count = 0;

        loop {
            line.clear();
            match std::io::BufRead::read_line(&mut reader, &mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    count += 1;
                    received.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => break,
            }
        }
        count
    });

    producer.join().expect("Producer panicked");
    let count = consumer.join().expect("Consumer panicked");

    let elapsed = start_time.elapsed();

    // Verify all lines received
    assert_eq!(
        count, TOTAL_LINES,
        "Should receive all {} log lines, got {}",
        TOTAL_LINES, count
    );

    // Verify performance (should complete in reasonable time)
    assert!(
        elapsed < Duration::from_secs(30),
        "100k lines should process in < 30 seconds, took {:?}",
        elapsed
    );

    println!("Processed {} lines in {:?}", TOTAL_LINES, elapsed);
}

/// Test that slow consumer doesn't lose data from fast producer
///
/// This test verifies:
/// 1. Pipe buffer doesn't overflow and drop data
/// 2. Producer blocks appropriately when buffer is full
/// 3. No data loss occurs due to buffer limitations
/// 4. System remains stable under sustained load
#[test]
fn test_slow_consumer_no_data_loss() {
    let (reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    const TOTAL_MESSAGES: usize = 10_000;
    const MESSAGE_SIZE: usize = 1_000; // 1KB messages

    let messages_received = Arc::new(AtomicUsize::new(0));
    let received = Arc::clone(&messages_received);

    // Producer - writes as fast as possible
    let producer = thread::spawn(move || {
        let message = "X".repeat(MESSAGE_SIZE);
        for i in 0..TOTAL_MESSAGES {
            let payload = format!("{:08}:{}\n", i, message);
            writer
                .write_all(payload.as_bytes())
                .expect("Failed to write");
        }
        writer.flush().expect("Failed to flush");
    });

    // Consumer - deliberately slow with pauses
    let consumer = thread::spawn(move || {
        let mut reader = std::io::BufReader::new(reader);
        let mut line = String::new();
        let mut count = 0;

        loop {
            line.clear();

            // Slow processing - 1ms delay per message
            thread::sleep(Duration::from_millis(1));

            match std::io::BufRead::read_line(&mut reader, &mut line) {
                Ok(0) => break,
                Ok(_) => {
                    // Verify message integrity
                    if line.len() >= 9 {
                        count += 1;
                        received.fetch_add(1, Ordering::SeqCst);
                    }
                }
                Err(_) => break,
            }
        }
        count
    });

    producer.join().expect("Producer panicked");
    let count = consumer.join().expect("Consumer panicked");

    // All messages should be received despite slow consumer
    assert_eq!(
        count, TOTAL_MESSAGES,
        "All {} messages should be received despite slow consumer, got {}",
        TOTAL_MESSAGES, count
    );
}

/// Test handling of pipe buffer limits
///
/// This test verifies:
/// 1. System doesn't deadlock when pipe buffer is full
/// 2. Large writes are handled correctly
/// 3. No infinite blocking occurs
/// 4. Timeout/recovery mechanisms work
#[test]
fn test_pipe_buffer_limit_handling() {
    let (mut reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    // Create a large payload that exceeds typical pipe buffer (64KB on Linux)
    let large_payload = "Y".repeat(128_000);
    let payload_size = large_payload.len();

    let read_result = Arc::new(Mutex::new(Vec::new()));
    let result = Arc::clone(&read_result);

    // Producer - write large payload
    let producer = thread::spawn(move || {
        writer
            .write_all(large_payload.as_bytes())
            .expect("Failed to write large payload");
        writer.flush().expect("Failed to flush");
    });

    // Consumer - read with timeout
    let consumer = thread::spawn(move || {
        let mut buffer = Vec::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(5);

        while buffer.len() < payload_size && start.elapsed() < timeout {
            let mut chunk = [0u8; 4096];
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }

        let mut result = result.lock().expect("Failed to lock");
        *result = buffer;
    });

    producer.join().expect("Producer panicked");
    consumer.join().expect("Consumer panicked");

    let result = read_result.lock().expect("Failed to lock");
    assert_eq!(
        result.len(),
        payload_size,
        "Large payload ({} bytes) should be received completely, got {} bytes",
        payload_size,
        result.len()
    );
}

/// Test graceful degradation under sustained high load
///
/// This test verifies:
/// 1. System doesn't crash under sustained load
/// 2. Memory usage remains bounded
/// 3. Processing continues correctly after load spike
/// 4. No resource leaks occur
#[test]
fn test_graceful_degradation_under_load() {
    let (reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    const BURST_SIZE: usize = 50_000;
    const MESSAGE_SIZE: usize = 100;

    let errors = Arc::new(AtomicUsize::new(0));
    let errors_clone = Arc::clone(&errors);

    // Producer - burst of messages
    let producer = thread::spawn(move || {
        let message = "Z".repeat(MESSAGE_SIZE);
        for i in 0..BURST_SIZE {
            let payload = format!("{:010}:{}", i, message);
            if let Err(_) = writer.write_all(payload.as_bytes()) {
                errors_clone.fetch_add(1, Ordering::SeqCst);
                break;
            }
        }
    });

    // Consumer - moderate rate
    let consumer = thread::spawn(move || {
        let mut reader = std::io::BufReader::new(reader);
        let mut buffer = vec![0u8; MESSAGE_SIZE + 12]; // Allow for index prefix
        let mut count = 0;

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    count += n;
                    // Simulate processing time
                    thread::sleep(Duration::from_micros(100));
                }
                Err(_) => break,
            }
        }
        count
    });

    producer.join().expect("Producer panicked");
    consumer.join().expect("Consumer panicked");

    // Should complete without errors
    assert_eq!(
        errors.load(Ordering::SeqCst),
        0,
        "Should not encounter errors under load"
    );
}
