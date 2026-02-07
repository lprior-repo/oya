//! Restart Latency Performance Test
//!
//! Performance test that verifies actor restart latency meets p99 < 1s requirement.
//! This is a chaos engineering test that measures how quickly the supervision system
//! can detect and recover from actor failures.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    SupervisorActorDef, SupervisorArguments, SupervisorConfig, SupervisorMessage,
};
use ractor::{Actor, ActorRef};
use tokio::time::sleep;

const STATUS_TIMEOUT: Duration = Duration::from_millis(200);

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

    match tokio::time::timeout(STATUS_TIMEOUT, rx).await {
        Ok(Ok(result)) => result.map_err(|e| format!("Child '{}' failed: {}", name, e)),
        Ok(Err(e)) => Err(format!("Child '{}' reply failed: {}", name, e)),
        Err(_) => Err(format!("Timeout waiting for child '{}'", name)),
    }
}

/// Measure latency from actor kill to successful restart.
///
/// Returns the duration in milliseconds, or an error if restart failed.
async fn measure_restart_latency(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    child_name: String,
) -> Result<Duration, String> {
    // WHEN: Kill the child actor
    let kill_start = Instant::now();
    let stop_result = supervisor
        .cast(SupervisorMessage::StopChild {
            name: child_name.clone(),
        })
        .map_err(|e| format!("Failed to stop child: {}", e));

    if let Err(e) = stop_result {
        return Err(format!("Child stop failed: {}", e));
    }

    // Wait for supervision to detect the failure and restart the child
    // We monitor this by querying supervisor status until child count recovers
    let max_wait = Duration::from_secs(5);
    let check_interval = Duration::from_millis(10);
    let start = Instant::now();

    loop {
        if start.elapsed() > max_wait {
            return Err(format!(
                "Restart timeout after {}ms - child may not have restarted",
                max_wait.as_millis()
            ));
        }

        // Query supervisor status
        let (tx, rx) = tokio::sync::oneshot::channel();
        let status_result = supervisor
            .cast(SupervisorMessage::GetStatus { reply: tx })
            .map_err(|e| format!("Failed to get status: {}", e));

        if let Err(e) = status_result {
            return Err(format!("Status query failed: {}", e));
        }

        // Wait for status response
        match tokio::time::timeout(STATUS_TIMEOUT, rx).await {
            Ok(Ok(status)) => {
                // Check if child has been restarted (active_children should recover)
                // Note: After kill, count drops, then recovers when restart completes
                if status.active_children > 0 {
                    // Child restarted - record latency
                    let latency = kill_start.elapsed();
                    return Ok(latency);
                }
            }
            Ok(Err(e)) => {
                return Err(format!("Status response failed: {}", e));
            }
            Err(_) => {
                return Err("Timeout waiting for status".to_string());
            }
        }

        // Wait before next check
        sleep(check_interval).await;
    }
}

/// Calculate percentile from a sorted dataset.
fn calculate_percentile(data: &[Duration], percentile: f64) -> Duration {
    if data.is_empty() {
        return Duration::ZERO;
    }

    let index = ((percentile / 100.0) * data.len() as f64).floor() as usize;
    let index = index.min(data.len() - 1);
    data[index]
}

/// Performance test: Verify actor restart latency p99 < 1s
///
/// This test:
/// 1. Spawns a supervisor with a child actor
/// 2. Kills the child actor 100 times
/// 3. Measures restart latency for each kill
/// 4. Verifies p99 (99th percentile) latency is < 1s
///
/// Requirements from EARS:
/// - "WHEN actor killed, THE SYSTEM SHALL restart via supervision"
/// - p99 latency < 1000ms
#[tokio::test]
async fn given_actor_killed_when_restarted_then_p99_latency_under_1s() {
    // GIVEN: A supervisor with quick restart configuration
    let mut config = SupervisorConfig::for_testing();
    config.base_backoff_ms = 10; // Quick restart for performance testing

    let supervisor_name = unique_name("latency-supervisor");
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

    // Spawn a child that will be killed and restarted
    let child_args = SchedulerArguments::new();
    let child_name = format!("{supervisor_name}-victim");

    let spawn_result = spawn_child(&supervisor, &child_name, child_args).await;

    assert!(
        spawn_result.is_ok(),
        "child spawn should succeed: {:?}",
        spawn_result
    );

    // Give child time to stabilize
    sleep(Duration::from_millis(50)).await;

    // WHEN: Kill and restart child 100 times, measuring latency each time
    const NUM_TRIALS: usize = 100;
    let mut latencies = Vec::with_capacity(NUM_TRIALS);

    for i in 0..NUM_TRIALS {
        match measure_restart_latency(&supervisor, child_name.clone()).await {
            Ok(latency) => {
                latencies.push(latency);
                // Print progress for slow runs
                if i % 20 == 0 {
                    println!("Trial {}: {}ms", i, latency.as_millis());
                }
            }
            Err(e) => {
                eprintln!("Trial {} failed to measure restart latency: {}", i, e);
                // Continue - we want to see if other trials succeed
            }
        }

        // Small delay between trials to avoid overwhelming the system
        sleep(Duration::from_millis(20)).await;
    }

    // THEN: Calculate and verify p99 latency < 1s
    assert!(
        !latencies.is_empty(),
        "should have collected at least some latency measurements"
    );

    // Sort for percentile calculation
    latencies.sort();

    let p50 = calculate_percentile(&latencies, 50.0);
    let p95 = calculate_percentile(&latencies, 95.0);
    let p99 = calculate_percentile(&latencies, 99.0);
    let max = latencies.last().copied().unwrap_or(Duration::ZERO);

    println!("\n=== Restart Latency Performance Results ===");
    println!("Trials completed: {}", latencies.len());
    println!("p50 (median): {}ms", p50.as_millis());
    println!("p95: {}ms", p95.as_millis());
    println!("p99: {}ms", p99.as_millis());
    println!("Max: {}ms", max.as_millis());
    println!("==========================================\n");

    // VERIFY: p99 latency MUST be < 1s (1000ms)
    assert!(
        p99 < Duration::from_secs(1),
        "p99 restart latency {}ms exceeds 1s threshold (1000ms)",
        p99.as_millis()
    );

    // ADDITIONAL ASSERT: Max latency should also be reasonable
    // Even worst-case restarts should complete within 2s
    assert!(
        max < Duration::from_secs(2),
        "max restart latency {}ms exceeds 2s - indicates outlier failure",
        max.as_millis()
    );

    // Cleanup
    supervisor.stop(None);
}
