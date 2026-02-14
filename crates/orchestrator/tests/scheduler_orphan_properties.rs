#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property-based tests for scheduler orphan bead invariants.
//!
//! ## Bead src-pnkk: No Orphaned Beads After Operations
//!
//! ∀ workflow operations: no orphaned beads remain in scheduler state
//!
//! An orphaned bead is defined as:
//! 1. A bead in `pending_beads` that references a non-existent workflow
//! 2. A bead in `ready_beads` that doesn't exist in `pending_beads`
//! 3. A worker assignment for a bead that doesn't exist in `pending_beads`
//! 4. A worker assignment to an agent that doesn't exist in `agents`

use proptest::collection::vec;
use proptest::prelude::*;

use orchestrator::actors::messages::SchedulerMessage;
use orchestrator::actors::scheduler::core;
use orchestrator::actors::scheduler::CoreSchedulerState;

type DynStrat = BoxedStrategy<String>;

fn workflow_id_strategy() -> DynStrat {
    "wf-[a-z0-9]{3,6}".boxed()
}

fn bead_id_strategy() -> DynStrat {
    "bead-[a-z0-9]{3,6}".boxed()
}

fn agent_id_strategy() -> DynStrat {
    "agent-[a-z0-9]{2,4}".boxed()
}

#[derive(Debug, Clone)]
enum SchedulerOp {
    RegisterWorkflow {
        workflow_id: String,
    },
    UnregisterWorkflow {
        workflow_id: String,
    },
    ScheduleBead {
        workflow_id: String,
        bead_id: String,
    },
    CompleteBead {
        workflow_id: String,
        bead_id: String,
    },
    RegisterAgent {
        agent_id: String,
    },
    UnregisterAgent {
        agent_id: String,
    },
    ClaimBead {
        bead_id: String,
        agent_id: String,
    },
    ReleaseBead {
        bead_id: String,
    },
}

fn scheduler_op_strategy() -> BoxedStrategy<SchedulerOp> {
    prop_oneof![
        workflow_id_strategy().prop_map(|wid| SchedulerOp::RegisterWorkflow { workflow_id: wid }),
        workflow_id_strategy().prop_map(|wid| SchedulerOp::UnregisterWorkflow { workflow_id: wid }),
        (workflow_id_strategy(), bead_id_strategy()).prop_map(|(wid, bid)| {
            SchedulerOp::ScheduleBead {
                workflow_id: wid,
                bead_id: bid,
            }
        }),
        (workflow_id_strategy(), bead_id_strategy()).prop_map(|(wid, bid)| {
            SchedulerOp::CompleteBead {
                workflow_id: wid,
                bead_id: bid,
            }
        }),
        agent_id_strategy().prop_map(|aid| SchedulerOp::RegisterAgent { agent_id: aid }),
        agent_id_strategy().prop_map(|aid| SchedulerOp::UnregisterAgent { agent_id: aid }),
        (bead_id_strategy(), agent_id_strategy()).prop_map(|(bid, aid)| SchedulerOp::ClaimBead {
            bead_id: bid,
            agent_id: aid,
        }),
        bead_id_strategy().prop_map(|bid| SchedulerOp::ReleaseBead { bead_id: bid }),
    ]
    .boxed()
}

fn apply_op(state: CoreSchedulerState, op: &SchedulerOp) -> CoreSchedulerState {
    let msg = match op {
        SchedulerOp::RegisterWorkflow { workflow_id } => SchedulerMessage::RegisterWorkflow {
            workflow_id: workflow_id.clone(),
        },
        SchedulerOp::UnregisterWorkflow { workflow_id } => SchedulerMessage::UnregisterWorkflow {
            workflow_id: workflow_id.clone(),
        },
        SchedulerOp::ScheduleBead {
            workflow_id,
            bead_id,
        } => SchedulerMessage::ScheduleBead {
            workflow_id: workflow_id.clone(),
            bead_id: bead_id.clone(),
        },
        SchedulerOp::CompleteBead {
            workflow_id,
            bead_id,
        } => SchedulerMessage::OnBeadCompleted {
            workflow_id: workflow_id.clone(),
            bead_id: bead_id.clone(),
        },
        SchedulerOp::RegisterAgent { agent_id } => SchedulerMessage::RegisterAgent {
            agent_id: agent_id.clone(),
            capabilities: vec!["default".to_string()],
        },
        SchedulerOp::UnregisterAgent { agent_id } => SchedulerMessage::UnregisterAgent {
            agent_id: agent_id.clone(),
        },
        SchedulerOp::ClaimBead { bead_id, agent_id } => SchedulerMessage::ClaimBead {
            bead_id: bead_id.clone(),
            worker_id: agent_id.clone(),
        },
        SchedulerOp::ReleaseBead { bead_id } => SchedulerMessage::ReleaseBead {
            bead_id: bead_id.clone(),
        },
    };
    core::handle(state, msg).0
}

fn has_orphaned_beads(state: &CoreSchedulerState) -> Result<bool, String> {
    for (bead_id, scheduled_bead) in &state.pending_beads {
        if !state.workflows.contains_key(&scheduled_bead.workflow_id) {
            return Err(format!(
                "Orphaned bead '{}' references non-existent workflow '{}'",
                bead_id, scheduled_bead.workflow_id
            ));
        }
    }

    for bead_id in &state.ready_beads {
        if !state.pending_beads.contains_key(bead_id) {
            return Err(format!(
                "Bead '{}' in ready_beads but not in pending_beads",
                bead_id
            ));
        }
    }

    for (bead_id, agent_id) in &state.worker_assignments {
        if !state.pending_beads.contains_key(bead_id) {
            return Err(format!(
                "Worker assignment for bead '{}' which doesn't exist in pending_beads",
                bead_id
            ));
        }
        if !state.agents.contains_key(agent_id) {
            return Err(format!(
                "Bead '{}' assigned to non-existent agent '{}'",
                bead_id, agent_id
            ));
        }
    }

    Ok(false)
}

proptest! {
    /// Property: No orphaned beads after arbitrary sequence of operations
    ///
    /// ∀ operations sequence: after applying all ops, no orphaned beads exist
    #[test]
    fn prop_no_orphaned_beads_after_operations(ops in vec(scheduler_op_strategy(), 1..50)) {
        let mut state = CoreSchedulerState::default();

        for op in &ops {
            state = apply_op(state, op);
        }

        match has_orphaned_beads(&state) {
            Ok(false) => (),
            Ok(true) => return Err(TestCaseError::fail("has_orphaned_beads returned true but should have errored")),
            Err(e) => return Err(TestCaseError::fail(e)),
        }
    }

    /// Property: Agent unregistration cleans up bead assignments
    ///
    /// WHEN an agent is unregistered
    /// THEN no beads should be assigned to that agent
    #[test]
    fn prop_agent_unregister_clears_assignments(
        workflow_id in workflow_id_strategy(),
        bead_ids in vec(bead_id_strategy(), 1..5),
        agent_id in agent_id_strategy(),
    ) {
        let mut state = CoreSchedulerState::default();

        state = apply_op(state, &SchedulerOp::RegisterWorkflow {
            workflow_id: workflow_id.clone(),
        });

        state = apply_op(state, &SchedulerOp::RegisterAgent {
            agent_id: agent_id.clone(),
        });

        for bead_id in &bead_ids {
            state = apply_op(state, &SchedulerOp::ScheduleBead {
                workflow_id: workflow_id.clone(),
                bead_id: bead_id.clone(),
            });
            state = apply_op(state, &SchedulerOp::ClaimBead {
                bead_id: bead_id.clone(),
                agent_id: agent_id.clone(),
            });
        }

        state = apply_op(state, &SchedulerOp::UnregisterAgent {
            agent_id: agent_id.clone(),
        });

        for (_, assigned_agent) in &state.worker_assignments {
            prop_assert_ne!(
                assigned_agent, &agent_id,
                "Bead still assigned to unregistered agent '{}'",
                agent_id
            );
        }

        match has_orphaned_beads(&state) {
            Ok(false) => (),
            Err(e) => return Err(TestCaseError::fail(e)),
            _ => (),
        }
    }

    /// Property: Workflow unregistration cleans up related beads
    ///
    /// WHEN a workflow is unregistered
    /// THEN beads referencing that workflow should be handled properly
    #[test]
    fn prop_workflow_unregister_handles_beads(
        workflow_ids in vec(workflow_id_strategy(), 2..4),
        bead_id in bead_id_strategy(),
        agent_id in agent_id_strategy(),
    ) {
        prop_assume!(workflow_ids.len() >= 2);

        let mut state = CoreSchedulerState::default();

        for wid in &workflow_ids {
            state = apply_op(state, &SchedulerOp::RegisterWorkflow {
                workflow_id: wid.clone(),
            });
        }

        state = apply_op(state, &SchedulerOp::RegisterAgent {
            agent_id: agent_id.clone(),
        });

        state = apply_op(state, &SchedulerOp::ScheduleBead {
            workflow_id: workflow_ids[0].clone(),
            bead_id: bead_id.clone(),
        });

        state = apply_op(state, &SchedulerOp::UnregisterWorkflow {
            workflow_id: workflow_ids[0].clone(),
        });

        let orphaned = state.pending_beads.iter().any(|(_, sb)| {
            !state.workflows.contains_key(&sb.workflow_id)
        });

        prop_assert!(
            !orphaned,
            "After unregistering workflow '{}', beads still reference it",
            workflow_ids[0]
        );
    }

    /// Property: Bead completion cleans up assignments
    ///
    /// WHEN a bead is completed
    /// THEN its worker assignment should be removed
    #[test]
    fn prop_bead_completion_clears_assignment(
        workflow_id in workflow_id_strategy(),
        bead_id in bead_id_strategy(),
        agent_id in agent_id_strategy(),
    ) {
        let mut state = CoreSchedulerState::default();

        state = apply_op(state, &SchedulerOp::RegisterWorkflow {
            workflow_id: workflow_id.clone(),
        });
        state = apply_op(state, &SchedulerOp::RegisterAgent {
            agent_id: agent_id.clone(),
        });
        state = apply_op(state, &SchedulerOp::ScheduleBead {
            workflow_id: workflow_id.clone(),
            bead_id: bead_id.clone(),
        });
        state = apply_op(state, &SchedulerOp::ClaimBead {
            bead_id: bead_id.clone(),
            agent_id: agent_id.clone(),
        });

        prop_assert!(
            state.worker_assignments.contains_key(&bead_id),
            "Bead should be assigned before completion"
        );

        state = apply_op(state, &SchedulerOp::CompleteBead {
            workflow_id: workflow_id.clone(),
            bead_id: bead_id.clone(),
        });

        prop_assert!(
            !state.worker_assignments.contains_key(&bead_id),
            "Bead assignment should be cleared after completion"
        );

        prop_assert!(
            !state.ready_beads.contains(&bead_id),
            "Bead should not be in ready list after completion"
        );
    }

    /// Property: Multiple operations maintain consistency
    ///
    /// ∀ complex operation sequences: state remains consistent
    #[test]
    fn prop_complex_operation_sequence_maintains_consistency(
        workflow_id in workflow_id_strategy(),
        bead_ids in vec(bead_id_strategy(), 3..6),
        agent_ids in vec(agent_id_strategy(), 2..4),
    ) {
        prop_assume!(!bead_ids.is_empty() && !agent_ids.is_empty());

        let mut state = CoreSchedulerState::default();

        state = apply_op(state, &SchedulerOp::RegisterWorkflow {
            workflow_id: workflow_id.clone(),
        });

        for agent_id in &agent_ids {
            state = apply_op(state, &SchedulerOp::RegisterAgent {
                agent_id: agent_id.clone(),
            });
        }

        for bead_id in &bead_ids {
            state = apply_op(state, &SchedulerOp::ScheduleBead {
                workflow_id: workflow_id.clone(),
                bead_id: bead_id.clone(),
            });
        }

        for (i, bead_id) in bead_ids.iter().enumerate() {
            let agent_id = &agent_ids[i % agent_ids.len()];
            state = apply_op(state, &SchedulerOp::ClaimBead {
                bead_id: bead_id.clone(),
                agent_id: agent_id.clone(),
            });
        }

        for bead_id in &bead_ids {
            state = apply_op(state, &SchedulerOp::CompleteBead {
                workflow_id: workflow_id.clone(),
                bead_id: bead_id.clone(),
            });
        }

        match has_orphaned_beads(&state) {
            Ok(false) => (),
            Err(e) => return Err(TestCaseError::fail(e)),
            _ => (),
        }

        for (_, assigned_agent) in &state.worker_assignments {
            prop_assert!(
                state.agents.contains_key(assigned_agent),
                "Assignment to non-existent agent '{}'",
                assigned_agent
            );
        }
    }

    /// Property: Idempotent operations don't create orphans
    ///
    /// WHEN operations are applied multiple times
    /// THEN no orphans are created
    #[test]
    fn prop_idempotent_operations_no_orphans(
        workflow_id in workflow_id_strategy(),
        bead_id in bead_id_strategy(),
        agent_id in agent_id_strategy(),
    ) {
        let mut state = CoreSchedulerState::default();

        for _ in 0..3 {
            state = apply_op(state, &SchedulerOp::RegisterWorkflow {
                workflow_id: workflow_id.clone(),
            });
        }

        for _ in 0..3 {
            state = apply_op(state, &SchedulerOp::RegisterAgent {
                agent_id: agent_id.clone(),
            });
        }

        for _ in 0..3 {
            state = apply_op(state, &SchedulerOp::ScheduleBead {
                workflow_id: workflow_id.clone(),
                bead_id: bead_id.clone(),
            });
        }

        match has_orphaned_beads(&state) {
            Ok(false) => (),
            Err(e) => return Err(TestCaseError::fail(e)),
            _ => (),
        }
    }
}

#[test]
fn test_empty_state_has_no_orphans() {
    let state = CoreSchedulerState::default();
    assert!(
        !has_orphaned_beads(&state).unwrap_or(true),
        "Empty state should have no orphans"
    );
}

#[test]
fn test_single_workflow_bead_no_orphans() {
    let mut state = CoreSchedulerState::default();

    state = apply_op(
        state,
        &SchedulerOp::RegisterWorkflow {
            workflow_id: "wf-test".to_string(),
        },
    );
    state = apply_op(
        state,
        &SchedulerOp::ScheduleBead {
            workflow_id: "wf-test".to_string(),
            bead_id: "bead-1".to_string(),
        },
    );

    assert!(
        !has_orphaned_beads(&state).unwrap_or(true),
        "Single bead should not be orphaned"
    );
}

#[test]
fn test_unregister_workflow_removes_beads() {
    let mut state = CoreSchedulerState::default();

    state = apply_op(
        state,
        &SchedulerOp::RegisterWorkflow {
            workflow_id: "wf-test".to_string(),
        },
    );
    state = apply_op(
        state,
        &SchedulerOp::ScheduleBead {
            workflow_id: "wf-test".to_string(),
            bead_id: "bead-1".to_string(),
        },
    );
    state = apply_op(
        state,
        &SchedulerOp::UnregisterWorkflow {
            workflow_id: "wf-test".to_string(),
        },
    );

    let orphaned = state
        .pending_beads
        .iter()
        .any(|(_, sb)| !state.workflows.contains_key(&sb.workflow_id));
    assert!(!orphaned, "Bead should not reference removed workflow");
}
