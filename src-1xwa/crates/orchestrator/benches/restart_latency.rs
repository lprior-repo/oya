// Chaos test: Supervisor restart latency performance test
//
// Requirements:
// - Measure p50, p95, p99 latencies for supervisor restart
// - Test with varying load levels (idle, light, heavy)
// - Assert p99 < 1s
// - Add flamegraph profiling support

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use orchestrator::actors::supervisor::{
    GenericSupervisableActor, SupervisorArguments, SupervisorConfig, SupervisorMessage,
    spawn_supervisor_with_name,
};
use orchestrator::actors::worker::{WorkerActorDef, WorkerConfig, WorkerMessage};
use ractor::{Actor, ActorRef};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tracing::info;

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
                "p99 latency {} exceeds requirement {}",
                self.p99.as_millis(),
                max_p99.as_millis()
            ))
        }
    }
}

/// Test actor for chaos testing
#[derive(Clone, Default)]
struct TestActorDef;

impl Actor for TestActorDef {
    type Msg = TestMessage;
    type State = TestState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ractor::ActorProcessingErr> {
        Ok(TestState { message_count: 0 })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ractor::ActorProcessingErr> {
        match message {
            TestMessage::Ping => {
                state.message_count = state.message_count.wrapping_add(1);
            }
        }
        Ok(())
    }
}

impl GenericSupervisableActor for TestActorDef {
    fn default_args() -> Self::Arguments {
        ()
    }
}

#[derive(Debug, Clone)]
enum TestMessage {
    Ping,
}

#[derive(Debug, Clone)]
struct TestState {
    message_count: u64,
}

/// Measure restart latency under specific load conditions
async fn measure_restart_latency(load: LoadLevel, iterations: usize) -> RestartMetrics {
    let config = SupervisorConfig::for_testing();
    let args = SupervisorArguments::new().with_config(config);

    let supervisor = spawn_supervisor_with_name::<TestActorDef>(
        args,
        &format!("restart-latency-test-{}", load.as_str()),
    )
    .await
    .expect("Failed to spawn supervisor");

    // Spawn child actors
    let child_count = 5;
    for i in 0..child_count {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = supervisor.send_message(SupervisorMessage::<TestActorDef>::SpawnChild {
            name: format!("child-{}", i),
            args: (),
            reply: tx,
        });

        let _ = rx.await;
    }

    // Generate load if needed
    let mut handles = Vec::new();
    for _ in 0..load.concurrent_ops() {
        let supervisor_clone = supervisor.clone();
        let handle = tokio::spawn(async move {
            let mut counter = 0u64;
            while counter < 1000 {
                // Simulate background work
                let (tx, rx) = tokio::sync::oneshot::channel();
                let _ = supervisor_clone
                    .send_message(SupervisorMessage::<TestActorDef>::GetStatus { reply: tx });
                let _ = rx.await;
                counter = counter.wrapping_add(1);
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }

    // Warmup
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Measure restart latencies
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let child_name = format!("child-{}", i % child_count);

        let start = Instant::now();

        // Stop child (simulates crash)
        supervisor
            .send_message(SupervisorMessage::<TestActorDef>::StopChild {
                name: child_name.clone(),
            })
            .expect("Failed to send stop message");

        // Wait for restart by checking status
        let mut restarted = false;
        let timeout = Duration::from_secs(5);
        let check_start = Instant::now();

        while !restarted && check_start.elapsed() < timeout {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ =
                supervisor.send_message(SupervisorMessage::<TestActorDef>::GetStatus { reply: tx });

            if let Ok(status) = rx.await {
                if status.active_children >= child_count {
                    restarted = true;
                }
            }

            if !restarted {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        let elapsed = start.elapsed();
        latencies.push(elapsed);

        // Cooldown between restarts
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Cancel load tasks
    for handle in handles {
        handle.abort();
    }

    // Cleanup
    supervisor.stop(None);

    RestartMetrics::from_latencies(latencies)
}

/// Test worker actor restart latency (more realistic scenario)
async fn measure_worker_restart_latency(load: LoadLevel, iterations: usize) -> RestartMetrics {
    let config = SupervisorConfig::for_testing();
    let args = SupervisorArguments::new().with_config(config);

    let supervisor = spawn_supervisor_with_name::<WorkerActorDef>(
        args,
        &format!("worker-restart-test-{}", load.as_str()),
    )
    .await
    .expect("Failed to spawn supervisor");

    // Spawn worker children
    let child_count = 3;
    for i in 0..child_count {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let worker_config = WorkerConfig::default();
        let _ = supervisor.send_message(SupervisorMessage::<WorkerActorDef>::SpawnChild {
            name: format!("worker-{}", i),
            args: worker_config,
            reply: tx,
        });

        let _ = rx.await;
    }

    // Generate load
    let mut handles = Vec::new();
    for _ in 0..load.concurrent_ops() {
        let supervisor_clone = supervisor.clone();
        let handle = tokio::spawn(async move {
            let mut counter = 0u64;
            while counter < 500 {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let _ = supervisor_clone
                    .send_message(SupervisorMessage::<WorkerActorDef>::GetStatus { reply: tx });
                let _ = rx.await;
                counter = counter.wrapping_add(1);
                tokio::time::sleep(Duration::from_micros(200)).await;
            }
        });
        handles.push(handle);
    }

    // Warmup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Measure restart latencies
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let child_name = format!("worker-{}", i % child_count);

        let start = Instant::now();

        // Stop worker
        supervisor
            .send_message(SupervisorMessage::<WorkerActorDef>::StopChild {
                name: child_name.clone(),
            })
            .expect("Failed to send stop message");

        // Wait for restart
        let mut restarted = false;
        let timeout = Duration::from_secs(5);
        let check_start = Instant::now();

        while !restarted && check_start.elapsed() < timeout {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = supervisor
                .send_message(SupervisorMessage::<WorkerActorDef>::GetStatus { reply: tx });

            if let Ok(status) = rx.await {
                if status.active_children >= child_count {
                    restarted = true;
                }
            }

            if !restarted {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        let elapsed = start.elapsed();
        latencies.push(elapsed);

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Cancel load
    for handle in handles {
        handle.abort();
    }

    supervisor.stop(None);

    RestartMetrics::from_latencies(latencies)
}

fn benchmark_restart_latency(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create runtime");

    // Test different load levels
    for load in [LoadLevel::Idle, LoadLevel::Light, LoadLevel::Heavy] {
        let mut group = c.benchmark_group(format!("restart_latency_{}", load.as_str()));

        // Sample size for statistical significance
        let sample_size = 100;

        group.throughput(Throughput::Elements(sample_size as u64));

        group.bench_function(BenchmarkId::new("test_actor", load.as_str()), |b| {
            b.to_async(&rt)
                .iter(|| measure_restart_latency(black_box(load), black_box(sample_size)))
        });

        group.finish();
    }
}

fn benchmark_worker_restart_latency(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create runtime");

    for load in [LoadLevel::Idle, LoadLevel::Light] {
        let mut group = c.benchmark_group(format!("worker_restart_{}", load.as_str()));

        let sample_size = 50;

        group.throughput(Throughput::Elements(sample_size as u64));

        group.bench_function(BenchmarkId::new("worker_actor", load.as_str()), |b| {
            b.to_async(&rt)
                .iter(|| measure_worker_restart_latency(black_box(load), black_box(sample_size)))
        });

        group.finish();
    }
}

fn benchmark_scalability(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create runtime");

    let mut group = c.benchmark_group("restart_scalability");

    for child_count in [1, 3, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("children", child_count),
            &child_count,
            |b, &count| {
                b.to_async(&rt).iter(|| {
                    let rt = Runtime::new().expect("Failed to create runtime");

                    rt.block_on(async {
                        let config = SupervisorConfig::for_testing();
                        let args = SupervisorArguments::new().with_config(config);

                        let supervisor = spawn_supervisor_with_name::<TestActorDef>(
                            args,
                            &format!("scalability-test-{}", count),
                        )
                        .await
                        .expect("Failed to spawn supervisor");

                        // Spawn children
                        for i in 0..count {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            let _ = supervisor.send_message(
                                SupervisorMessage::<TestActorDef>::SpawnChild {
                                    name: format!("child-{}", i),
                                    args: (),
                                    reply: tx,
                                },
                            );

                            let _ = rx.await;
                        }

                        let start = Instant::now();

                        // Restart first child
                        supervisor
                            .send_message(SupervisorMessage::<TestActorDef>::StopChild {
                                name: "child-0".to_string(),
                            })
                            .expect("Failed to send stop message");

                        // Wait for restart
                        let mut restarted = false;
                        while !restarted && start.elapsed() < Duration::from_secs(5) {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            let _ = supervisor.send_message(
                                SupervisorMessage::<TestActorDef>::GetStatus { reply: tx },
                            );

                            if let Ok(status) = rx.await {
                                if status.active_children >= count {
                                    restarted = true;
                                }
                            }

                            if !restarted {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }

                        let elapsed = start.elapsed();

                        supervisor.stop(None);

                        elapsed
                    })
                })
            },
        );
    }

    group.finish();
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_p99_under_load() {
        let metrics = measure_worker_restart_latency(LoadLevel::Light, 50).await;

        info!(
            p50_ms = metrics.p50.as_millis(),
            p95_ms = metrics.p95.as_millis(),
            p99_ms = metrics.p99.as_millis(),
            "Worker restart latency under light load"
        );

        // Assert p99 < 1s (1000ms)
        let result = metrics.validate_p99(Duration::from_secs(1));
        assert!(
            result.is_ok(),
            "p99 latency validation failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_p99_idle() {
        let metrics = measure_restart_latency(LoadLevel::Idle, 100).await;

        info!(
            p50_ms = metrics.p50.as_millis(),
            p95_ms = metrics.p95.as_millis(),
            p99_ms = metrics.p99.as_millis(),
            "Restart latency at idle"
        );

        let result = metrics.validate_p99(Duration::from_secs(1));
        assert!(
            result.is_ok(),
            "p99 latency validation failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_mean_latency_reasonable() {
        let metrics = measure_restart_latency(LoadLevel::Idle, 50).await;

        // Mean should be significantly less than p99
        assert!(
            metrics.mean < metrics.p99,
            "Mean {} should be less than p99 {}",
            metrics.mean.as_millis(),
            metrics.p99.as_millis()
        );

        // Mean should be reasonable (< 500ms)
        assert!(
            metrics.mean < Duration::from_millis(500),
            "Mean latency {} exceeds 500ms",
            metrics.mean.as_millis()
        );
    }
}

criterion_group!(
    benches,
    benchmark_restart_latency,
    benchmark_worker_restart_latency,
    benchmark_scalability
);
criterion_main!(benches);
