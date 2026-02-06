//! Scheduler DAG Integration Tests
//!
//! Integration tests verifying that SchedulerActor correctly uses WorkflowDAG
//! for dependency tracking in real workflow scenarios.
//!
//! Following BDD naming convention: given_<context>_when_<action>_then_<outcome>

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use im::HashSet;
use orchestrator::dag::{DependencyType, WorkflowDAG};
use orchestrator::scheduler::{BeadId, SchedulerActor, WorkflowId};

// ============================================================================
// DEPENDENCY CHAIN TESTS (5 tests)
// ============================================================================

#[test]
fn given_linear_chain_a_b_c_when_get_ready_then_returns_in_dependency_order() {
    // GIVEN: A workflow with linear chain A -> B -> C
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-linear".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    // Schedule beads
    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();

    let schedule_a = scheduler.schedule_bead(workflow_id.clone(), a.clone());
    let schedule_b = scheduler.schedule_bead(workflow_id.clone(), b.clone());
    let schedule_c = scheduler.schedule_bead(workflow_id.clone(), c.clone());

    assert!(schedule_a.is_ok(), "Scheduling A should succeed");
    assert!(schedule_b.is_ok(), "Scheduling B should succeed");
    assert!(schedule_c.is_ok(), "Scheduling C should succeed");

    // Add dependencies: A -> B -> C
    let dep_a_b = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());
    let dep_b_c = scheduler.add_dependency(&workflow_id, b.clone(), c.clone());

    assert!(dep_a_b.is_ok(), "Adding A->B dependency should succeed");
    assert!(dep_b_c.is_ok(), "Adding B->C dependency should succeed");

    // WHEN: Getting ready beads initially
    let ready_initial = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: Only A should be ready (no dependencies)
    assert!(ready_initial.is_ok(), "Getting ready beads should succeed");
    let ready = ready_initial.unwrap_or_default();
    assert_eq!(ready.len(), 1, "Only root node should be ready initially");
    assert!(ready.contains(&a), "A should be the only ready bead");
    assert!(!ready.contains(&b), "B should not be ready yet");
    assert!(!ready.contains(&c), "C should not be ready yet");
}

#[test]
fn given_fan_out_a_to_bcd_when_a_completes_then_all_children_ready() {
    // GIVEN: A workflow with fan-out pattern A -> {B, C, D}
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-fanout".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();
    let d = "bead-d".to_string();

    // Schedule all beads
    for bead in [&a, &b, &c, &d] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling {} should succeed", bead);
    }

    // Add dependencies: A -> B, A -> C, A -> D
    for child in [&b, &c, &d] {
        let result = scheduler.add_dependency(&workflow_id, a.clone(), child.clone());
        assert!(
            result.is_ok(),
            "Adding A->{} dependency should succeed",
            child
        );
    }

    // Initially only A is ready
    let ready_initial = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_initial.is_ok(), "Getting ready beads should succeed");
    let ready = ready_initial.unwrap_or_default();
    assert_eq!(ready.len(), 1, "Only A should be ready initially");
    assert!(ready.contains(&a), "A should be ready");

    // WHEN: A is marked completed
    let workflow_state = scheduler.get_workflow_mut(&workflow_id);
    assert!(workflow_state.is_some(), "Workflow should exist");
    if let Some(state) = workflow_state {
        state.mark_completed(&a);
    }

    // THEN: B, C, and D should all be ready
    let ready_after = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_after.is_ok(), "Getting ready beads should succeed");
    let ready = ready_after.unwrap_or_default();

    assert_eq!(
        ready.len(),
        3,
        "All three children should be ready after A completes"
    );
    assert!(ready.contains(&b), "B should be ready after A completes");
    assert!(ready.contains(&c), "C should be ready after A completes");
    assert!(ready.contains(&d), "D should be ready after A completes");
    assert!(
        !ready.contains(&a),
        "A should not be in ready list (already completed)"
    );
}

#[test]
fn given_fan_in_abc_to_d_when_only_some_complete_then_d_not_ready() {
    // GIVEN: A workflow with fan-in pattern {A, B, C} -> D
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-fanin".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();
    let d = "bead-d".to_string();

    // Schedule all beads
    for bead in [&a, &b, &c, &d] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling {} should succeed", bead);
    }

    // Add dependencies: A -> D, B -> D, C -> D
    for parent in [&a, &b, &c] {
        let result = scheduler.add_dependency(&workflow_id, parent.clone(), d.clone());
        assert!(
            result.is_ok(),
            "Adding {}->{} dependency should succeed",
            parent,
            d
        );
    }

    // Initially A, B, C are ready (no dependencies), D is not
    let ready_initial = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_initial.is_ok(), "Getting ready beads should succeed");
    let ready = ready_initial.unwrap_or_default();
    assert_eq!(ready.len(), 3, "A, B, C should be ready initially");
    assert!(!ready.contains(&d), "D should not be ready initially");

    // WHEN: Only A and B complete (C still pending)
    let workflow_state = scheduler.get_workflow_mut(&workflow_id);
    assert!(workflow_state.is_some(), "Workflow should exist");
    if let Some(state) = workflow_state {
        state.mark_completed(&a);
        state.mark_completed(&b);
    }

    // THEN: D should still not be ready (C not done)
    let ready_partial = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_partial.is_ok(), "Getting ready beads should succeed");
    let ready = ready_partial.unwrap_or_default();

    assert!(
        ready.contains(&c),
        "C should still be ready (not completed)"
    );
    assert!(!ready.contains(&d), "D should NOT be ready (C not done)");

    // WHEN: C also completes
    let workflow_state = scheduler.get_workflow_mut(&workflow_id);
    if let Some(state) = workflow_state {
        state.mark_completed(&c);
    }

    // THEN: D becomes ready
    let ready_all = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_all.is_ok(), "Getting ready beads should succeed");
    let ready = ready_all.unwrap_or_default();
    assert!(
        ready.contains(&d),
        "D should be ready after all parents complete"
    );
}

#[test]
fn given_diamond_a_bc_d_when_process_sequentially_then_correct_ready_sequence() {
    // GIVEN: A diamond pattern workflow
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-diamond".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();
    let d = "bead-d".to_string();

    // Schedule all beads
    for bead in [&a, &b, &c, &d] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling {} should succeed", bead);
    }

    // Add diamond dependencies
    let dep_ab = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());
    let dep_ac = scheduler.add_dependency(&workflow_id, a.clone(), c.clone());
    let dep_bd = scheduler.add_dependency(&workflow_id, b.clone(), d.clone());
    let dep_cd = scheduler.add_dependency(&workflow_id, c.clone(), d.clone());

    assert!(dep_ab.is_ok(), "A->B dependency should succeed");
    assert!(dep_ac.is_ok(), "A->C dependency should succeed");
    assert!(dep_bd.is_ok(), "B->D dependency should succeed");
    assert!(dep_cd.is_ok(), "C->D dependency should succeed");

    // Step 1: Initially only A is ready
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert_eq!(ready, vec![a.clone()], "Initially only A should be ready");

    // Step 2: Complete A, B and C become ready
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&a);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert_eq!(ready.len(), 2, "After A, both B and C should be ready");
    assert!(ready.contains(&b), "B should be ready");
    assert!(ready.contains(&c), "C should be ready");
    assert!(!ready.contains(&d), "D should not be ready yet");

    // Step 3: Complete B only, D still not ready
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&b);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&c), "C should still be ready");
    assert!(!ready.contains(&d), "D should NOT be ready (C not done)");

    // Step 4: Complete C, D becomes ready
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&c);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(
        ready.contains(&d),
        "D should be ready after both B and C complete"
    );
}

#[test]
fn given_complex_dag_with_multiple_paths_when_process_then_respects_all_dependencies() {
    // GIVEN: A complex DAG with multiple paths
    //     A
    //    /|\
    //   B C D
    //   |\ /|
    //   | X |
    //   |/ \|
    //   E   F
    //    \ /
    //     G
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-complex".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();
    let d = "bead-d".to_string();
    let e = "bead-e".to_string();
    let f = "bead-f".to_string();
    let g = "bead-g".to_string();

    // Schedule all beads
    for bead in [&a, &b, &c, &d, &e, &f, &g] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling {} should succeed", bead);
    }

    // Add dependencies representing the complex DAG
    // A -> B, A -> C, A -> D
    for child in [&b, &c, &d] {
        let result = scheduler.add_dependency(&workflow_id, a.clone(), child.clone());
        assert!(result.is_ok(), "A->{} should succeed", child);
    }

    // B -> E, C -> E (E depends on B and C)
    let dep_be = scheduler.add_dependency(&workflow_id, b.clone(), e.clone());
    let dep_ce = scheduler.add_dependency(&workflow_id, c.clone(), e.clone());
    assert!(dep_be.is_ok(), "B->E should succeed");
    assert!(dep_ce.is_ok(), "C->E should succeed");

    // C -> F, D -> F (F depends on C and D)
    let dep_cf = scheduler.add_dependency(&workflow_id, c.clone(), f.clone());
    let dep_df = scheduler.add_dependency(&workflow_id, d.clone(), f.clone());
    assert!(dep_cf.is_ok(), "C->F should succeed");
    assert!(dep_df.is_ok(), "D->F should succeed");

    // E -> G, F -> G (G depends on E and F)
    let dep_eg = scheduler.add_dependency(&workflow_id, e.clone(), g.clone());
    let dep_fg = scheduler.add_dependency(&workflow_id, f.clone(), g.clone());
    assert!(dep_eg.is_ok(), "E->G should succeed");
    assert!(dep_fg.is_ok(), "F->G should succeed");

    // WHEN/THEN: Verify processing order

    // Initially only A is ready
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert_eq!(ready, vec![a.clone()], "Only A should be ready initially");

    // Complete A
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&a);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert_eq!(ready.len(), 3, "B, C, D should be ready after A");
    assert!(ready.contains(&b) && ready.contains(&c) && ready.contains(&d));

    // Complete B and D (not C yet)
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&b);
        state.mark_completed(&d);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&c), "C should still be ready");
    assert!(!ready.contains(&e), "E needs C too");
    assert!(!ready.contains(&f), "F needs C too");

    // Complete C
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&c);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&e), "E should be ready (B and C done)");
    assert!(ready.contains(&f), "F should be ready (C and D done)");
    assert!(!ready.contains(&g), "G not ready (E and F not done)");

    // Complete E and F
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&e);
        state.mark_completed(&f);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&g), "G should finally be ready");
}

// ============================================================================
// WORKFLOW LIFECYCLE TESTS (4 tests)
// ============================================================================

#[test]
fn given_workflow_with_beads_when_all_complete_then_workflow_is_complete() {
    // GIVEN: A workflow with beads and dependencies
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-lifecycle".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();

    // Schedule beads with A -> B -> C
    for bead in [&a, &b, &c] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling should succeed");
    }
    let _ = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());
    let _ = scheduler.add_dependency(&workflow_id, b.clone(), c.clone());

    // Verify workflow is not complete initially
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some(), "Workflow should exist");
    if let Some(state) = workflow {
        assert!(
            !state.is_complete(),
            "Workflow should not be complete initially"
        );
        assert_eq!(state.len(), 3, "Should have 3 beads");
        assert_eq!(state.completed_count(), 0, "No beads completed yet");
    }

    // WHEN: Complete all beads in order
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&a);
        assert_eq!(state.completed_count(), 1, "1 bead completed");
        assert!(!state.is_complete(), "Not complete yet");

        state.mark_completed(&b);
        assert_eq!(state.completed_count(), 2, "2 beads completed");
        assert!(!state.is_complete(), "Not complete yet");

        state.mark_completed(&c);
        assert_eq!(state.completed_count(), 3, "3 beads completed");
    }

    // THEN: Workflow should be complete
    let workflow = scheduler.get_workflow(&workflow_id);
    if let Some(state) = workflow {
        assert!(
            state.is_complete(),
            "Workflow should be complete when all beads done"
        );
    }
}

#[test]
fn given_empty_workflow_when_check_complete_then_returns_true() {
    // GIVEN: An empty workflow (no beads)
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-empty".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    // WHEN: Checking if complete
    let workflow = scheduler.get_workflow(&workflow_id);

    // THEN: Empty workflow is considered complete (0/0 = complete)
    assert!(workflow.is_some(), "Workflow should exist");
    if let Some(state) = workflow {
        assert!(state.is_empty(), "Workflow should be empty");
        assert!(state.is_complete(), "Empty workflow should be complete");
    }
}

#[test]
fn given_workflow_with_cycle_in_dag_when_topological_sort_then_error() {
    // GIVEN: A workflow with a cyclic dependency in its DAG
    let mut dag = WorkflowDAG::new();

    // Add nodes
    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();

    let _ = dag.add_node(a.clone());
    let _ = dag.add_node(b.clone());
    let _ = dag.add_node(c.clone());

    // Create cycle: A -> B -> C -> A
    let _ = dag.add_edge(a.clone(), b.clone(), DependencyType::BlockingDependency);
    let _ = dag.add_edge(b.clone(), c.clone(), DependencyType::BlockingDependency);
    let _ = dag.add_edge(c.clone(), a.clone(), DependencyType::BlockingDependency);

    // WHEN: Attempting topological sort
    let result = dag.topological_sort();

    // THEN: Should fail due to cycle
    assert!(
        result.is_err(),
        "Topological sort should fail on cyclic graph"
    );

    // Verify has_cycle also detects it
    assert!(dag.has_cycle(), "has_cycle should detect the cycle");
}

#[test]
fn given_workflow_when_unregister_then_state_cleaned_up() {
    // GIVEN: A scheduler with a registered workflow
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-to-remove".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    // Add some beads
    let a = "bead-a".to_string();
    let _ = scheduler.schedule_bead(workflow_id.clone(), a.clone());

    assert_eq!(scheduler.workflow_count(), 1, "Should have 1 workflow");

    // WHEN: Unregistering the workflow
    let removed = scheduler.unregister_workflow(&workflow_id);

    // THEN: Workflow should be removed and state cleaned up
    assert!(removed.is_some(), "Should return removed workflow state");
    assert_eq!(scheduler.workflow_count(), 0, "Should have 0 workflows");
    assert!(
        scheduler.get_workflow(&workflow_id).is_none(),
        "Workflow should no longer be accessible"
    );

    // Verify the removed state contains the bead
    if let Some(state) = removed {
        assert!(
            state.contains_bead(&a),
            "Removed state should contain the bead"
        );
    }
}

// ============================================================================
// CONCURRENT READY DETECTION TESTS (3 tests)
// ============================================================================

#[test]
fn given_multiple_roots_when_get_ready_then_all_returned_simultaneously() {
    // GIVEN: A workflow with multiple root nodes (no dependencies)
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-multi-root".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    // Create 5 independent beads (all roots)
    let beads: Vec<BeadId> = (1..=5).map(|i| format!("bead-{}", i)).collect::<Vec<_>>();

    for bead in &beads {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling {} should succeed", bead);
    }

    // No dependencies added - all beads are roots

    // WHEN: Getting ready beads
    let ready = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: All 5 beads should be ready simultaneously
    assert!(ready.is_ok(), "Getting ready beads should succeed");
    let ready = ready.unwrap_or_default();

    assert_eq!(ready.len(), 5, "All 5 root beads should be ready");
    for bead in &beads {
        assert!(ready.contains(bead), "{} should be ready", bead);
    }
}

#[test]
fn given_parallel_branches_when_complete_at_different_times_then_correct_ready_detection() {
    // GIVEN: A workflow with parallel branches that complete at different "times"
    //   A -----> B -----> C
    //   D -----> E
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-parallel".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();
    let d = "bead-d".to_string();
    let e = "bead-e".to_string();

    // Schedule all beads
    for bead in [&a, &b, &c, &d, &e] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling {} should succeed", bead);
    }

    // Add dependencies for two parallel chains
    let _ = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());
    let _ = scheduler.add_dependency(&workflow_id, b.clone(), c.clone());
    let _ = scheduler.add_dependency(&workflow_id, d.clone(), e.clone());

    // Initially A and D are ready (roots of both chains)
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert_eq!(ready.len(), 2, "Two roots should be ready");
    assert!(ready.contains(&a), "A should be ready");
    assert!(ready.contains(&d), "D should be ready");

    // WHEN: Complete D first (shorter chain)
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&d);
    }

    // THEN: E becomes ready, A still ready
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&a), "A should still be ready");
    assert!(ready.contains(&e), "E should be ready after D");
    assert!(!ready.contains(&b), "B not ready (A not done)");

    // Complete E (D's chain done), then A
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&e);
        state.mark_completed(&a);
    }

    // B should now be ready
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&b), "B should be ready after A");
    assert!(!ready.contains(&c), "C not ready (B not done)");

    // Complete B
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&b);
    }

    // C should now be ready
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(ready.contains(&c), "C should be ready after B");
}

#[test]
fn given_bead_with_incomplete_deps_when_check_ready_then_not_ready() {
    // GIVEN: A workflow where bead D depends on A, B, C
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-verify-deps".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();
    let d = "bead-d".to_string();

    // Schedule all beads
    for bead in [&a, &b, &c, &d] {
        let result = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
        assert!(result.is_ok(), "Scheduling should succeed");
    }

    // D depends on A, B, and C
    for parent in [&a, &b, &c] {
        let _ = scheduler.add_dependency(&workflow_id, parent.clone(), d.clone());
    }

    // Verify D is not ready before any completions
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(!ready.contains(&d), "D should not be ready initially");

    // Complete A only
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&a);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(
        !ready.contains(&d),
        "D should not be ready with only A done"
    );

    // Complete B (A and B done, C pending)
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&b);
    }
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(
        !ready.contains(&d),
        "D should not be ready with only A,B done"
    );

    // WHEN: Complete C (all dependencies satisfied)
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&c);
    }

    // THEN: D should finally be ready
    let ready = scheduler
        .get_workflow_ready_beads(&workflow_id)
        .unwrap_or_default();
    assert!(
        ready.contains(&d),
        "D should be ready when all deps complete"
    );
}

// ============================================================================
// ADDITIONAL INTEGRATION TESTS
// ============================================================================

#[test]
fn given_workflow_when_check_bead_ready_individually_then_respects_dependencies() {
    // GIVEN: A workflow with dependencies A -> B
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-individual".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();

    let _ = scheduler.schedule_bead(workflow_id.clone(), a.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), b.clone());
    let _ = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());

    // WHEN/THEN: Check individual bead readiness
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some(), "Workflow should exist");

    if let Some(state) = workflow {
        // A should be ready (no dependencies)
        let a_ready = state.is_bead_ready(&a);
        assert!(a_ready.is_ok(), "Checking A readiness should succeed");
        assert!(a_ready.unwrap_or(false), "A should be ready");

        // B should not be ready (depends on A)
        let b_ready = state.is_bead_ready(&b);
        assert!(b_ready.is_ok(), "Checking B readiness should succeed");
        assert!(!b_ready.unwrap_or(true), "B should not be ready");
    }

    // Complete A
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&a);
    }

    // Now B should be ready
    let workflow = scheduler.get_workflow(&workflow_id);
    if let Some(state) = workflow {
        let b_ready = state.is_bead_ready(&b);
        assert!(b_ready.is_ok(), "Checking B readiness should succeed");
        assert!(
            b_ready.unwrap_or(false),
            "B should be ready after A completes"
        );
    }
}

#[test]
fn given_dag_directly_when_manipulate_through_workflow_state_then_changes_reflected() {
    // GIVEN: A workflow with access to its DAG
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-dag-access".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    // Schedule beads
    let a = "bead-a".to_string();
    let b = "bead-b".to_string();

    let _ = scheduler.schedule_bead(workflow_id.clone(), a.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), b.clone());

    // WHEN: Access DAG through workflow state
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some(), "Workflow should exist");

    if let Some(state) = workflow {
        let dag = state.dag();

        // THEN: DAG should reflect scheduled beads
        assert_eq!(dag.node_count(), 2, "DAG should have 2 nodes");
        assert!(dag.contains_node(&a), "DAG should contain A");
        assert!(dag.contains_node(&b), "DAG should contain B");

        // Initially no edges
        assert_eq!(dag.edge_count(), 0, "DAG should have no edges initially");
    }

    // Add dependency and verify
    let _ = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());

    let workflow = scheduler.get_workflow(&workflow_id);
    if let Some(state) = workflow {
        let dag = state.dag();
        assert_eq!(
            dag.edge_count(),
            1,
            "DAG should have 1 edge after adding dependency"
        );

        // Verify dependency direction
        let deps = dag.get_dependencies(&b);
        assert!(deps.is_ok(), "Getting B's dependencies should succeed");
        let deps = deps.unwrap_or_default();
        assert!(deps.contains(&a), "B should depend on A");
    }
}

#[test]
fn given_workflow_when_get_all_beads_then_returns_complete_list() {
    // GIVEN: A workflow with multiple beads
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-all-beads".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Workflow registration should succeed");

    let beads: Vec<BeadId> = vec![
        "bead-alpha".to_string(),
        "bead-beta".to_string(),
        "bead-gamma".to_string(),
        "bead-delta".to_string(),
    ];

    for bead in &beads {
        let _ = scheduler.schedule_bead(workflow_id.clone(), bead.clone());
    }

    // WHEN: Getting all beads from workflow
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some(), "Workflow should exist");

    if let Some(state) = workflow {
        let all_beads = state.beads();

        // THEN: All beads should be returned
        assert_eq!(all_beads.len(), 4, "Should return all 4 beads");

        let bead_set: HashSet<BeadId> = all_beads.into_iter().collect::<HashSet<BeadId>>();
        for bead in &beads {
            assert!(bead_set.contains(bead), "{} should be in the list", bead);
        }
    }
}

// ============================================================================
// BEAD COMPLETED EVENT SUBSCRIPTION INTEGRATION TESTS
// ============================================================================

#[test]
fn given_scheduler_with_chain_when_bead_completed_event_then_dependents_become_ready() {
    // GIVEN: A scheduler with chain A -> B -> C
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-event-chain".to_string();

    let _ = scheduler.register_workflow(workflow_id.clone());

    let a = "bead-a".to_string();
    let b = "bead-b".to_string();
    let c = "bead-c".to_string();

    let _ = scheduler.schedule_bead(workflow_id.clone(), a.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), b.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), c.clone());

    let _ = scheduler.add_dependency(&workflow_id, a.clone(), b.clone());
    let _ = scheduler.add_dependency(&workflow_id, b.clone(), c.clone());

    // Initially only A is ready
    let ready_initial = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_initial.is_ok());
    let ready_initial = ready_initial.unwrap_or_default();
    assert!(ready_initial.contains(&a), "A should be ready initially");
    assert!(!ready_initial.contains(&b), "B should NOT be ready initially");
    assert!(!ready_initial.contains(&c), "C should NOT be ready initially");

    // WHEN: Simulate BeadCompleted event for A
    let _ = scheduler.handle_bead_completed(&a);

    // THEN: B should become ready (A is complete, C still depends on B)
    let ready_after_a = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_after_a.is_ok());
    let ready_after_a = ready_after_a.unwrap_or_default();
    assert!(!ready_after_a.contains(&a), "A should NOT be ready (completed)");
    assert!(ready_after_a.contains(&b), "B should be ready after A completes");
    assert!(!ready_after_a.contains(&c), "C should NOT be ready (B not complete)");

    // WHEN: Simulate BeadCompleted event for B
    let _ = scheduler.handle_bead_completed(&b);

    // THEN: C should become ready
    let ready_after_b = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_after_b.is_ok());
    let ready_after_b = ready_after_b.unwrap_or_default();
    assert!(!ready_after_b.contains(&a), "A should NOT be ready");
    assert!(!ready_after_b.contains(&b), "B should NOT be ready (completed)");
    assert!(ready_after_b.contains(&c), "C should be ready after B completes");

    // Verify workflow state reflects completions
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some());
    if let Some(state) = workflow {
        assert_eq!(state.completed_count(), 2, "Should have 2 completed beads");
        assert!(state.is_bead_ready(&c).unwrap_or(false), "C should be ready");
    }
}

// ============================================================================
// BEAD COMPLETED EVENT SUBSCRIPTION INTEGRATION TESTS
// ============================================================================
