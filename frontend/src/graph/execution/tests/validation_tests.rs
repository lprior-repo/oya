use super::*;

// ---------------------------------------------------------------------------
// verify_graph_connectivity
// ---------------------------------------------------------------------------

#[test]
fn given_isolated_subgraph_when_preparing_run_then_connectivity_error_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    // Node C is isolated — no connection to/from A or B
    let _c = workflow.add_node("run", 20.0, 0.0);

    add_connection(&mut workflow, a, b);

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::InvalidWorkflowState { .. })),
        "isolated node should cause connectivity violation, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// validate_dependencies_exist
// ---------------------------------------------------------------------------

#[test]
fn given_connection_to_nonexistent_target_when_preparing_run_then_unresolved_deps_error_is_returned(
) {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let ghost = NodeId::new();

    // Directly inject a connection referencing a non-existent node
    workflow.connections.push(Connection {
        id: uuid::Uuid::new_v4(),
        source: a,
        target: ghost,
        source_port: main_port(),
        target_port: main_port(),
    });

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::UnresolvedDependencies { .. })),
        "connection to ghost node should fail, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// check_duplicate_connections
// ---------------------------------------------------------------------------

#[test]
fn given_duplicate_connections_when_preparing_run_then_invalid_state_error_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);

    // Add two identical connections (same source -> target)
    add_connection(&mut workflow, a, b);
    add_connection(&mut workflow, a, b);

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::InvalidWorkflowState { .. })),
        "duplicate connections should be rejected, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// check_dirty_state
// ---------------------------------------------------------------------------

#[test]
fn given_nonempty_execution_queue_when_preparing_run_then_dirty_state_error_is_returned() {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 0.0, 0.0);
    workflow.execution_queue.push(NodeId::new());

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::InvalidWorkflowState { .. })),
        "dirty execution_queue should be rejected, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// check_dirty_state — node executing flag
// ---------------------------------------------------------------------------

#[test]
fn given_node_in_executing_state_when_preparing_run_then_invalid_state_error_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    add_connection(&mut workflow, a, b);

    // Manually set a node to executing state to simulate dirty state
    workflow.nodes[0].executing = true;

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::InvalidWorkflowState { .. })),
        "executing node should cause dirty state error, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// verify_graph_connectivity — multiple isolated subgraphs
// ---------------------------------------------------------------------------

#[test]
fn given_two_disconnected_components_when_preparing_run_then_connectivity_error_is_returned() {
    let mut workflow = Workflow::new();
    // Component 1: A -> B
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    add_connection(&mut workflow, a, b);

    // Component 2: C -> D  (disconnected from A-B)
    let c = workflow.add_node("run", 0.0, 100.0);
    let d = workflow.add_node("run", 10.0, 100.0);
    add_connection(&mut workflow, c, d);

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::InvalidWorkflowState { .. })),
        "two disconnected components should cause connectivity error, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// validate_dependencies_exist — connection from nonexistent source
// ---------------------------------------------------------------------------

#[test]
fn given_connection_from_nonexistent_source_when_preparing_run_then_no_unresolved_deps_error() {
    // When the source node is missing, the connection is skipped per the
    // implementation (only checks targets). This test verifies that behavior.
    let mut workflow = Workflow::new();
    let target = workflow.add_node("run", 0.0, 0.0);
    let ghost_source = NodeId::new();

    // Inject connection from ghost source to existing target
    workflow.connections.push(Connection {
        id: uuid::Uuid::new_v4(),
        source: ghost_source,
        target,
        source_port: main_port(),
        target_port: main_port(),
    });

    // The validate_dependencies_exist function skips connections whose source
    // is not in node_ids, so this particular case passes dependency validation
    // (but may fail elsewhere, e.g. connectivity check).
    let result = workflow.prepare_run();
    // It should NOT return UnresolvedDependencies since the source is skipped
    assert!(
        !matches!(result, Err(WorkflowExecutionError::UnresolvedDependencies { .. })),
        "missing source should not trigger unresolved deps error, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// collect_descendants
// ---------------------------------------------------------------------------

#[test]
fn given_branching_graph_when_collecting_descendants_then_all_downstream_nodes_are_found() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    let c = workflow.add_node("run", 10.0, 100.0);
    let d = workflow.add_node("run", 20.0, 0.0);

    add_connection(&mut workflow, a, b);
    add_connection(&mut workflow, a, c);
    add_connection(&mut workflow, b, d);

    let descendants = workflow.collect_descendants(&[a]);

    assert!(descendants.contains(&a), "should include start node");
    assert!(descendants.contains(&b), "should include b");
    assert!(descendants.contains(&c), "should include c");
    assert!(descendants.contains(&d), "should include d");
    assert_eq!(descendants.len(), 4);
}

#[test]
fn given_no_connections_when_collecting_descendants_then_only_start_ids_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let _b = workflow.add_node("run", 10.0, 0.0);

    let descendants = workflow.collect_descendants(&[a]);
    assert_eq!(descendants.len(), 1);
    assert!(descendants.contains(&a));
}

// ---------------------------------------------------------------------------
// resolve_expressions — depth guard
// ---------------------------------------------------------------------------

#[test]
fn given_deeply_nested_config_when_resolving_expressions_then_no_stack_overflow() {
    let workflow = Workflow::new();

    // Build a deeply nested JSON structure (150 levels deep)
    let mut config = serde_json::Value::String("leaf".to_string());
    for _ in 0..150 {
        let mut map = serde_json::Map::new();
        map.insert("nested".to_string(), config);
        config = serde_json::Value::Object(map);
    }

    // Should not panic or stack overflow; depth limit returns config unchanged
    let result = workflow.resolve_expressions(&config);
    assert!(result.is_object(), "deeply nested config should still resolve to an object");
}

// ---------------------------------------------------------------------------
// check_self_references — direct self-loop
// ---------------------------------------------------------------------------

#[test]
fn given_connection_with_same_source_and_target_when_preparing_run_then_cycle_detected() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);

    // Directly inject a self-referencing connection (bypasses add_connection_checked
    // which would reject self-connections)
    workflow.connections.push(Connection {
        id: uuid::Uuid::new_v4(),
        source: a,
        target: a,
        source_port: main_port(),
        target_port: main_port(),
    });

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::CycleDetected { .. })),
        "self-referencing connection should cause CycleDetected, got {result:?}"
    );
}
