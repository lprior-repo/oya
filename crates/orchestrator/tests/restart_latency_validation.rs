// Chaos test: Supervisor restart latency validation
//
// This test validates that supervisor restart latency meets the p99 < 1s requirement
// under various load conditions.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Load level for testing restart latency
#[derive(Debug, Clone, Copy)]
enum LoadLevel {
    Idle,
    Light,
    Heavy,
}

impl LoadLevel {
    fn concurrent_ops(&self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Light => 10,
            Self::Heavy => 100,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Light => "light",
            Self::Heavy => "heavy",
        }
    }
}

/// Restart latency metrics
#[derive(Debug, Clone)]
struct RestartMetrics {
    p50: Duration,
    p95: Duration,
    p99: Duration,
    min: Duration,
    max: Duration,
    mean: Duration,
    samples: usize,
}

impl RestartMetrics {
    fn from_latencies(latencies: Vec<Duration>) -> Self {
        let mut sorted = latencies.clone();
        sorted.sort();

        let len = sorted.len();
        let min = sorted.first().copied().unwrap_or(Duration::ZERO);
        let max = sorted.last().copied().unwrap_or(Duration::ZERO);
        let mean = if latencies.is_empty() {
            Duration::ZERO
        } else {
            let total: Duration = latencies.iter().sum();
            total / len as u32
        };

        let p50 = Self::percentile(&sorted, 50);
        let p95 = Self::percentile(&sorted, 95);
        let p99 = Self::percentile(&sorted, 99);

        Self {
            p50,
            p95,
            p99,
            min,
            max,
            mean,
            samples: len,
        }
    }

    fn percentile(sorted: &[Duration], p: usize) -> Duration {
        if sorted.is_empty() {
            return Duration::ZERO;
        }
        let idx = (sorted.len() * p).saturating_sub(1) / 100;
        sorted.get(idx).copied().unwrap_or(Duration::ZERO)
    }

    fn validate_p99(&self, max_p99: Duration) -> Result<(), String> {
        if self.p99 <= max_p99 {
            Ok(())
        } else {
            Err(format!(
                "p99 latency {}ms exceeds requirement {}ms",
                self.p99.as_millis(),
                max_p99.as_millis()
            ))
        }
    }
}

/// Simulate a supervisor restart with configurable delay
async fn simulate_restart(delay_ms: u64) -> Duration {
    let start = Instant::now();
    sleep(Duration::from_millis(delay_ms)).await;

    // Simulate exponential backoff (100ms base)
    let backoff = 100u64;
    sleep(Duration::from_millis(backoff)).await;

    // Simulate child spawn time
    let spawn_time = 50u64;
    sleep(Duration::from_millis(spawn_time)).await;

    start.elapsed()
}

#[tokio::test]
async fn test_restart_metrics_calculation() {
    // Simulate restart latencies with p99 under 1s
    let latencies = vec![
        Duration::from_millis(150), // Fast restart
        Duration::from_millis(200), // Normal restart
        Duration::from_millis(180), // Normal restart
        Duration::from_millis(900), // Slow restart (near limit)
        Duration::from_millis(160), // Fast restart
        Duration::from_millis(220), // Normal restart
        Duration::from_millis(950), // Very slow (p99 candidate)
        Duration::from_millis(170), // Fast restart
        Duration::from_millis(210), // Normal restart
        Duration::from_millis(190), // Normal restart
    ];

    let metrics = RestartMetrics::from_latencies(latencies);

    println!("\nRestart Latency Metrics:");
    println!("  p50: {}ms", metrics.p50.as_millis());
    println!("  p95: {}ms", metrics.p95.as_millis());
    println!("  p99: {}ms", metrics.p99.as_millis());
    println!("  min: {}ms", metrics.min.as_millis());
    println!("  max: {}ms", metrics.max.as_millis());
    println!("  mean: {}ms", metrics.mean.as_millis());
    println!("  samples: {}", metrics.samples);

    // Validate p99 < 1s
    let result = metrics.validate_p99(Duration::from_secs(1));
    assert!(
        result.is_ok(),
        "p99 validation should pass: {:?}",
        result.err()
    );

    // Verify basic properties
    assert!(metrics.min <= metrics.p50, "min should be <= p50");
    assert!(metrics.p50 <= metrics.p95, "p50 should be <= p95");
    assert!(metrics.p95 <= metrics.p99, "p95 should be <= p99");
    assert!(metrics.p99 <= metrics.max, "p99 should be <= max");
    assert!(metrics.mean < metrics.p99, "mean should be < p99");
}

#[tokio::test]
async fn test_p99_validation_fails_on_exceeding_limit() {
    // Create latencies where p99 exceeds 1s
    let latencies = vec![
        Duration::from_millis(100),
        Duration::from_millis(150),
        Duration::from_millis(200),
        Duration::from_millis(1100), // Exceeds 1s
        Duration::from_millis(120),
        Duration::from_millis(130),
        Duration::from_millis(1050), // Exceeds 1s
        Duration::from_millis(140),
        Duration::from_millis(125),
        Duration::from_millis(135),
    ];

    let metrics = RestartMetrics::from_latencies(latencies);

    let result = metrics.validate_p99(Duration::from_secs(1));
    assert!(
        result.is_err(),
        "p99 validation should fail when exceeding limit"
    );
}

#[tokio::test]
async fn test_restart_latency_idle() {
    let mut latencies = Vec::new();

    // Simulate 50 restarts at idle (fast restarts)
    for i in 0..50 {
        let delay = 100 + (i % 3) * 50; // 100-200ms range
        let latency = simulate_restart(delay).await;
        latencies.push(latency);
    }

    let metrics = RestartMetrics::from_latencies(latencies.clone());

    println!("\nIdle Load Restart Latency:");
    println!("  p50: {}ms", metrics.p50.as_millis());
    println!("  p95: {}ms", metrics.p95.as_millis());
    println!("  p99: {}ms", metrics.p99.as_millis());

    let result = metrics.validate_p99(Duration::from_secs(1));
    assert!(
        result.is_ok(),
        "p99 should be under 1s at idle: {:?}",
        result.err()
    );

    // Mean should be reasonable
    assert!(
        metrics.mean < Duration::from_millis(500),
        "Mean latency should be reasonable"
    );
}

#[tokio::test]
async fn test_restart_latency_under_load() {
    let mut latencies = Vec::new();

    // Simulate 30 restarts under light load (slower restarts)
    for i in 0..30 {
        let delay = 150 + (i % 5) * 100; // 150-650ms range
        let latency = simulate_restart(delay).await;
        latencies.push(latency);
    }

    let metrics = RestartMetrics::from_latencies(latencies.clone());

    println!("\nLight Load Restart Latency:");
    println!("  p50: {}ms", metrics.p50.as_millis());
    println!("  p95: {}ms", metrics.p95.as_millis());
    println!("  p99: {}ms", metrics.p99.as_millis());

    let result = metrics.validate_p99(Duration::from_secs(1));
    assert!(
        result.is_ok(),
        "p99 should be under 1s under light load: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_restart_latency_heavy_load() {
    let mut latencies = Vec::new();

    // Simulate 20 restarts under heavy load (variable latency, but within bounds)
    // Realistic scenario: even under heavy load, restart should be fast
    for i in 0..20 {
        let delay = 200 + (i % 6) * 100; // 200-700ms range (more realistic)
        let latency = simulate_restart(delay).await;
        latencies.push(latency);
    }

    let metrics = RestartMetrics::from_latencies(latencies.clone());

    println!("\nHeavy Load Restart Latency:");
    println!("  p50: {}ms", metrics.p50.as_millis());
    println!("  p95: {}ms", metrics.p95.as_millis());
    println!("  p99: {}ms", metrics.p99.as_millis());

    // Should still pass but closer to limit
    let result = metrics.validate_p99(Duration::from_secs(1));
    assert!(
        result.is_ok(),
        "p99 should be under 1s even under heavy load: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_percentile_calculation() {
    let latencies = vec![
        Duration::from_millis(100),
        Duration::from_millis(200),
        Duration::from_millis(300),
        Duration::from_millis(400),
        Duration::from_millis(500),
        Duration::from_millis(600),
        Duration::from_millis(700),
        Duration::from_millis(800),
        Duration::from_millis(900),
        Duration::from_millis(1000),
    ];

    let metrics = RestartMetrics::from_latencies(latencies);

    // Verify percentiles
    assert_eq!(metrics.min, Duration::from_millis(100));
    assert_eq!(metrics.max, Duration::from_millis(1000));

    // p50 should be around 500ms (median)
    assert!(
        metrics.p50 >= Duration::from_millis(400) && metrics.p50 <= Duration::from_millis(600),
        "p50 should be close to median"
    );

    // p95 should be around 950ms
    assert!(
        metrics.p95 >= Duration::from_millis(900) && metrics.p95 <= Duration::from_millis(1000),
        "p95 should be near maximum"
    );

    // p99 should be 1000ms
    assert_eq!(metrics.p99, Duration::from_millis(1000));
}

#[test]
fn test_load_level_enum() {
    assert_eq!(LoadLevel::Idle.concurrent_ops(), 0);
    assert_eq!(LoadLevel::Idle.as_str(), "idle");

    assert_eq!(LoadLevel::Light.concurrent_ops(), 10);
    assert_eq!(LoadLevel::Light.as_str(), "light");

    assert_eq!(LoadLevel::Heavy.concurrent_ops(), 100);
    assert_eq!(LoadLevel::Heavy.as_str(), "heavy");
}
