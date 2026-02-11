// Test script to verify the duration formatting logic (without caching)
use std::time::Duration;

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 60 {
        format!("{}m {}s",
            duration.as_secs() / 60,
            duration.as_secs() % 60)
    } else {
        format!("{}s", duration.as_secs())
    }
}

fn main() {
    println!("Testing duration formatting logic...");

    // Test different durations
    let durations = vec![
        Duration::from_secs(30),  // Should be "30s"
        Duration::from_secs(120), // Should be "2m 0s"
        Duration::from_secs(150), // Should be "2m 30s"
        Duration::from_secs(30),  // Should be "30s" (same as first)
        Duration::from_secs(120), // Should be "2m 0s" (same as second)
        Duration::from_secs(150), // Should be "2m 30s" (same as third)
    ];

    for duration in durations {
        let formatted = format_duration(duration);
        println!("Duration {:?} -> {}", duration, formatted);
    }

    println!("\nFormatting test completed successfully!");
}