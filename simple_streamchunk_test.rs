//! Simple StreamChunk performance test
//! Run with: rustc simple_streamchunk_test.rs && ./simple_streamchunk_test

use std::time::Instant;

fn main() {
    println!("🚀 StreamChunk Performance Comparison");
    println!("=====================================");

    // Test data sizes
    let sizes = [1000, 10000, 100000];

    for size in sizes {
        println!("\n📊 Testing with {} bytes:", size);

        // Generate test data
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        // Test 1: Vec<u8> slice (copies data)
        let start = Instant::now();
        let mut vec_copies = Vec::new();
        for _ in 0..1000 {
            let chunk = Vec::from(&data[0..size/2]);  // Copy
            vec_copies.push(chunk);
        }
        let vec_duration = start.elapsed();

        // Test 2: Simulating zero-copy with Bytes
        let start = Instant::now();
        let mut byte_slices = Vec::new();
        for _ in 0..1000 {
            // Simulate zero-copy slicing (just reference)
            let _chunk = &data[0..size/2];  // Reference, no copy
            byte_slices.push(_chunk);
        }
        let bytes_duration = start.elapsed();

        // Calculate performance improvement
        let speedup = vec_duration.as_nanos() as f64 / bytes_duration.as_nanos() as f64;

        println!("  Vec<u8> slice (copy): {:?}", vec_duration);
        println!("  Simulated zero-copy (reference): {:?}", bytes_duration);
        println!("  📈 Speedup: {:.1}x faster", speedup);

        // Show memory usage difference
        let vec_allocated: usize = vec_copies.iter().map(|v| v.len()).sum();
        let bytes_allocated: usize = byte_slices.iter().map(|v| v.len()).sum();
        println!("  📦 Memory allocated: vec={} vs simulated={} bytes",
                 vec_allocated, bytes_allocated);
    }

    println!("\n✨ With real Bytes type from bytes crate:");
    println!("  • Actual zero-copy slicing with Bytes::slice()");
    println!("  • Reference-counted shared ownership");
    println!("  • No allocations for slices");
    println!("  • Thread-safe sharing across async boundaries");
    println!("  • Compatible with tokio streams and async runtime");
}