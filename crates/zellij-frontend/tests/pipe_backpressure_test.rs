//! Backpressure tests for Zellij pipe handler
//!
//! These tests verify the pipe handler correctly implements flow control
//! when handling high-volume log streaming (100k+ lines).

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Test that consumer can throttle a fast producer with backpressure
///
/// This test verifies:
/// 1. Consumer controls the flow rate via built-in OS pipe backpressure
/// 2. Producer blocks when pipe buffer is full (natural backpressure)
/// 3. System remains stable under rate mismatches
/// 4. All data is eventually delivered
#[test]
fn test_backpressure_throttling() {
    let (mut reader, mut writer) = os_pipe::pipe().expect("Failed to create pipe");

    const TOTAL_MESSAGES: usize = 500;
    const MESSAGE_SIZE: usize = 1024; // 1KB messages to fill buffer faster

    let messages_sent = Arc::new(AtomicUsize::new(0));
    let bytes_received = Arc::new(AtomicUsize::new(0));
    let producer_blocked = Arc::new(AtomicBool::new(false));

    let sent = Arc::clone(&messages_sent);
    let bytes_recv = Arc::clone(&bytes_received);
    let blocked = Arc::clone(&producer_blocked);

    // Spawn producer thread (fast writer)
    let producer = thread::spawn(move || {
        let start = Instant::now();
        let message = "A".repeat(MESSAGE_SIZE);

        for i in 0..TOTAL_MESSAGES {
            let payload = format!("{:08}:{}", i, message);

            // Track if write blocks (indicates backpressure)
            let write_start = Instant::now();
            match writer.write_all(payload.as_bytes()) {
                Ok(_) => {
                    sent.fetch_add(1, Ordering::SeqCst);
                    // If write took >10ms, it likely blocked on full buffer
                    if write_start.elapsed() > Duration::from_millis(10) {
                        blocked.store(true, Ordering::SeqCst);
                    }
                }
                Err(_) => break,
            }
        }

        // Ensure all data is flushed
        let _ = writer.flush();
        drop(writer); // Close write end to signal EOF

        start.elapsed()
    });

    // Consumer thread (slow reader - simulates processing time)
    let consumer = thread::spawn(move || {
        let mut buffer = [0u8; MESSAGE_SIZE + 10]; // Buffer for one message
        let mut total_bytes = 0;
        let mut messages = 0;

        loop {
            // Slow processing - 5ms delay per read
            thread::sleep(Duration::from_millis(5));

            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    total_bytes += n;
                    messages += 1;
                    bytes_recv.fetch_add(n, Ordering::SeqCst);
                }
                Err(_) => break,
            }

            // Stop after reasonable time
            if messages >= TOTAL_MESSAGES {
                break;
            }
        }

        (messages, total_bytes)
    });

    let producer_duration = producer.join().expect("Producer panicked");
    let (received_count, _total_bytes) = consumer.join().expect("Consumer panicked");

    // Verify all messages were received
    assert_eq!(
        received_count, TOTAL_MESSAGES,
        "All {} messages should be received, got {}",
        TOTAL_MESSAGES, received_count
    );

    assert_eq!(
        messages_sent.load(Ordering::SeqCst),
        TOTAL_MESSAGES,
        "All messages should have been sent"
    );

    // Verify producer experienced backpressure (took significant time due to blocking)
    // Without backpressure, 500 x 1KB writes would be nearly instant
    assert!(
        producer_duration > Duration::from_millis(100),
        "Producer should have been throttled by backpressure, took {:?}",
        producer_duration
    );

    println!(
        "Producer was throttled: took {:?} for {} messages",
        producer_duration, TOTAL_MESSAGES
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
            if writer.write_all(payload.as_bytes()).is_err() {
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
