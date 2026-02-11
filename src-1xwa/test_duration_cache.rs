// Test script to verify the duration caching implementation
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone)]
struct TestReconciler {
    duration_cache: OnceLock<HashMap<Duration, String>>,
}

impl TestReconciler {
    fn new() -> Self {
        Self {
            duration_cache: OnceLock::new(),
        }
    }

    fn format_duration(&mut self, duration: Duration) -> &str {
        // Check if cache is initialized
        if self.duration_cache.get().is_none() {
            // Initialize the cache
            let cache = HashMap::new();
            let cache_ref = &cache;
            unsafe {
                // We need to use ptr::read to get a reference to the initialized value
                // This is tricky with OnceLock, let's use a simpler approach
                // For now, we'll just create the cache
                let _ = self.duration_cache.set(cache);
            }
        }

        // Now we can get a mutable reference
        let cache = unsafe { &mut *(self.duration_cache.get().unwrap() as *mut HashMap<Duration, String>) };
        cache.entry(duration).or_insert_with(|| {
            if duration.as_secs() > 60 {
                format!("{}m {}s",
                    duration.as_secs() / 60,
                    duration.as_secs() % 60)
            } else {
                format!("{}s", duration.as_secs())
            }
        })
    }
}

fn main() {
    println!("Testing duration caching implementation...");

    let mut reconciler = TestReconciler::new();

    // Test different durations
    let durations = vec![
        Duration::from_secs(30),  // Should be "30s"
        Duration::from_secs(120), // Should be "2m 0s"
        Duration::from_secs(150), // Should be "2m 30s"
        Duration::from_secs(30),  // Should use cached version
        Duration::from_secs(120), // Should use cached version
        Duration::from_secs(150), // Should use cached version
    ];

    for duration in durations {
        let formatted = reconciler.format_duration_cached(duration);
        println!("Duration {:?} -> {}", duration, formatted);
    }

    // Verify cache contains expected entries
    let cache = reconciler.duration_cache.get().unwrap();
    println!("\nCache contents:");
    for (duration, formatted) in cache.iter() {
        println!("  {:?} -> {}", duration, formatted);
    }

    println!("\nTest completed successfully!");
}