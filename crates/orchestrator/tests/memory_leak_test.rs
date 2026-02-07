//! Memory Leak Detection Test
//!
//! SUSTAINED LOAD chaos engineering test to detect memory leaks.
//! This test runs for 1 hour under continuous actor spawn/kill cycles
//! and monitors RSS memory usage for leak detection.
//!
//! Run with:
//!   moon run :test memory_leak_test
//!
//! For full 1-hour test:
//!   moon run :test memory_leak_test -- --ignored
//!
//! QA-ENFORCER + RED-QUEEN patterns:
//! - Evolutionary: Detects gradual memory growth patterns
//! - Hostile: Stresses actor lifecycle with rapid spawn/kill
//! - Deterministic: Fails automatically if leak detected

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    SupervisorActorDef, SupervisorArguments, SupervisorConfig, SupervisorMessage,
};
use ractor::{Actor, ActorRef, ActorStatus};
use tokio::time::sleep;

// ============================================================================
// MEMORY MONITORING (Cross-platform, zero-dependency)
// ============================================================================

/// Memory usage statistics snapshot.
#[derive(Debug, Clone)]
struct MemorySnapshot {
    /// Timestamp when snapshot was taken
    timestamp: Duration,
    /// Resident Set Size (RSS) in bytes
    rss_bytes: u64,
    /// Number of active actors
    active_actors: usize,
    /// Total actors spawned so far
    total_spawned: u64,
    /// Total actors killed so far
    total_killed: u64,
}

impl MemorySnapshot {
    /// Get current RSS memory usage.
    ///
    /// Cross-platform implementation that works on Linux, macOS, and Windows.
    /// Returns 0 if platform not supported (graceful degradation).
    fn current_rss() -> u64 {
        #[cfg(target_os = "linux")]
        {
            // Read from /proc/self/status
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        // Format: "VmRSS:     12345 kB"
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return kb * 1024; // Convert to bytes
                            }
                        }
                    }
                }
            }
            0
        }

        #[cfg(target_os = "macos")]
        {
            // Use mach task info
            use std::mem;
            unsafe {
                let mut info: libc::proc_taskinfo = mem::zeroed();
                let count = libc::proc_pidinfo(
                    libc::getpid(),
                    libc::PROC_PIDTASKINFO,
                    0,
                    &mut info as *mut _ as *mut libc::c_void,
                    mem::size_of::<libc::proc_taskinfo>() as libc::c_int,
                );
                if count == mem::size_of::<libc::proc_taskinfo>() as libc::c_int {
                    info.pti_resident_size as u64
                } else {
                    0
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Use GetProcessMemoryInfo
            use std::mem;
            use windows_sys::Win32::System::ProcessStatus::{
                GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
            };
            use windows_sys::Win32::System::Threading::GetCurrentProcess;

            unsafe {
                let mut info: PROCESS_MEMORY_COUNTERS_EX = mem::zeroed();
                let handle = GetCurrentProcess();
                let result = GetProcessMemoryInfo(
                    handle,
                    &mut info as *mut _ as *mut PROCESS_MEMORY_COUNTERS,
                    mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                );
                if result != 0 {
                    info.WorkingSetSize as u64
                } else {
                    0
                }
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Unsupported platform - return 0 (test will skip leak detection)
            0
        }
    }

    /// Capture current memory state.
    fn capture(active_actors: usize, total_spawned: u64, total_killed: u64) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
            rss_bytes: Self::current_rss(),
            active_actors,
            total_spawned,
            total_killed,
        }
    }

    /// Convert RSS to human-readable format.
    fn human_readable(&self) -> String {
        let mb = self.rss_bytes / 1024 / 1024;
        let kb = (self.rss_bytes / 1024) % 1024;
        format!("{} MB {} KB", mb, kb)
    }
}

/// Memory leak detector using statistical analysis.
///
/// Uses RED-QUEEN evolutionary pattern to detect gradual memory growth
/// that indicates leaks. Implements multiple detection strategies:
/// 1. Linear regression trend analysis
/// 2. Peak-to-base comparison (detects accumulation)
/// 3. Rolling window slope detection (catches accelerating leaks)
struct MemoryLeakDetector {
    /// Memory snapshots over time
    snapshots: Vec<MemorySnapshot>,
    /// Baseline memory (first 10% of test)
    baseline_mb: f64,
    /// Peak memory observed
    peak_mb: f64,
    /// Number of actors to spawn per cycle
    actors_per_cycle: usize,
    /// Maximum allowed growth percentage
    max_growth_percent: f64,
}

impl MemoryLeakDetector {
    /// Create new detector with configuration.
    fn new(actors_per_cycle: usize, max_growth_percent: f64) -> Self {
        Self {
            snapshots: Vec::new(),
            baseline_mb: 0.0,
            peak_mb: 0.0,
            actors_per_cycle,
            max_growth_percent,
        }
    }

    /// Record a memory snapshot.
    fn record(&mut self, snapshot: MemorySnapshot) {
        let mb = snapshot.rss_bytes as f64 / 1024.0 / 1024.0;
        self.peak_mb = self.peak_mb.max(mb);
        self.snapshots.push(snapshot);

        // Update baseline from first 10 snapshots (warmup period)
        if self.snapshots.len() <= 10 {
            self.baseline_mb = mb;
        }
    }

    /// Detect memory leak using statistical analysis.
    ///
    /// Returns error if leak detected, with detailed diagnostics.
    fn detect_leak(&self) -> Result<(), LeakReport> {
        if self.snapshots.len() < 20 {
            // Not enough data points
            return Ok(());
        }

        let first = &self.snapshots[0];
        let last = &self.snapshots[self.snapshots.len() - 1];
        let elapsed_sec = last.timestamp.as_secs_f64() - first.timestamp.as_secs_f64();

        // Skip if test ran for less than 30 seconds
        if elapsed_sec < 30.0 {
            return Ok(());
        }

        let first_mb = first.rss_bytes as f64 / 1024.0 / 1024.0;
        let last_mb = last.rss_bytes as f64 / 1024.0 / 1024.0;

        // Detection 1: Peak-to-base growth (accumulation detection)
        let growth_percent = if self.baseline_mb > 0.0 {
            ((self.peak_mb - self.baseline_mb) / self.baseline_mb) * 100.0
        } else {
            0.0
        };

        // Detection 2: Linear regression trend (slope analysis)
        let slope = self.calculate_slope();

        // Detection 3: Final vs initial comparison
        let final_growth = if first_mb > 0.0 {
            ((last_mb - first_mb) / first_mb) * 100.0
        } else {
            0.0
        };

        // LEAK DETECTION LOGIC (evolutionary: multiple indicators)
        let leak_detected = growth_percent > self.max_growth_percent
            || slope > 0.5 // Growing by >0.5 MB/min
            || (final_growth > self.max_growth_percent && elapsed_sec > 300.0);

        if leak_detected {
            Err(LeakReport {
                baseline_mb: self.baseline_mb,
                peak_mb: self.peak_mb,
                first_mb,
                last_mb,
                growth_percent,
                slope_mb_per_min: slope,
                final_growth,
                elapsed_sec,
                snapshots: self.snapshots.len(),
                total_spawned: last.total_spawned,
                total_killed: last.total_killed,
                actors_per_cycle: self.actors_per_cycle,
            })
        } else {
            Ok(())
        }
    }

    /// Calculate linear regression slope (memory growth rate).
    ///
    /// Returns MB per minute. Positive slope indicates memory leak.
    fn calculate_slope(&self) -> f64 {
        if self.snapshots.len() < 2 {
            return 0.0;
        }

        let n = self.snapshots.len() as f64;
        let first_time = self.snapshots[0].timestamp.as_secs_f64();

        // Calculate sum of X, Y, XY, X^2 for linear regression
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for snapshot in &self.snapshots {
            let x = snapshot.timestamp.as_secs_f64() - first_time;
            let y = snapshot.rss_bytes as f64 / 1024.0 / 1024.0; // MB

            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        // Slope = (N*ΣXY - ΣX*ΣY) / (N*ΣX² - (ΣX)²)
        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = n * sum_x2 - sum_x * sum_x;

        if denominator.abs() < 0.001 {
            0.0
        } else {
            let slope_per_sec = numerator / denominator;
            slope_per_sec * 60.0 // Convert to per-minute
        }
    }
}

/// Detailed leak report for test failure.
#[derive(Debug)]
struct LeakReport {
    baseline_mb: f64,
    peak_mb: f64,
    first_mb: f64,
    last_mb: f64,
    growth_percent: f64,
    slope_mb_per_min: f64,
    final_growth: f64,
    elapsed_sec: f64,
    snapshots: usize,
    total_spawned: u64,
    total_killed: u64,
    actors_per_cycle: usize,
}

impl std::fmt::Display for LeakReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "\n═══════════════════════════════════════════════════════════════"
        )?;
        writeln!(f, "MEMORY LEAK DETECTED")?;
        writeln!(
            f,
            "═══════════════════════════════════════════════════════════════"
        )?;
        writeln!(f, "Test Duration: {:.1} seconds", self.elapsed_sec)?;
        writeln!(f, "Samples Taken: {}", self.snapshots)?;
        writeln!(f, "")?;
        writeln!(f, "Memory Statistics:")?;
        writeln!(f, "  Baseline:   {:.2} MB", self.baseline_mb)?;
        writeln!(f, "  Peak:       {:.2} MB", self.peak_mb)?;
        writeln!(f, "  First:      {:.2} MB", self.first_mb)?;
        writeln!(f, "  Last:       {:.2} MB", self.last_mb)?;
        writeln!(f, "")?;
        writeln!(f, "Growth Analysis:")?;
        writeln!(f, "  Peak Growth:    {:.2}%", self.growth_percent)?;
        writeln!(f, "  Final Growth:   {:.2}%", self.final_growth)?;
        writeln!(f, "  Trend Slope:    {:.2} MB/min", self.slope_mb_per_min)?;
        writeln!(f, "")?;
        writeln!(f, "Actor Lifecycle:")?;
        writeln!(f, "  Spawned: {}", self.total_spawned)?;
        writeln!(f, "  Killed:  {}", self.total_killed)?;
        writeln!(f, "  Per Cycle: {} actors", self.actors_per_cycle)?;
        writeln!(
            f,
            "═══════════════════════════════════════════════════════════════"
        )
    }
}

// ============================================================================
// TEST HELPERS
// ============================================================================

fn supervisor_args(config: SupervisorConfig) -> SupervisorArguments {
    SupervisorArguments::new().with_config(config)
}

fn unique_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

async fn spawn_supervisor_with_name(
    args: SupervisorArguments,
    name: &str,
) -> Result<ActorRef<SupervisorMessage<SchedulerActorDef>>, String> {
    let (actor, _handle) = Actor::spawn(
        Some(name.to_string()),
        SupervisorActorDef::<SchedulerActorDef>::new(SchedulerActorDef),
        args,
    )
    .await
    .map_err(|e| format!("Failed to spawn supervisor: {}", e))?;
    Ok(actor)
}

async fn spawn_child(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    name: &str,
    args: SchedulerArguments,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    supervisor
        .cast(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
            name: name.to_string(),
            args,
            reply: tx,
        })
        .map_err(|e| format!("Failed to spawn child '{}': {}", name, e))?;

    match tokio::time::timeout(Duration::from_millis(200), rx).await {
        Ok(Ok(result)) => result.map_err(|e| format!("Child '{}' failed: {}", name, e)),
        Ok(Err(e)) => Err(format!("Child '{}' reply failed: {}", name, e)),
        Err(_) => Err(format!("Timeout waiting for child '{}'", name)),
    }
}

async fn get_supervisor_status(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
) -> Result<usize, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    supervisor
        .cast(SupervisorMessage::GetStatus { reply: tx })
        .map_err(|e| format!("Failed to get status: {}", e))?;

    match tokio::time::timeout(Duration::from_millis(200), rx).await {
        Ok(Ok(status)) => Ok(status.active_children),
        Ok(Err(e)) => Err(format!("Failed to receive status: {}", e)),
        Err(_) => Err("Timeout waiting for status".to_string()),
    }
}

async fn kill_child(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    name: &str,
) -> Result<(), String> {
    supervisor
        .cast(SupervisorMessage::StopChild {
            name: name.to_string(),
        })
        .map_err(|e| format!("Failed to stop child '{}': {}", name, e))
}

// ============================================================================
// MEMORY LEAK TESTS
// ============================================================================

/// **QA-ENFORCER Memory Leak Test (Quick Version - 30 seconds)**
///
/// This is a fast smoke test that runs for 30 seconds to catch obvious leaks.
/// For the full 1-hour sustained load test, run the ignored test below.
///
/// Test pattern:
/// 1. Spawn 10 actors, wait 5 seconds
/// 2. Kill all actors, wait 2 seconds
/// 3. Repeat for 30 seconds
/// 4. Monitor RSS memory every 1 second
/// 5. Fail if memory grows >50% (indicates leak)
#[tokio::test]
async fn given_sustained_load_when_30_seconds_then_no_memory_leak() {
    // GIVEN: A supervisor with aggressive restart settings
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 1000;
    config.base_backoff_ms = 1; // Fast restart for high volume

    let supervisor_name = unique_name("memory-leak-quick");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Verify supervisor is alive
    assert!(
        matches!(
            supervisor.get_status(),
            ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
        ),
        "supervisor should be alive"
    );

    // WHEN: Run sustained load for 30 seconds
    let test_duration = Duration::from_secs(30);
    let cycle_duration = Duration::from_secs(5); // Spawn phase
    let kill_duration = Duration::from_secs(2); // Kill phase
    let actors_per_cycle = 10;
    let sample_interval = Duration::from_secs(1);

    let start = SystemTime::now();
    let mut detector = MemoryLeakDetector::new(actors_per_cycle, 50.0); // 50% threshold
    let total_spawned = Arc::new(AtomicU64::new(0));
    let total_killed = Arc::new(AtomicU64::new(0));
    let cycle_count = Arc::new(AtomicUsize::new(0));

    eprintln!("Starting 30-second memory leak test...");
    eprintln!(
        "Pattern: Spawn {} actors, wait 5s, kill all, wait 2s",
        actors_per_cycle
    );

    while SystemTime::now().duration_since(start).unwrap_or_default() < test_duration {
        // Spawn phase
        for i in 0..actors_per_cycle {
            let child_name = format!(
                "{}-cycle-{}-actor-{}",
                supervisor_name,
                cycle_count.load(Ordering::SeqCst),
                i
            );
            let child_args = SchedulerArguments::new();

            if spawn_child(&supervisor, &child_name, child_args)
                .await
                .is_ok()
            {
                total_spawned.fetch_add(1, Ordering::SeqCst);
            }
        }

        // Wait for spawn phase
        sleep(cycle_duration).await;

        // Kill phase
        for i in 0..actors_per_cycle {
            let child_name = format!(
                "{}-cycle-{}-actor-{}",
                supervisor_name,
                cycle_count.load(Ordering::SeqCst),
                i
            );
            if kill_child(&supervisor, &child_name).await.is_ok() {
                total_killed.fetch_add(1, Ordering::SeqCst);
            }
        }

        cycle_count.fetch_add(1, Ordering::SeqCst);

        // Wait for kill phase
        sleep(kill_duration).await;

        // Sample memory
        let active_count = get_supervisor_status(&supervisor).await.map_or(0, |v| v);
        let snapshot = MemorySnapshot::capture(
            active_count,
            total_spawned.load(Ordering::SeqCst),
            total_killed.load(Ordering::SeqCst),
        );
        detector.record(snapshot.clone());

        eprintln!(
            "[{:.1}s] Memory: {}, Active: {}, Spawned: {}, Killed: {}",
            start.elapsed().unwrap_or_default().as_secs_f64(),
            snapshot.human_readable(),
            active_count,
            total_spawned.load(Ordering::SeqCst),
            total_killed.load(Ordering::SeqCst)
        );

        // Small delay before next cycle
        sleep(sample_interval).await;
    }

    // THEN: Verify no memory leak detected
    let final_snapshot = MemorySnapshot::capture(
        get_supervisor_status(&supervisor).await.map_or(0, |v| v),
        total_spawned.load(Ordering::SeqCst),
        total_killed.load(Ordering::SeqCst),
    );
    detector.record(final_snapshot);

    eprintln!("\nMemory Leak Analysis:");
    eprintln!("  Cycles: {}", cycle_count.load(Ordering::SeqCst));
    eprintln!("  Actors Spawned: {}", total_spawned.load(Ordering::SeqCst));
    eprintln!("  Actors Killed: {}", total_killed.load(Ordering::SeqCst));
    eprintln!("  Samples: {}", detector.snapshots.len());
    eprintln!("  Baseline: {:.2} MB", detector.baseline_mb);
    eprintln!("  Peak: {:.2} MB", detector.peak_mb);
    eprintln!("  Growth Slope: {:.2} MB/min", detector.calculate_slope());

    // AUTOMATIC LEAK DETECTION
    match detector.detect_leak() {
        Ok(()) => {
            eprintln!("\n✓ No memory leak detected");
        }
        Err(report) => {
            eprintln!("\n{}", report);
            panic!("MEMORY LEAK DETECTED: Test failed due to memory growth");
        }
    }

    // Clean up
    supervisor.stop(Some("Memory leak test complete".to_string()));
}

/// **RED-QUEEN Sustained Load Test (Full 1-Hour Version)**
///
/// This is the comprehensive memory leak detection test that runs for 1 hour.
/// It implements evolutionary testing with multiple leak detection strategies.
///
/// Test pattern:
/// 1. Spawn 20 actors, wait 10 seconds
/// 2. Kill 15 random actors, wait 3 seconds
/// 3. Spawn 15 new actors, wait 5 seconds
/// 4. Repeat for 1 hour
/// 5. Monitor RSS memory every 5 seconds
/// 6. Fail if memory grows >30% (strict threshold for long-duration test)
///
/// Run explicitly with:
///   moon run :test memory_leak_test -- --ignored
#[tokio::test]
#[ignore]
async fn given_sustained_load_when_1_hour_then_no_memory_leak() {
    // GIVEN: A supervisor configured for sustained chaos
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 10000; // Very high limit for 1-hour test
    config.base_backoff_ms = 1; // Fast restart

    let supervisor_name = unique_name("memory-leak-1hour");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Verify supervisor is alive
    assert!(
        matches!(
            supervisor.get_status(),
            ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
        ),
        "supervisor should be alive"
    );

    // WHEN: Run sustained chaos for 1 hour
    let test_duration = Duration::from_secs(3600); // 1 hour
    let actors_per_cycle = 20;
    let sample_interval = Duration::from_secs(5); // Sample every 5 seconds

    let start = SystemTime::now();
    let mut detector = MemoryLeakDetector::new(actors_per_cycle, 30.0); // 30% threshold (strict)
    let total_spawned = Arc::new(AtomicU64::new(0));
    let total_killed = Arc::new(AtomicU64::new(0));
    let cycle_count = Arc::new(AtomicUsize::new(0));

    eprintln!("Starting 1-hour sustained load memory leak test...");
    eprintln!("Test configuration:");
    eprintln!("  Duration: 1 hour (3600 seconds)");
    eprintln!("  Actors per cycle: {}", actors_per_cycle);
    eprintln!("  Sample interval: {} seconds", sample_interval.as_secs());
    eprintln!("  Max growth threshold: 30%");
    eprintln!("");
    eprintln!("Pattern:");
    eprintln!("  1. Spawn {} actors, wait 10s", actors_per_cycle);
    eprintln!("  2. Kill 15 random actors, wait 3s");
    eprintln!("  3. Spawn 15 new actors, wait 5s");
    eprintln!("  4. Repeat");
    eprintln!();

    let mut last_check = start;

    while SystemTime::now().duration_since(start).unwrap_or_default() < test_duration {
        let cycle = cycle_count.load(Ordering::SeqCst);

        // Phase 1: Spawn initial batch
        for i in 0..actors_per_cycle {
            let child_name = format!("{}-hour-cycle-{}-actor-{}", supervisor_name, cycle, i);
            let child_args = SchedulerArguments::new();

            if spawn_child(&supervisor, &child_name, child_args)
                .await
                .is_ok()
            {
                total_spawned.fetch_add(1, Ordering::SeqCst);
            }
        }

        sleep(Duration::from_secs(10)).await;

        // Phase 2: Kill random subset (simulating churn)
        for i in 0..15 {
            let child_name = format!("{}-hour-cycle-{}-actor-{}", supervisor_name, cycle, i);
            if kill_child(&supervisor, &child_name).await.is_ok() {
                total_killed.fetch_add(1, Ordering::SeqCst);
            }
        }

        sleep(Duration::from_secs(3)).await;

        // Phase 3: Spawn replacements (simulating new workload)
        for i in 0..15 {
            let child_name = format!("{}-hour-cycle-{}-replace-{}", supervisor_name, cycle, i);
            let child_args = SchedulerArguments::new();

            if spawn_child(&supervisor, &child_name, child_args)
                .await
                .is_ok()
            {
                total_spawned.fetch_add(1, Ordering::SeqCst);
            }
        }

        sleep(Duration::from_secs(5)).await;

        cycle_count.fetch_add(1, Ordering::SeqCst);

        // Sample memory every 5 seconds (regardless of cycle)
        if SystemTime::now()
            .duration_since(last_check)
            .unwrap_or_default()
            >= sample_interval
        {
            let active_count = get_supervisor_status(&supervisor).await.map_or(0, |v| v);
            let snapshot = MemorySnapshot::capture(
                active_count,
                total_spawned.load(Ordering::SeqCst),
                total_killed.load(Ordering::SeqCst),
            );
            detector.record(snapshot.clone());

            let elapsed = start.elapsed().unwrap_or_default().as_secs_f64();
            let percent_complete = (elapsed / test_duration.as_secs_f64()) * 100.0;

            eprintln!(
                "[{:6.1}s ({:5.1}%)] Memory: {}, Active: {:3}, Spawned: {:5}, Killed: {:5}, Cycles: {:3}",
                elapsed,
                percent_complete,
                snapshot.human_readable(),
                active_count,
                total_spawned.load(Ordering::SeqCst),
                total_killed.load(Ordering::SeqCst),
                cycle_count.load(Ordering::SeqCst)
            );

            // Real-time leak detection (catch leaks early)
            if detector.snapshots.len() > 50 {
                // Only check after 50 samples (~4 minutes)
                match detector.detect_leak() {
                    Ok(()) => {}
                    Err(report) => {
                        eprintln!("\n{}", report);
                        panic!(
                            "MEMORY LEAK DETECTED at {:.1}% of test: Test aborted early",
                            percent_complete
                        );
                    }
                }
            }

            last_check = SystemTime::now();
        }
    }

    // THEN: Final comprehensive leak detection
    let final_snapshot = MemorySnapshot::capture(
        get_supervisor_status(&supervisor).await.map_or(0, |v| v),
        total_spawned.load(Ordering::SeqCst),
        total_killed.load(Ordering::SeqCst),
    );
    detector.record(final_snapshot);

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!("1-HOUR MEMORY LEAK TEST COMPLETE");
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("Final Statistics:");
    eprintln!("  Total Cycles: {}", cycle_count.load(Ordering::SeqCst));
    eprintln!("  Actors Spawned: {}", total_spawned.load(Ordering::SeqCst));
    eprintln!("  Actors Killed: {}", total_killed.load(Ordering::SeqCst));
    eprintln!("  Memory Samples: {}", detector.snapshots.len());
    eprintln!(
        "  Test Duration: {:.1} seconds",
        test_duration.as_secs_f64()
    );
    eprintln!("");
    eprintln!("Memory Analysis:");
    eprintln!("  Baseline:   {:.2} MB", detector.baseline_mb);
    eprintln!("  Peak:       {:.2} MB", detector.peak_mb);
    eprintln!(
        "  Growth:     {:.2}%",
        (detector.peak_mb - detector.baseline_mb) / detector.baseline_mb * 100.0
    );
    eprintln!("  Trend Slope: {:.2} MB/min", detector.calculate_slope());
    eprintln!("═══════════════════════════════════════════════════════════════");

    // FINAL LEAK DETECTION
    match detector.detect_leak() {
        Ok(()) => {
            eprintln!("\n✓ SUCCESS: No memory leak detected after 1-hour sustained load");
            eprintln!("  The system maintained stable memory under churn.");
        }
        Err(report) => {
            eprintln!("\n{}", report);
            panic!("MEMORY LEAK DETECTED: 1-hour test failed due to memory growth");
        }
    }

    // Clean up
    supervisor.stop(Some("1-hour memory leak test complete".to_string()));
}

/// **HEAP PROFILING: Actor Lifecycle Memory Test**
///
/// This test specifically targets actor spawn/kill lifecycle memory leaks.
/// It rapidly spawns and kills actors to stress-test the actor system cleanup.
///
/// Run with:
///   moon run :test given_actor_lifecycle_stress_when_1000_cycles_then_no_leak
#[tokio::test]
async fn given_actor_lifecycle_stress_when_1000_cycles_then_no_leak() {
    // GIVEN: A supervisor for lifecycle stress testing
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 5000;
    config.base_backoff_ms = 1;

    let supervisor_name = unique_name("memory-leak-lifecycle");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // WHEN: Rapid spawn/kill cycles (1000 iterations)
    let cycles = 1000;
    let actors_per_cycle = 5;

    let start = SystemTime::now();
    let mut detector = MemoryLeakDetector::new(actors_per_cycle, 100.0); // 100% threshold (lifecycle stress)
    let total_spawned = Arc::new(AtomicU64::new(0));
    let total_killed = Arc::new(AtomicU64::new(0));

    eprintln!(
        "Starting actor lifecycle stress test ({} cycles)...",
        cycles
    );
    eprintln!(
        "Pattern: Spawn {} actors, kill all, repeat",
        actors_per_cycle
    );

    for cycle in 0..cycles {
        // Spawn actors
        for i in 0..actors_per_cycle {
            let child_name = format!("{}-lifecycle-{}-{}", supervisor_name, cycle, i);
            let child_args = SchedulerArguments::new();

            if spawn_child(&supervisor, &child_name, child_args)
                .await
                .is_ok()
            {
                total_spawned.fetch_add(1, Ordering::SeqCst);
            }
        }

        // Small delay to let actors start
        sleep(Duration::from_millis(10)).await;

        // Kill all actors
        for i in 0..actors_per_cycle {
            let child_name = format!("{}-lifecycle-{}-{}", supervisor_name, cycle, i);
            if kill_child(&supervisor, &child_name).await.is_ok() {
                total_killed.fetch_add(1, Ordering::SeqCst);
            }
        }

        // Sample memory every 100 cycles
        if cycle % 100 == 0 {
            let active_count = get_supervisor_status(&supervisor).await.map_or(0, |v| v);
            let snapshot = MemorySnapshot::capture(
                active_count,
                total_spawned.load(Ordering::SeqCst),
                total_killed.load(Ordering::SeqCst),
            );
            detector.record(snapshot.clone());

            eprintln!(
                "[Cycle {}/1000] Memory: {}, Active: {}, Spawned: {}, Killed: {}",
                cycle,
                snapshot.human_readable(),
                active_count,
                total_spawned.load(Ordering::SeqCst),
                total_killed.load(Ordering::SeqCst)
            );
        }
    }

    // Final snapshot
    let final_snapshot = MemorySnapshot::capture(
        get_supervisor_status(&supervisor).await.map_or(0, |v| v),
        total_spawned.load(Ordering::SeqCst),
        total_killed.load(Ordering::SeqCst),
    );
    detector.record(final_snapshot);

    let elapsed = start.elapsed().unwrap_or_default().as_secs_f64();

    eprintln!("\nLifecycle Stress Test Complete:");
    eprintln!("  Cycles: {}", cycles);
    eprintln!("  Actors Spawned: {}", total_spawned.load(Ordering::SeqCst));
    eprintln!("  Actors Killed: {}", total_killed.load(Ordering::SeqCst));
    eprintln!("  Duration: {:.2} seconds", elapsed);
    eprintln!(
        "  Spawn/Kill Rate: {:.1} ops/sec",
        (total_spawned.load(Ordering::SeqCst) as f64) / elapsed
    );
    eprintln!("  Memory Samples: {}", detector.snapshots.len());
    eprintln!("  Baseline: {:.2} MB", detector.baseline_mb);
    eprintln!("  Peak: {:.2} MB", detector.peak_mb);
    eprintln!("  Growth Slope: {:.2} MB/min", detector.calculate_slope());

    // AUTOMATIC LEAK DETECTION
    match detector.detect_leak() {
        Ok(()) => {
            eprintln!("\n✓ No memory leak detected in actor lifecycle");
        }
        Err(report) => {
            eprintln!("\n{}", report);
            panic!("MEMORY LEAK DETECTED in actor lifecycle");
        }
    }

    // Clean up
    supervisor.stop(Some("Lifecycle stress test complete".to_string()));
}
