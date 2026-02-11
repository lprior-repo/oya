# Martin Fowler Test Plan: IPC Worker for Zellij Integration

**Bead ID:** `src-wkni`
**Generated:** 2026-02-07 23:24:48
**Reference:** [Martin Fowler's Test Patterns](https://martinfowler.com/bliki/TestPyramid.html)
**Type:** Feature - Actor-based IPC Worker

## Test Strategy Overview

This test plan follows Martin Fowler's testing philosophy with a balanced test pyramid:
- **Unit tests**: Fast, isolated, numerous (message handlers, event conversion)
- **Integration tests**: Slower, realistic interactions (mock orchestrator, real transport)
- **End-to-end tests**: Slowest, critical paths only (full plugin lifecycle)

## Test Categories

### 1. Unit Tests (70% of tests)

#### 1.1 Message Handler Tests

```rust
#[cfg(test)]
mod message_handler_tests {
    use super::*;
    use rstest::*;
    use crate::test_utils::MockOrchestrator;

    #[tokio::test]
    async fn test_handle_get_bead_list() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.add_bead(Bead {
            id: "test-1".to_string(),
            title: "Test Bead 1".to_string(),
            status: BeadStatus::Open,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetBeadList;

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::BeadList { beads } => {
                assert_eq!(beads.len(), 1);
                assert_eq!(beads[0].id, "test-1");
            }
            _ => panic!("Expected BeadList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_bead_detail_found() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.add_bead(Bead {
            id: "test-1".to_string(),
            title: "Test Bead 1".to_string(),
            description: Some("Test description".to_string()),
            status: BeadStatus::InProgress,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetBeadDetail {
            id: "test-1".to_string(),
        };

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::BeadDetail { bead } => {
                assert_eq!(bead.id, "test-1");
                assert_eq!(bead.title, "Test Bead 1");
            }
            _ => panic!("Expected BeadDetail response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_bead_detail_not_found() {
        let mock_orchestrator = MockOrchestrator::new();
        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetBeadDetail {
            id: "nonexistent".to_string(),
        };

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::Error { code, message, .. } => {
                assert_eq!(code, "BEAD_NOT_FOUND");
                assert!(message.contains("nonexistent"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_workflow_graph() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.set_dag(TestDag::with_nodes(vec!["node1", "node2"]));

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetWorkflowGraph;

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::WorkflowGraph { graph } => {
                assert_eq!(graph.nodes.len(), 2);
            }
            _ => panic!("Expected WorkflowGraph response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_agent_pool() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.add_agent(Agent {
            id: "agent-1".to_string(),
            status: AgentStatus::Idle,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetAgentPool;

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::AgentPoolStats { stats } => {
                assert_eq!(stats.total_agents, 1);
                assert_eq!(stats.idle_agents, 1);
            }
            _ => panic!("Expected AgentPoolStats response"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_system_health() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.set_health(SystemHealth {
            status: HealthStatus::Healthy,
            uptime_seconds: 3600,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetSystemHealth;

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::SystemHealth { health } => {
                assert_eq!(health.status, HealthStatus::Healthy);
            }
            _ => panic!("Expected SystemHealth response"),
        }
    }
}
```

**Coverage goal:** >90% of message handler code

#### 1.2 Command Execution Tests

```rust
#[cfg(test)]
mod command_execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_start_bead_success() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.add_bead(Bead {
            id: "test-1".to_string(),
            status: BeadStatus::Open,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::StartBead {
            id: "test-1".to_string(),
        };

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::Ack => {
                assert!(mock_orchestrator.was_bead_started("test-1"));
            }
            _ => panic!("Expected Ack response"),
        }
    }

    #[tokio::test]
    async fn test_handle_start_bead_not_found() {
        let mock_orchestrator = MockOrchestrator::new();
        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::StartBead {
            id: "nonexistent".to_string(),
        };

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::Error { code, .. } => {
                assert_eq!(code, "BEAD_NOT_FOUND");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_cancel_bead_success() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.add_bead(Bead {
            id: "test-1".to_string(),
            status: BeadStatus::InProgress,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::CancelBead {
            id: "test-1".to_string(),
        };

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::Ack => {
                assert!(mock_orchestrator.was_bead_cancelled("test-1"));
            }
            _ => panic!("Expected Ack response"),
        }
    }

    #[tokio::test]
    async fn test_handle_retry_bead_success() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.add_bead(Bead {
            id: "test-1".to_string(),
            status: BeadStatus::Failed,
        });

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::RetryBead {
            id: "test-1".to_string(),
        };

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::Ack => {
                assert!(mock_orchestrator.was_bead_retried("test-1"));
            }
            _ => panic!("Expected Ack response"),
        }
    }
}
```

#### 1.3 Event Conversion Tests

```rust
#[cfg(test)]
mod event_conversion_tests {
    use super::*;

    #[test]
    fn test_event_to_host_message_bead_state_changed() {
        let worker = create_test_worker(MockOrchestrator::new());
        let event = OrchestratorEvent::BeadStateChanged {
            bead_id: "test-1".to_string(),
            old_status: BeadStatus::Open,
            new_status: BeadStatus::InProgress,
        };

        let host_msg = worker.event_to_host_message(event);

        assert!(host_msg.is_some());
        match host_msg.unwrap() {
            HostMessage::BeadStateChanged { bead_id, new_status } => {
                assert_eq!(bead_id, "test-1");
                assert_eq!(new_status, BeadStatus::InProgress);
            }
            _ => panic!("Expected BeadStateChanged message"),
        }
    }

    #[test]
    fn test_event_to_host_message_phase_progress() {
        let worker = create_test_worker(MockOrchestrator::new());
        let event = OrchestratorEvent::PhaseProgress {
            bead_id: "test-1".to_string(),
            phase: "building".to_string(),
            progress: 0.5,
        };

        let host_msg = worker.event_to_host_message(event);

        assert!(host_msg.is_some());
        match host_msg.unwrap() {
            HostMessage::PhaseProgress { bead_id, phase, progress } => {
                assert_eq!(bead_id, "test-1");
                assert_eq!(phase, "building");
                assert_eq!(progress, 0.5);
            }
            _ => panic!("Expected PhaseProgress message"),
        }
    }

    #[test]
    fn test_event_to_host_message_agent_heartbeat() {
        let worker = create_test_worker(MockOrchestrator::new());
        let event = OrchestratorEvent::AgentHeartbeat {
            agent_id: "agent-1".to_string(),
            timestamp: Utc::now(),
        };

        let host_msg = worker.event_to_host_message(event);

        assert!(host_msg.is_some());
        match host_msg.unwrap() {
            HostMessage::AgentHeartbeat { agent_id, .. } => {
                assert_eq!(agent_id, "agent-1");
            }
            _ => panic!("Expected AgentHeartbeat message"),
        }
    }

    #[test]
    fn test_event_to_host_message_system_alert() {
        let worker = create_test_worker(MockOrchestrator::new());
        let event = OrchestratorEvent::SystemAlert {
            level: AlertLevel::Warning,
            message: "Test alert".to_string(),
        };

        let host_msg = worker.event_to_host_message(event);

        assert!(host_msg.is_some());
        match host_msg.unwrap() {
            HostMessage::SystemAlert { level, message } => {
                assert_eq!(level, AlertLevel::Warning);
                assert_eq!(message, "Test alert");
            }
            _ => panic!("Expected SystemAlert message"),
        }
    }
}
```

#### 1.4 Error Path Tests

```rust
#[cfg(test)]
mod error_path_tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_error_returns_error_response() {
        let mock_orchestrator = MockOrchestrator::new();
        mock_orchestrator.set_error("Orchestrator unavailable");

        let worker = create_test_worker(mock_orchestrator);
        let msg = GuestMessage::GetBeadList;

        let response = worker.handle_message(msg).await.unwrap();

        match response {
            HostMessage::Error { code, message, .. } => {
                assert_eq!(code, "ORCHESTRATOR_ERROR");
                assert!(message.contains("unavailable"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_serialization_error_returns_error_response() {
        let worker = create_test_worker(MockOrchestrator::new());
        // Send malformed message (this would be caught at transport layer)
        // But we can test the error handling path

        let response = worker.handle_invalid_message().await.unwrap();

        match response {
            HostMessage::Error { code, .. } => {
                assert_eq!(code, "SERIALIZATION_ERROR");
            }
            _ => panic!("Expected Error response"),
        }
    }
}
```

**Coverage goal:** 100% of error paths

#### 1.5 Property-Based Tests

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_bead_list_preserves_all_beads(
            bead_ids in prop::collection::hash_set("[a-z0-9-]{1,20}", 1..100)
        ) {
            let mock_orchestrator = MockOrchestrator::new();
            for id in &bead_ids {
                mock_orchestrator.add_bead(Bead {
                    id: id.clone(),
                    title: format!("Bead {}", id),
                    status: BeadStatus::Open,
                });
            }

            let worker = create_test_worker(mock_orchestrator);
            let msg = GuestMessage::GetBeadList;

            let response = worker.handle_message(msg).await.unwrap();

            match response {
                HostMessage::BeadList { beads } => {
                    assert_eq!(beads.len(), bead_ids.len());
                    let returned_ids: HashSet<_> = beads.iter().map(|b| b.id.clone()).collect();
                    assert_eq!(returned_ids, bead_ids);
                }
                _ => panic!("Expected BeadList response"),
            }
        }
    }
}
```

### 2. Integration Tests (20% of tests)

#### 2.1 Transport Integration Tests

```rust
#[tokio::test]
async fn test_real_transport_message_roundtrip() {
    // Create mock stdin/stdout
    let (mut stdin_writer, mut stdin_reader) = pipe();
    let (mut stdout_writer, mut stdout_reader) = pipe();

    let transport = IpcTransport::new(stdout_reader, stdin_writer);

    // Spawn worker task
    let worker_handle = tokio::spawn(async move {
        let mock_orchestrator = MockOrchestrator::new();
        let mut worker = IpcWorker::new_with_transport(
            transport,
            Arc::new(mock_orchestrator),
        ).unwrap();
        worker.start().await
    });

    // Send message via stdin
    let msg = GuestMessage::GetBeadList;
    let msg_json = serde_json::to_string(&msg).unwrap();
    stdin_writer.write_all(msg_json.as_bytes()).await.unwrap();
    stdin_writer.write_all(b"\n").await.unwrap();

    // Read response from stdout
    let mut response_buf = Vec::new();
    stdout_reader.read_to_end(&mut response_buf).await.unwrap();
    let response: HostMessage = serde_json::from_slice(&response_buf).unwrap();

    match response {
        HostMessage::BeadList { .. } => {
            // Success
        }
        _ => panic!("Expected BeadList response"),
    }

    worker_handle.await.unwrap().unwrap();
}
```

#### 2.2 Orchestrator Integration Tests

```rust
#[tokio::test]
async fn test_real_orchestrator_queries() {
    let real_orchestrator = Orchestrator::new_test().await;

    // Add test bead
    real_orchestrator.add_bead(Bead {
        id: "test-1".to_string(),
        title: "Test Bead 1".to_string(),
        status: BeadStatus::Open,
    }).await.unwrap();

    let worker = IpcWorker::new(
        mock_plugin_process(),
        Arc::new(real_orchestrator),
    ).unwrap();

    let msg = GuestMessage::GetBeadDetail {
        id: "test-1".to_string(),
    };

    let response = worker.handle_message(msg).await.unwrap();

    match response {
        HostMessage::BeadDetail { bead } => {
            assert_eq!(bead.id, "test-1");
        }
        _ => panic!("Expected BeadDetail response"),
    }
}
```

#### 2.3 Event Broadcasting Tests

```rust
#[tokio::test]
async fn test_event_broadcast_to_multiple_subscribers() {
    let mock_orchestrator = MockOrchestrator::new();
    let worker = create_test_worker(mock_orchestrator);

    // Create multiple subscribers
    let mut rx1 = worker.event_tx.subscribe();
    let mut rx2 = worker.event_tx.subscribe();
    let mut rx3 = worker.event_tx.subscribe();

    // Start broadcasting task
    let broadcast_handle = tokio::spawn(worker.broadcast_events());

    // Trigger orchestrator event
    mock_orchestrator.emit_event(OrchestratorEvent::BeadStateChanged {
        bead_id: "test-1".to_string(),
        old_status: BeadStatus::Open,
        new_status: BeadStatus::InProgress,
    });

    // All subscribers should receive the event
    let msg1 = rx1.recv().await.unwrap();
    let msg2 = rx2.recv().await.unwrap();
    let msg3 = rx3.recv().await.unwrap();

    match msg1 {
        HostMessage::BeadStateChanged { bead_id, .. } => {
            assert_eq!(bead_id, "test-1");
        }
        _ => panic!("Expected BeadStateChanged"),
    }

    // All should be the same
    assert_eq!(msg1, msg2);
    assert_eq!(msg2, msg3);

    broadcast_handle.abort();
}
```

### 3. End-to-End Tests (10% of tests)

#### 3.1 Full Plugin Lifecycle

```rust
#[tokio::test]
async fn e2e_plugin_spawn_to_shutdown() {
    let orchestrator = Orchestrator::new_test().await;

    // Spawn plugin process
    let (worker, worker_handle) = IpcWorker::spawn_plugin(
        PathBuf::from("./test-plugin.sh"),
        Arc::new(orchestrator),
    ).await.unwrap();

    // Send test message
    worker.transport.send(GuestMessage::GetBeadList).await.unwrap();

    // Wait for response
    let response = worker.transport.recv::<HostMessage>().await.unwrap().unwrap();
    match response {
        HostMessage::BeadList { .. } => {
            // Success
        }
        _ => panic!("Expected BeadList response"),
    }

    // Shutdown
    worker.shutdown().await.unwrap();
    worker_handle.await.unwrap();
}
```

#### 3.2 Multiple Concurrent Connections

```rust
#[tokio::test]
async fn e2e_multiple_concurrent_workers() {
    let orchestrator = Arc::new(Orchestrator::new_test().await);

    // Spawn 10 workers
    let mut workers = Vec::new();
    for i in 0..10 {
        let (worker, handle) = IpcWorker::spawn_plugin(
            PathBuf::from("./test-plugin.sh"),
            orchestrator.clone(),
        ).await.unwrap();
        workers.push((worker, handle));
    }

    // Send messages to all workers
    for (worker, _) in &mut workers {
        worker.transport.send(GuestMessage::GetBeadList).await.unwrap();
    }

    // Receive responses from all workers
    for (worker, _) in &mut workers {
        let response = worker.transport.recv::<HostMessage>().await.unwrap().unwrap();
        match response {
            HostMessage::BeadList { .. } => {
                // Success
            }
            _ => panic!("Expected BeadList response"),
        }
    }

    // Shutdown all workers
    for (mut worker, handle) in workers {
        worker.shutdown().await.unwrap();
        handle.await.unwrap();
    }
}
```

#### 3.3 Connection Crash Recovery

```rust
#[tokio::test]
async fn e2e_plugin_crash_recovery() {
    let orchestrator = Arc::new(Orchestrator::new_test().await);

    // Spawn plugin
    let (mut worker, worker_handle) = IpcWorker::spawn_plugin(
        PathBuf::from("./test-plugin-crash.sh"), // Script that crashes
        Arc::clone(&orchestrator),
    ).await.unwrap();

    // Send message that triggers crash
    worker.transport.send(GuestMessage::GetBeadList).await.unwrap();

    // Worker should terminate gracefully
    let result = worker_handle.await;
    assert!(result.is_err()); // Worker crashed

    // Verify orchestrator state is still valid
    let health = orchestrator.get_health().await.unwrap();
    assert_eq!(health.status, HealthStatus::Healthy);
}
```

### 4. Performance Tests

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

    fn benchmark_query_handling(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut group = c.benchmark_group("query_handling");

        for bead_count in [10, 100, 1000].iter() {
            let mock_orchestrator = MockOrchestrator::new();
            for i in 0..*bead_count {
                mock_orchestrator.add_bead(Bead {
                    id: format!("bead-{}", i),
                    title: format!("Bead {}", i),
                    status: BeadStatus::Open,
                });
            }

            let worker = rt.block_on(async {
                create_test_worker(mock_orchestrator)
            });

            group.bench_with_input(
                BenchmarkId::from_parameter(bead_count),
                bead_count,
                |b, _| {
                    b.iter(|| {
                        let msg = GuestMessage::GetBeadList;
                        rt.block_on(async {
                            black_box(worker.handle_message(msg).await.unwrap())
                        })
                    })
                },
            );
        }

        group.finish();
    }

    fn benchmark_event_broadcast(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut group = c.benchmark_group("event_broadcast");

        for subscriber_count in [1, 10, 100].iter() {
            let worker = rt.block_on(async {
                let mock_orchestrator = MockOrchestrator::new();
                let worker = create_test_worker(mock_orchestrator);

                // Create subscribers
                for _ in 0..*subscriber_count {
                    let _ = worker.event_tx.subscribe();
                }

                worker
            });

            group.bench_with_input(
                BenchmarkId::from_parameter(subscriber_count),
                subscriber_count,
                |b, _| {
                    b.iter(|| {
                        let event = OrchestratorEvent::BeadStateChanged {
                            bead_id: "test-1".to_string(),
                            old_status: BeadStatus::Open,
                            new_status: BeadStatus::InProgress,
                        };
                        black_box(worker.event_to_host_message(event))
                    })
                },
            );
        }

        group.finish();
    }

    criterion_group!(benches, benchmark_query_handling, benchmark_event_broadcast);
    criterion_main!(benches);
}
```

## Test Organization

```
crates/orchestrator/
├── tests/
│   ├── unit/                         # Pure logic tests
│   │   ├── message_handlers.rs       # All GuestMessage handlers
│   │   ├── command_execution.rs      # Command operations
│   │   ├── event_conversion.rs       # Event → HostMessage
│   │   ├── error_paths.rs            # All error branches
│   │   └── properties.rs             # Property-based tests
│   ├── integration/                  # Real dependencies
│   │   ├── transport_integration.rs  # Real IpcTransport
│   │   ├── orchestrator_integration.rs # Real Orchestrator
│   │   └── event_broadcasting.rs     # Multiple subscribers
│   └── e2e/                          # Full system tests
│       ├── plugin_lifecycle.rs       # Spawn → crash → reconnect
│       ├── concurrent_connections.rs  # 100 concurrent workers
│       └── crash_recovery.rs         # Worker crash handling
├── benches/
│   └── ipc_worker.rs                 # Performance benchmarks
└── src/
    └── actors/
        └── ipc_worker.rs             # Includes unit tests as mod tests
```

## Test Data Management

### Fixtures

```rust
#[fixture]
fn mock_orchestrator() -> MockOrchestrator {
    MockOrchestrator::new()
}

#[fixture]
fn test_worker() -> IpcWorker {
    create_test_worker(MockOrchestrator::new())
}

#[fixture]
fn test_bead() -> Bead {
    Bead {
        id: "test-1".to_string(),
        title: "Test Bead".to_string(),
        status: BeadStatus::Open,
    }
}
```

### Test Factories

```rust
fn create_test_worker(orchestrator: MockOrchestrator) -> IpcWorker {
    IpcWorker::new(
        mock_plugin_process(),
        Arc::new(orchestrator),
    ).unwrap()
}

fn mock_plugin_process() -> Child {
    // Create mock child process with pipe stdin/stdout
    // ...
}
```

## Mock Strategy

- **Use MockOrchestrator** for unit tests (fast, deterministic)
- **Use real IpcTransport** for integration tests (fast, realistic)
- **Use real Orchestrator** for some integration tests (slow, comprehensive)
- **No mocks for external services** (this is isolated integration)

## Test Execution

```bash
# Unit tests only (fast feedback)
moon run :test-unit --package oya-orchestrator

# Integration tests
moon run :test-integration --package oya-orchestrator

# Full test suite
moon run :test --package oya-orchestrator

# With coverage
moon run :test-coverage --package oya-orchestrator

# Performance benchmarks
moon run :bench --package oya-orchestrator
```

## Acceptance Criteria

1. [ ] All unit tests passing (>90% coverage)
2. [ ] All integration tests passing
3. [ ] All E2E tests passing
4. [ ] Property tests finding no counterexamples
5. [ ] Performance benchmarks meet targets:
   - Query handling <10µs (p99)
   - Event broadcast <5µs per subscriber
   - 100 concurrent workers
6. [ ] Zero flaky tests (100% deterministic)
7. [ ] All GuestMessage types handled
8. [ ] All commands execute successfully
9. [ ] Event broadcasting works to multiple subscribers
10. [ ] Connection lifecycle works (spawn, crash, reconnect)

## Test Metrics

| Metric | Target | Actual |
|--------|--------|--------|
| Unit test coverage | >90% | TBD |
| Integration test count | 20% of unit tests | TBD |
| E2E test count | 10% of unit tests | TBD |
| Test execution time | <30s (unit) | TBD |
| Flaky test rate | 0% | TBD |
| Message handlers covered | 100% (8 handlers) | TBD |
| Event types covered | 100% (4 types) | TBD |

## Test Checklist

### Unit Tests
- [ ] GetBeadList handler
- [ ] GetBeadDetail handler (found, not found)
- [ ] GetWorkflowGraph handler
- [ ] GetAgentPool handler
- [ ] GetSystemHealth handler
- [ ] StartBead handler (success, not found)
- [ ] CancelBead handler (success, not found)
- [ ] RetryBead handler (success, not found)
- [ ] Event conversion (4 event types)
- [ ] Error paths (orchestrator error, serialization error)
- [ ] Property tests (bead list preservation)

### Integration Tests
- [ ] Real transport message roundtrip
- [ ] Real orchestrator queries
- [ ] Event broadcast to multiple subscribers
- [ ] Lag handling (subscriber falls behind)

### E2E Tests
- [ ] Plugin spawn to shutdown lifecycle
- [ ] Multiple concurrent connections (10 workers)
- [ ] Plugin crash recovery
- [ ] Connection timeout handling

### Performance Tests
- [ ] Query handling (10, 100, 1000 beads)
- [ ] Event broadcast (1, 10, 100 subscribers)
- [ ] Concurrent connection scaling

---

*Generated by Architect Agent*
*Test plan status: COMPLETE - Ready for implementation*
