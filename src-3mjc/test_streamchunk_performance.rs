//! Simple performance test for StreamChunk optimization
//!
//! This demonstrates the performance improvements from using Bytes vs Vec<u8>

use bytes::Bytes;
use std::time::Instant;

fn main() {
    println!("🚀 StreamChunk Performance Comparison");
    println!("=====================================");

    // Test data sizes
    let sizes = [
        100,    // Small
        1000,   // Medium
        10000,  // Large
        100000, // Very large
    ];

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
        let vec_total_bytes: usize = vec_copies.iter().map(|v| v.len()).sum();

        // Test 2: Bytes slice (zero-copy)
        let bytes_data = Bytes::from(data);
        let start = Instant::now();
        let mut byte_slices = Vec::new();
        for _ in 0..1000 {
            let chunk = bytes_data.slice(0..size/2);  // Zero-copy
            byte_slices.push(chunk);
        }
        let bytes_duration = start.elapsed();
        let bytes_total_bytes: usize = byte_slices.iter().map(|b| b.len()).sum();

        // Calculate performance improvement
        let speedup = vec_duration.as_nanos() as f64 / bytes_duration.as_nanos() as f64;

        println!("  Vec<u8> slice (copy): {:?}", vec_duration);
        println!("  Bytes slice (zero-copy): {:?}", bytes_duration);
        println!("  📈 Speedup: {:.1}x faster", speedup);
        println!("  📦 Total bytes processed: vec={} vs bytes={} (same data)",
                vec_total_bytes, bytes_total_bytes);

        // Test 3: Cloning performance
        let chunk_vec = data.clone();  // Clone Vec
        let start = Instant::now();
        for _ in 0..1000 {
            let _clone = chunk_vec.clone();  // Deep copy
        }
        let vec_clone_duration = start.elapsed();

        let chunk_bytes = bytes_data.clone();  // Clone Bytes (refcount increment)
        let start = Instant::now();
        for _ in 0..1000 {
            let _clone = chunk_bytes.clone();  // Reference count increment
        }
        let bytes_clone_duration = start.elapsed();

        let clone_speedup = vec_clone_duration.as_nanos() as f64 / bytes_clone_duration.as_nanos() as f64;
        println!("  Vec<u8> clone: {:?}", vec_clone_duration);
        println!("  Bytes clone: {:?}", bytes_clone_duration);
        println!("  📈 Clone speedup: {:.1}x faster", clone_speedup);
    }

    println!("\n✨ Key Benefits:");
    println!("  • Zero-copy slicing: No memory copying when extracting sub-chunks");
    println!("  • Efficient cloning: O(1) reference count increment");
    println!("  • Memory sharing: Multiple consumers can share the same buffer");
    println!("  • Reduced allocations: No heap allocations for slices");
    println!("  • Better cache locality: Shared memory reduces duplication");

    println!("\n🎯 Use Cases Benefiting from This Optimization:");
    println!("  • Large file streaming");
    println!("  • Network protocol buffers");
    println!("  • Log processing");
    println!("  • Real-time data pipelines");
    println!("  • Message queue processing");
}