//! Test script to verify checkpoint header optimization.

use std::time::Instant;

// Constants from the actual code
const MAGIC_BYTES: &[u8; 8] = b"OYACPT01";
const CHECKPOINT_VERSION: u32 = 1;

/// Original implementation using iterator chains
fn original_header_creation(data: &[u8]) -> Vec<u8> {
    let header = MAGIC_BYTES
        .iter()
        .chain(CHECKPOINT_VERSION.to_le_bytes().iter())
        .chain(data.iter())
        .copied()
        .collect::<Vec<_>>();
    header
}

/// Optimized implementation using pre-allocated Vec
fn optimized_header_creation(data: &[u8]) -> Vec<u8> {
    let mut header = Vec::with_capacity(MAGIC_BYTES.len() + 4 + data.len());
    header.extend_from_slice(MAGIC_BYTES);
    header.extend_from_slice(&CHECKPOINT_VERSION.to_le_bytes());
    header.extend_from_slice(data);
    header
}

/// Generate test data of various sizes
fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

fn main() {
    println!("Checkpoint Header Optimization Test");
    println!("===================================");

    // Test data sizes
    let test_sizes = [
        10,
        100,
        1000,
        10000,
        100000,
    ];

    let iterations = 10000;

    for &size in &test_sizes {
        let test_data = generate_test_data(size);

        // Verify both implementations produce the same result
        let original = original_header_creation(&test_data);
        let optimized = optimized_header_creation(&test_data);

        assert_eq!(original, optimized, "Implementations should produce identical results");

        // Benchmark original implementation
        let start = Instant::now();
        for _ in 0..iterations {
            original_header_creation(&test_data);
        }
        let original_duration = start.elapsed();

        // Benchmark optimized implementation
        let start = Instant::now();
        for _ in 0..iterations {
            optimized_header_creation(&test_data);
        }
        let optimized_duration = start.elapsed();

        // Calculate improvement
        let improvement = (original_duration.as_nanos() - optimized_duration.as_nanos()) as f64
            / original_duration.as_nanos() as f64 * 100.0;

        println!("Data size: {:>6} bytes", size);
        println!("  Original:  {:>10.2} ns/iter", original_duration.as_nanos() as f64 / iterations as f64);
        println!("  Optimized: {:>10.2} ns/iter", optimized_duration.as_nanos() as f64 / iterations as f64);
        println!("  Improvement: {:>6.1}%", improvement);
        println!("  Data length: {} bytes", original.len());
        println!();
    }

    println!("Test completed successfully!");
}