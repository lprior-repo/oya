# Martin Fowler Test Plan

## Happy Path Tests

### Gate Enum Tests
- test_gate_enum_has_five_variants_after_removal
- test_gate_compiles_as_str_returns_compiles
- test_gate_tests_pass_as_str_returns_tests_pass
- test_gate_moon_ci_as_str_returns_moon_ci
- test_gate_holdout_scenarios_as_str_returns_holdout_scenarios
- test_gate_cue_artifact_generated_as_str_returns_cue_artifact_generated

### Stage Configuration Tests
- test_ship_gate_stage_has_only_cue_artifact_generated_gate
- test_explore_stage_has_no_gates
- test_contract_stage_has_compiles_gate
- test_red_stage_has_compiles_gate
- test_implementation_stage_has_compiles_and_tests_pass_gates
- test_witness_stage_has_holdout_scenarios_gate

### Gate Parsing Tests
- test_gate_try_from_compiles_returns_ok
- test_gate_try_from_tests_pass_returns_ok
- test_gate_try_from_moon_ci_returns_ok
- test_gate_try_from_holdout_scenarios_returns_ok
- test_gate_try_from_cue_artifact_generated_returns_ok

### Gate Command Parsing Tests
- test_parse_moon_check_command
- test_parse_moon_test_command
- test_parse_moon_ci_command
- test_parse_moon_holdout_command
- test_parse_moon_cue_check_command
- test_parse_moon_command_with_passthrough_args
- test_parse_moon_command_with_quoted_passthrough
- test_parse_moon_command_with_escaped_whitespace

### Gate Execution Tests
- test_execute_gate_uses_moon_timeout_for_all_gates
- test_execute_compiles_gate_success
- test_execute_tests_pass_gate_success
- test_execute_moon_ci_gate_success
- test_execute_holdout_scenarios_gate_success
- test_execute_cue_artifact_generated_gate_success

### Runtime Configuration Tests
- test_runtime_config_load_without_zjj_env_vars
- test_runtime_config_has_workspace_policy_field
- test_runtime_config_has_repo_root_field
- test_runtime_config_loads_repo_root_from_env_var
- test_runtime_config_loads_repo_root_from_current_dir_when_env_not_set

### Gate Command Type Tests
- test_gate_command_has_only_moon_variant
- test_gate_command_moon_variant_command_parts
- test_moon_task_from_task_name
- test_moon_task_as_task_name

### Failure Mapping Tests
- test_compiles_gate_failure_routes_to_same_stage
- test_tests_pass_gate_failure_routes_to_implementation
- test_moon_ci_gate_failure_routes_to_implementation
- test_holdout_scenarios_gate_failure_routes_to_implementation
- test_cue_artifact_generated_gate_failure_routes_to_implementation
- test_witness_holdout_failure_routes_to_implementation

## Error Path Tests

### Gate Parsing Errors
- test_gate_try_from_zjj_merge_queue_returns_error
- test_gate_try_from_unknown_string_returns_error
- test_gate_try_from_empty_string_returns_error

### Gate Command Parsing Errors
- test_parse_gate_command_returns_error_for_zjj_sync_status
- test_parse_gate_command_returns_error_for_zjj_command
- test_parse_gate_command_returns_error_for_unknown_program
- test_parse_gate_command_returns_error_for_empty_command
- test_parse_gate_command_returns_error_for_unquoted_whitespace
- test_parse_gate_command_returns_error_for_unclosed_single_quote
- test_parse_gate_command_returns_error_for_unclosed_double_quote
- test_parse_gate_command_returns_error_for_trailing_escape

### Gate Execution Errors
- test_execute_gate_returns_error_when_command_fails
- test_execute_gate_returns_error_when_revision_mismatch
- test_execute_gate_returns_error_on_timeout

### Runtime Configuration Errors
- test_runtime_config_load_returns_error_when_repo_root_empty
- test_runtime_config_load_returns_error_when_current_dir_fails

### Revision Validation Errors
- test_validate_revision_pair_returns_error_on_mismatch
- test_parse_revision_returns_error_for_short_sha
- test_parse_revision_returns_error_for_non_hex_chars

## Edge Case Tests

### Gate Enum Edge Cases
- test_gate_serialization_and_deserialization_roundtrip
- test_gate_hash_works_for_all_variants
- test_gate_equality_works_for_all_variants

### Stage Configuration Edge Cases
- test_all_stages_have_valid_gates
- test_ship_gate_has_single_gate_after_removal
- test_no_stage_references_zjj_merge_queue_gate

### Gate Command Parsing Edge Cases
- test_parse_command_parts_handles_multiple_quoted_args
- test_parse_command_parts_handles_empty_passthrough
- test_parse_command_parts_handles_special_characters
- test_parse_command_parts_handles_unicode

### Revision Validation Edge Cases
- test_validate_revision_pair_allows_both_none
- test_validate_revision_pair_allows_one_none
- test_is_full_sha_rejects_39_chars
- test_is_full_sha_accepts_40_chars
- test_is_full_sha_rejects_41_chars

### Runtime Configuration Edge Cases
- test_workspace_policy_from_skip_flag_true_returns_skip
- test_workspace_policy_from_skip_flag_false_returns_prepare
- test_workspace_policy_should_skip_returns_true_for_skip

## Contract Verification Tests

### Type System Contracts
- test_precondition_gate_enum_does_not_contain_zjj_merge_queue
- test_postcondition_gate_enum_has_five_variants
- test_invariant_all_gates_have_valid_as_str
- test_invariant_all_gates_have_valid_try_from

### Stage Configuration Contracts
- test_precondition_ship_gate_gates_do_not_contain_zjj_merge_queue
- test_postcondition_ship_gate_has_only_cue_artifact_generated
- test_invariant_all_stage_gates_are_valid_gate_variants

### Runtime Configuration Contracts
- test_precondition_runtime_config_does_not_have_merge_queue_policy_field
- test_postcondition_runtime_config_has_only_workspace_and_repo_fields
- test_invariant_runtime_config_fields_are_initialized
- test_precondition_merge_queue_policy_enum_does_not_exist
- test_postcondition_runtime_config_load_does_not_read_zjj_env_vars

### Gate Command Parsing Contracts
- test_precondition_gate_command_enum_does_not_have_zjj_sync_status
- test_postcondition_gate_command_has_only_moon_variant
- test_invariant_all_gate_commands_are_parseable

### Gate Execution Contracts
- test_precondition_execute_gate_uses_moon_timeout_for_all_gates
- test_postcondition_zjj_timeout_constant_not_used
- test_invariant_all_gates_execute_with_valid_commands

### Failure Mapping Contracts
- test_precondition_gate_failure_mapping_does_not_handle_zjj_merge_queue
- test_postcondition_no_zjj_specific_failure_mapping
- test_invariant_all_gates_have_failure_mapping_or_return_none

## Given-When-Then Scenarios

### Scenario 1: ShipGate stage has reduced gate list
Given: Pipeline is in ShipGate stage
When: Stage.gates() is called
Then:
- Result is a Vec containing exactly one Gate
- The Gate is CueArtifactGenerated
- No other gates are present

### Scenario 2: Attempting to parse ZJJ gate returns error
Given: A ZJJ gate command string "zjj sync --status"
When: parse_gate_command() is called
Then:
- Result is Err(OyaError)
- Error message indicates unsupported gate command
- ZjjSyncStatus variant is not returned

### Scenario 3: RuntimeConfig loads without ZJJ env vars
Given: WorkflowContext is available
And: OYA_REPO_ROOT is set to valid path
And: No ZJJ-related env vars are set
When: RuntimeConfig::load() is called
Then:
- Result is Ok(RuntimeConfig)
- RuntimeConfig has workspace_policy field
- RuntimeConfig has repo_root field
- RuntimeConfig does NOT have merge_queue_policy field
- No attempt is made to read OYA_DISABLE_ZJJ
- No attempt is made to read OYA_SKIP_ZJJ_GATE
- No attempt is made to read OYA_SKIP_ZJJ_WORKSPACE

### Scenario 4: Attempting to parse Gate::ZjjMergeQueue from string
Given: String "zjj_merge_queue"
When: Gate::try_from() is called
Then:
- Result is Err(String)
- Error message indicates unknown gate
- ZjjMergeQueue variant is not available

### Scenario 5: All gates use Moon timeout
Given: Any Gate variant (Compiles, TestsPass, MoonCi, HoldoutScenarios, CueArtifactGenerated)
When: execute_gate() is called
Then:
- MOON_TIMEOUT_SECONDS is used
- ZJJ_TIMEOUT_SECONDS is not referenced
- Timeout value is 900 seconds

### Scenario 6: GateCommand enum has only Moon variant
Given: GateCommand enum is available
When: Pattern matching on GateCommand
Then:
- Only Moon variant is available
- ZjjSyncStatus variant is not present
- Compile-time error occurs if ZjjSyncStatus is referenced

### Scenario 7: ShipGate failure routes to Implementation
Given: Pipeline is in ShipGate stage
And: Gate::CueArtifactGenerated fails
When: gate_failure_outcome() is called
Then:
- Failure category is OutputParseFailure
- Next stage is Implementation
- No ZJJ-specific failure mapping exists

### Scenario 8: Parsing unsupported gate command returns error
Given: String "zjj sync --status" or "zjj other-command"
When: parse_gate_command() is called
Then:
- Result is Err(OyaError)
- Error message indicates unsupported gate command
- No crash or panic occurs

### Scenario 9: All non-ZJJ gates continue to work
Given: Any non-ZJJ gate (Compiles, TestsPass, MoonCi, HoldoutScenarios, CueArtifactGenerated)
When: execute_gate() is called with valid repo_root
Then:
- Gate executes successfully
- Command output is captured
- Revision validation occurs for moon gates
- Result is Ok(GateEvidence)

### Scenario 10: Removal is complete and backward-incompatible
Given: Old code attempting to use Gate::ZjjMergeQueue
When: Code is compiled
Then:
- Compile-time error occurs
- Error message indicates ZjjMergeQueue does not exist
- Code must be updated to remove references

## Integration Tests

### End-to-End Pipeline Test
- test_pipeline_execution_without_zjj_gate_completes_successfully
- test_ship_gate_stage_completes_with_only_cue_artifact_generated

### Configuration Loading Test
- test_runtime_config_integration_with_restate_workflow

## Test Removal List

These tests should be REMOVED as they test ZJJ-specific functionality:

- test_all_ship_gate_failures_route_to_implementation (if it only tests ZjjMergeQueue)
- Any tests specifically testing ZJJ merge queue behavior
- Any tests referencing MergeQueuePolicy
- Any tests referencing ZjjSyncStatus
- Any tests referencing OYA_SKIP_ZJJ_GATE
- Any tests referencing OYA_SKIP_ZJJ_WORKSPACE
- Any tests referencing OYA_DISABLE_ZJJ

Note: Some existing tests may need to be MODIFIED (e.g., test_all_ship_gate_failures_route_to_implementation should be updated to test only CueArtifactGenerated).
