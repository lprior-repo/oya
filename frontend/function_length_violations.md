# Function Length Violations Report (Black Hat Phase 2)

**Date:** 2026-04-19
**Auditor:** polecat/ember (of-bl1)
**Threshold:** 25 lines (Farley Engineering Rigor)

## Summary

| Severity | Count | Description |
|----------|-------|-------------|
| **SEVERE** (>100 lines) | 18 | Requires immediate decomposition |
| **HIGH** (51-100 lines) | 40 | Requires near-term decomposition |
| **MAJOR** (26-50 lines) | 110 | Address during normal maintenance |
| **Total** | **168** | Production functions exceeding 25 lines |

## Top 10 Priority Refactor Targets

| # | File | Func | Lines | Action |
|---|------|------|-------|--------|
| 1 | `ui/app_shell.rs` | `AppShell` | 497 | Dioxus component — extract sub-components |
| 2 | `ui/restate/details_panel.rs` | `RestateInvocationDetails` | 363 | Extract journal/action sub-panels |
| 3 | `ui/workflow_nodes/delayed_message.rs` | `DelayedSendForm` | 221 | Extract form sections |
| 4 | `coverage/mod.rs` | `analyze_spec` | 208 | Extract analysis phases |
| 5 | `linter/engine.rs` | `check_completeness` | 206 | Extract rule checks |
| 6 | `ui/restate/panel.rs` | `RestateInvocationsPanel` | 198 | Extract filter/table sub-components |
| 7 | `ui/restate/deployment_browser.rs` | `DeploymentBrowserPanel` | 192 | Extract table/row components |
| 8 | `ui/restate/promise_browser.rs` | `PromiseBrowserPanel` | 187 | Extract table/row components |
| 9 | `graph/layout.rs` | `apply` | 178 | Extract phase sub-functions |
| 10 | `ui/workflow_nodes/service_call.rs` | `ServiceCallForm` | 177 | Extract form sections |

## Top 10 Files by Violation Count

| Violations | File | Worst Offender (lines) |
|-----------|------|----------------------|
| 14 | `flow_extender/mod.rs` | `rules` (98) |
| 12 | `ui/edges.rs` | `find_parallel_branches` (71) |
| 6 | `linter/engine.rs` | `check_completeness` (206) |
| 6 | `graph/layout.rs` | `apply` (178) |
| 6 | `graph/workflow_node_impl.rs` | `from_str` (31) |
| 5 | `flow_extender/preview_calc.rs` | `multiple_edges_in_single_patch_use_correct_indices` (42) |
| 5 | `ui/inline_config_panel.rs` | `entry_config` (41) |
| 5 | `restate_client/client.rs` | `query` (36) |
| 4 | `scenario_runner/runner.rs` | `execute_action` (62) |
| 4 | `restate_client/types.rs` | `journal_entry_type_all_variants` (41) |

## Severity Distribution

```
SEVERE (>100 lines):  ██████████████████  18 functions
HIGH (51-100 lines):  ██████████████████████████████████████████  40 functions
MAJOR (26-50 lines):  ████████████████████████████████████████████████████████████████████████████████████████████████████  110 functions
```

## Category Analysis

### Dioxus RSX Components (largest category)
Most severe violations are Dioxus components with large `rsx!` macro bodies. These are structurally different from logic functions — the RSX template DSL makes decomposition harder. Recommended approach:
- Extract sub-render functions returning `Element`
- Use `children` props for composability
- Separate event handlers into standalone functions

**Affected files:** `app_shell.rs`, `details_panel.rs`, `panel.rs`, `deployment_browser.rs`, `promise_browser.rs`, `state_browser.rs`, `journal_viewer.rs`, all `workflow_nodes/*.rs`

### Logic/Algorithm Functions
Pure logic functions exceeding 25 lines are more straightforward to refactor:
- `coverage::analyze_spec` (208) → split into analysis phases
- `linter::check_completeness` (206) → one function per rule category
- `linter::check_security` (137) → one function per security check
- `graph::layout::apply` (178) → extract layout phases
- `flow_extender::rules` (98) → extract rule definitions

### Data Mapping/Formatting
Functions doing row-to-struct mapping or Display formatting:
- `graph::execution_errors::fmt` (89) → use Display derive or match table
- `restate_client::row_to_invocation` (36) → builder pattern
- `metrics::get_summary` (90) → split aggregation from formatting

## Full Violations List (SEVERE — >50 lines)

| File | Line | Lines | Function |
|------|------|-------|----------|
| ui/app_shell.rs | 22 | 497 | AppShell |
| ui/restate/details_panel.rs | 85 | 363 | RestateInvocationDetails |
| ui/workflow_nodes/delayed_message.rs | 14 | 221 | DelayedSendForm |
| coverage/mod.rs | 187 | 208 | analyze_spec |
| linter/engine.rs | 102 | 206 | check_completeness |
| ui/restate/panel.rs | 62 | 198 | RestateInvocationsPanel |
| ui/restate/deployment_browser.rs | 73 | 192 | DeploymentBrowserPanel |
| ui/restate/promise_browser.rs | 31 | 187 | PromiseBrowserPanel |
| graph/layout.rs | 36 | 178 | apply |
| ui/workflow_nodes/service_call.rs | 14 | 177 | ServiceCallForm |
| ui/restate/state_browser.rs | 94 | 171 | StateBrowserPanel |
| linter/engine.rs | 364 | 137 | check_security |
| ui/restate/journal_viewer.rs | 39 | 133 | RestateJournalViewer |
| ui/workflow_nodes/delay.rs | 9 | 123 | SleepForm |
| ui/workflow_nodes/send_message/mod.rs | 13 | 119 | SendMessageForm |
| graph/execution_runtime/step_runner.rs | 105 | 115 | step |
| graph/execution_engine.rs | 36 | 113 | prepare_execution |
| ui/payload_preview_panel.rs | 72 | 108 | PayloadPreviewPanel |
| ui/workflow_nodes/workflow_submit.rs | 12 | 106 | WorkflowSubmitForm |
| ui/config_panel/mod.rs | 113 | 104 | ConfigTab |
| flow_extender/mod.rs | 636 | 98 | rules |
| hooks/use_restate_sync.rs | 51 | 94 | provide_restate_sync_context |
| ui/workflow_nodes/workflow_call.rs | 12 | 91 | WorkflowCallForm |
| metrics/report.rs | 7 | 90 | get_summary |
| graph/execution_errors.rs | 106 | 89 | fmt |
| ui/shortcuts_overlay.rs | 25 | 89 | shortcut_categories |
| ui/workflow_nodes/shared.rs | 124 | 77 | NodeCard |
| ui/execution_plan_panel.rs | 59 | 76 | build_plan_snapshot |
| graph/execution_runtime/workflow.rs | 10 | 72 | run |
| ui/edges.rs | 156 | 71 | find_parallel_branches |
| flow_extender/mod.rs | 1259 | 70 | confidence_score_for |
| graph/expressions.rs | 22 | 69 | resolve |
| metrics/report.rs | 112 | 69 | format_text_report |
| agent_feedback/mod.rs | 52 | 64 | new |
| ui/app_io.rs | 46 | 63 | download_workflow_json |
| ui/empty_canvas.rs | 13 | 63 | EmptyCanvas |
| ui/workflow_nodes/http_trigger.rs | 9 | 63 | HttpHandlerForm |
| bin/agent_feedback.rs | 40 | 62 | main |
| scenario_runner/runner.rs | 89 | 62 | execute_action |
| ui/edges.rs | 675 | 60 | given_parallel_groups_when_resolve_anchors_then_offsets_applied_to_targets |
| ui/edges.rs | 1026 | 60 | given_shared_target_across_sources_when_resolve_anchors_then_uses_source_target_match |
| ui/workflow_nodes/schedule_trigger.rs | 11 | 59 | CronTriggerForm |
| bin/flow_extend.rs | 61 | 58 | main |
| ui/edges.rs | 763 | 57 | given_mixed_parallel_and_non_parallel_edges_when_resolve_anchors |
| ui/workflow_nodes/save_to_memory.rs | 12 | 57 | SetStateForm |
| feedback/mod.rs | 132 | 55 | from_level |
| ui/editor_interactions.rs | 175 | 55 | given_zoom_level_when_snapping_then_behavior_is_zoom_invariant |
| connectivity/port_type.rs | 34 | 54 | parse |
| ui/workflow_nodes/condition.rs | 9 | 54 | ConditionForm |
| ui/workflow_nodes/object_call.rs | 9 | 54 | ObjectCallForm |
| hooks/use_workflow_state.rs | 461 | 53 | provide_workflow_state_context |
| linter/engine.rs | 23 | 53 | validate_rules |
| linter/engine.rs | 309 | 53 | check_clarity |
| ui/app_io.rs | 211 | 52 | export_restate_history |
| coverage/mod.rs | 79 | 51 | analyze |
| flow_extender/mod.rs | 1177 | 51 | order_keys_with_dependencies |
| restate_sync/poller.rs | 206 | 51 | poll |
| restate_sync/poller.rs | 488 | 51 | test_poller_transparency_logic |
