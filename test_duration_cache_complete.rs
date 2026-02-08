// Complete test of the OnceLock caching pattern for duration formatting
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug)]
struct Reconciler {
    duration_cache: OnceLock<HashMap<Duration, String>>,
}

impl Reconciler {
    fn new() -> Self {
        Self {
            duration_cache: OnceLock::new(),
        }
    }

    /// Format a Duration into a human-readable string with caching.
    /// This avoids repeated string conversions for the same duration values.
    fn format_duration(&self, duration: Duration) -> String {
        // Use OnceLock to store the cache, compute once per reconciler instance
        let cache = self.duration_cache.get_or_init(|| {
            // Pre-populate with common threshold values
            let mut map = HashMap::new();

            // Common threshold values that are frequently used
            map.insert(Duration::from_secs(60), "1m 0s".to_string());
            map.insert(Duration::from_secs(120), "2m 0s".to_string());
            map.insert(Duration::from_secs(300), "5m 0s".to_string());
            map.insert(Duration::from_secs(600), "10m 0s".to_string());
            map.insert(Duration::from_secs(1800), "30m 0s".to_string());

            // Add some common values
            map.insert(Duration::from_secs(30), "30s".to_string());
            map.insert(Duration::from_secs(90), "1m 30s".to_string());
            map.insert(Duration::from_secs(150), "2m 30s".to_string());

            map
        });

        // Look up the duration in the cache
        if let Some(cached) = cache.get(&duration) {
            cached.clone()
        } else {
            // If not in cache, compute it
            if duration.as_secs() > 60 {
                format!("{}m {}s",
                    duration.as_secs() / 60,
                    duration.as_secs() % 60)
            } else {
                format!("{}s", duration.as_secs())
            }
        }
    }

    /// Get the cache statistics (for testing)
    fn get_cache_stats(&self) -> (usize, HashMap<Duration, String>) {
        if let Some(cache) = self.duration_cache.get() {
            (cache.len(), cache.clone())
        } else {
            (0, HashMap::new())
        }
    }
}

fn main() {
    println!("Testing duration caching with OnceLock...");

    let reconciler = Reconciler::new();

    // Test different durations
    let durations = vec![
        Duration::from_secs(60),   // In cache
        Duration::from_secs(30),   // In cache
        Duration::from_secs(120),  // In cache
        Duration::from_secs(90),   // In cache
        Duration::from_secs(150),  // In cache
        Duration::from_secs(45),   // Not in cache - will be computed
        Duration::from_secs(200),  // Not in cache - will be computed
        Duration::from_secs(60),   // In cache - should show cached result
        Duration::from_secs(120),  // In cache - should show cached result
        Duration::from_secs(180),  // Not in cache - will be computed
    ];

    println!("\nDuration formatting results:");
    for duration in durations {
        let formatted = reconciler.format_duration(duration);
        println!("Duration {:?} -> {}", duration, formatted);
    }

    // Show cache statistics
    let (cache_size, cache_contents) = reconciler.get_cache_stats();
    println!("\nCache statistics:");
    println!("  Cache size: {}", cache_size);
    println!("  Cache contents:");
    for (duration, formatted) in cache_contents.iter() {
        println!("    {:?} -> {}", duration, formatted);
    }

    println!("\nTest completed successfully!");
    println!("\nThis demonstrates OnceLock caching where common durations are pre-computed");
    println!("and stored in a HashMap on first use. This avoids repeated string formatting");
    println!("for frequently used threshold values.");
}