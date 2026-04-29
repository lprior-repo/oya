# Martin Fowler Test Plan: HTML/CSS -> Dioxus Parity Migration

## Test Strategy
- Test style: executable specifications using `given_X_when_Y_then_Z` naming.
- Coverage model: happy path, error path, edge cases, contract checks, and end-to-end parity.
- Baseline model: source snapshots and structure maps from `professional-flow-builder` compared against Dioxus render output.

## Happy Path Tests
- `given_app_loaded_when_rendering_root_shell_then_toolbar_sidebar_canvas_and_optional_panel_match_expected_order`
- `given_toolbar_rendered_when_workflow_name_changes_then_name_field_and_stats_render_with_parity_classes`
- `given_sidebar_templates_when_search_text_matches_then_filtered_templates_render_in_category_order`
- `given_canvas_interaction_idle_when_background_dragged_then_pan_updates_and_grid_tracks_transform`
- `given_node_present_when_node_dragged_then_position_updates_with_zoom_corrected_delta`
- `given_two_nodes_when_connecting_source_to_target_then_edge_is_created_and_path_renders_with_marker`
- `given_node_selected_when_config_panel_opens_then_panel_slides_in_from_right_with_expected_sections`
- `given_zoom_controls_when_zoom_in_out_fit_invoked_then_zoom_text_and_transform_remain_consistent`
- `given_canvas_has_nodes_when_minimap_rendered_then_mini_nodes_and_edges_match_graph_state`

## Error Path Tests (One per MigrationError)
- `given_missing_source_file_when_build_source_contract_then_returns_source_file_missing`
- `given_unparseable_source_content_when_build_source_contract_then_returns_source_parse_failed`
- `given_missing_required_component_when_build_source_contract_then_returns_source_component_missing`
- `given_required_token_absent_when_validate_component_structure_then_returns_required_class_missing`
- `given_unmappable_tailwind_token_when_map_source_tokens_then_returns_unsupported_css_token`
- `given_token_collision_when_map_source_tokens_then_returns_token_mapping_collision`
- `given_wrong_dom_hierarchy_when_validate_component_structure_then_returns_dom_structure_mismatch`
- `given_spacing_delta_outside_tolerance_when_validate_visual_metrics_then_returns_layout_tolerance_exceeded`
- `given_breakpoint_controls_clipped_when_validate_responsive_layout_then_returns_responsive_regression`
- `given_panel_appears_without_slide_intent_when_validate_animation_intent_then_returns_animation_intent_regression`
- `given_illegal_state_jump_when_validate_interaction_machine_then_returns_invalid_interaction_transition`
- `given_connect_attempt_self_or_duplicate_when_finalize_connection_then_returns_invalid_connection_attempt`
- `given_trace_references_unknown_node_when_validate_interaction_machine_then_returns_node_not_found`
- `given_edge_endpoint_missing_when_render_edges_then_returns_edge_endpoint_missing`
- `given_zoom_outside_bounds_when_apply_zoom_then_returns_viewport_invariant_violation`
- `given_storage_read_fails_when_load_workflow_then_returns_local_storage_read_failure`
- `given_storage_write_fails_when_persist_workflow_then_returns_local_storage_write_failure`
- `given_storage_json_corrupt_when_load_workflow_then_returns_local_storage_data_corrupted`
- `given_minimap_scale_invalid_when_render_minimap_then_returns_minimap_regression`
- `given_overall_parity_gate_fails_when_finalize_migration_report_then_returns_parity_verification_failed`

## Edge Case Tests
- `given_no_selected_node_when_rendering_main_layout_then_config_panel_is_not_rendered`
- `given_mouse_leaves_canvas_while_dragging_when_mouseup_normalized_then_interaction_state_returns_to_idle`
- `given_connecting_state_when_release_occurs_over_non_handle_then_no_connection_created_and_temp_edge_cleared`
- `given_duplicate_edge_exists_when_same_connection_attempted_then_existing_edge_remains_single`
- `given_zoom_at_min_bound_when_zoom_out_requested_then_zoom_remains_at_min_bound`
- `given_zoom_at_max_bound_when_zoom_in_requested_then_zoom_remains_at_max_bound`
- `given_empty_node_list_when_fit_view_requested_then_viewport_remains_stable_without_panic`
- `given_reduced_motion_preference_when_panel_opens_then_state_visibility_preserved_with_equivalent_motion_policy`
- `given_mobile_width_when_layout_renders_then_core_controls_remain_reachable_without_horizontal_lock`

## Contract Verification Tests

## Preconditions
- `given_uninitialized_viewport_when_interaction_starts_then_precondition_failure_is_reported`
- `given_nonfinite_dimensions_when_layout_calculated_then_precondition_failure_is_reported`

## Postconditions
- `given_desktop_breakpoint_when_layout_settles_then_sidebar_canvas_panel_are_visible_without_overlap`
- `given_tablet_breakpoint_when_layout_settles_then_primary_interactions_remain_operable`
- `given_mobile_breakpoint_when_layout_settles_then_toolbar_and_canvas_primary_actions_remain_reachable`
- `given_connection_finalize_event_when_processing_completes_then_transient_connect_state_is_cleared`

## Invariants
- `given_any_interaction_sequence_when_observing_state_then_interaction_machine_is_single_state_only`
- `given_any_zoom_event_when_applied_then_zoom_stays_within_0_15_and_3_0`
- `given_any_edge_render_when_nodes_missing_then_error_variant_is_returned_not_silent_corruption`
- `given_any_pan_zoom_update_when_grid_recomputed_then_grid_alignment_tracks_pan_and_zoom`

## Structural Parity Scenarios (Given-When-Then)

### Scenario: Root shell parity
Given: source contract for `FlowCanvas` has been loaded.
When: Dioxus root app is rendered.
Then: toolbar appears first, workspace row appears second.
Then: workspace row contains left sidebar and central canvas.
Then: right config panel appears only when a node is selected.

### Scenario: Toolbar parity
Given: workflow has `N` nodes and `E` edges.
When: toolbar renders.
Then: workflow name input exists and is editable.
Then: counters display `N nodes` and `E edges`.
Then: zoom controls show current percent and respond to clicks.
Then: execute action is present with primary emphasis styling intent.

### Scenario: Sidebar parity
Given: node templates exist across trigger/action/logic/output.
When: sidebar renders with empty search.
Then: groups render in canonical category order.
Then: each item has icon badge, label, and description.
Then: each item supports click add and drag payload add.

### Scenario: Canvas parity
Given: canvas has pan `(x, y)` and zoom `z`.
When: canvas render occurs.
Then: transform layer style contains `translate(x, y) scale(z)`.
Then: grid background tracks pan and zoom values.
Then: edges and nodes render in transformed coordinate space.
Then: minimap is visible at bottom-right.

### Scenario: Node card parity
Given: a node with icon, label, description, category, and status.
When: node card renders.
Then: top target handle and bottom source handle are present.
Then: icon block, text block, and status indicator are present.
Then: selected state adds ring/border/shadow emphasis.
Then: category accent bar appears at the card bottom.

### Scenario: Config panel parity
Given: a node is selected.
When: config panel opens.
Then: panel enters from right with slide intent.
Then: header shows icon, title, and close action.
Then: body is scrollable and includes editable fields.
Then: footer includes duplicate and delete actions.

### Scenario: Interaction parity for connect
Given: interaction state is `Idle`.
When: user presses a node source handle, drags, and releases on another node.
Then: state transitions `Idle -> Connecting -> Idle`.
Then: temporary edge is visible during drag only.
Then: valid non-duplicate connection is added once.

## Visual Parity Metrics
- Pixel delta tolerance (layout boxes): <= 2px on desktop, <= 3px on tablet/mobile.
- Text style parity: font size and weight exact match, line-height tolerance <= 1px.
- Color parity: semantic token identity must match (primary/muted/border/destructive roles).
- Radius and border parity: exact utility token match where supported; otherwise documented equivalent fallback only.

## End-to-End Parity Scenarios

### Scenario: Full migration acceptance
Given: source UI baseline artifacts and Dioxus build are available.
When: parity suite runs across desktop/tablet/mobile viewports.
Then: all structural contracts pass.
Then: all interaction state-machine contracts pass.
Then: all visual metric checks pass within tolerance.
Then: every `MigrationError` variant has at least one passing or intentionally triggered test.
Then: migration is marked complete.
