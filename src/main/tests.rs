use super::*;
use crate::operator_visibility::{
    bounded_tail_events, build_cleanup_reconcile_plan, deterministic_prune_run_artifacts,
    doctor_report_from_events, status_snapshot_from_events,
};
use crate::pipeline::MergeQueuePolicy;
use chrono::Datelike;
use clap::{error::ErrorKind, CommandFactory};
use oya::types::Gate;
use serde_json::json;

#[test]
fn test_parse_rfc3339_stable_valid_input() {
    let result = parse_rfc3339_stable("2024-01-15T10:30:00Z");
    assert_eq!(result.year(), 2024);
    assert_eq!(result.month(), 1);
    assert_eq!(result.day(), 15);
}

#[test]
fn test_parse_rfc3339_stable_with_timezone() {
    let result = parse_rfc3339_stable("2024-01-15T10:30:00+05:00");
    assert_eq!(result.year(), 2024);
}

#[test]
fn test_parse_rfc3339_stable_invalid_returns_epoch() {
    let result = parse_rfc3339_stable("invalid-timestamp");
    assert_eq!(result, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_rfc3339_stable_empty_returns_epoch() {
    let result = parse_rfc3339_stable("");
    assert_eq!(result, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_rfc3339_stable_malformed_returns_epoch() {
    let result = parse_rfc3339_stable("2024-13-45T99:99:99Z");
    assert_eq!(result, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_rfc3339_stable_is_stable() {
    let result1 = parse_rfc3339_stable("garbage");
    let result2 = parse_rfc3339_stable("garbage");
    assert_eq!(result1, result2);
    assert_eq!(result1, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_command_parts_moon_command() {
    let parsed = parse_command_parts("moon run :check");
    assert!(parsed.is_ok());

    let command = parsed.map_or_else(
        |_| ParsedCommandParts { program: String::new(), args: Vec::new() },
        std::convert::identity,
    );
    assert_eq!(command.program, "moon");
    assert_eq!(command.args, vec!["run".to_string(), ":check".to_string()]);
}

#[test]
fn test_parse_command_parts_rejects_empty_command() {
    let parsed = parse_command_parts("   ");
    assert!(parsed.is_err());
}

#[test]
fn test_parse_gate_command_accepts_moon_gate_with_passthrough_args() {
    let parsed = parse_gate_command("moon run :test -- --test-threads=1");
    assert!(parsed.is_ok());

    match parsed {
        Ok(GateCommand::Moon { task: MoonTask::Test, passthrough }) => {
            assert_eq!(passthrough, vec!["--".to_string(), "--test-threads=1".to_string()]);
        }
        Ok(_) => panic!("expected moon test gate command"),
        Err(_) => panic!("expected successful gate command parse"),
    }
}

#[test]
fn test_parse_gate_command_rejects_unknown_program() {
    let parsed = parse_gate_command("cargo check");
    assert!(parsed.is_err());
}

#[test]
fn test_parse_gate_command_rejects_unknown_moon_task() {
    let parsed = parse_gate_command("moon run :quick");
    assert!(parsed.is_err());
}

#[test]
fn test_parse_gate_command_accepts_zjj_sync_status() {
    let parsed = parse_gate_command("zjj sync --status");
    assert!(matches!(parsed, Ok(GateCommand::ZjjSyncStatus)));
}

#[test]
fn test_parse_gate_command_accepts_cue_check_task() {
    let parsed = parse_gate_command("moon run :cue-check");
    assert!(matches!(
        parsed,
        Ok(GateCommand::Moon { task: MoonTask::CueCheck, passthrough }) if passthrough.is_empty()
    ));
}

#[test]
fn test_gate_failure_outcome_shipgate_merge_conflict_routes_to_implementation() {
    let outcome = gate_failure_outcome(&Stage::ShipGate, &Gate::ZjjMergeQueue);
    assert_eq!(outcome, (FailureCategory::MergeConflict, Stage::Implementation));
}

#[test]
fn test_execute_ship_gate_skip_zjj_gate() {
    use std::cell::RefCell;

    let seen = RefCell::new(Vec::new());
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Skip, |gate| {
        seen.borrow_mut().push(gate.clone());
        Ok(GateEvidence {
            command: "moon run :gate".to_string(),
            passed: true,
            exit_code: 0,
            output: "ok".to_string(),
            revision: None,
            current_revision: None,
        })
    })
    .expect("ship gate should pass when cue check passes and zjj is skipped");

    assert_eq!(seen.into_inner(), vec![Gate::CueArtifactGenerated]);
    assert!(result.passed);
    assert_eq!(result.next_stage, None);
}

#[test]
fn test_execute_ship_gate_enforce_still_skips_zjj_gate() {
    use std::cell::RefCell;

    let seen = RefCell::new(Vec::new());
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Enforce, |gate: Gate| {
        seen.borrow_mut().push(gate.clone());
        match gate {
            Gate::CueArtifactGenerated => Ok(GateEvidence {
                command: "moon run :cue-check".to_string(),
                passed: true,
                exit_code: 0,
                output: "cue ok".to_string(),
                revision: None,
                current_revision: None,
            }),
            _ => Ok(GateEvidence {
                command: "unexpected".to_string(),
                passed: false,
                exit_code: 1,
                output: "unexpected".to_string(),
                revision: None,
                current_revision: None,
            }),
        }
    })
    .expect("ship gate execution should return a stage result");

    assert_eq!(seen.into_inner(), vec![Gate::CueArtifactGenerated]);
    assert!(result.passed);
    assert_eq!(result.failure_category, None);
    assert_eq!(result.next_stage, None);
}

#[test]
fn test_execute_ship_gate_rejects_stale_cue_evidence() {
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Skip, |_gate| {
        Ok(GateEvidence {
            command: "moon run :cue-check".to_string(),
            passed: true,
            exit_code: 0,
            output: "cue ok".to_string(),
            revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            current_revision: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
        })
    })
    .expect("ship gate execution should return a stage result");

    assert!(!result.passed);
    assert_eq!(result.failure_category, Some(FailureCategory::OutputParseFailure));
}

#[test]
fn test_cli_tail_mode_parses_default_interval() {
    let mode = parse_cli_mode_from(["oya", "tail"]);
    assert_eq!(
        mode,
        CliMode::Tail(TailArgs {
            interval: 2,
            run_id: None,
            events: false,
            follow: false,
            limit: 50,
        })
    );
}

#[test]
fn test_cli_tail_mode_parses_interval_and_run_id() {
    let mode = parse_cli_mode_from(["oya", "tail", "--interval", "5", "run-123"]);
    assert_eq!(
        mode,
        CliMode::Tail(TailArgs {
            interval: 5,
            run_id: Some("run-123".to_string()),
            events: false,
            follow: false,
            limit: 50,
        })
    );
}

#[test]
fn test_status_command_returns_stage_attempt_reason_and_next_action() {
    let events = vec![json!({
        "run_id": "run-42",
        "stage": "implementation",
        "attempt": 2,
        "reason": "test_failed",
        "next_action": "rerun implementation",
        "at": "2026-02-22T10:00:00Z"
    })];

    let snapshot = status_snapshot_from_events(&events, Some("run-42"));
    assert_eq!(snapshot.run_id, "run-42");
    assert_eq!(snapshot.stage, "implementation");
    assert_eq!(snapshot.attempt, 2);
    assert_eq!(snapshot.reason, "test_failed");
    assert_eq!(snapshot.next_action, "rerun implementation");
}

#[test]
fn test_tail_streams_ordered_events_with_bounded_output_window() {
    let events = vec![
        json!({"seq": 1, "run_id": "r1"}),
        json!({"seq": 2, "run_id": "r1"}),
        json!({"seq": 3, "run_id": "r1"}),
    ];

    let tail = bounded_tail_events(&events, None, 2);
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0]["seq"], 2);
    assert_eq!(tail[1]["seq"], 3);
}

#[test]
fn test_doctor_flags_stuck_runs_with_reason_codes() {
    let events = vec![json!({
        "run_id": "run-stuck",
        "status": "running",
        "stage": "contract",
        "at": "2026-02-22T09:00:00Z"
    })];

    let report = doctor_report_from_events(&events, "2026-02-22T10:00:00Z", 300);
    assert_eq!(report.stuck_runs, 1);
    assert_eq!(report.issues.len(), 1);
    assert_eq!(report.issues[0].reason_code, "stuck_running");
}

#[test]
fn test_doctor_flags_cleanup_pending_backlog_with_counts() {
    let events = vec![
        json!({
            "run_id": "run-a",
            "status": "cleanup_pending",
            "at": "2026-02-22T09:59:00Z"
        }),
        json!({
            "run_id": "run-b",
            "cleanup_pending": true,
            "at": "2026-02-22T09:59:30Z"
        }),
    ];

    let report = doctor_report_from_events(&events, "2026-02-22T10:00:00Z", 300);
    assert_eq!(report.cleanup_pending_backlog, 2);
}

#[test]
fn test_doctor_flags_cleanup_retry_exhausted_with_reason_code() {
    let events = vec![json!({
        "run_id": "run-exhausted",
        "status": "cleanup_pending",
        "stage": "implementation",
        "at": "2026-02-22T09:00:00Z"
    })];

    let report = doctor_report_from_events(&events, "2026-02-22T10:00:00Z", 300);
    assert_eq!(report.issues.len(), 1);
    assert_eq!(report.issues[0].reason_code, "cleanup_retry_exhausted");
}

#[test]
fn test_cleanup_reconcile_plan_prunes_oldest_cleanup_pending_first() {
    let events = vec![
        json!({"run_id":"run-b","status":"cleanup_pending","at":"2026-02-22T10:01:00Z"}),
        json!({"run_id":"run-a","status":"cleanup_pending","at":"2026-02-22T09:59:00Z"}),
        json!({"run_id":"run-c","cleanup_pending":true,"at":"2026-02-22T09:59:00Z"}),
    ];

    let plan = build_cleanup_reconcile_plan(&events, 1);

    assert_eq!(plan.prune_run_ids, vec!["run-a".to_string(), "run-c".to_string()]);
}

#[test]
fn test_deterministic_prune_run_artifacts_removes_sorted_unique_run_dirs() {
    let root = std::env::temp_dir().join(format!(
        "oya-prune-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |value| value.as_nanos())
    ));
    let runs_root = root.join(".orchestrator").join("runs");
    std::fs::create_dir_all(runs_root.join("run-b")).expect("create run-b artifact dir");
    std::fs::create_dir_all(runs_root.join("run-a")).expect("create run-a artifact dir");

    let removed = deterministic_prune_run_artifacts(
        &root,
        &["run-b".to_string(), "run-a".to_string(), "run-a".to_string(), "run-missing".to_string()],
    )
    .expect("artifact pruning should succeed");

    assert_eq!(removed, vec!["run-a".to_string(), "run-b".to_string()]);
    assert!(!runs_root.join("run-a").exists());
    assert!(!runs_root.join("run-b").exists());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_cli_init_mode_parses() {
    let mode = parse_cli_mode_from(["oya", "init"]);
    assert_eq!(mode, CliMode::Init);
}

#[test]
fn test_cli_check_mode_parses() {
    let mode = parse_cli_mode_from(["oya", "check"]);
    assert_eq!(mode, CliMode::Check);
}

#[test]
fn test_cli_cleanup_reconcile_parses_defaults() {
    let mode = parse_cli_mode_from(["oya", "cleanup-reconcile"]);
    assert_eq!(mode, CliMode::CleanupReconcile(CleanupReconcileArgs { keep_latest: 10 }));
}

#[test]
fn test_cli_up_alias_parses_as_init() {
    let mode = parse_cli_mode_from(["oya", "up"]);
    assert_eq!(mode, CliMode::Init);
}

#[test]
fn test_cli_bundle_mode_parses() {
    let mode = parse_cli_mode_from(["oya", "bundle"]);
    assert_eq!(mode, CliMode::Bundle);
}

#[test]
fn test_cli_observe_mode_parses_defaults() {
    let mode = parse_cli_mode_from(["oya", "observe"]);
    assert_eq!(
        mode,
        CliMode::Observe(ObserveArgs { run_id: None, follow: false, interval: 2, limit: 50_u64 })
    );
}

#[test]
fn test_cli_run_mode_parses_unique_run_id_mode() {
    let mode = parse_cli_mode_from(["oya", "run", "src-1", "--run-id-mode", "unique"]);
    assert_eq!(
        mode,
        CliMode::Run(RunArgs {
            bead_id: "src-1".to_string(),
            restate_url: "http://127.0.0.1:8080".to_string(),
            context: "local docker validation".to_string(),
            timeout: 3600,
            poll_interval: None,
            model: None,
            run_id_mode: RunIdMode::Unique,
        })
    );
}

#[test]
fn test_cli_tail_interval_rejects_zero() {
    let result = Cli::try_parse_from(["oya", "tail", "--interval", "0"]);
    assert!(result.is_err());
}

#[test]
fn test_cli_timeout_rejects_zero() {
    let result = Cli::try_parse_from(["oya", "run", "src-1", "--timeout", "0"]);
    assert!(result.is_err());
}

#[test]
fn test_cli_timeout_rejects_too_large() {
    let result = Cli::try_parse_from(["oya", "run", "src-1", "--timeout", "86401"]);
    assert!(result.is_err());
}

#[test]
fn test_cli_supports_version_flag() {
    let command = Cli::command();
    let result = command.try_get_matches_from(["oya", "--version"]);
    assert!(matches!(result, Err(error) if error.kind() == ErrorKind::DisplayVersion));
}

#[test]
fn test_tests_unexpectedly_green_maps_to_retry_loop() {
    let mut state =
        test_pipeline_state(FailureCategory::TestsUnexpectedlyGreen, Stage::Implementation, 1);
    assert!(should_retry_after_failure(&state));

    state.attempt = 2;
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_evaluate_start_request_allows_fresh_claim() {
    let parsed = StartRequestPayload {
        bead_id: Some("src-2s0".to_string()),
        context: Some("context".to_string()),
        model: None,
    };
    let admission = evaluate_start_request(None, &parsed, "run-1");
    assert!(matches!(admission, Ok(StartAdmission::Fresh)));
}

#[test]
fn test_evaluate_start_request_returns_idempotent_for_same_payload() {
    let parsed = StartRequestPayload {
        bead_id: Some("src-2s0".to_string()),
        context: Some("context".to_string()),
        model: None,
    };
    let existing = RunRequestEvent {
        run_id: "run-1".to_string(),
        bead_id: "src-2s0".to_string(),
        context: "context".to_string(),
        started_at: "2026-02-22T00:00:00Z".to_string(),
    };
    let admission = evaluate_start_request(Some(existing), &parsed, "run-1");
    assert!(matches!(admission, Ok(StartAdmission::Idempotent(run_id)) if run_id == "run-1"));
}

#[test]
fn test_evaluate_start_request_rejects_stale_lease_token() {
    let parsed = StartRequestPayload {
        bead_id: Some("src-2s0".to_string()),
        context: Some("context".to_string()),
        model: None,
    };
    let existing = RunRequestEvent {
        run_id: "run-old".to_string(),
        bead_id: "src-2s0".to_string(),
        context: "context".to_string(),
        started_at: "2026-02-22T00:00:00Z".to_string(),
    };
    let admission = evaluate_start_request(Some(existing), &parsed, "run-1");
    assert!(admission.is_err());
}

#[test]
fn test_evaluate_start_request_rejects_different_payload() {
    let parsed = StartRequestPayload {
        bead_id: Some("src-2s0".to_string()),
        context: Some("new-context".to_string()),
        model: None,
    };
    let existing = RunRequestEvent {
        run_id: "run-1".to_string(),
        bead_id: "src-2s0".to_string(),
        context: "old-context".to_string(),
        started_at: "2026-02-22T00:00:00Z".to_string(),
    };
    let admission = evaluate_start_request(Some(existing), &parsed, "run-1");
    assert!(admission.is_err());
}

#[test]
fn test_rate_limited_is_non_retryable() {
    // RateLimited triggers model rotation, NOT stage retry
    let state = test_pipeline_state(FailureCategory::RateLimited, Stage::Red, 1);
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_provider_unavailable_retries_for_all_stages() {
    let explore = test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::Explore, 1);
    assert!(should_retry_after_failure(&explore));

    let contract = test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::Contract, 1);
    assert!(should_retry_after_failure(&contract));

    let implementation =
        test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::Implementation, 1);
    assert!(should_retry_after_failure(&implementation));

    let witness = test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::Witness, 1);
    assert!(should_retry_after_failure(&witness));

    let ship_gate = test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::ShipGate, 1);
    assert!(should_retry_after_failure(&ship_gate));
}

#[test]
fn test_watchdog_timeout_is_terminal_and_non_retryable() {
    let state = test_pipeline_state(FailureCategory::WatchdogTimeout, Stage::Explore, 1);
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_watchdog_completed_result_returns_stage_result_when_under_budget() {
    let artifact = StageArtifact {
        stage: "explore".to_string(),
        attempt: 1,
        failure_category: None,
        next_stage: Some("contract".to_string()),
        timing: StageTiming {
            started_at: "2026-02-22T10:00:00Z".to_string(),
            completed_at: "2026-02-22T10:00:01Z".to_string(),
            duration_ms: 1000,
        },
        workspace: None,
        input: StageInputData {
            run_id: "run-1".to_string(),
            bead_id: "src-1".to_string(),
            context: "ctx".to_string(),
            model: "openai/gpt-5".to_string(),
            last_failure: None,
        },
        prompt: "prompt".to_string(),
        output: StageOutputData {
            success: true,
            exit_code: 0,
            full_log: "ok".to_string(),
            feedback: "ok".to_string(),
            contract_document: None,
            implementation_code: None,
            test_results: None,
            adversarial_report: None,
        },
        task_tracking: None,
        gates: vec![],
        status: StageStatus::Completed,
    };

    let result = watchdog_completed_result(Ok::<StageArtifact, tokio::time::error::Elapsed>(
        artifact.clone(),
    ));
    let preserved = result.expect("under-budget stage should preserve execution result");
    assert_eq!(preserved.stage, "explore");
    assert_eq!(preserved.attempt, 1);
}

#[test]
fn test_provider_retry_backoff_grows_and_caps() {
    assert_eq!(provider_retry_backoff(1), std::time::Duration::from_millis(1_000));
    assert_eq!(provider_retry_backoff(2), std::time::Duration::from_millis(2_000));
    assert_eq!(provider_retry_backoff(6), std::time::Duration::from_millis(8_000));
}

#[test]
fn test_parse_failure_category_supports_watchdog_timeout() {
    let parsed = parse_failure_category(&Some("watchdog_timeout".to_string()));
    assert_eq!(parsed, Some(FailureCategory::WatchdogTimeout));
}

#[test]
fn test_provider_unavailable_rotates_provider() {
    assert!(should_rotate_provider_on_failure(&FailureCategory::ProviderUnavailable));
}

#[test]
fn test_provider_rotation_reason_includes_category_and_models() {
    let reason = provider_rotation_reason(
        "openai/gpt-5",
        "anthropic/claude-sonnet-4",
        &FailureCategory::ProviderUnavailable,
    );
    assert!(reason.contains("provider_unavailable"));
    assert!(reason.contains("openai/gpt-5"));
    assert!(reason.contains("anthropic/claude-sonnet-4"));
}

#[test]
fn test_should_rotate_provider_on_failure_includes_rate_limited_only_failure_classes() {
    assert!(should_rotate_provider_on_failure(&FailureCategory::RateLimited));
    assert!(should_rotate_provider_on_failure(&FailureCategory::ProviderUnavailable));
    assert!(!should_rotate_provider_on_failure(&FailureCategory::CompileFailed));
}

#[test]
fn test_provider_unavailable_retry_honors_stage_max_attempts() {
    let mut state = test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::Explore, 1);
    assert!(should_retry_after_failure(&state));

    state.attempt = provider_unavailable_max_attempts();
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_provider_unavailable_max_attempts_defaults_and_clamps() {
    std::env::remove_var("OYA_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS");
    assert_eq!(provider_unavailable_max_attempts(), 3);

    std::env::set_var("OYA_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS", "1");
    assert_eq!(provider_unavailable_max_attempts(), 2);

    std::env::set_var("OYA_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS", "99");
    assert_eq!(provider_unavailable_max_attempts(), 6);

    std::env::remove_var("OYA_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS");
}

#[test]
fn test_transient_provider_retry_stage_is_enabled_for_all_stages() {
    let stages = vec![
        Stage::Explore,
        Stage::Contract,
        Stage::Red,
        Stage::Implementation,
        Stage::Witness,
        Stage::ShipGate,
    ];

    let all_retryable = stages
        .iter()
        .all(|stage| transient_provider_retry_stage(stage, &FailureCategory::ProviderUnavailable));

    assert!(all_retryable);
}

#[test]
fn test_failure_transition_policy_allows_same_stage_and_implementation() {
    assert!(is_allowed_failure_transition(&Stage::Explore, &Stage::Explore));
    assert!(is_allowed_failure_transition(&Stage::Witness, &Stage::Implementation));
}

#[test]
fn test_failure_transition_policy_rejects_non_implementation_hops() {
    assert!(!is_allowed_failure_transition(&Stage::Explore, &Stage::Contract));
    assert!(!is_allowed_failure_transition(&Stage::Contract, &Stage::Red));
}

#[test]
fn test_lifecycle_transition_accepts_running_to_terminal() {
    let shipped = lifecycle_transition("running", LifecycleStatus::Shipped)
        .expect("running -> shipped should be valid");
    let failed = lifecycle_transition("running", LifecycleStatus::Failed)
        .expect("running -> failed should be valid");
    assert_eq!(shipped, Some(LifecycleStatus::Shipped));
    assert_eq!(failed, Some(LifecycleStatus::Failed));
}

#[test]
fn test_lifecycle_transition_rejects_invalid_edges_without_mutation() {
    let result = lifecycle_transition("queued", LifecycleStatus::Failed);
    assert!(result.is_err());
}

#[test]
fn test_lifecycle_transition_duplicate_is_idempotent_noop() {
    let result = lifecycle_transition("running", LifecycleStatus::Running)
        .expect("duplicate transition should be accepted as no-op");
    assert_eq!(result, None);
}

#[test]
fn test_provider_pool_recovery_seconds_defaults_and_clamps() {
    std::env::remove_var("OYA_PROVIDER_POOL_RECOVERY_SECONDS");
    assert_eq!(provider_pool_recovery_seconds(), 180);

    std::env::set_var("OYA_PROVIDER_POOL_RECOVERY_SECONDS", "15");
    assert_eq!(provider_pool_recovery_seconds(), 60);

    std::env::set_var("OYA_PROVIDER_POOL_RECOVERY_SECONDS", "1500");
    assert_eq!(provider_pool_recovery_seconds(), 900);

    std::env::remove_var("OYA_PROVIDER_POOL_RECOVERY_SECONDS");
}

#[test]
fn test_infra_failed_is_non_retryable() {
    let state = test_pipeline_state(FailureCategory::TestInfraFailed, Stage::Implementation, 1);
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_tracker_backpressure_error_detection() {
    assert!(is_tracker_backpressure_error("tier_circuit_open tier=c retry_after_ms=1200"));
    assert!(is_tracker_backpressure_error("all_models_rate_limited tier=d retry_after_ms=30000"));
    assert!(is_tracker_backpressure_error("tier_token_exhausted tier=b"));
    assert!(!is_tracker_backpressure_error("provider unavailable"));
}

#[test]
fn test_tracker_backoff_duration_grows_and_caps() {
    let first = tracker_backoff_duration("c", 1);
    let second = tracker_backoff_duration("c", 2);
    let tenth = tracker_backoff_duration("c", 10);

    assert!(second > first);
    assert!(tenth <= std::time::Duration::from_millis(8_000));
}

#[test]
fn test_remediation_description_includes_failure_context() {
    let state = test_pipeline_state(FailureCategory::TestFailed, Stage::Implementation, 2);
    let artifact = StageArtifact {
        stage: "implementation".to_string(),
        attempt: 2,
        failure_category: Some("test_failed".to_string()),
        next_stage: Some("implementation".to_string()),
        timing: StageTiming {
            started_at: "2026-02-20T00:00:00Z".to_string(),
            completed_at: "2026-02-20T00:00:01Z".to_string(),
            duration_ms: 1000,
        },
        workspace: None,
        input: StageInputData {
            run_id: "run-1".to_string(),
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model: "model".to_string(),
            last_failure: None,
        },
        prompt: "prompt".to_string(),
        output: StageOutputData {
            success: false,
            exit_code: 1,
            full_log: "tests failed".to_string(),
            feedback: "test_failed".to_string(),
            contract_document: None,
            implementation_code: None,
            test_results: None,
            adversarial_report: None,
        },
        task_tracking: None,
        gates: vec![],
        status: StageStatus::Failed,
    };

    let description = remediation_description(&state, &artifact);
    assert!(description.contains("exhausted automatic retries"));
    assert!(description.contains("test_failed"));
}

#[test]
fn test_stack_transition_parent_blocked_child_ready_is_deterministic() {
    let before = StackPairState::new(StackReadiness::Ready, StackReadiness::Ready);
    let after =
        apply_stack_transition(before.clone(), StackTransitionKind::ParentBlockedChildReady);

    assert!(after.is_ok());
    let after = after.unwrap_or(before);
    assert_eq!(after.parent, StackReadiness::Blocked);
    assert_eq!(after.child, StackReadiness::Ready);
}

#[test]
fn test_stack_transition_parent_ready_rejects_when_child_still_blocked() {
    let before = StackPairState::new(StackReadiness::Blocked, StackReadiness::Blocked);

    let result = apply_stack_transition(before, StackTransitionKind::ParentReady);

    assert!(result.is_err());
}

#[test]
fn test_extract_child_bead_id_ignores_parent_and_returns_new_child() {
    let output = BrCommandOutput {
        stdout: "created src-16e with parent src-16d".to_string(),
        stderr: String::new(),
    };

    let child = extract_child_bead_id(&output, "src-16d");

    assert_eq!(child, Some("src-16e".to_string()));
}

#[test]
fn test_stack_transition_metadata_representation_captures_before_after() {
    let before = StackPairState::new(StackReadiness::Ready, StackReadiness::Ready);
    let after = StackPairState::new(StackReadiness::Blocked, StackReadiness::Ready);

    let metadata = build_stack_transition_metadata(StackTransitionRecordInput {
        parent_bead_id: "src-16d".to_string(),
        child_bead_id: Some("src-16e".to_string()),
        transition: StackTransitionKind::ParentBlockedChildReady,
        before,
        after,
        reason: "retry_exhausted".to_string(),
        recorded_at: "2026-02-22T12:00:00Z".to_string(),
    });

    assert_eq!(metadata.parent_bead_id, "src-16d");
    assert_eq!(metadata.child_bead_id, Some("src-16e".to_string()));
    assert_eq!(metadata.before.parent, StackReadiness::Ready);
    assert_eq!(metadata.after.parent, StackReadiness::Blocked);
}

#[test]
fn test_terminal_pipeline_failure_message_includes_stage_and_category() {
    let state = test_pipeline_state(FailureCategory::TestFailed, Stage::Implementation, 2);
    let artifact = StageArtifact {
        stage: "implementation".to_string(),
        attempt: 2,
        failure_category: Some("test_failed".to_string()),
        next_stage: Some("implementation".to_string()),
        timing: StageTiming {
            started_at: "2026-02-20T00:00:00Z".to_string(),
            completed_at: "2026-02-20T00:00:01Z".to_string(),
            duration_ms: 1000,
        },
        workspace: None,
        input: StageInputData {
            run_id: "run-1".to_string(),
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model: "model".to_string(),
            last_failure: None,
        },
        prompt: "prompt".to_string(),
        output: StageOutputData {
            success: false,
            exit_code: 1,
            full_log: "tests failed".to_string(),
            feedback: "test_failed".to_string(),
            contract_document: None,
            implementation_code: None,
            test_results: None,
            adversarial_report: None,
        },
        task_tracking: None,
        gates: vec![],
        status: StageStatus::Failed,
    };

    let message = terminal_pipeline_failure_message(&state, &artifact);
    assert!(message.contains("pipeline failed:"));
    assert!(message.contains("stage=implementation"));
    assert!(message.contains("category=test_failed"));
}

#[test]
fn test_pipeline_stage_watchdog_seconds_defaults_and_clamps() {
    std::env::remove_var("OYA_PIPELINE_STAGE_WATCHDOG_SECONDS");
    assert_eq!(pipeline_stage_watchdog_seconds(), 480);

    std::env::set_var("OYA_PIPELINE_STAGE_WATCHDOG_SECONDS", "30");
    assert_eq!(pipeline_stage_watchdog_seconds(), 60);

    std::env::set_var("OYA_PIPELINE_STAGE_WATCHDOG_SECONDS", "9000");
    assert_eq!(pipeline_stage_watchdog_seconds(), 3_600);

    std::env::remove_var("OYA_PIPELINE_STAGE_WATCHDOG_SECONDS");
}

#[test]
fn test_stage_timeout_duration_ms_never_negative() {
    let duration = stage_timeout_duration_ms("2026-02-22T02:10:00Z", "2026-02-22T02:09:00Z");
    assert_eq!(duration, 0);
}

#[test]
fn test_stage_timeout_log_mentions_stage_attempt_and_watchdog() {
    let mut state = test_pipeline_state(FailureCategory::ProviderUnavailable, Stage::Explore, 2);
    state.orchestrator.model = "free-tier-model".to_string();
    let input = PipelineRunInput {
        run_id: "run-123".to_string(),
        bead_id: "bead-abc".to_string(),
        context: "ctx".to_string(),
    };
    let message = stage_timeout_log(&state, &input, 480);
    assert!(message.contains("outcome=terminal"));
    assert!(message.contains("failure_category=watchdog_timeout"));
    assert!(message.contains("stage=explore"));
    assert!(message.contains("attempt=2"));
    assert!(message.contains("watchdog_seconds=480"));
    assert!(message.contains("model=free-tier-model"));
    assert!(message.contains("run_id=run-123"));
}

#[test]
fn test_stage_kind_helpers_match_expected_stage_names() {
    let red_artifact = StageArtifact {
        stage: "red".to_string(),
        attempt: 1,
        failure_category: None,
        next_stage: Some("implementation".to_string()),
        timing: StageTiming {
            started_at: "2026-02-20T00:00:00Z".to_string(),
            completed_at: "2026-02-20T00:00:01Z".to_string(),
            duration_ms: 1000,
        },
        workspace: None,
        input: StageInputData {
            run_id: "run-1".to_string(),
            bead_id: "bead-1".to_string(),
            context: "ctx".to_string(),
            model: "model".to_string(),
            last_failure: None,
        },
        prompt: "prompt".to_string(),
        output: StageOutputData {
            success: true,
            exit_code: 0,
            full_log: "ok".to_string(),
            feedback: "Success".to_string(),
            contract_document: None,
            implementation_code: None,
            test_results: Some("tests are red".to_string()),
            adversarial_report: None,
        },
        task_tracking: None,
        gates: vec![],
        status: StageStatus::Completed,
    };

    assert!(stage_is_red(&red_artifact));
    assert!(!stage_is_implementation(&red_artifact));
}

#[test]
fn test_red_seal_record_tracks_artifact_identity() {
    let state = test_pipeline_state(FailureCategory::TestFailed, Stage::Red, 1);
    let artifact = StageArtifact {
        stage: "red".to_string(),
        attempt: 2,
        failure_category: None,
        next_stage: Some("implementation".to_string()),
        timing: StageTiming {
            started_at: "2026-02-20T00:00:00Z".to_string(),
            completed_at: "2026-02-20T00:00:01Z".to_string(),
            duration_ms: 1000,
        },
        workspace: None,
        input: StageInputData {
            run_id: "run-1".to_string(),
            bead_id: "bead-1".to_string(),
            context: "ctx".to_string(),
            model: "model".to_string(),
            last_failure: None,
        },
        prompt: "prompt".to_string(),
        output: StageOutputData {
            success: true,
            exit_code: 0,
            full_log: "ok".to_string(),
            feedback: "Success".to_string(),
            contract_document: None,
            implementation_code: None,
            test_results: Some("tests are red".to_string()),
            adversarial_report: None,
        },
        task_tracking: None,
        gates: vec![],
        status: StageStatus::Completed,
    };

    let seal = red_seal_record(&state, &artifact);
    assert_eq!(seal.bead_id, "bead");
    assert_eq!(seal.stage, "red");
    assert_eq!(seal.artifact_key, "red_2");
}

fn test_pipeline_state(category: FailureCategory, stage: Stage, attempt: u32) -> PipelineState {
    PipelineState {
        current_stage: stage.clone(),
        attempt,
        red_seal_ready: false,
        last_failure: Some(StageFailure {
            category: category.clone(),
            message: "failure".to_string(),
            retryable: oya::is_retryable_failure(&category),
            failed_at: "2026-02-20T00:00:00Z".to_string(),
        }),
        resolved_models: std::collections::HashMap::new(),
        orchestrator: OrchestratorState {
            status: "running".to_string(),
            stage: stage.as_str().to_string(),
            attempt,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model: "model".to_string(),
            last_failure: String::new(),
            last_output: String::new(),
            last_prompt: String::new(),
            updated_at: "2026-02-20T00:00:00Z".to_string(),
        },
    }
}
