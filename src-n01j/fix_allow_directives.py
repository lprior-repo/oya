#!/usr/bin/env python3
import subprocess
import re

files = [
    "crates/events/src/bus.rs",
    "crates/core/src/execution.rs",
    "crates/pipeline/src/domain.rs",
    "crates/oya-ipc/src/transport.rs",
    "crates/orchestrator/tests/e2e_crash_recovery_test.rs",
    "crates/orchestrator/src/actors/storage.rs",
    "crates/core/src/workflow.rs",
    "crates/core/src/visualization.rs",
    "crates/events/src/store.rs",
    "crates/zellij-frontend/src/integration_test_worker.rs",
    "crates/zellij-frontend/src/layout.rs",
    "crates/pipeline/src/stages.rs",
    "crates/zellij-frontend/src/command.rs",
    "crates/workflow/src/cleanup/mod.rs",
    "crates/pipeline/src/persistence.rs",
    "crates/oya/src/commands/storm.rs",
    "crates/oya-web/tests/workflow_graph_test.rs",
    "crates/oya-web/tests/agent_metrics_test.rs",
    "crates/oya-ipc/tests/transport_happy_path_tests.rs",
    "crates/oya-ipc/tests/transport_error_tests.rs",
    "crates/oya-web/src/validation.rs",
    "crates/oya-ipc/benches/transport_bench.rs",
    "crates/orchestrator/tests/surreal_connection_chaos.rs",
    "crates/orchestrator/tests/state_manager_actor_test.rs",
    "crates/orchestrator/tests/distribution_queue_bdd.rs",
    "crates/orchestrator/tests/agent_slot_loop_integration_test.rs",
    "crates/orchestrator/tests/agent_health_test.rs",
    "crates/orchestrator/tests/agent_health_integration_test.rs",
    "crates/orchestrator/tests/agent_assignment_test.rs",
    "crates/orchestrator/src/actors/examples/messaging/logger.rs",
    "crates/events/tests/worker_assignment_test.rs",
    "crates/events/tests/schema_test.rs",
    "crates/events/tests/event_serialization_tests.rs",
    "crates/core/src/slug.rs",
]

for f in files:
    with open(f, "r") as file:
        content = file.read()

    # Remove lines with allow directives for unwrap/expect
    content = re.sub(r"#\!\[allow\(clippy::unwrap_used\)\]", "", content)
    content = re.sub(r"#\!\[allow\(clippy::expect_used\)\]", "", content)
    content = re.sub(
        r"#\[allow\(clippy::expect_used, clippy::unwrap_used\)\]", "", content
    )
    content = re.sub(
        r"#\[allow\(clippy::unwrap_used, clippy::expect_used\)\]", "", content
    )

    with open(f, "w") as file:
        file.write(content)

    print(f"Processed {f}")
