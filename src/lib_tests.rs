use super::*;
use chrono::Duration;

fn make_valid_smoke_report() -> SmokeReport {
    let base = Utc::now();
    SmokeReport {
        run_id: "run-test".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-test/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::seconds(1),
            },
        ],
        stages: vec![
            SmokeStageReport {
                stage: SmokeStageName::IngressHealth,
                status: SmokeStageStatus::Passed,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            SmokeStageReport {
                stage: SmokeStageName::OrchestratorStatus,
                status: SmokeStageStatus::Passed,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::seconds(1),
            },
            SmokeStageReport {
                stage: SmokeStageName::FinalDecision,
                status: SmokeStageStatus::Passed,
                diagnostics: "smoke checks passed".to_string(),
                timestamp: base + Duration::seconds(2),
            },
        ],
        decision: SmokeDecision::Pass,
    }
}

fn make_valid_smoke_bead_report() -> SmokeBeadReport {
    let base = Utc::now();
    SmokeBeadReport {
        run_id: "run-smoke-bead-test".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-bead-test/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::seconds(1),
            },
        ],
        stages: vec![
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::IngressHealth,
                status: SmokeBeadStageStatus::Passed,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::OrchestratorStatus,
                status: SmokeBeadStageStatus::Passed,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::seconds(1),
            },
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::FinalDecision,
                status: SmokeBeadStageStatus::Passed,
                diagnostics: "smoke-bead checks passed".to_string(),
                timestamp: base + Duration::seconds(2),
            },
        ],
        decision: SmokeBeadDecision::Pass,
    }
}

fn make_valid_bead_min_report() -> BeadMinReport {
    let base = Utc::now();
    BeadMinReport {
        run_id: "run-bead-min-test".to_string(),
        checks: vec![
            BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-bead-min-test/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
        ],
        stages: vec![
            BeadMinStageReport {
                stage: BeadMinStageName::IngressHealth,
                status: BeadMinStageStatus::Passed,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            BeadMinStageReport {
                stage: BeadMinStageName::OrchestratorStatus,
                status: BeadMinStageStatus::Passed,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            BeadMinStageReport {
                stage: BeadMinStageName::FinalDecision,
                status: BeadMinStageStatus::Passed,
                diagnostics: "bead-min checks passed".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
        ],
        decision: BeadMinDecision::Pass,
    }
}

fn make_valid_bead_cupid_report() -> BeadCupidReport {
    let plan_result = build_bead_cupid_plan(&BeadCupidInput {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
    });
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => {
            return BeadCupidReport {
                plan: BeadCupidPlan {
                    run_id: "run-cupid-001".to_string(),
                    bead_id: "bead-cupid-001".to_string(),
                    runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
                    ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                    orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status"
                        .to_string(),
                },
                checks: vec![],
                stages: vec![],
                decision: BeadCupidDecision::Fail,
            };
        }
    };

    let runtime_result = start_bead_cupid_runtime(&plan);
    let runtime = match runtime_result {
        Ok(value) => value,
        Err(_) => {
            return BeadCupidReport {
                plan,
                checks: vec![],
                stages: vec![],
                decision: BeadCupidDecision::Fail,
            };
        }
    };

    let observation_result = capture_bead_cupid_observation(&runtime);
    let observation = match observation_result {
        Ok(value) => value,
        Err(_) => {
            return BeadCupidReport {
                plan: BeadCupidPlan {
                    run_id: runtime.run_id,
                    bead_id: runtime.bead_id,
                    runtime_command: runtime.runtime_command,
                    ingress_health_url: runtime.ingress_health_url,
                    orchestrator_status_url: runtime.orchestrator_status_url,
                },
                checks: vec![],
                stages: vec![],
                decision: BeadCupidDecision::Fail,
            };
        }
    };

    let report_result = evaluate_bead_cupid_result(&observation);
    match report_result {
        Ok(value) => value,
        Err(_) => BeadCupidReport {
            plan: BeadCupidPlan {
                run_id: observation.run_id,
                bead_id: observation.bead_id,
                runtime_command: observation.runtime_command,
                ingress_health_url: observation.ingress_health_url,
                orchestrator_status_url: observation.orchestrator_status_url,
            },
            checks: observation.checks,
            stages: vec![],
            decision: BeadCupidDecision::Fail,
        },
    }
}

fn make_valid_src_1ew_report() -> Src1ewReport {
    let plan_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "get-pokemon".to_string(),
        query: "pikachu".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => {
            return Src1ewReport {
                plan: Src1ewPlan {
                    mode: Src1ewCommandMode::GetPokemon,
                    query: Some("pikachu".to_string()),
                    limit: 20,
                    offset: 0,
                    base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
                },
                checks: vec![],
                stages: vec![],
                decision: Src1ewDecision::Fail,
            };
        }
    };

    let runtime_result = start_src_1ew_runtime(&plan);
    let runtime = match runtime_result {
        Ok(value) => value,
        Err(_) => {
            return Src1ewReport {
                plan,
                checks: vec![],
                stages: vec![],
                decision: Src1ewDecision::Fail,
            };
        }
    };

    let observation_result = capture_src_1ew_observation(&runtime);
    let observation = match observation_result {
        Ok(value) => value,
        Err(_) => {
            return Src1ewReport {
                plan: Src1ewPlan {
                    mode: runtime.mode,
                    query: runtime.query,
                    limit: runtime.limit,
                    offset: runtime.offset,
                    base_url: runtime.base_url,
                },
                checks: vec![],
                stages: vec![],
                decision: Src1ewDecision::Fail,
            };
        }
    };

    match evaluate_src_1ew_observation(&observation) {
        Ok(value) => value,
        Err(_) => Src1ewReport {
            plan: observation.plan,
            checks: observation.checks,
            stages: vec![],
            decision: Src1ewDecision::Fail,
        },
    }
}

fn make_valid_src_1ew_observation() -> Src1ewObservation {
    let now = Utc::now();
    Src1ewObservation {
        plan: Src1ewPlan {
            mode: Src1ewCommandMode::Search,
            query: Some("pikachu".to_string()),
            limit: 20,
            offset: 0,
            base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
        },
        checks: vec![
            Src1ewCheckObservation {
                check: Src1ewCheckName::EndpointContract,
                success: true,
                diagnostics: "endpoint ok".to_string(),
                timestamp: now,
            },
            Src1ewCheckObservation {
                check: Src1ewCheckName::InputContract,
                success: true,
                diagnostics: "input ok".to_string(),
                timestamp: now + Duration::milliseconds(1),
            },
        ],
    }
}

#[test]
fn build_src_1ew_plan_rejects_empty_query_for_get() {
    let result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "get-pokemon".to_string(),
        query: "   ".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });

    assert_eq!(result, Err(Src1ewError::EmptyField("query")));
}

#[test]
fn build_src_1ew_plan_rejects_oversized_query() {
    let result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "q".repeat(MAX_SRC_1EW_QUERY_LEN + 1),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });

    assert_eq!(result, Err(Src1ewError::FieldTooLong("query", MAX_SRC_1EW_QUERY_LEN)));
}

#[test]
fn build_src_1ew_plan_rejects_control_characters() {
    let result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pik\u{0007}achu".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });

    assert_eq!(result, Err(Src1ewError::InvalidFieldContent("query")));
}

#[test]
fn start_src_1ew_runtime_rejects_non_contract_base_url() {
    let plan_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "get-pokemon".to_string(),
        query: "pikachu".to_string(),
        limit: 20,
        offset: 0,
        base_url: "https://example.com/api/v2".to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let result = start_src_1ew_runtime(&plan);
    assert_eq!(result, Err(Src1ewError::InvalidEndpoint("base_url_contract")));
}

#[test]
fn capture_src_1ew_observation_emits_ordered_checks_and_timestamps() {
    let plan_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika chu".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let runtime_result = start_src_1ew_runtime(&plan);
    assert!(runtime_result.is_ok());
    let runtime = match runtime_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let observation_result = capture_src_1ew_observation(&runtime);
    assert!(observation_result.is_ok());
    let observation = match observation_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(observation.checks.len(), 2);
    assert_eq!(observation.checks[0].check, Src1ewCheckName::EndpointContract);
    assert_eq!(observation.checks[1].check, Src1ewCheckName::InputContract);
    assert!(observation.checks[0].timestamp <= observation.checks[1].timestamp);
    assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
}

#[test]
fn validate_src_1ew_report_rejects_invalid_stage_order() {
    let mut report = make_valid_src_1ew_report();
    assert_eq!(report.stages.len(), 4);
    report.stages.swap(0, 1);

    let result = validate_src_1ew_report(&report);
    assert_eq!(result, Err(Src1ewError::InvalidReport("invalid stage order")));
}

#[test]
fn validate_src_1ew_report_rejects_non_monotonic_stage_timestamps() {
    let mut report = make_valid_src_1ew_report();
    assert_eq!(report.stages.len(), 4);
    let first_timestamp = report.stages[0].timestamp;
    report.stages[1].timestamp = first_timestamp - Duration::milliseconds(1);

    let result = validate_src_1ew_report(&report);
    assert_eq!(result, Err(Src1ewError::InvalidReport("non-monotonic stage timestamps")));
}

#[test]
fn evaluate_src_1ew_observation_derives_fail_when_any_check_fails() {
    let observation = Src1ewObservation {
        plan: Src1ewPlan {
            mode: Src1ewCommandMode::Search,
            query: Some("pika".to_string()),
            limit: 20,
            offset: 0,
            base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
        },
        checks: vec![
            Src1ewCheckObservation {
                check: Src1ewCheckName::EndpointContract,
                success: true,
                diagnostics: "endpoint ok".to_string(),
                timestamp: Utc::now(),
            },
            Src1ewCheckObservation {
                check: Src1ewCheckName::InputContract,
                success: false,
                diagnostics: "input bad".to_string(),
                timestamp: Utc::now() + Duration::milliseconds(1),
            },
        ],
    };

    let result = evaluate_src_1ew_observation(&observation);
    assert!(result.is_ok());
    let report = match result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(report.decision, Src1ewDecision::Fail);
}

#[test]
fn build_src_1ew_plan_supports_mode_aliases_and_query_normalization() {
    let get_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "get".to_string(),
        query: "  PIKACHU  ".to_string(),
        limit: 1,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert!(get_result.is_ok());
    let get_plan = match get_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(get_plan.mode, Src1ewCommandMode::GetPokemon);
    assert_eq!(get_plan.query, Some("pikachu".to_string()));

    let list_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "list".to_string(),
        query: "   ".to_string(),
        limit: MAX_SRC_1EW_LIMIT,
        offset: MAX_SRC_1EW_OFFSET,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert!(list_result.is_ok());
    let list_plan = match list_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(list_plan.mode, Src1ewCommandMode::ListPokemon);
    assert_eq!(list_plan.query, None);

    let search_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "  Pika    Chu  ".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert!(search_result.is_ok());
    let search_plan = match search_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(search_plan.query, Some("pika chu".to_string()));
}

#[test]
fn build_src_1ew_plan_rejects_invalid_modes_and_list_query() {
    let invalid_mode_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "unknown".to_string(),
        query: "pikachu".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert_eq!(invalid_mode_result, Err(Src1ewError::InvalidFieldFormat("command_mode")));

    let list_query_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "list-pokemon".to_string(),
        query: "pikachu".to_string(),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert_eq!(list_query_result, Err(Src1ewError::InvalidFieldFormat("query")));
}

#[test]
fn build_src_1ew_plan_rejects_limit_offset_and_base_url_errors() {
    let limit_zero_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: 0,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert_eq!(limit_zero_result, Err(Src1ewError::InvalidFieldFormat("limit")));

    let limit_oversized_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: MAX_SRC_1EW_LIMIT + 1,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert_eq!(limit_oversized_result, Err(Src1ewError::InvalidFieldFormat("limit")));

    let offset_oversized_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: 20,
        offset: MAX_SRC_1EW_OFFSET + 1,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    });
    assert_eq!(offset_oversized_result, Err(Src1ewError::InvalidFieldFormat("offset")));

    let empty_base_url_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: 20,
        offset: 0,
        base_url: "   ".to_string(),
    });
    assert_eq!(empty_base_url_result, Err(Src1ewError::EmptyField("base_url")));

    let control_char_base_url_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: 20,
        offset: 0,
        base_url: "https://pokeapi.co/api/v2\u{0007}".to_string(),
    });
    assert_eq!(control_char_base_url_result, Err(Src1ewError::InvalidFieldContent("base_url")));

    let invalid_scheme_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: 20,
        offset: 0,
        base_url: "ftp://pokeapi.co/api/v2".to_string(),
    });
    assert_eq!(invalid_scheme_result, Err(Src1ewError::InvalidEndpoint("base_url")));

    let credentialed_url_result = build_src_1ew_plan(&Src1ewInput {
        command_mode: "search".to_string(),
        query: "pika".to_string(),
        limit: 20,
        offset: 0,
        base_url: "https://user:secret@pokeapi.co/api/v2".to_string(),
    });
    assert_eq!(credentialed_url_result, Err(Src1ewError::InvalidEndpoint("base_url")));
}

#[test]
fn start_src_1ew_runtime_rejects_query_contract_violations() {
    let list_with_query = Src1ewPlan {
        mode: Src1ewCommandMode::ListPokemon,
        query: Some("pikachu".to_string()),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    };
    assert_eq!(
        start_src_1ew_runtime(&list_with_query),
        Err(Src1ewError::InvalidFieldFormat("query"))
    );

    let search_without_query = Src1ewPlan {
        mode: Src1ewCommandMode::Search,
        query: None,
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    };
    assert_eq!(start_src_1ew_runtime(&search_without_query), Err(Src1ewError::EmptyField("query")));

    let get_with_unsafe_identifier = Src1ewPlan {
        mode: Src1ewCommandMode::GetPokemon,
        query: Some("../../etc/passwd".to_string()),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    };
    assert_eq!(
        start_src_1ew_runtime(&get_with_unsafe_identifier),
        Err(Src1ewError::InvalidFieldFormat("query"))
    );

    let search_with_non_canonical_query = Src1ewPlan {
        mode: Src1ewCommandMode::Search,
        query: Some("  Pika    Chu  ".to_string()),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
    };
    assert_eq!(
        start_src_1ew_runtime(&search_with_non_canonical_query),
        Err(Src1ewError::InvalidFieldFormat("query"))
    );
}

#[test]
fn validate_src_1ew_report_rejects_non_contract_base_url() {
    let mut report = make_valid_src_1ew_report();
    report.plan.base_url = "https://example.com/api/v2".to_string();

    let result = validate_src_1ew_report(&report);
    assert_eq!(result, Err(Src1ewError::InvalidEndpoint("base_url_contract")));
}

#[test]
fn validate_src_1ew_report_rejects_non_canonical_query_in_plan() {
    let mut report = make_valid_src_1ew_report();
    report.plan.mode = Src1ewCommandMode::Search;
    report.plan.query = Some("Pika    Chu".to_string());

    let result = validate_src_1ew_report(&report);
    assert_eq!(result, Err(Src1ewError::InvalidFieldFormat("query")));
}

#[test]
fn capture_src_1ew_observation_rejects_unready_runtime() {
    let handle = Src1ewRuntimeHandle {
        mode: Src1ewCommandMode::Search,
        query: Some("pikachu".to_string()),
        limit: 20,
        offset: 0,
        base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
        started_at: Utc::now(),
        runtime_ready: false,
    };

    assert_eq!(capture_src_1ew_observation(&handle), Err(Src1ewError::RuntimeNotReady));
}

#[test]
fn evaluate_src_1ew_observation_rejects_invalid_check_shapes() {
    let now = Utc::now();
    let missing_input = Src1ewObservation {
        plan: Src1ewPlan {
            mode: Src1ewCommandMode::GetPokemon,
            query: Some("pikachu".to_string()),
            limit: 20,
            offset: 0,
            base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
        },
        checks: vec![Src1ewCheckObservation {
            check: Src1ewCheckName::EndpointContract,
            success: true,
            diagnostics: "ok".to_string(),
            timestamp: now,
        }],
    };
    assert_eq!(
        evaluate_src_1ew_observation(&missing_input),
        Err(Src1ewError::InvalidReport("invalid check count"))
    );

    let duplicate_input = Src1ewObservation {
        plan: Src1ewPlan {
            mode: Src1ewCommandMode::GetPokemon,
            query: Some("pikachu".to_string()),
            limit: 20,
            offset: 0,
            base_url: DEFAULT_SRC_1EW_BASE_URL.to_string(),
        },
        checks: vec![
            Src1ewCheckObservation {
                check: Src1ewCheckName::EndpointContract,
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            Src1ewCheckObservation {
                check: Src1ewCheckName::EndpointContract,
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now + Duration::milliseconds(1),
            },
        ],
    };
    assert_eq!(
        evaluate_src_1ew_observation(&duplicate_input),
        Err(Src1ewError::MissingCheck("endpoint_contract"))
    );
}

#[test]
fn evaluate_src_1ew_result_matches_observation_evaluation() {
    let observation = make_valid_src_1ew_observation();

    let via_alias = evaluate_src_1ew_result(&observation);
    let direct = evaluate_src_1ew_observation(&observation);

    assert!(via_alias.is_ok());
    assert!(direct.is_ok());

    let via_alias_report = match via_alias {
        Ok(value) => value,
        Err(_) => return,
    };
    let direct_report = match direct {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(via_alias_report.plan, direct_report.plan);
    assert_eq!(via_alias_report.checks, direct_report.checks);
    assert_eq!(via_alias_report.decision, direct_report.decision);
    assert_eq!(via_alias_report.stages.len(), direct_report.stages.len());

    let via_alias_stage_shape = via_alias_report
        .stages
        .iter()
        .map(|stage| (stage.stage.clone(), stage.status.clone(), stage.diagnostics.clone()))
        .collect::<Vec<_>>();
    let direct_stage_shape = direct_report
        .stages
        .iter()
        .map(|stage| (stage.stage.clone(), stage.status.clone(), stage.diagnostics.clone()))
        .collect::<Vec<_>>();
    assert_eq!(via_alias_stage_shape, direct_stage_shape);
}

#[test]
fn validate_src_1ew_report_rejects_check_and_stage_diagnostics_errors() {
    let mut empty_check_diagnostics_report = make_valid_src_1ew_report();
    empty_check_diagnostics_report.checks[0].diagnostics = "  ".to_string();
    assert_eq!(
        validate_src_1ew_report(&empty_check_diagnostics_report),
        Err(Src1ewError::InvalidReport("empty check diagnostics"))
    );

    let mut oversized_check_diagnostics_report = make_valid_src_1ew_report();
    oversized_check_diagnostics_report.checks[0].diagnostics =
        "d".repeat(MAX_SRC_1EW_DIAGNOSTICS_LEN + 1);
    assert_eq!(
        validate_src_1ew_report(&oversized_check_diagnostics_report),
        Err(Src1ewError::InvalidReport("check diagnostics exceed max length"))
    );

    let mut invalid_check_diagnostics_report = make_valid_src_1ew_report();
    invalid_check_diagnostics_report.checks[0].diagnostics = "ok\u{0007}".to_string();
    assert_eq!(
        validate_src_1ew_report(&invalid_check_diagnostics_report),
        Err(Src1ewError::InvalidReport("check diagnostics contain invalid control characters"))
    );

    let mut empty_stage_diagnostics_report = make_valid_src_1ew_report();
    empty_stage_diagnostics_report.stages[1].diagnostics = "  ".to_string();
    assert_eq!(
        validate_src_1ew_report(&empty_stage_diagnostics_report),
        Err(Src1ewError::InvalidReport("empty stage diagnostics"))
    );

    let mut oversized_stage_diagnostics_report = make_valid_src_1ew_report();
    oversized_stage_diagnostics_report.stages[1].diagnostics =
        "d".repeat(MAX_SRC_1EW_DIAGNOSTICS_LEN + 1);
    assert_eq!(
        validate_src_1ew_report(&oversized_stage_diagnostics_report),
        Err(Src1ewError::InvalidReport("stage diagnostics exceed max length"))
    );

    let mut invalid_stage_diagnostics_report = make_valid_src_1ew_report();
    invalid_stage_diagnostics_report.stages[1].diagnostics = "ok\u{0007}".to_string();
    assert_eq!(
        validate_src_1ew_report(&invalid_stage_diagnostics_report),
        Err(Src1ewError::InvalidReport("stage diagnostics contain invalid control characters"))
    );
}

#[test]
fn validate_src_1ew_report_rejects_check_timestamps_and_decision_mismatches() {
    let mut non_monotonic_checks = make_valid_src_1ew_report();
    non_monotonic_checks.checks[1].timestamp =
        non_monotonic_checks.checks[0].timestamp - Duration::milliseconds(1);
    assert_eq!(
        validate_src_1ew_report(&non_monotonic_checks),
        Err(Src1ewError::InvalidReport("non-monotonic check timestamps"))
    );

    let mut decision_mismatch = make_valid_src_1ew_report();
    decision_mismatch.decision = Src1ewDecision::Fail;
    assert_eq!(
        validate_src_1ew_report(&decision_mismatch),
        Err(Src1ewError::InvalidReport("decision mismatch"))
    );

    let mut final_stage_status_mismatch = make_valid_src_1ew_report();
    final_stage_status_mismatch.stages[3].status = Src1ewStageStatus::Failed;
    assert_eq!(
        validate_src_1ew_report(&final_stage_status_mismatch),
        Err(Src1ewError::InvalidReport("final decision stage mismatch"))
    );
}

#[test]
fn validate_src_1ew_report_rejects_invalid_check_and_stage_counts() {
    let mut invalid_checks = make_valid_src_1ew_report();
    invalid_checks.checks.remove(0);
    assert_eq!(
        validate_src_1ew_report(&invalid_checks),
        Err(Src1ewError::InvalidReport("invalid check count"))
    );

    let mut invalid_stages = make_valid_src_1ew_report();
    invalid_stages.stages.pop();
    assert_eq!(
        validate_src_1ew_report(&invalid_stages),
        Err(Src1ewError::InvalidReport("unexpected stage count"))
    );
}

#[test]
fn build_bead_cupid_plan_rejects_empty_run_id() {
    let result = build_bead_cupid_plan(&BeadCupidInput {
        run_id: "   ".to_string(),
        bead_id: "bead-cupid-001".to_string(),
    });
    assert_eq!(result, Err(BeadCupidError::EmptyField("run_id")));
}

#[test]
fn build_bead_cupid_plan_normalizes_ids_and_sets_default_contract() {
    let result = build_bead_cupid_plan(&BeadCupidInput {
        run_id: "  run-cupid-001  ".to_string(),
        bead_id: "  bead-cupid-001  ".to_string(),
    });
    assert!(result.is_ok());
    let plan = match result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(plan.run_id, "run-cupid-001");
    assert_eq!(plan.bead_id, "bead-cupid-001");
    assert_eq!(plan.runtime_command, DEFAULT_BEAD_CUPID_RUNTIME_COMMAND);
    assert_eq!(plan.ingress_health_url, DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL);
    assert_eq!(plan.orchestrator_status_url, "http://localhost:8080/Oya/run-cupid-001/get_status");
}

#[test]
fn start_bead_cupid_runtime_rejects_non_default_runtime_command() {
    let plan = BeadCupidPlan {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: "scripts/other.sh".to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
    };

    let result = start_bead_cupid_runtime(&plan);
    assert_eq!(result, Err(BeadCupidError::InvalidRuntimeCommand));
}

#[test]
fn capture_bead_cupid_observation_emits_required_checks_once() {
    let plan_result = build_bead_cupid_plan(&BeadCupidInput {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let runtime_result = start_bead_cupid_runtime(&plan);
    assert!(runtime_result.is_ok());
    let runtime = match runtime_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let observation_result = capture_bead_cupid_observation(&runtime);
    assert!(observation_result.is_ok());
    let observation = match observation_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(observation.checks.len(), 2);

    let ingress_count = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadCupidCheckName::IngressHealth)
        .count();
    let orchestrator_count = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadCupidCheckName::OrchestratorStatus)
        .count();

    assert_eq!(ingress_count, 1);
    assert_eq!(orchestrator_count, 1);
    assert!(observation.checks[0].timestamp <= observation.checks[1].timestamp);
    assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
}

#[test]
fn evaluate_bead_cupid_result_preserves_stage_order_and_decision() {
    let report = make_valid_bead_cupid_report();
    assert_eq!(
        report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<BeadCupidStageName>>(),
        vec![
            BeadCupidStageName::IngressHealth,
            BeadCupidStageName::OrchestratorStatus,
            BeadCupidStageName::FinalDecision,
        ]
    );
    assert_eq!(report.decision, BeadCupidDecision::Pass);
}

#[test]
fn validate_bead_cupid_report_rejects_decision_mismatch() {
    let valid_report = make_valid_bead_cupid_report();
    let invalid_report = BeadCupidReport { decision: BeadCupidDecision::Fail, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("decision mismatch")));
}

#[test]
fn build_bead_cupid_plan_accepts_max_length_identifiers() {
    let result = build_bead_cupid_plan(&BeadCupidInput {
        run_id: "r".repeat(MAX_BEAD_CUPID_RUN_ID_LEN),
        bead_id: "b".repeat(MAX_BEAD_CUPID_BEAD_ID_LEN),
    });
    assert!(result.is_ok());
}

#[test]
fn build_bead_cupid_plan_rejects_invalid_bead_id_characters() {
    let result = build_bead_cupid_plan(&BeadCupidInput {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead/cupid".to_string(),
    });
    assert_eq!(result, Err(BeadCupidError::InvalidIdentifier("bead_id")));
}

#[test]
fn start_bead_cupid_runtime_rejects_non_normalized_run_id() {
    let plan = BeadCupidPlan {
        run_id: " run-cupid-001 ".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
    };

    let result = start_bead_cupid_runtime(&plan);
    assert_eq!(result, Err(BeadCupidError::InvalidFieldContent("run_id")));
}

#[test]
fn start_bead_cupid_runtime_rejects_invalid_ingress_endpoint() {
    let plan = BeadCupidPlan {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "localhost:8080/restate/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
    };

    let result = start_bead_cupid_runtime(&plan);
    assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("ingress_health_url")));
}

#[test]
fn start_bead_cupid_runtime_rejects_non_contract_orchestrator_endpoint() {
    let plan = BeadCupidPlan {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/status".to_string(),
    };

    let result = start_bead_cupid_runtime(&plan);
    assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn capture_bead_cupid_observation_rejects_runtime_not_ready() {
    let handle = BeadCupidRuntimeHandle {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: false,
    };

    let result = capture_bead_cupid_observation(&handle);
    assert_eq!(result, Err(BeadCupidError::RuntimeNotReady));
}

#[test]
fn capture_bead_cupid_observation_rejects_non_contract_ingress_endpoint() {
    let handle = BeadCupidRuntimeHandle {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = capture_bead_cupid_observation(&handle);
    assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("ingress_health_url")));
}

#[test]
fn capture_bead_cupid_observation_rejects_non_contract_orchestrator_endpoint() {
    let handle = BeadCupidRuntimeHandle {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = capture_bead_cupid_observation(&handle);
    assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn evaluate_bead_cupid_result_rejects_missing_ingress_check() {
    let base = Utc::now();
    let observation = BeadCupidObservation {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
        checks: vec![BeadCupidCheckObservation {
            check: BeadCupidCheckName::OrchestratorStatus,
            endpoint: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
            success: true,
            diagnostics: "orchestrator status check passed".to_string(),
            timestamp: base,
        }],
    };

    let result = evaluate_bead_cupid_result(&observation);
    assert_eq!(result, Err(BeadCupidError::MissingCheck("ingress_health")));
}

#[test]
fn evaluate_bead_cupid_result_rejects_duplicate_orchestrator_checks() {
    let base = Utc::now();
    let observation = BeadCupidObservation {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
        checks: vec![
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
                success: true,
                diagnostics: "duplicate orchestrator status check".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
        ],
    };

    let result = evaluate_bead_cupid_result(&observation);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("duplicate orchestrator_status checks")));
}

#[test]
fn evaluate_bead_cupid_result_sets_fail_when_any_check_fails() {
    let base = Utc::now();
    let observation = BeadCupidObservation {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
        checks: vec![
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                success: false,
                diagnostics: "ingress health check failed".to_string(),
                timestamp: base,
            },
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
        ],
    };

    let result = evaluate_bead_cupid_result(&observation);
    assert!(result.is_ok());
    let report = match result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(report.decision, BeadCupidDecision::Fail);
    assert_eq!(report.stages[2].status, BeadCupidStageStatus::Failed);
}

#[test]
fn evaluate_bead_cupid_result_normalizes_orchestrator_stage_timestamp_floor() {
    let base = Utc::now();
    let observation = BeadCupidObservation {
        run_id: "run-cupid-001".to_string(),
        bead_id: "bead-cupid-001".to_string(),
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
        checks: vec![
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base,
            },
            BeadCupidCheckObservation {
                check: BeadCupidCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-cupid-001/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base - Duration::milliseconds(1),
            },
        ],
    };

    let result = evaluate_bead_cupid_result(&observation);
    assert!(result.is_ok());
    let report = match result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(report.stages[1].timestamp, report.stages[0].timestamp);
    assert_eq!(report.stages[2].timestamp, report.stages[1].timestamp + Duration::milliseconds(1));
}

#[test]
fn validate_bead_cupid_report_rejects_check_endpoint_mismatch() {
    let valid_report = make_valid_bead_cupid_report();
    let mut checks = valid_report.checks.clone();
    checks[0].endpoint = "http://localhost:8080/restate/not-health".to_string();
    let invalid_report = BeadCupidReport { checks, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("check endpoint mismatch")));
}

#[test]
fn validate_bead_cupid_report_rejects_invalid_check_diagnostics() {
    let valid_report = make_valid_bead_cupid_report();
    let mut checks = valid_report.checks.clone();
    checks[0].diagnostics = "\u{0007}".to_string();
    let invalid_report = BeadCupidReport { checks, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("invalid check diagnostics")));
}

#[test]
fn validate_bead_cupid_report_rejects_invalid_stage_order() {
    let valid_report = make_valid_bead_cupid_report();
    let stages = vec![
        valid_report.stages[1].clone(),
        valid_report.stages[0].clone(),
        valid_report.stages[2].clone(),
    ];
    let invalid_report = BeadCupidReport { stages, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("invalid stage order")));
}

#[test]
fn validate_bead_cupid_report_rejects_non_monotonic_stage_timestamps() {
    let valid_report = make_valid_bead_cupid_report();
    let mut stages = valid_report.stages.clone();
    stages[1].timestamp = stages[0].timestamp - Duration::milliseconds(1);
    let invalid_report = BeadCupidReport { stages, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("non-monotonic stage timestamps")));
}

#[test]
fn validate_bead_cupid_report_rejects_final_decision_stage_mismatch() {
    let valid_report = make_valid_bead_cupid_report();
    let mut stages = valid_report.stages.clone();
    stages[2].status = BeadCupidStageStatus::Failed;
    let invalid_report = BeadCupidReport { stages, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("final decision stage mismatch")));
}

#[test]
fn validate_bead_cupid_report_rejects_ingress_stage_diagnostics_mismatch() {
    let valid_report = make_valid_bead_cupid_report();
    let mut stages = valid_report.stages.clone();
    stages[0].diagnostics = "tampered ingress message".to_string();
    let invalid_report = BeadCupidReport { stages, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("ingress diagnostics mismatch")));
}

#[test]
fn validate_bead_cupid_report_rejects_orchestrator_stage_diagnostics_mismatch() {
    let valid_report = make_valid_bead_cupid_report();
    let mut stages = valid_report.stages.clone();
    stages[1].diagnostics = "tampered orchestrator message".to_string();
    let invalid_report = BeadCupidReport { stages, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("orchestrator diagnostics mismatch")));
}

#[test]
fn validate_bead_cupid_report_rejects_final_stage_diagnostics_mismatch() {
    let valid_report = make_valid_bead_cupid_report();
    let mut stages = valid_report.stages.clone();
    stages[2].diagnostics = "tampered final message".to_string();
    let invalid_report = BeadCupidReport { stages, ..valid_report };

    let result = validate_bead_cupid_report(&invalid_report);
    assert_eq!(result, Err(BeadCupidError::InvalidReport("final diagnostics mismatch")));
}

#[test]
fn build_smoke_plan_rejects_empty_run_id() {
    let result = build_smoke_plan(&SmokeInput { run_id: "   ".to_string() });
    assert_eq!(result, Err(SmokeError::EmptyField("run_id")));
}

#[test]
fn build_smoke_plan_sets_docker_default_endpoints() {
    let result = build_smoke_plan(&SmokeInput { run_id: "run-001".to_string() });
    assert!(result.is_ok());

    let plan = match result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(plan.runtime_command, DEFAULT_SMOKE_RUNTIME_COMMAND);
    assert_eq!(plan.ingress_health_url, DEFAULT_SMOKE_INGRESS_HEALTH_URL);
    assert_eq!(plan.orchestrator_status_url, "http://localhost:8080/Oya/run-001/get_status");
}

#[test]
fn build_smoke_plan_trims_run_id_and_accepts_max_boundary_length() {
    let trimmed_run_id = "run_boundary-01";
    let padded_input = format!("  {}  ", trimmed_run_id);
    let max_boundary_run_id = "r".repeat(MAX_SMOKE_RUN_ID_LEN);

    let trimmed_result = build_smoke_plan(&SmokeInput { run_id: padded_input });
    assert!(trimmed_result.is_ok());
    let trimmed_plan = match trimmed_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(trimmed_plan.run_id, trimmed_run_id);

    let max_boundary_result = build_smoke_plan(&SmokeInput { run_id: max_boundary_run_id });
    assert!(max_boundary_result.is_ok());
}

#[test]
fn build_smoke_plan_rejects_control_characters_in_run_id() {
    let result = build_smoke_plan(&SmokeInput { run_id: "run-001\u{0007}".to_string() });
    assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
}

#[test]
fn build_smoke_plan_rejects_oversized_run_id() {
    let result = build_smoke_plan(&SmokeInput { run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1) });
    assert_eq!(result, Err(SmokeError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN)));
}

#[test]
fn build_smoke_plan_rejects_path_and_query_injection_run_id() {
    let path_injection = build_smoke_plan(&SmokeInput { run_id: "../run-001".to_string() });
    assert_eq!(path_injection, Err(SmokeError::InvalidFieldContent("run_id")));

    let query_injection = build_smoke_plan(&SmokeInput { run_id: "run-001?x=1".to_string() });
    assert_eq!(query_injection, Err(SmokeError::InvalidFieldContent("run_id")));
}

#[test]
fn start_docker_default_runtime_rejects_invalid_runtime_command() {
    let plan = SmokePlan {
        run_id: "run-001".to_string(),
        runtime_command: "scripts/not-default.sh".to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidRuntimeCommand));
}

#[test]
fn start_docker_default_runtime_rejects_invalid_run_id_in_plan() {
    let plan = SmokePlan {
        run_id: " run-001 ".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
}

#[test]
fn start_docker_default_runtime_rejects_empty_run_id_in_plan() {
    let plan = SmokePlan {
        run_id: "".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::EmptyField("run_id")));
}

#[test]
fn start_docker_default_runtime_rejects_invalid_ingress_endpoint() {
    let plan = SmokePlan {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "localhost:8080/restate/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
}

#[test]
fn start_docker_default_runtime_rejects_non_default_ingress_contract() {
    let plan = SmokePlan {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
}

#[test]
fn start_docker_default_runtime_starts_with_valid_default_contract() {
    let plan_result = build_smoke_plan(&SmokeInput { run_id: "run-001".to_string() });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let handle_result = start_docker_default_runtime(&plan);
    assert!(handle_result.is_ok());
    let handle = match handle_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert!(handle.runtime_ready);
    assert_eq!(handle.run_id, "run-001");
    assert_eq!(handle.runtime_command, DEFAULT_SMOKE_RUNTIME_COMMAND);
    assert_eq!(handle.ingress_health_url, DEFAULT_SMOKE_INGRESS_HEALTH_URL);
    assert_eq!(handle.orchestrator_status_url, "http://localhost:8080/Oya/run-001/get_status");
}

#[test]
fn start_docker_default_runtime_rejects_invalid_orchestrator_endpoint() {
    let plan = SmokePlan {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "  ".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn start_docker_default_runtime_rejects_orchestrator_endpoint_with_credentials() {
    let plan = SmokePlan {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://user:secret@localhost:8080/Oya/run-001/get_status"
            .to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn start_docker_default_runtime_rejects_orchestrator_contract_mismatch() {
    let plan = SmokePlan {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-xyz/get_status".to_string(),
    };

    let result = start_docker_default_runtime(&plan);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn run_default_smoke_checks_rejects_unready_runtime() {
    let handle = RuntimeHandle {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: false,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::RuntimeNotReady));
}

#[test]
fn run_default_smoke_checks_rejects_invalid_runtime_command() {
    let handle = RuntimeHandle {
        run_id: "run-001".to_string(),
        runtime_command: "scripts/other.sh".to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::InvalidRuntimeCommand));
}

#[test]
fn run_default_smoke_checks_rejects_invalid_ingress_endpoint() {
    let handle = RuntimeHandle {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "restate/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
}

#[test]
fn run_default_smoke_checks_rejects_non_default_ingress_contract() {
    let handle = RuntimeHandle {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
}

#[test]
fn run_default_smoke_checks_rejects_invalid_orchestrator_endpoint() {
    let handle = RuntimeHandle {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "https://localhost:8080\u{0007}".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn run_default_smoke_checks_rejects_orchestrator_contract_mismatch() {
    let handle = RuntimeHandle {
        run_id: "run-001".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/other/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
}

#[test]
fn run_default_smoke_checks_rejects_invalid_run_id_in_handle() {
    let handle = RuntimeHandle {
        run_id: " run-001 ".to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-001/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };

    let result = run_default_smoke_checks(&handle);
    assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
}

#[test]
fn smoke_pipeline_passes_for_valid_default_input() {
    let plan_result = build_smoke_plan(&SmokeInput { run_id: "run-002".to_string() });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let handle_result = start_docker_default_runtime(&plan);
    assert!(handle_result.is_ok());
    let handle = match handle_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let observation_result = run_default_smoke_checks(&handle);
    assert!(observation_result.is_ok());
    let observation = match observation_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let report_result = evaluate_smoke_result(&observation);
    assert!(report_result.is_ok());
    let report = match report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(report.decision, SmokeDecision::Pass);
    assert_eq!(report.stages.len(), 3);
    assert_eq!(validate_smoke_report(&report), Ok(()));
}

#[test]
fn evaluate_smoke_result_fails_when_orchestrator_check_fails() {
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress ok".to_string(),
                timestamp: Utc::now(),
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
                success: false,
                diagnostics: "orchestrator timeout".to_string(),
                timestamp: Utc::now(),
            },
        ],
    };

    let report_result = evaluate_smoke_result(&observation);
    assert!(report_result.is_ok());
    let report = match report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(report.decision, SmokeDecision::Fail);
}

#[test]
fn evaluate_smoke_result_fails_when_ingress_check_fails() {
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: false,
                diagnostics: "ingress unavailable".to_string(),
                timestamp: Utc::now(),
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: Utc::now(),
            },
        ],
    };

    let report_result = evaluate_smoke_result(&observation);
    assert!(report_result.is_ok());
    let report = match report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(report.decision, SmokeDecision::Fail);
}

#[test]
fn evaluate_smoke_result_rejects_missing_ingress_check() {
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![SmokeCheckObservation {
            check: SmokeCheckName::OrchestratorStatus,
            endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
            success: true,
            diagnostics: "ok".to_string(),
            timestamp: Utc::now(),
        }],
    };

    let result = evaluate_smoke_result(&observation);
    assert_eq!(result, Err(SmokeError::MissingCheck("ingress_health")));
}

#[test]
fn evaluate_smoke_result_rejects_missing_orchestrator_check() {
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![SmokeCheckObservation {
            check: SmokeCheckName::IngressHealth,
            endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            success: true,
            diagnostics: "ok".to_string(),
            timestamp: Utc::now(),
        }],
    };

    let result = evaluate_smoke_result(&observation);
    assert_eq!(result, Err(SmokeError::MissingCheck("orchestrator_status")));
}

#[test]
fn evaluate_smoke_result_rejects_duplicate_ingress_checks() {
    let now = Utc::now();
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
        ],
    };

    let result = evaluate_smoke_result(&observation);
    assert_eq!(result, Err(SmokeError::InvalidReport("duplicate ingress_health checks")));
}

#[test]
fn evaluate_smoke_result_rejects_duplicate_orchestrator_checks() {
    let now = Utc::now();
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
        ],
    };

    let result = evaluate_smoke_result(&observation);
    assert_eq!(result, Err(SmokeError::InvalidReport("duplicate orchestrator_status checks")));
}

#[test]
fn evaluate_smoke_result_rejects_empty_diagnostics_from_observation_checks() {
    let observation = SmokeObservation {
        run_id: "run-003".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "".to_string(),
                timestamp: Utc::now(),
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-003/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            },
        ],
    };

    let result = evaluate_smoke_result(&observation);
    assert_eq!(result, Err(SmokeError::InvalidReport("empty check diagnostics")));
}

#[test]
fn validate_smoke_report_rejects_unexpected_stage_count() {
    let base = Utc::now();
    let report = SmokeReport {
        run_id: "run-005".to_string(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: base,
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-005/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: base,
            },
        ],
        stages: vec![SmokeStageReport {
            stage: SmokeStageName::IngressHealth,
            status: SmokeStageStatus::Passed,
            diagnostics: "ok".to_string(),
            timestamp: Utc::now(),
        }],
        decision: SmokeDecision::Pass,
    };

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("unexpected stage count")));
}

#[test]
fn validate_smoke_report_rejects_invalid_orchestrator_endpoint_in_checks() {
    let mut report = make_valid_smoke_report();
    report.checks[1].endpoint = "http://localhost:8080/Oya/other/get_status".to_string();

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("invalid orchestrator check endpoint")));
}

#[test]
fn validate_smoke_report_rejects_invalid_ingress_endpoint_in_checks() {
    let mut report = make_valid_smoke_report();
    report.checks[0].endpoint = "http://localhost:8080/health".to_string();

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("invalid ingress check endpoint")));
}

#[test]
fn validate_smoke_report_rejects_missing_ingress_check() {
    let mut report = make_valid_smoke_report();
    report.checks[0].check = SmokeCheckName::OrchestratorStatus;

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::MissingCheck("ingress_health")));
}

#[test]
fn validate_smoke_report_rejects_missing_orchestrator_check() {
    let mut report = make_valid_smoke_report();
    report.checks.truncate(1);

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::MissingCheck("orchestrator_status")));
}

#[test]
fn validate_smoke_report_rejects_invalid_control_characters_in_check_diagnostics() {
    let mut report = make_valid_smoke_report();
    report.checks[0].diagnostics = "ok\u{0007}".to_string();

    let result = validate_smoke_report(&report);
    assert_eq!(
        result,
        Err(SmokeError::InvalidReport("check diagnostics contain invalid control characters"))
    );
}

#[test]
fn validate_smoke_report_rejects_invalid_run_id() {
    let mut report = make_valid_smoke_report();
    report.run_id = " run-test ".to_string();

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
}

#[test]
fn validate_smoke_report_rejects_invalid_stage_order() {
    let mut report = make_valid_smoke_report();
    report.stages =
        vec![report.stages[1].clone(), report.stages[0].clone(), report.stages[2].clone()];

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("invalid stage order")));
}

#[test]
fn validate_smoke_report_rejects_empty_stage_diagnostics() {
    let mut report = make_valid_smoke_report();
    report.stages[1].diagnostics = "   ".to_string();

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("empty stage diagnostics")));
}

#[test]
fn validate_smoke_report_rejects_non_monotonic_timestamps() {
    let mut report = make_valid_smoke_report();
    report.stages[1].timestamp = report.stages[0].timestamp - Duration::seconds(1);

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("non-monotonic stage timestamps")));
}

#[test]
fn validate_smoke_report_rejects_decision_mismatch() {
    let mut report = make_valid_smoke_report();
    report.stages[2].status = SmokeStageStatus::Failed;

    let result = validate_smoke_report(&report);
    assert_eq!(result, Err(SmokeError::InvalidReport("decision mismatch")));
}

#[test]
fn validate_smoke_report_accepts_equal_consecutive_timestamps() {
    let mut report = make_valid_smoke_report();
    report.stages[1].timestamp = report.stages[0].timestamp;
    report.stages[2].timestamp = report.stages[1].timestamp;

    let result = validate_smoke_report(&report);
    assert_eq!(result, Ok(()));
}

#[test]
fn smoke_decision_is_deterministic_for_same_valid_input() {
    let plan_result = build_smoke_plan(&SmokeInput { run_id: "run-004".to_string() });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let first_report_result = start_docker_default_runtime(&plan)
        .and_then(|handle| run_default_smoke_checks(&handle))
        .and_then(|observation| evaluate_smoke_result(&observation));
    assert!(first_report_result.is_ok());
    let first_report = match first_report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let second_report_result = start_docker_default_runtime(&plan)
        .and_then(|handle| run_default_smoke_checks(&handle))
        .and_then(|observation| evaluate_smoke_result(&observation));
    assert!(second_report_result.is_ok());
    let second_report = match second_report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(first_report.decision, second_report.decision);
    assert_eq!(validate_smoke_report(&first_report), Ok(()));
    assert_eq!(validate_smoke_report(&second_report), Ok(()));
}

#[test]
fn build_smoke_bead_plan_rejects_empty_and_malformed_run_id() {
    let empty = build_smoke_bead_plan(&SmokeBeadInput { run_id: "  ".to_string() });
    assert_eq!(empty, Err(SmokeBeadError::EmptyField("run_id")));

    let malformed = build_smoke_bead_plan(&SmokeBeadInput { run_id: "../run-001".to_string() });
    assert_eq!(malformed, Err(SmokeBeadError::InvalidFieldContent("run_id")));
}

#[test]
fn build_smoke_bead_plan_trims_run_id_and_accepts_max_boundary_length() {
    let trimmed_run_id = "run_smoke-bead_boundary-01";
    let padded_input = format!("  {}  ", trimmed_run_id);
    let max_boundary_run_id = "r".repeat(MAX_SMOKE_RUN_ID_LEN);

    let trimmed_result = build_smoke_bead_plan(&SmokeBeadInput { run_id: padded_input });
    assert!(trimmed_result.is_ok());
    let Ok(trimmed_plan) = trimmed_result else {
        return;
    };
    assert_eq!(trimmed_plan.run_id, trimmed_run_id);

    let max_boundary_result =
        build_smoke_bead_plan(&SmokeBeadInput { run_id: max_boundary_run_id });
    assert!(max_boundary_result.is_ok());
}

#[test]
fn build_smoke_bead_plan_rejects_control_characters_and_oversized_run_id() {
    let control_char_result =
        build_smoke_bead_plan(&SmokeBeadInput { run_id: "run-smoke-01\u{0007}".to_string() });
    assert_eq!(control_char_result, Err(SmokeBeadError::InvalidFieldContent("run_id")));

    let oversized_result =
        build_smoke_bead_plan(&SmokeBeadInput { run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1) });
    assert_eq!(oversized_result, Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN)));
}

#[test]
fn start_smoke_bead_runtime_enforces_default_runtime_contract() {
    let plan = SmokeBeadPlan {
        run_id: "run-smoke-01".to_string(),
        runtime_command: "scripts/not-default.sh".to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-01/get_status".to_string(),
    };

    let result = start_smoke_bead_runtime(&plan);
    assert_eq!(result, Err(SmokeBeadError::InvalidRuntimeCommand));
}

#[test]
fn start_smoke_bead_runtime_rejects_invalid_run_id_and_endpoints() {
    let invalid_run_id_plan = SmokeBeadPlan {
        run_id: " run-smoke-01 ".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-01/get_status".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&invalid_run_id_plan),
        Err(SmokeBeadError::InvalidFieldContent("run_id"))
    );

    let invalid_ingress_endpoint_plan = SmokeBeadPlan {
        run_id: "run-smoke-01".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "localhost:8080/restate/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-01/get_status".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&invalid_ingress_endpoint_plan),
        Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
    );

    let invalid_orchestrator_endpoint_plan = SmokeBeadPlan {
        run_id: "run-smoke-01".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "  ".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&invalid_orchestrator_endpoint_plan),
        Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn start_smoke_bead_runtime_rejects_empty_and_oversized_run_id() {
    let empty_run_id_plan = SmokeBeadPlan {
        run_id: "".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-01/get_status".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&empty_run_id_plan),
        Err(SmokeBeadError::EmptyField("run_id"))
    );

    let oversized_run_id_plan = SmokeBeadPlan {
        run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-01/get_status".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&oversized_run_id_plan),
        Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN))
    );
}

#[test]
fn start_smoke_bead_runtime_rejects_orchestrator_endpoint_with_credentials() {
    let plan = SmokeBeadPlan {
        run_id: "run-smoke-01".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://user:secret@localhost:8080/Oya/run-smoke-01/get_status"
            .to_string(),
    };

    assert_eq!(
        start_smoke_bead_runtime(&plan),
        Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn start_smoke_bead_runtime_rejects_contract_mismatches() {
    let ingress_contract_mismatch_plan = SmokeBeadPlan {
        run_id: "run-smoke-01".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-01/get_status".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&ingress_contract_mismatch_plan),
        Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
    );

    let orchestrator_contract_mismatch_plan = SmokeBeadPlan {
        run_id: "run-smoke-01".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-other/get_status".to_string(),
    };
    assert_eq!(
        start_smoke_bead_runtime(&orchestrator_contract_mismatch_plan),
        Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn capture_smoke_bead_observation_emits_exactly_two_named_checks() {
    let plan_result = build_smoke_bead_plan(&SmokeBeadInput { run_id: "run-smoke-02".into() });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let handle_result = start_smoke_bead_runtime(&plan);
    assert!(handle_result.is_ok());
    let Ok(handle) = handle_result else { return };

    let observation_result = capture_smoke_bead_observation(&handle);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    assert_eq!(observation.checks.len(), 2);

    let ingress_count = observation
        .checks
        .iter()
        .filter(|check| check.check == SmokeBeadCheckName::IngressHealth)
        .count();
    let orchestrator_count = observation
        .checks
        .iter()
        .filter(|check| check.check == SmokeBeadCheckName::OrchestratorStatus)
        .count();

    assert_eq!(ingress_count, 1);
    assert_eq!(orchestrator_count, 1);
    assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
}

#[test]
fn capture_smoke_bead_observation_rejects_runtime_and_endpoint_errors() {
    let unready_handle = SmokeBeadRuntimeHandle {
        run_id: "run-smoke-02".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: false,
    };
    assert_eq!(
        capture_smoke_bead_observation(&unready_handle),
        Err(SmokeBeadError::RuntimeNotReady)
    );

    let invalid_run_id_handle = SmokeBeadRuntimeHandle {
        run_id: " run-smoke-02 ".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&invalid_run_id_handle),
        Err(SmokeBeadError::InvalidFieldContent("run_id"))
    );

    let invalid_orchestrator_contract_handle = SmokeBeadRuntimeHandle {
        run_id: "run-smoke-02".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/other/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&invalid_orchestrator_contract_handle),
        Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn capture_smoke_bead_observation_rejects_invalid_runtime_command_and_ingress() {
    let invalid_runtime_command_handle = SmokeBeadRuntimeHandle {
        run_id: "run-smoke-02".to_string(),
        runtime_command: "scripts/other.sh".to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&invalid_runtime_command_handle),
        Err(SmokeBeadError::InvalidRuntimeCommand)
    );

    let invalid_ingress_endpoint_handle = SmokeBeadRuntimeHandle {
        run_id: "run-smoke-02".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "restate/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&invalid_ingress_endpoint_handle),
        Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
    );

    let ingress_contract_mismatch_handle = SmokeBeadRuntimeHandle {
        run_id: "run-smoke-02".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&ingress_contract_mismatch_handle),
        Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
    );
}

#[test]
fn capture_smoke_bead_observation_rejects_empty_run_id_and_invalid_orchestrator_endpoint() {
    let empty_run_id_handle = SmokeBeadRuntimeHandle {
        run_id: "".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&empty_run_id_handle),
        Err(SmokeBeadError::EmptyField("run_id"))
    );

    let invalid_orchestrator_endpoint_handle = SmokeBeadRuntimeHandle {
        run_id: "run-smoke-02".to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "localhost:8080/Oya/run-smoke-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_smoke_bead_observation(&invalid_orchestrator_endpoint_handle),
        Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn evaluate_smoke_bead_result_uses_deterministic_stage_order_and_decision() {
    let base = Utc::now();
    let observation = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: false,
                diagnostics: "orchestrator timeout".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
        ],
    };

    let report_result = evaluate_smoke_bead_result(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else { return };

    let stage_order = report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>();
    assert_eq!(
        stage_order,
        vec![
            SmokeBeadStageName::IngressHealth,
            SmokeBeadStageName::OrchestratorStatus,
            SmokeBeadStageName::FinalDecision,
        ]
    );
    assert_eq!(report.decision, SmokeBeadDecision::Fail);
}

#[test]
fn evaluate_smoke_bead_result_rejects_missing_and_duplicate_checks() {
    let missing_ingress_observation = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![SmokeBeadCheckObservation {
            check: SmokeBeadCheckName::OrchestratorStatus,
            endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
            success: true,
            diagnostics: "ok".to_string(),
            timestamp: Utc::now(),
        }],
    };
    assert_eq!(
        evaluate_smoke_bead_result(&missing_ingress_observation),
        Err(SmokeBeadError::MissingCheck("ingress_health"))
    );

    let now = Utc::now();
    let duplicate_orchestrator_observation = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
        ],
    };
    assert_eq!(
        evaluate_smoke_bead_result(&duplicate_orchestrator_observation),
        Err(SmokeBeadError::InvalidReport("duplicate orchestrator_status checks"))
    );
}

#[test]
fn evaluate_smoke_bead_result_rejects_invalid_run_id_and_other_check_shapes() {
    let invalid_run_id_observation =
        SmokeBeadObservation { run_id: " run-smoke-03 ".to_string(), checks: vec![] };
    assert_eq!(
        evaluate_smoke_bead_result(&invalid_run_id_observation),
        Err(SmokeBeadError::InvalidFieldContent("run_id"))
    );

    let missing_orchestrator_observation = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![SmokeBeadCheckObservation {
            check: SmokeBeadCheckName::IngressHealth,
            endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            success: true,
            diagnostics: "ok".to_string(),
            timestamp: Utc::now(),
        }],
    };
    assert_eq!(
        evaluate_smoke_bead_result(&missing_orchestrator_observation),
        Err(SmokeBeadError::MissingCheck("orchestrator_status"))
    );

    let now = Utc::now();
    let duplicate_ingress_observation = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: now,
            },
        ],
    };
    assert_eq!(
        evaluate_smoke_bead_result(&duplicate_ingress_observation),
        Err(SmokeBeadError::InvalidReport("duplicate ingress_health checks"))
    );
}

#[test]
fn evaluate_smoke_bead_result_uses_latest_check_timestamp_for_final_stage() {
    let base = Utc::now();
    let ingress_timestamp = base + Duration::milliseconds(5);
    let orchestrator_timestamp = base;

    let observation = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: ingress_timestamp,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: orchestrator_timestamp,
            },
        ],
    };

    let report_result = evaluate_smoke_bead_result(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else { return };

    assert_eq!(report.stages[1].timestamp, ingress_timestamp);
    assert_eq!(report.stages[2].timestamp, ingress_timestamp + Duration::milliseconds(1));
}

#[test]
fn evaluate_smoke_bead_result_rejects_invalid_check_diagnostics() {
    let empty_diagnostics = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: " ".to_string(),
                timestamp: Utc::now(),
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            },
        ],
    };
    assert_eq!(
        evaluate_smoke_bead_result(&empty_diagnostics),
        Err(SmokeBeadError::InvalidReport("empty check diagnostics"))
    );

    let control_char_diagnostics = SmokeBeadObservation {
        run_id: "run-smoke-03".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok\u{0007}".to_string(),
                timestamp: Utc::now(),
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-03/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            },
        ],
    };
    assert_eq!(
        evaluate_smoke_bead_result(&control_char_diagnostics),
        Err(SmokeBeadError::InvalidReport("check diagnostics contain invalid control characters"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_decision_stage_mismatch() {
    let base = Utc::now();
    let report = SmokeBeadReport {
        run_id: "run-smoke-04".to_string(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-smoke-04/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
        ],
        stages: vec![
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::IngressHealth,
                status: SmokeBeadStageStatus::Passed,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            },
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::OrchestratorStatus,
                status: SmokeBeadStageStatus::Passed,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::FinalDecision,
                status: SmokeBeadStageStatus::Failed,
                diagnostics: "mismatch".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
        ],
        decision: SmokeBeadDecision::Pass,
    };

    assert_eq!(
        validate_smoke_bead_report(&report),
        Err(SmokeBeadError::InvalidReport("final decision stage mismatch"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_invalid_stage_count_and_order() {
    let mut invalid_stage_count = make_valid_smoke_bead_report();
    invalid_stage_count.stages.truncate(1);
    assert_eq!(
        validate_smoke_bead_report(&invalid_stage_count),
        Err(SmokeBeadError::InvalidReport("unexpected stage count"))
    );

    let mut invalid_stage_order = make_valid_smoke_bead_report();
    invalid_stage_order.stages = vec![
        invalid_stage_order.stages[1].clone(),
        invalid_stage_order.stages[0].clone(),
        invalid_stage_order.stages[2].clone(),
    ];
    assert_eq!(
        validate_smoke_bead_report(&invalid_stage_order),
        Err(SmokeBeadError::InvalidReport("invalid stage order"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_invalid_checks_and_diagnostics() {
    let mut invalid_orchestrator_endpoint = make_valid_smoke_bead_report();
    invalid_orchestrator_endpoint.checks[1].endpoint =
        "http://localhost:8080/Oya/other/get_status".to_string();
    assert_eq!(
        validate_smoke_bead_report(&invalid_orchestrator_endpoint),
        Err(SmokeBeadError::InvalidReport("invalid orchestrator check endpoint"))
    );

    let mut invalid_check_diagnostics = make_valid_smoke_bead_report();
    invalid_check_diagnostics.checks[0].diagnostics = "ok\u{0007}".to_string();
    assert_eq!(
        validate_smoke_bead_report(&invalid_check_diagnostics),
        Err(SmokeBeadError::InvalidReport("check diagnostics contain invalid control characters"))
    );

    let mut invalid_stage_diagnostics = make_valid_smoke_bead_report();
    invalid_stage_diagnostics.stages[1].diagnostics = "\u{0007}".to_string();
    assert_eq!(
        validate_smoke_bead_report(&invalid_stage_diagnostics),
        Err(SmokeBeadError::InvalidReport("stage diagnostics contain invalid control characters"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_missing_required_checks() {
    let mut missing_ingress = make_valid_smoke_bead_report();
    missing_ingress.checks.remove(0);
    assert_eq!(
        validate_smoke_bead_report(&missing_ingress),
        Err(SmokeBeadError::MissingCheck("ingress_health"))
    );

    let mut missing_orchestrator = make_valid_smoke_bead_report();
    missing_orchestrator.checks.remove(1);
    assert_eq!(
        validate_smoke_bead_report(&missing_orchestrator),
        Err(SmokeBeadError::MissingCheck("orchestrator_status"))
    );
}

#[test]
fn validate_smoke_bead_report_accepts_equal_consecutive_timestamps() {
    let mut report = make_valid_smoke_bead_report();
    report.stages[2].timestamp = report.stages[1].timestamp;

    assert_eq!(validate_smoke_bead_report(&report), Ok(()));
}

#[test]
fn validate_smoke_bead_report_rejects_non_monotonic_timestamps_and_decision_mismatch() {
    let mut non_monotonic_report = make_valid_smoke_bead_report();
    non_monotonic_report.stages[1].timestamp =
        non_monotonic_report.stages[0].timestamp - Duration::seconds(1);
    assert_eq!(
        validate_smoke_bead_report(&non_monotonic_report),
        Err(SmokeBeadError::InvalidReport("non-monotonic stage timestamps"))
    );

    let mut decision_mismatch_report = make_valid_smoke_bead_report();
    decision_mismatch_report.decision = SmokeBeadDecision::Fail;
    assert_eq!(
        validate_smoke_bead_report(&decision_mismatch_report),
        Err(SmokeBeadError::InvalidReport("decision mismatch"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_run_id_check_counts_and_stage_status_mismatches() {
    let mut invalid_run_id_report = make_valid_smoke_bead_report();
    invalid_run_id_report.run_id = " run-smoke-bead-test ".to_string();
    assert_eq!(
        validate_smoke_bead_report(&invalid_run_id_report),
        Err(SmokeBeadError::InvalidFieldContent("run_id"))
    );

    let mut invalid_ingress_count_report = make_valid_smoke_bead_report();
    invalid_ingress_count_report.checks[1].check = SmokeBeadCheckName::IngressHealth;
    assert_eq!(
        validate_smoke_bead_report(&invalid_ingress_count_report),
        Err(SmokeBeadError::InvalidReport("invalid ingress check count"))
    );

    let mut invalid_orchestrator_count_report = make_valid_smoke_bead_report();
    invalid_orchestrator_count_report
        .checks
        .push(invalid_orchestrator_count_report.checks[1].clone());
    assert_eq!(
        validate_smoke_bead_report(&invalid_orchestrator_count_report),
        Err(SmokeBeadError::InvalidReport("invalid orchestrator check count"))
    );

    let mut empty_check_diagnostics_report = make_valid_smoke_bead_report();
    empty_check_diagnostics_report.checks[0].diagnostics = "  ".to_string();
    assert_eq!(
        validate_smoke_bead_report(&empty_check_diagnostics_report),
        Err(SmokeBeadError::InvalidReport("empty check diagnostics"))
    );

    let mut empty_stage_diagnostics_report = make_valid_smoke_bead_report();
    empty_stage_diagnostics_report.stages[1].diagnostics = "  ".to_string();
    assert_eq!(
        validate_smoke_bead_report(&empty_stage_diagnostics_report),
        Err(SmokeBeadError::InvalidReport("empty stage diagnostics"))
    );

    let mut ingress_stage_mismatch_report = make_valid_smoke_bead_report();
    ingress_stage_mismatch_report.stages[0].status = SmokeBeadStageStatus::Failed;
    assert_eq!(
        validate_smoke_bead_report(&ingress_stage_mismatch_report),
        Err(SmokeBeadError::InvalidReport("ingress stage mismatch"))
    );

    let mut ingress_stage_diagnostics_mismatch_report = make_valid_smoke_bead_report();
    ingress_stage_diagnostics_mismatch_report.stages[0].diagnostics =
        "forged ingress diagnostics".to_string();
    assert_eq!(
        validate_smoke_bead_report(&ingress_stage_diagnostics_mismatch_report),
        Err(SmokeBeadError::InvalidReport("ingress stage diagnostics mismatch"))
    );

    let mut orchestrator_stage_mismatch_report = make_valid_smoke_bead_report();
    orchestrator_stage_mismatch_report.stages[1].status = SmokeBeadStageStatus::Failed;
    assert_eq!(
        validate_smoke_bead_report(&orchestrator_stage_mismatch_report),
        Err(SmokeBeadError::InvalidReport("orchestrator stage mismatch"))
    );

    let mut orchestrator_stage_diagnostics_mismatch_report = make_valid_smoke_bead_report();
    orchestrator_stage_diagnostics_mismatch_report.stages[1].diagnostics =
        "forged orchestrator diagnostics".to_string();
    assert_eq!(
        validate_smoke_bead_report(&orchestrator_stage_diagnostics_mismatch_report),
        Err(SmokeBeadError::InvalidReport("orchestrator stage diagnostics mismatch"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_oversized_diagnostics() {
    let mut oversized_check_diagnostics = make_valid_smoke_bead_report();
    oversized_check_diagnostics.checks[0].diagnostics =
        "d".repeat(MAX_SMOKE_BEAD_DIAGNOSTICS_LEN + 1);
    assert_eq!(
        validate_smoke_bead_report(&oversized_check_diagnostics),
        Err(SmokeBeadError::InvalidReport("check diagnostics exceed max length"))
    );

    let mut oversized_stage_diagnostics = make_valid_smoke_bead_report();
    oversized_stage_diagnostics.stages[1].diagnostics =
        "d".repeat(MAX_SMOKE_BEAD_DIAGNOSTICS_LEN + 1);
    assert_eq!(
        validate_smoke_bead_report(&oversized_stage_diagnostics),
        Err(SmokeBeadError::InvalidReport("stage diagnostics exceed max length"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_invalid_ingress_check_endpoint() {
    let mut report = make_valid_smoke_bead_report();
    report.checks[0].endpoint = "http://localhost:8080/health".to_string();

    assert_eq!(
        validate_smoke_bead_report(&report),
        Err(SmokeBeadError::InvalidReport("invalid ingress check endpoint"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_final_decision_diagnostics_mismatch() {
    let mut report = make_valid_smoke_bead_report();
    report.stages[2].diagnostics = "smoke-bead checks failed".to_string();

    assert_eq!(
        validate_smoke_bead_report(&report),
        Err(SmokeBeadError::InvalidReport("final decision diagnostics mismatch"))
    );
}

#[test]
fn validate_smoke_bead_report_rejects_stage_timestamps_before_checks() {
    let mut ingress_stage_before_check = make_valid_smoke_bead_report();
    ingress_stage_before_check.stages[0].timestamp =
        ingress_stage_before_check.checks[0].timestamp - Duration::milliseconds(1);
    assert_eq!(
        validate_smoke_bead_report(&ingress_stage_before_check),
        Err(SmokeBeadError::InvalidReport("ingress stage timestamp precedes check"))
    );

    let mut orchestrator_stage_before_check = make_valid_smoke_bead_report();
    orchestrator_stage_before_check.stages[1].timestamp =
        orchestrator_stage_before_check.checks[1].timestamp - Duration::milliseconds(1);
    assert_eq!(
        validate_smoke_bead_report(&orchestrator_stage_before_check),
        Err(SmokeBeadError::InvalidReport("orchestrator stage timestamp precedes check"))
    );
}

#[test]
fn smoke_bead_pipeline_passes_for_valid_default_contract() {
    let plan_result = build_smoke_bead_plan(&SmokeBeadInput { run_id: "run-smoke-05".into() });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let handle_result = start_smoke_bead_runtime(&plan);
    assert!(handle_result.is_ok());
    let Ok(handle) = handle_result else { return };

    let observation_result = capture_smoke_bead_observation(&handle);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    let report_result = evaluate_smoke_bead_result(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else { return };

    assert_eq!(report.decision, SmokeBeadDecision::Pass);
    assert_eq!(validate_smoke_bead_report(&report), Ok(()));
}

#[test]
fn build_bead_min_plan_rejects_empty_run_id_and_sets_defaults() {
    let empty_result = build_bead_min_plan(&BeadMinInput { run_id: "  ".to_string() });
    assert_eq!(empty_result, Err(BeadMinError::EmptyField("run_id")));

    let plan_result = build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min-01".into() });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    assert_eq!(plan.runtime_command, DEFAULT_BEAD_MIN_RUNTIME_COMMAND);
    assert_eq!(plan.ingress_health_url, DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL);
    assert_eq!(
        plan.orchestrator_status_url,
        "http://localhost:8080/Oya/run-bead-min-01/get_status"
    );
}

#[test]
fn build_bead_min_plan_rejects_invalid_run_id_boundaries_and_content() {
    let oversized_result =
        build_bead_min_plan(&BeadMinInput { run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1) });
    assert_eq!(oversized_result, Err(BeadMinError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN)));

    let control_char_result =
        build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min\u{0007}".to_string() });
    assert_eq!(control_char_result, Err(BeadMinError::InvalidFieldContent("run_id")));

    let path_injection_result =
        build_bead_min_plan(&BeadMinInput { run_id: "../run-bead-min".to_string() });
    assert_eq!(path_injection_result, Err(BeadMinError::InvalidFieldContent("run_id")));

    let query_injection_result =
        build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min?x=1".to_string() });
    assert_eq!(query_injection_result, Err(BeadMinError::InvalidFieldContent("run_id")));
}

#[test]
fn start_bead_min_runtime_rejects_non_default_runtime_command() {
    let plan = BeadMinPlan {
        run_id: "run-bead-min-01".to_string(),
        runtime_command: "scripts/not-default.sh".to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-bead-min-01/get_status".to_string(),
    };

    let result = start_bead_min_runtime(&plan);
    assert_eq!(result, Err(BeadMinError::InvalidRuntimeCommand));
}

#[test]
fn start_bead_min_runtime_rejects_invalid_run_id_and_endpoints() {
    let invalid_run_id_plan = BeadMinPlan {
        run_id: " run-bead-min-01 ".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-bead-min-01/get_status".to_string(),
    };
    assert_eq!(
        start_bead_min_runtime(&invalid_run_id_plan),
        Err(BeadMinError::InvalidFieldContent("run_id"))
    );

    let invalid_ingress_url_plan = BeadMinPlan {
        run_id: "run-bead-min-01".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "localhost:8080/restate/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-bead-min-01/get_status".to_string(),
    };
    assert_eq!(
        start_bead_min_runtime(&invalid_ingress_url_plan),
        Err(BeadMinError::InvalidEndpoint("ingress_health_url"))
    );

    let invalid_ingress_contract_plan = BeadMinPlan {
        run_id: "run-bead-min-01".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-bead-min-01/get_status".to_string(),
    };
    assert_eq!(
        start_bead_min_runtime(&invalid_ingress_contract_plan),
        Err(BeadMinError::InvalidEndpoint("ingress_health_url"))
    );

    let invalid_orchestrator_url_plan = BeadMinPlan {
        run_id: "run-bead-min-01".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "localhost:8080/Oya/run-bead-min-01/get_status".to_string(),
    };
    assert_eq!(
        start_bead_min_runtime(&invalid_orchestrator_url_plan),
        Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"))
    );

    let invalid_orchestrator_contract_plan = BeadMinPlan {
        run_id: "run-bead-min-01".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/other/get_status".to_string(),
    };
    assert_eq!(
        start_bead_min_runtime(&invalid_orchestrator_contract_plan),
        Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn capture_bead_min_observation_emits_exactly_one_check_per_stage() {
    let plan_result = build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min-02".into() });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let handle_result = start_bead_min_runtime(&plan);
    assert!(handle_result.is_ok());
    let Ok(handle) = handle_result else { return };

    let observation_result = capture_bead_min_observation(&handle);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    let ingress_count = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadMinCheckName::IngressHealth)
        .count();
    let orchestrator_count = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadMinCheckName::OrchestratorStatus)
        .count();

    assert_eq!(ingress_count, 1);
    assert_eq!(orchestrator_count, 1);
    assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
}

#[test]
fn capture_bead_min_observation_rejects_runtime_state_and_endpoint_violations() {
    let mut not_ready_handle = BeadMinRuntimeHandle {
        run_id: "run-bead-min-02".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-bead-min-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: false,
    };
    assert_eq!(capture_bead_min_observation(&not_ready_handle), Err(BeadMinError::RuntimeNotReady));

    not_ready_handle.runtime_ready = true;
    not_ready_handle.runtime_command = "scripts/not-default.sh".to_string();
    assert_eq!(
        capture_bead_min_observation(&not_ready_handle),
        Err(BeadMinError::InvalidRuntimeCommand)
    );

    let invalid_ingress_handle = BeadMinRuntimeHandle {
        run_id: "run-bead-min-02".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: "http://localhost:8080/health".to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/run-bead-min-02/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_bead_min_observation(&invalid_ingress_handle),
        Err(BeadMinError::InvalidEndpoint("ingress_health_url"))
    );

    let invalid_orchestrator_handle = BeadMinRuntimeHandle {
        run_id: "run-bead-min-02".to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: "http://localhost:8080/Oya/other/get_status".to_string(),
        started_at: Utc::now(),
        runtime_ready: true,
    };
    assert_eq!(
        capture_bead_min_observation(&invalid_orchestrator_handle),
        Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"))
    );
}

#[test]
fn evaluate_bead_min_result_uses_strict_stage_order_and_derived_decision() {
    let base = Utc::now();
    let observation = BeadMinObservation {
        run_id: "run-bead-min-03".to_string(),
        checks: vec![
            BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-bead-min-03/get_status".to_string(),
                success: false,
                diagnostics: "orchestrator timeout".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
        ],
    };

    let report_result = evaluate_bead_min_result(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else { return };

    assert_eq!(
        report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>(),
        vec![
            BeadMinStageName::IngressHealth,
            BeadMinStageName::OrchestratorStatus,
            BeadMinStageName::FinalDecision,
        ]
    );
    assert_eq!(report.decision, BeadMinDecision::Fail);
}

#[test]
fn evaluate_bead_min_result_rejects_missing_or_duplicate_checks() {
    let base = Utc::now();

    let missing_ingress = BeadMinObservation {
        run_id: "run-bead-min-03".to_string(),
        checks: vec![BeadMinCheckObservation {
            check: BeadMinCheckName::OrchestratorStatus,
            endpoint: "http://localhost:8080/Oya/run-bead-min-03/get_status".to_string(),
            success: true,
            diagnostics: "orchestrator healthy".to_string(),
            timestamp: base,
        }],
    };
    assert_eq!(
        evaluate_bead_min_result(&missing_ingress),
        Err(BeadMinError::MissingCheck("ingress_health"))
    );

    let duplicate_ingress = BeadMinObservation {
        run_id: "run-bead-min-03".to_string(),
        checks: vec![
            BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress duplicate".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-bead-min-03/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
        ],
    };
    assert_eq!(
        evaluate_bead_min_result(&duplicate_ingress),
        Err(BeadMinError::InvalidReport("duplicate ingress_health checks"))
    );

    let missing_orchestrator = BeadMinObservation {
        run_id: "run-bead-min-03".to_string(),
        checks: vec![BeadMinCheckObservation {
            check: BeadMinCheckName::IngressHealth,
            endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            success: true,
            diagnostics: "ingress healthy".to_string(),
            timestamp: base,
        }],
    };
    assert_eq!(
        evaluate_bead_min_result(&missing_orchestrator),
        Err(BeadMinError::MissingCheck("orchestrator_status"))
    );

    let duplicate_orchestrator = BeadMinObservation {
        run_id: "run-bead-min-03".to_string(),
        checks: vec![
            BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-bead-min-03/get_status".to_string(),
                success: true,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/Oya/run-bead-min-03/get_status".to_string(),
                success: false,
                diagnostics: "orchestrator duplicate".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
        ],
    };
    assert_eq!(
        evaluate_bead_min_result(&duplicate_orchestrator),
        Err(BeadMinError::InvalidReport("duplicate orchestrator_status checks"))
    );
}

#[test]
fn validate_bead_min_report_rejects_endpoint_and_decision_mismatches() {
    let mut invalid_endpoint_report = make_valid_bead_min_report();
    invalid_endpoint_report.checks[1].endpoint =
        "http://localhost:8080/Oya/other/get_status".to_string();
    assert_eq!(
        validate_bead_min_report(&invalid_endpoint_report),
        Err(BeadMinError::InvalidReport("invalid orchestrator check endpoint"))
    );

    let mut decision_mismatch_report = make_valid_bead_min_report();
    decision_mismatch_report.decision = BeadMinDecision::Fail;
    assert_eq!(
        validate_bead_min_report(&decision_mismatch_report),
        Err(BeadMinError::InvalidReport("decision mismatch"))
    );
}

#[test]
fn validate_bead_min_report_rejects_stage_shape_and_timestamp_mismatches() {
    let mut invalid_stage_count = make_valid_bead_min_report();
    invalid_stage_count.stages.pop();
    assert_eq!(
        validate_bead_min_report(&invalid_stage_count),
        Err(BeadMinError::InvalidReport("unexpected stage count"))
    );

    let mut invalid_stage_order = make_valid_bead_min_report();
    invalid_stage_order.stages.swap(0, 1);
    assert_eq!(
        validate_bead_min_report(&invalid_stage_order),
        Err(BeadMinError::InvalidReport("invalid stage order"))
    );

    let mut empty_stage_diagnostics = make_valid_bead_min_report();
    empty_stage_diagnostics.stages[1].diagnostics = "  ".to_string();
    assert_eq!(
        validate_bead_min_report(&empty_stage_diagnostics),
        Err(BeadMinError::InvalidReport("empty stage diagnostics"))
    );

    let mut invalid_stage_diagnostics = make_valid_bead_min_report();
    invalid_stage_diagnostics.stages[1].diagnostics = "\u{0007}".to_string();
    assert_eq!(
        validate_bead_min_report(&invalid_stage_diagnostics),
        Err(BeadMinError::InvalidReport("stage diagnostics contain invalid control characters"))
    );

    let mut oversized_stage_diagnostics = make_valid_bead_min_report();
    oversized_stage_diagnostics.stages[1].diagnostics =
        "d".repeat(MAX_BEAD_MIN_DIAGNOSTICS_LEN + 1);
    assert_eq!(
        validate_bead_min_report(&oversized_stage_diagnostics),
        Err(BeadMinError::InvalidReport("stage diagnostics exceed max length"))
    );

    let mut non_monotonic_timestamps = make_valid_bead_min_report();
    non_monotonic_timestamps.stages[1].timestamp =
        non_monotonic_timestamps.stages[0].timestamp - Duration::milliseconds(1);
    assert_eq!(
        validate_bead_min_report(&non_monotonic_timestamps),
        Err(BeadMinError::InvalidReport("non-monotonic stage timestamps"))
    );

    let mut ingress_before_check = make_valid_bead_min_report();
    ingress_before_check.stages[0].timestamp =
        ingress_before_check.checks[0].timestamp - Duration::milliseconds(1);
    assert_eq!(
        validate_bead_min_report(&ingress_before_check),
        Err(BeadMinError::InvalidReport("ingress stage timestamp precedes check"))
    );

    let mut orchestrator_before_check = make_valid_bead_min_report();
    orchestrator_before_check.stages[1].timestamp =
        orchestrator_before_check.checks[1].timestamp - Duration::milliseconds(1);
    assert_eq!(
        validate_bead_min_report(&orchestrator_before_check),
        Err(BeadMinError::InvalidReport("orchestrator stage timestamp precedes check"))
    );
}

#[test]
fn validate_bead_min_report_rejects_missing_checks_and_diagnostics_mismatches() {
    let mut missing_ingress = make_valid_bead_min_report();
    missing_ingress.checks.remove(0);
    assert_eq!(
        validate_bead_min_report(&missing_ingress),
        Err(BeadMinError::MissingCheck("ingress_health"))
    );

    let mut missing_orchestrator = make_valid_bead_min_report();
    missing_orchestrator.checks.remove(1);
    assert_eq!(
        validate_bead_min_report(&missing_orchestrator),
        Err(BeadMinError::MissingCheck("orchestrator_status"))
    );

    let mut empty_check_diagnostics = make_valid_bead_min_report();
    empty_check_diagnostics.checks[0].diagnostics = "  ".to_string();
    assert_eq!(
        validate_bead_min_report(&empty_check_diagnostics),
        Err(BeadMinError::InvalidReport("empty check diagnostics"))
    );

    let mut oversized_check_diagnostics = make_valid_bead_min_report();
    oversized_check_diagnostics.checks[0].diagnostics =
        "d".repeat(MAX_BEAD_MIN_DIAGNOSTICS_LEN + 1);
    assert_eq!(
        validate_bead_min_report(&oversized_check_diagnostics),
        Err(BeadMinError::InvalidReport("check diagnostics exceed max length"))
    );

    let mut invalid_check_diagnostics = make_valid_bead_min_report();
    invalid_check_diagnostics.checks[0].diagnostics = "\u{0007}".to_string();
    assert_eq!(
        validate_bead_min_report(&invalid_check_diagnostics),
        Err(BeadMinError::InvalidReport("check diagnostics contain invalid control characters"))
    );

    let mut final_decision_mismatch = make_valid_bead_min_report();
    final_decision_mismatch.stages[2].diagnostics = "bead-min checks failed".to_string();
    assert_eq!(
        validate_bead_min_report(&final_decision_mismatch),
        Err(BeadMinError::InvalidReport("final decision diagnostics mismatch"))
    );
}

#[test]
fn bead_min_pipeline_passes_for_valid_default_contract() {
    let plan_result = build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min-04".into() });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let handle_result = start_bead_min_runtime(&plan);
    assert!(handle_result.is_ok());
    let Ok(handle) = handle_result else { return };

    let observation_result = capture_bead_min_observation(&handle);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    let report_result = evaluate_bead_min_result(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else { return };

    assert_eq!(report.decision, BeadMinDecision::Pass);
    assert_eq!(validate_bead_min_report(&report), Ok(()));
}

#[test]
fn parse_opencode_output_rejects_empty() {
    let result = parse_opencode_output("  \n\t ");
    assert!(result.is_err());
}

#[test]
fn parse_opencode_output_rejects_invalid_json() {
    let result = parse_opencode_output("not json");
    assert!(result.is_err());
}

#[test]
fn parse_opencode_output_requires_stdout_field() {
    let result = parse_opencode_output("{\"status\":\"ok\"}");
    assert!(result.is_err());
}

#[test]
fn parse_opencode_output_requires_stdout_string() {
    let result = parse_opencode_output("{\"stdout\":123}");
    assert!(result.is_err());
}

#[test]
fn parse_opencode_output_accepts_stdout_string() {
    let result = parse_opencode_output("{\"stdout\":\"ok\"}");
    assert_eq!(result, Ok(OpencodeRunOutput { stdout: "ok".to_string() }));
}

#[test]
fn parse_opencode_output_trims_outer_whitespace() {
    let result = parse_opencode_output("  {\"stdout\":\"ok\"}  ");
    assert_eq!(result, Ok(OpencodeRunOutput { stdout: "ok".to_string() }));
}

#[test]
fn parse_opencode_output_accepts_event_stream_text_parts() {
    let payload = concat!(
        "[hypr-notifier] plugin initialized\n",
        "{\"type\":\"step_start\",\"part\":{\"id\":\"p1\"}}\n",
        "{\"type\":\"text\",\"part\":{\"text\":\"ok\"}}\n"
    );
    let result = parse_opencode_output(payload);
    assert_eq!(result, Ok(OpencodeRunOutput { stdout: "ok".to_string() }));
}

#[test]
fn parse_opencode_output_rejects_oversized_json_payload() {
    let oversized_payload =
        format!("{{\"stdout\":\"{}\"}}", "x".repeat(MAX_OPENCODE_OUTPUT_JSON_LEN + 1));

    let result = parse_opencode_output(&oversized_payload);
    assert!(result.is_err());
    assert_eq!(
        result.err().map(|error| error.to_string()),
        Some("opencode output exceeds maximum length".to_string())
    );
}

#[test]
fn parse_opencode_output_rejects_oversized_stdout_field() {
    let oversized_stdout = "x".repeat(MAX_OPENCODE_STDOUT_LEN + 1);
    let payload = format!("{{\"stdout\":\"{}\"}}", oversized_stdout);

    let result = parse_opencode_output(&payload);
    assert!(result.is_err());
    assert_eq!(
        result.err().map(|error| error.to_string()),
        Some("opencode stdout exceeds maximum length".to_string())
    );
}

#[test]
fn parse_opencode_output_rejects_invalid_control_characters_in_stdout() {
    let result = parse_opencode_output("{\"stdout\":\"ok\\u0000bad\"}");
    assert!(result.is_err());
    assert_eq!(
        result.err().map(|error| error.to_string()),
        Some("opencode stdout contains invalid control characters".to_string())
    );
}

#[test]
fn opencode_parse_error_display_returns_message() {
    let error = OpencodeParseError::new("boom");
    assert_eq!(error.to_string(), "boom");
}

#[test]
fn opencode_poll_snapshot_is_debug_clone_and_eq() {
    let snapshot = OpencodePollSnapshot {
        busy_sessions: vec!["ses_1".to_string()],
        pending_permissions: 1,
        pending_questions: 2,
    };
    let cloned = snapshot.clone();
    assert_eq!(snapshot, cloned);
    let debug_str = format!("{:?}", snapshot);
    assert!(debug_str.contains("busy_sessions"));
    assert!(debug_str.contains("pending_permissions"));
    assert!(debug_str.contains("pending_questions"));
}

#[test]
fn ops_monitor_error_display_formats_correctly() {
    assert_eq!(OpsMonitorError::EmptyField("test").to_string(), "ops monitor field is empty: test");
    assert_eq!(
        OpsMonitorError::FieldTooLong("test", 100).to_string(),
        "ops monitor field exceeds max length: test > 100"
    );
    assert_eq!(
        OpsMonitorError::InvalidFieldContent("test").to_string(),
        "ops monitor field has invalid control characters: test"
    );
    assert_eq!(
        OpsMonitorError::InvalidFieldFormat("test").to_string(),
        "ops monitor field has invalid format: test"
    );
    assert_eq!(
        OpsMonitorError::InvalidJson("parse error".to_string()).to_string(),
        "ops monitor json parse failed: parse error"
    );
}

#[test]
fn zjj_workspace_given_valid_inputs_when_build_then_returns_normalized_name() {
    let result = build_zjj_workspace_name(" Run_ABC ", "Tdd15", 2);
    assert_eq!(result, Ok("oya-run_abc-tdd15-a2".to_string()));
}

#[test]
fn zjj_workspace_given_attempt_zero_when_build_then_returns_invalid_attempt_error() {
    let result = build_zjj_workspace_name("run-1", "qa", 0);
    assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("attempt")));
}

#[test]
fn zjj_workspace_given_minimal_valid_input_when_build_then_returns_prefixed_name() {
    let result = build_zjj_workspace_name("run", "qa", 1);
    assert_eq!(result, Ok("oya-run-qa-a1".to_string()));
}

#[test]
fn zjj_workspace_given_uppercase_input_when_build_then_normalizes_to_lowercase() {
    let result = build_zjj_workspace_name("RUN-ID", "TDD15", 3);
    assert_eq!(result, Ok("oya-run-id-tdd15-a3".to_string()));
}

#[test]
fn zjj_workspace_given_special_characters_when_build_then_converts_to_dashes() {
    let result = build_zjj_workspace_name("run@id#test", "qa", 1);
    assert_eq!(result, Ok("oya-run-id-test-qa-a1".to_string()));
}

#[test]
fn zjj_workspace_given_consecutive_special_chars_when_build_then_collapses_to_single_dash() {
    let result = build_zjj_workspace_name("run---id", "qa", 1);
    assert_eq!(result, Ok("oya-run-id-qa-a1".to_string()));
}

#[test]
fn zjj_workspace_given_underscores_when_build_then_preserves_them() {
    let result = build_zjj_workspace_name("run_id_test", "stage", 1);
    assert_eq!(result, Ok("oya-run_id_test-stage-a1".to_string()));
}

#[test]
fn zjj_workspace_given_whitespace_padding_when_build_then_trims_it() {
    let result = build_zjj_workspace_name("  run-id  ", "  qa  ", 1);
    assert_eq!(result, Ok("oya-run-id-qa-a1".to_string()));
}

#[test]
fn zjj_workspace_given_empty_run_id_when_build_then_returns_empty_field_error() {
    let result = build_zjj_workspace_name("", "qa", 1);
    assert_eq!(result, Err(OpsMonitorError::EmptyField("run_id")));
}

#[test]
fn zjj_workspace_given_whitespace_only_run_id_when_build_then_returns_empty_field_error() {
    let result = build_zjj_workspace_name("   ", "qa", 1);
    assert_eq!(result, Err(OpsMonitorError::EmptyField("run_id")));
}

#[test]
fn zjj_workspace_given_empty_stage_when_build_then_returns_empty_field_error() {
    let result = build_zjj_workspace_name("run", "", 1);
    assert_eq!(result, Err(OpsMonitorError::EmptyField("stage")));
}

#[test]
fn zjj_workspace_given_control_char_in_run_id_when_build_then_returns_invalid_content_error() {
    let result = build_zjj_workspace_name("run\u{0000}id", "qa", 1);
    assert_eq!(result, Err(OpsMonitorError::InvalidFieldContent("run_id")));
}

#[test]
fn zjj_workspace_given_oversized_inputs_when_build_then_returns_field_too_long_error() {
    let long_run_id = "x".repeat(50);
    let long_stage = "y".repeat(20);
    let result = build_zjj_workspace_name(&long_run_id, &long_stage, 999);
    assert_eq!(result, Err(OpsMonitorError::FieldTooLong("workspace", MAX_ZJJ_WORKSPACE_NAME_LEN)));
}

#[test]
fn zjj_workspace_given_only_special_chars_when_build_then_returns_invalid_format_error() {
    let result = build_zjj_workspace_name("@@@", "qa", 1);
    assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("run_id")));
}

#[test]
fn opencode_status_given_empty_json_when_parse_then_returns_empty_list() {
    let result = parse_opencode_busy_sessions("");
    assert_eq!(result, Ok(Vec::<String>::new()));
}

#[test]
fn opencode_status_given_whitespace_when_parse_then_returns_empty_list() {
    let result = parse_opencode_busy_sessions("   ");
    assert_eq!(result, Ok(Vec::<String>::new()));
}

#[test]
fn opencode_status_given_only_idle_sessions_when_parse_then_returns_empty_list() {
    let result = parse_opencode_busy_sessions("{\"ses_a\":{\"type\":\"idle\"}}");
    assert_eq!(result, Ok(Vec::<String>::new()));
}

#[test]
fn opencode_status_given_mixed_sessions_when_parse_then_returns_only_busy_sorted() {
    let result = parse_opencode_busy_sessions(
        "{\"ses_c\":{\"type\":\"busy\"},\"ses_a\":{\"type\":\"busy\"}}",
    );
    assert_eq!(result, Ok(vec!["ses_a".to_string(), "ses_c".to_string()]));
}

#[test]
fn opencode_status_given_unknown_type_when_parse_then_ignores_it() {
    let result = parse_opencode_busy_sessions(
        "{\"ses_a\":{\"type\":\"busy\"},\"ses_b\":{\"type\":\"unknown\"}}",
    );
    assert_eq!(result, Ok(vec!["ses_a".to_string()]));
}

#[test]
fn opencode_status_given_missing_type_field_when_parse_then_ignores_session() {
    let result = parse_opencode_busy_sessions("{\"ses_a\":{}}");
    assert_eq!(result, Ok(Vec::<String>::new()));
}

#[test]
fn opencode_status_given_invalid_json_when_parse_then_returns_invalid_json_error() {
    let result = parse_opencode_busy_sessions("not json");
    let Err(OpsMonitorError::InvalidJson(msg)) = result else {
        panic!("Expected InvalidJson error");
    };
    assert!(msg.contains("expected"));
}

#[test]
fn opencode_status_given_array_root_when_parse_then_returns_invalid_format_error() {
    let result = parse_opencode_busy_sessions("[{\"type\":\"busy\"}]");
    assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("session_status")));
}

#[test]
fn opencode_pending_given_empty_string_when_parse_then_returns_zero() {
    let result = parse_opencode_pending_count("", "test");
    assert_eq!(result, Ok(0));
}

#[test]
fn opencode_pending_given_null_when_parse_then_returns_zero() {
    let result = parse_opencode_pending_count("null", "test");
    assert_eq!(result, Ok(0));
}

#[test]
fn opencode_pending_given_json_array_when_parse_then_returns_length() {
    let result = parse_opencode_pending_count("[1,2,3,4,5]", "test");
    assert_eq!(result, Ok(5));
}

#[test]
fn opencode_pending_given_items_array_when_parse_then_returns_its_length() {
    let result = parse_opencode_pending_count("{\"items\":[1,2,3]}", "test");
    assert_eq!(result, Ok(3));
}

#[test]
fn opencode_pending_given_requests_array_when_parse_then_returns_its_length() {
    let result = parse_opencode_pending_count("{\"requests\":[1,2]}", "test");
    assert_eq!(result, Ok(2));
}

#[test]
fn opencode_pending_given_rows_array_when_parse_then_returns_its_length() {
    let result = parse_opencode_pending_count("{\"rows\":[1]}", "test");
    assert_eq!(result, Ok(1));
}

#[test]
fn opencode_pending_given_object_without_known_array_when_parse_then_returns_key_count() {
    let result = parse_opencode_pending_count("{\"a\":1,\"b\":2,\"c\":3}", "test");
    assert_eq!(result, Ok(3));
}

#[test]
fn opencode_pending_given_object_with_items_and_extras_when_parse_then_uses_items_count() {
    let result = parse_opencode_pending_count("{\"items\":[1,2],\"extra\":3}", "test");
    assert_eq!(result, Ok(2));
}

#[test]
fn opencode_pending_given_string_value_when_parse_then_returns_invalid_format_error() {
    let result = parse_opencode_pending_count("\"string\"", "test");
    assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("test")));
}

#[test]
fn opencode_pending_given_invalid_json_when_parse_then_returns_invalid_json_error() {
    let result = parse_opencode_pending_count("not json", "test");
    let Err(OpsMonitorError::InvalidJson(msg)) = result else {
        panic!("Expected InvalidJson error");
    };
    assert!(msg.contains("expected"));
}

#[test]
fn opencode_sse_given_empty_string_when_parse_then_returns_empty_list() {
    let result = parse_opencode_sse_events("", 10);
    assert_eq!(result, Ok(Vec::<String>::new()));
}

#[test]
fn opencode_sse_given_whitespace_when_parse_then_returns_empty_list() {
    let result = parse_opencode_sse_events("   ", 10);
    assert_eq!(result, Ok(Vec::<String>::new()));
}

#[test]
fn opencode_sse_given_single_data_line_when_parse_then_extracts_payload() {
    let result = parse_opencode_sse_events("data: hello\n\n", 10);
    assert_eq!(result, Ok(vec!["hello".to_string()]));
}

#[test]
fn opencode_sse_given_multiple_data_lines_in_event_when_parse_then_joins_with_newline() {
    let result = parse_opencode_sse_events("data: line1\ndata: line2\n\n", 10);
    assert_eq!(result, Ok(vec!["line1\nline2".to_string()]));
}

#[test]
fn opencode_sse_given_event_type_line_when_parse_then_ignores_it() {
    let result = parse_opencode_sse_events("event: ping\ndata: hello\n\n", 10);
    assert_eq!(result, Ok(vec!["hello".to_string()]));
}

#[test]
fn opencode_sse_given_max_events_limit_when_parse_then_truncates_to_limit() {
    let result = parse_opencode_sse_events("data: a\n\ndata: b\n\ndata: c\n\n", 2);
    assert_eq!(result, Ok(vec!["a".to_string(), "b".to_string()]));
}

#[test]
fn opencode_sse_given_crlf_line_endings_when_parse_then_normalizes_to_lf() {
    let result = parse_opencode_sse_events("data: hello\r\n\r\n", 10);
    assert_eq!(result, Ok(vec!["hello".to_string()]));
}

#[test]
fn opencode_sse_given_standalone_cr_when_parse_then_normalizes_to_lf() {
    let result = parse_opencode_sse_events("data: hello\r\r", 10);
    assert_eq!(result, Ok(vec!["hello".to_string()]));
}

#[test]
fn opencode_sse_given_mixed_line_endings_when_parse_then_normalizes_all() {
    let result = parse_opencode_sse_events("data: hello\r\n\ndata: world\r\r", 10);
    assert_eq!(result, Ok(vec!["hello".to_string(), "world".to_string()]));
}

#[test]
fn opencode_sse_given_oversized_chunk_when_parse_then_returns_field_too_long_error() {
    let oversized = "data: x\n\n".repeat(MAX_OPENCODE_SSE_RAW_CHUNK_LEN / 8 + 1);
    let result = parse_opencode_sse_events(&oversized, 10);
    assert_eq!(
        result,
        Err(OpsMonitorError::FieldTooLong("event_chunk", MAX_OPENCODE_SSE_RAW_CHUNK_LEN))
    );
}

#[test]
fn opencode_sse_given_control_char_in_chunk_when_parse_then_returns_invalid_content_error() {
    let result = parse_opencode_sse_events("data: hello\u{0000}world\n\n", 10);
    assert_eq!(result, Err(OpsMonitorError::InvalidFieldContent("event_chunk")));
}

#[test]
fn opencode_sse_given_oversized_payload_when_parse_then_returns_field_too_long_error() {
    let long_data = "x".repeat(MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN + 1);
    let chunk = format!("data: {}\n\n", long_data);
    let result = parse_opencode_sse_events(&chunk, 10);
    assert_eq!(
        result,
        Err(OpsMonitorError::FieldTooLong("event_payload", MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN))
    );
}

#[test]
fn opencode_sse_given_empty_data_line_when_parse_then_ignores_it() {
    let result = parse_opencode_sse_events("data: \n\ndata: hello\n\n", 10);
    assert_eq!(result, Ok(vec!["hello".to_string()]));
}

#[test]
fn opencode_sse_given_json_payload_when_parse_then_extracts_intact() {
    let raw =
            "event: session.status\ndata: {\"session\":\"ses_1\",\"type\":\"busy\"}\n\nevent: session.idle\ndata: {\"session\":\"ses_1\"}\n\n";
    let result = parse_opencode_sse_events(raw, 10);
    assert_eq!(
        result,
        Ok(vec![
            "{\"session\":\"ses_1\",\"type\":\"busy\"}".to_string(),
            "{\"session\":\"ses_1\"}".to_string()
        ])
    );
}

#[test]
fn opencode_poll_given_all_empty_when_build_then_returns_zeros() {
    let result = build_opencode_poll_snapshot("", "", "");
    assert_eq!(
        result,
        Ok(OpencodePollSnapshot {
            busy_sessions: vec![],
            pending_permissions: 0,
            pending_questions: 0,
        })
    );
}

#[test]
fn opencode_poll_given_valid_inputs_when_build_then_combines_all_sources() {
    let result = build_opencode_poll_snapshot(
        "{\"a\":{\"type\":\"busy\"},\"b\":{\"type\":\"idle\"}}",
        "[1,2,3]",
        "{\"items\":[1,2,3,4]}",
    );
    assert_eq!(
        result,
        Ok(OpencodePollSnapshot {
            busy_sessions: vec!["a".to_string()],
            pending_permissions: 3,
            pending_questions: 4,
        })
    );
}

#[test]
fn opencode_poll_given_invalid_status_json_when_build_then_propagates_error() {
    let result = build_opencode_poll_snapshot("invalid", "[]", "[]");
    let Err(OpsMonitorError::InvalidJson(msg)) = result else {
        panic!("Expected InvalidJson error");
    };
    assert!(msg.contains("expected"));
}

#[test]
fn opencode_poll_given_invalid_permission_json_when_build_then_propagates_error() {
    let result = build_opencode_poll_snapshot("{}", "invalid", "[]");
    let Err(OpsMonitorError::InvalidJson(msg)) = result else {
        panic!("Expected InvalidJson error");
    };
    assert!(msg.contains("expected"));
}

#[test]
fn opencode_poll_given_invalid_question_json_when_build_then_propagates_error() {
    let result = build_opencode_poll_snapshot("{}", "[]", "invalid");
    let Err(OpsMonitorError::InvalidJson(msg)) = result else {
        panic!("Expected InvalidJson error");
    };
    assert!(msg.contains("expected"));
}

#[test]
fn build_manual_e2e_plan_rejects_blank_fields() {
    let missing_scenario = build_manual_e2e_plan(&ManualE2eInput {
        scenario: " ".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });
    assert_eq!(missing_scenario, Err(ManualE2eError::EmptyField("scenario")));

    let missing_command = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "  ".to_string(),
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });
    assert_eq!(missing_command, Err(ManualE2eError::EmptyField("command")));

    let missing_output = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: " ".to_string(),
    });
    assert_eq!(missing_output, Err(ManualE2eError::EmptyField("raw_output")));
}

#[test]
fn build_manual_e2e_plan_rejects_boundary_and_malformed_inputs() {
    let oversized_scenario = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "s".repeat(MAX_MANUAL_E2E_SCENARIO_LEN + 1),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });
    assert_eq!(
        oversized_scenario,
        Err(ManualE2eError::FieldTooLong("scenario", MAX_MANUAL_E2E_SCENARIO_LEN))
    );

    let oversized_command = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "c".repeat(MAX_MANUAL_E2E_COMMAND_LEN + 1),
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });
    assert_eq!(
        oversized_command,
        Err(ManualE2eError::FieldTooLong("command", MAX_MANUAL_E2E_COMMAND_LEN))
    );

    let oversized_raw = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "x".repeat(MAX_MANUAL_E2E_RAW_OUTPUT_LEN + 1),
    });
    assert_eq!(
        oversized_raw,
        Err(ManualE2eError::FieldTooLong("raw_output", MAX_MANUAL_E2E_RAW_OUTPUT_LEN))
    );

    let scenario_with_control_char = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual\u{0007}e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });
    assert_eq!(scenario_with_control_char, Err(ManualE2eError::InvalidFieldContent("scenario")));

    let command_with_control_char = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya\u{0000} run manual-e2e".to_string(),
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });
    assert_eq!(command_with_control_char, Err(ManualE2eError::InvalidFieldContent("command")));
}

#[test]
fn parse_pipeline_output_rejects_empty_malformed_or_incomplete_payloads() {
    assert_eq!(parse_pipeline_output("   "), Err(ManualE2eError::EmptyField("raw_output")));

    let malformed = parse_pipeline_output("not json");
    assert!(matches!(malformed, Err(ManualE2eError::InvalidJson(_))));

    let missing = parse_pipeline_output("{\"success\":true}");
    assert_eq!(missing, Err(ManualE2eError::MissingField("diagnostics")));
}

#[test]
fn build_manual_e2e_plan_trims_whitespace_from_valid_fields() {
    let result = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "  manual-e2e  ".to_string(),
        command: "  oya run manual-e2e  ".to_string(),
        raw_output: "  {\"success\":true,\"diagnostics\":\"ok\"}  ".to_string(),
    });

    assert_eq!(
        result,
        Ok(ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        })
    );
}

#[test]
fn build_manual_e2e_plan_accepts_boundary_lengths_and_allowed_controls() {
    let scenario = format!("{}\n", "s".repeat(MAX_MANUAL_E2E_SCENARIO_LEN - 1));
    let command = format!("{}\t", "c".repeat(MAX_MANUAL_E2E_COMMAND_LEN - 1));
    let result = build_manual_e2e_plan(&ManualE2eInput {
        scenario,
        command,
        raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
    });

    assert!(result.is_ok());
}

#[test]
fn parse_pipeline_output_rejects_missing_success_field() {
    let result = parse_pipeline_output("{\"diagnostics\":\"ok\"}");
    assert_eq!(result, Err(ManualE2eError::MissingField("success")));
}

#[test]
fn parse_pipeline_output_rejects_non_boolean_success() {
    let result = parse_pipeline_output("{\"success\":\"yes\",\"diagnostics\":\"ok\"}");
    assert_eq!(result, Err(ManualE2eError::InvalidFieldType("success")));
}

#[test]
fn parse_pipeline_output_rejects_non_string_diagnostics() {
    let result = parse_pipeline_output("{\"success\":true,\"diagnostics\":123}");
    assert_eq!(result, Err(ManualE2eError::InvalidFieldType("diagnostics")));
}

#[test]
fn parse_pipeline_output_rejects_blank_diagnostics() {
    let result = parse_pipeline_output("{\"success\":true,\"diagnostics\":\"   \"}");
    assert_eq!(result, Err(ManualE2eError::EmptyField("diagnostics")));
}

#[test]
fn parse_pipeline_output_rejects_boundary_and_malformed_inputs() {
    let oversized_raw = "x".repeat(MAX_MANUAL_E2E_RAW_OUTPUT_LEN + 1);
    assert_eq!(
        parse_pipeline_output(&oversized_raw),
        Err(ManualE2eError::FieldTooLong("raw_output", MAX_MANUAL_E2E_RAW_OUTPUT_LEN))
    );

    let oversized_diagnostics = format!(
        "{{\"success\":true,\"diagnostics\":\"{}\"}}",
        "d".repeat(MAX_MANUAL_E2E_DIAGNOSTICS_LEN + 1)
    );
    assert_eq!(
        parse_pipeline_output(&oversized_diagnostics),
        Err(ManualE2eError::FieldTooLong("diagnostics", MAX_MANUAL_E2E_DIAGNOSTICS_LEN))
    );

    let invalid_control_diagnostics =
        parse_pipeline_output("{\"success\":true,\"diagnostics\":\"bad\\u0000data\"}");
    assert_eq!(
        invalid_control_diagnostics,
        Err(ManualE2eError::InvalidFieldContent("diagnostics"))
    );

    let multiline_diagnostics =
        parse_pipeline_output("{\"success\":true,\"diagnostics\":\"line1\\nline2\\tdata\"}");
    assert_eq!(
        multiline_diagnostics,
        Ok(ManualE2eOutput { success: true, diagnostics: "line1\nline2\tdata".to_string() })
    );
}

#[test]
fn parse_pipeline_output_accepts_diagnostics_at_max_length() {
    let max_diagnostics = "d".repeat(MAX_MANUAL_E2E_DIAGNOSTICS_LEN);
    let payload = format!("{{\"success\":false,\"diagnostics\":\"{}\"}}", max_diagnostics);
    let result = parse_pipeline_output(&payload);

    assert_eq!(result, Ok(ManualE2eOutput { success: false, diagnostics: max_diagnostics }));
}

#[test]
fn run_manual_e2e_pipeline_records_stage_results_in_order() {
    let plan_result = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":true,\"diagnostics\":\"pipeline green\"}".to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let report_result = run_manual_e2e_pipeline(&plan);
    assert!(report_result.is_ok());
    let report = match report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let order = report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>();
    assert_eq!(
        order,
        vec![
            ManualE2eStageName::ScenarioSetup,
            ManualE2eStageName::CommandInvocation,
            ManualE2eStageName::OutputParsing,
            ManualE2eStageName::GateEvaluation,
        ]
    );
    assert_eq!(report.decision, ManualE2eGateDecision::Allow);
    assert!(validate_manual_e2e_report(&report).is_ok());
}

#[test]
fn run_manual_e2e_pipeline_blocks_gate_when_any_stage_fails() {
    let plan_result = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":false,\"diagnostics\":\"output mismatch\"}".to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let report_result = run_manual_e2e_pipeline(&plan);
    assert!(report_result.is_ok());
    let report = match report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    assert_eq!(report.decision, ManualE2eGateDecision::Block);
    assert_eq!(derive_manual_e2e_gate(&report), ManualE2eGateDecision::Block);
    assert!(report.stages.iter().any(|stage| stage.status == ManualE2eStageStatus::Failed));

    let gate_stage =
        report.stages.iter().find(|stage| stage.stage == ManualE2eStageName::GateEvaluation);
    assert_eq!(gate_stage.map(|stage| stage.diagnostics.as_str()), Some("manual gate blocked"));
}

#[test]
fn rerunning_same_plan_yields_equivalent_validation_outcomes() {
    let plan_result = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":false,\"diagnostics\":\"gate blocked\"}".to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let first_result = run_manual_e2e_pipeline(&plan);
    let second_result = run_manual_e2e_pipeline(&plan);
    assert!(first_result.is_ok());
    assert!(second_result.is_ok());

    let first = match first_result {
        Ok(value) => value,
        Err(_) => return,
    };
    let second = match second_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let first_stage_statuses = first
        .stages
        .iter()
        .map(|stage| (stage.stage.clone(), stage.status.clone()))
        .collect::<Vec<_>>();
    let second_stage_statuses = second
        .stages
        .iter()
        .map(|stage| (stage.stage.clone(), stage.status.clone()))
        .collect::<Vec<_>>();

    assert_eq!(first.decision, second.decision);
    assert_eq!(first_stage_statuses, second_stage_statuses);
    assert_eq!(validate_manual_e2e_report(&first), Ok(()));
    assert_eq!(validate_manual_e2e_report(&second), Ok(()));
}

#[test]
fn validate_manual_e2e_report_rejects_inconsistent_decision() {
    let plan_result = build_manual_e2e_plan(&ManualE2eInput {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "{\"success\":false,\"diagnostics\":\"failed stage\"}".to_string(),
    });
    assert!(plan_result.is_ok());
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let report_result = run_manual_e2e_pipeline(&plan);
    assert!(report_result.is_ok());
    let mut report = match report_result {
        Ok(value) => value,
        Err(_) => return,
    };

    report.decision = ManualE2eGateDecision::Allow;
    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("decision mismatch"))
    );
}

#[test]
fn run_manual_e2e_pipeline_returns_parse_errors() {
    let plan = ManualE2ePlan {
        scenario: "manual-e2e".to_string(),
        command: "oya run manual-e2e".to_string(),
        raw_output: "not json".to_string(),
    };

    let result = run_manual_e2e_pipeline(&plan);
    assert!(matches!(result, Err(ManualE2eError::InvalidJson(_))));
}

#[test]
fn validate_manual_e2e_report_rejects_unexpected_stage_count() {
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![stage_report(
            ManualE2eStageName::ScenarioSetup,
            ManualE2eStageStatus::Passed,
            "scenario prepared",
        )],
        decision: ManualE2eGateDecision::Allow,
    };

    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("unexpected stage count"))
    );
}

#[test]
fn validate_manual_e2e_report_rejects_invalid_stage_order() {
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![
            stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            ),
            stage_report(ManualE2eStageName::OutputParsing, ManualE2eStageStatus::Passed, "parsed"),
            stage_report(
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageStatus::Passed,
                "invoked",
            ),
            stage_report(
                ManualE2eStageName::GateEvaluation,
                ManualE2eStageStatus::Passed,
                "gate open",
            ),
        ],
        decision: ManualE2eGateDecision::Allow,
    };

    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("invalid stage order"))
    );
}

#[test]
fn validate_manual_e2e_report_rejects_empty_stage_diagnostics() {
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![
            stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            ),
            stage_report(
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageStatus::Passed,
                "invoked",
            ),
            stage_report(ManualE2eStageName::OutputParsing, ManualE2eStageStatus::Passed, ""),
            stage_report(
                ManualE2eStageName::GateEvaluation,
                ManualE2eStageStatus::Passed,
                "gate open",
            ),
        ],
        decision: ManualE2eGateDecision::Allow,
    };

    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("empty stage diagnostics"))
    );
}

#[test]
fn validate_manual_e2e_report_rejects_oversized_stage_diagnostics() {
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![
            stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            ),
            stage_report(
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageStatus::Passed,
                "invoked",
            ),
            stage_report(
                ManualE2eStageName::OutputParsing,
                ManualE2eStageStatus::Passed,
                &"d".repeat(MAX_MANUAL_E2E_DIAGNOSTICS_LEN + 1),
            ),
            stage_report(
                ManualE2eStageName::GateEvaluation,
                ManualE2eStageStatus::Passed,
                "gate open",
            ),
        ],
        decision: ManualE2eGateDecision::Allow,
    };

    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("stage diagnostics exceed max length"))
    );
}

#[test]
fn validate_manual_e2e_report_rejects_invalid_stage_diagnostics_content() {
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![
            stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            ),
            stage_report(
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageStatus::Passed,
                "invoked",
            ),
            stage_report(
                ManualE2eStageName::OutputParsing,
                ManualE2eStageStatus::Passed,
                "bad\u{0000}data",
            ),
            stage_report(
                ManualE2eStageName::GateEvaluation,
                ManualE2eStageStatus::Passed,
                "gate open",
            ),
        ],
        decision: ManualE2eGateDecision::Allow,
    };

    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("stage diagnostics contain invalid control characters"))
    );
}

#[test]
fn validate_manual_e2e_report_rejects_non_monotonic_stage_timestamps() {
    let base_time = Utc::now();
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![
            ManualE2eStageReport {
                stage: ManualE2eStageName::ScenarioSetup,
                status: ManualE2eStageStatus::Passed,
                diagnostics: "scenario prepared".to_string(),
                timestamp: base_time,
            },
            ManualE2eStageReport {
                stage: ManualE2eStageName::CommandInvocation,
                status: ManualE2eStageStatus::Passed,
                diagnostics: "invoked".to_string(),
                timestamp: base_time - chrono::Duration::milliseconds(1),
            },
            ManualE2eStageReport {
                stage: ManualE2eStageName::OutputParsing,
                status: ManualE2eStageStatus::Passed,
                diagnostics: "parsed".to_string(),
                timestamp: base_time,
            },
            ManualE2eStageReport {
                stage: ManualE2eStageName::GateEvaluation,
                status: ManualE2eStageStatus::Passed,
                diagnostics: "gate open".to_string(),
                timestamp: base_time,
            },
        ],
        decision: ManualE2eGateDecision::Allow,
    };

    assert_eq!(
        validate_manual_e2e_report(&report),
        Err(ManualE2eError::InvalidReport("non-monotonic stage timestamps"))
    );
}

#[test]
fn derive_manual_e2e_gate_blocks_when_stage_has_error_status() {
    let report = ManualE2eReport {
        plan: ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        },
        output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
        stages: vec![
            stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            ),
            stage_report(
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageStatus::Passed,
                "invoked",
            ),
            stage_report(
                ManualE2eStageName::OutputParsing,
                ManualE2eStageStatus::Error,
                "parse adapter crash",
            ),
            stage_report(
                ManualE2eStageName::GateEvaluation,
                ManualE2eStageStatus::Passed,
                "gate open",
            ),
        ],
        decision: ManualE2eGateDecision::Block,
    };

    assert_eq!(derive_manual_e2e_gate(&report), ManualE2eGateDecision::Block);
}

#[test]
fn verify_state_typing_rejects_empty_container_id() {
    let state = DockerState {
        container_id: String::new(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Err(DockerFixError::EmptyStateField("container_id")));
}

#[test]
fn verify_state_typing_rejects_empty_image() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: String::new(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Err(DockerFixError::EmptyStateField("image")));
}

#[test]
fn verify_state_typing_accepts_valid_state() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "nginx:latest".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_accepts_none_port() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "nginx:latest".to_string(),
        port: None,
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_rejects_whitespace_only_container_id() {
    let state = DockerState {
        container_id: "   ".to_string(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Err(DockerFixError::EmptyStateField("container_id")));
}

#[test]
fn verify_state_typing_rejects_whitespace_only_image() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "\t\n".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Err(DockerFixError::EmptyStateField("image")));
}

#[test]
fn verify_state_typing_accepts_container_id_with_unicode() {
    let state = DockerState {
        container_id: "abc123-тест-🐳".to_string(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_accepts_image_with_unicode() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "nginx:latest-тест-🐳".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_trims_container_id_whitespace() {
    let state = DockerState {
        container_id: "  abc123  ".to_string(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_trims_image_whitespace() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "  nginx:latest  ".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_accepts_all_status_variants_running() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    assert_eq!(verify_state_typing(&state), Ok(()));
}

#[test]
fn verify_state_typing_accepts_all_status_variants_stopped() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Stopped,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    assert_eq!(verify_state_typing(&state), Ok(()));
}

#[test]
fn verify_state_typing_accepts_all_status_variants_exited() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Exited,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    assert_eq!(verify_state_typing(&state), Ok(()));
}

#[test]
fn verify_state_typing_accepts_all_status_variants_created() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Created,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    assert_eq!(verify_state_typing(&state), Ok(()));
}

#[test]
fn verify_state_typing_accepts_port_boundary_min() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(1),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_accepts_port_boundary_max() {
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(u16::MAX),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_accepts_common_ports() {
    let common_ports = vec![80, 443, 8080, 3000, 5000, 5432, 6379, 27017];
    for port in common_ports {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(port),
        };
        assert_eq!(verify_state_typing(&state), Ok(()), "port {}", port);
    }
}

#[test]
fn verify_state_typing_accepts_very_long_container_id() {
    let long_id = "a".repeat(1000);
    let state = DockerState {
        container_id: long_id,
        status: ContainerStatus::Running,
        image: "nginx".to_string(),
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn verify_state_typing_accepts_very_long_image_name() {
    let long_image = format!("{}:{}", "n".repeat(500), "v".repeat(100));
    let state = DockerState {
        container_id: "abc123".to_string(),
        status: ContainerStatus::Running,
        image: long_image,
        port: Some(8080),
    };
    let result = verify_state_typing(&state);
    assert_eq!(result, Ok(()));
}

#[test]
fn resolve_moon_path_rejects_empty_task_name() {
    let result = resolve_moon_path("");
    assert!(matches!(result, Err(DockerFixError::MoonTaskNotFound(_))));
}

#[test]
fn resolve_moon_path_returns_absolute_path_for_known_task() {
    let result = resolve_moon_path(":test");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert_eq!(path.task_name, ":test");
    assert!(path.absolute_path.is_absolute());
}

#[test]
fn resolve_moon_path_rejects_whitespace_only_task_name() {
    let result = resolve_moon_path("  \t\n ");
    assert!(matches!(result, Err(DockerFixError::MoonTaskNotFound(_))));
}

#[test]
fn resolve_moon_path_rejects_multiple_empty_strings() {
    let result = resolve_moon_path("   ");
    assert!(matches!(result, Err(DockerFixError::MoonTaskNotFound(_))));
}

#[test]
fn resolve_moon_path_strips_leading_colon() {
    let result = resolve_moon_path(":test");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert_eq!(path.task_name, ":test");
}

#[test]
fn resolve_moon_path_handles_multiple_leading_colons() {
    let result = resolve_moon_path("::test");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert_eq!(path.task_name, "::test");
}

#[test]
fn resolve_moon_path_accepts_task_name_with_dashes() {
    let result = resolve_moon_path("my-task");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert_eq!(path.task_name, "my-task");
}

#[test]
fn resolve_moon_path_accepts_task_name_with_underscores() {
    let result = resolve_moon_path("my_task");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert_eq!(path.task_name, "my_task");
}

#[test]
fn resolve_moon_path_accepts_task_name_with_numbers() {
    let result = resolve_moon_path("task123");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert_eq!(path.task_name, "task123");
}

#[test]
fn resolve_moon_path_returns_absolute_path() {
    let result = resolve_moon_path("test");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert!(path.absolute_path.is_absolute());
}

#[test]
fn resolve_moon_path_includes_task_name_in_path() {
    let result = resolve_moon_path("mytask");
    assert!(result.is_ok());
    let Ok(path) = result else { return };
    assert!(path.absolute_path.to_string_lossy().contains("mytask"));
}

#[test]
fn resolve_moon_path_rejects_path_traversal_and_absolute_paths() {
    let traversal_result = resolve_moon_path("../etc/passwd");
    assert_eq!(
        traversal_result,
        Err(DockerFixError::ConfigValidationFailed("moon task name contains invalid characters"))
    );

    let absolute_result = resolve_moon_path("/tmp/evil");
    assert_eq!(
        absolute_result,
        Err(DockerFixError::ConfigValidationFailed("moon task name contains invalid characters"))
    );

    let backslash_result = resolve_moon_path("..\\windows\\system32");
    assert_eq!(
        backslash_result,
        Err(DockerFixError::ConfigValidationFailed("moon task name contains invalid characters"))
    );
}

#[test]
fn resolve_moon_path_rejects_separator_only_and_oversized_names() {
    let separator_only_result = resolve_moon_path(":::");
    assert_eq!(
        separator_only_result,
        Err(DockerFixError::ConfigValidationFailed("moon task name is empty after normalization"))
    );

    let oversized_name = "a".repeat(MAX_MOON_TASK_NAME_LEN + 1);
    let oversized_result = resolve_moon_path(&oversized_name);
    assert_eq!(
        oversized_result,
        Err(DockerFixError::ConfigValidationFailed("moon task name exceeds max length"))
    );
}

#[test]
fn resolve_moon_path_rejects_malformed_task_names() {
    let malformed_cases = [
        "task name",
        "task\nname",
        "task\tname",
        "task;rm-rf",
        "task|pipe",
        "task*glob",
        "task?query",
    ];

    for malformed in malformed_cases {
        let result = resolve_moon_path(malformed);
        assert_eq!(
            result,
            Err(DockerFixError::ConfigValidationFailed(
                "moon task name contains invalid characters"
            ))
        );
    }
}

#[test]
fn validate_docker_config_rejects_empty_image_name() {
    let config = DockerConfig {
        image_name: String::new(),
        tag: Some("latest".to_string()),
        port_bindings: vec![8080],
        environment: vec!["RUST_LOG=debug".to_string()],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Err(DockerFixError::EmptyConfigField("image_name")));
}

#[test]
fn validate_docker_config_accepts_valid_config() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: Some("latest".to_string()),
        port_bindings: vec![80],
        environment: vec!["ENV=prod".to_string()],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_config_without_tag() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_rejects_whitespace_only_image_name() {
    let config = DockerConfig {
        image_name: "  \t\n ".to_string(),
        tag: Some("latest".to_string()),
        port_bindings: vec![8080],
        environment: vec!["RUST_LOG=debug".to_string()],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Err(DockerFixError::EmptyConfigField("image_name")));
}

#[test]
fn validate_docker_config_rejects_image_name_with_control_chars() {
    let config = DockerConfig {
        image_name: "nginx\u{0000}latest".to_string(),
        tag: None,
        port_bindings: vec![8080],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Err(DockerFixError::TypeConstraintViolation("image_name")));
}

#[test]
fn validate_docker_config_rejects_image_name_with_other_control_chars() {
    let control_chars = vec!['\x01', '\x02', '\x07', '\x1B'];
    for c in control_chars {
        let config = DockerConfig {
            image_name: format!("nginx{}latest", c),
            tag: None,
            port_bindings: vec![8080],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Err(DockerFixError::TypeConstraintViolation("image_name")));
    }
}

#[test]
fn validate_docker_config_accepts_allowed_whitespace_in_image_name() {
    let config = DockerConfig {
        image_name: "  nginx:latest  ".to_string(),
        tag: None,
        port_bindings: vec![8080],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_tag_without_trimming() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: Some("  latest  ".to_string()),
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_empty_tag() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: Some(String::new()),
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_port_boundary_min() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![1],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_port_boundary_max() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![u16::MAX],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_multiple_port_bindings() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![80, 443, 8080],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_common_ports() {
    let common_ports = vec![22, 80, 443, 3306, 5432, 6379, 27017, 8080];
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: common_ports,
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_valid_environment_variables() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![80],
        environment: vec![
            "RUST_LOG=debug".to_string(),
            "NODE_ENV=production".to_string(),
            "DATABASE_URL=postgresql://localhost".to_string(),
        ],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_empty_environment() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_environment_with_equals_in_value() {
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: vec![80],
        environment: vec!["PATH=/usr/local/bin:/usr/bin".to_string()],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_image_name_with_registry() {
    let config = DockerConfig {
        image_name: "docker.io/library/nginx".to_string(),
        tag: None,
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_image_name_with_special_chars() {
    let config = DockerConfig {
        image_name: "my-registry.io/my-org/my_image:v1.2.3".to_string(),
        tag: None,
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_unicode_in_image_name() {
    let config = DockerConfig {
        image_name: "nginx:тест-🐳".to_string(),
        tag: None,
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_very_long_image_name() {
    let long_name = format!("{}/{}", "registry.io".repeat(10), "n".repeat(500));
    let config = DockerConfig {
        image_name: long_name,
        tag: None,
        port_bindings: vec![80],
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_docker_config_accepts_many_port_bindings() {
    let ports: Vec<u16> = (8000..8050).collect();
    let config = DockerConfig {
        image_name: "nginx".to_string(),
        tag: None,
        port_bindings: ports,
        environment: vec![],
    };
    let result = validate_docker_config(&config);
    assert_eq!(result, Ok(()));
}

#[test]
fn build_onewf_bead_quick_plan_rejects_empty_fields() {
    let input = OnewfBeadQuickInput {
        workflow_id: "   ".to_string(),
        bead_id: "bead-1".to_string(),
        endpoint: "http://localhost:8080/endpoint".to_string(),
    };
    assert_eq!(
        build_onewf_bead_quick_plan(&input),
        Err(OnewfBeadQuickError::EmptyField("workflow_id"))
    );

    let input = OnewfBeadQuickInput {
        workflow_id: "workflow-1".to_string(),
        bead_id: " ".to_string(),
        endpoint: "http://localhost:8080/endpoint".to_string(),
    };
    assert_eq!(
        build_onewf_bead_quick_plan(&input),
        Err(OnewfBeadQuickError::EmptyField("bead_id"))
    );

    let input = OnewfBeadQuickInput {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-1".to_string(),
        endpoint: " ".to_string(),
    };
    assert_eq!(
        build_onewf_bead_quick_plan(&input),
        Err(OnewfBeadQuickError::EmptyField("endpoint"))
    );
}

#[test]
fn build_onewf_bead_quick_plan_rejects_invalid_identifiers_and_endpoint() {
    let invalid_identifier = OnewfBeadQuickInput {
        workflow_id: "workflow/1".to_string(),
        bead_id: "bead-1".to_string(),
        endpoint: "http://localhost:8080/endpoint".to_string(),
    };
    assert_eq!(
        build_onewf_bead_quick_plan(&invalid_identifier),
        Err(OnewfBeadQuickError::InvalidIdentifier("workflow_id"))
    );

    let invalid_endpoint = OnewfBeadQuickInput {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-1".to_string(),
        endpoint: "ftp://localhost:8080/endpoint".to_string(),
    };
    assert_eq!(
        build_onewf_bead_quick_plan(&invalid_endpoint),
        Err(OnewfBeadQuickError::InvalidEndpoint)
    );
}

#[test]
fn run_onewf_bead_quick_check_emits_single_visible_successful_check() {
    let plan_result = build_onewf_bead_quick_plan(&OnewfBeadQuickInput {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-quick-1".to_string(),
        endpoint: "http://localhost:8080/one-endpoint".to_string(),
    });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let observation_result = run_onewf_bead_quick_check(&plan);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    assert_eq!(observation.workflow_id, "workflow-1");
    assert_eq!(observation.bead_id, "bead-quick-1");
    assert_eq!(observation.checks.len(), 1);
    assert_eq!(observation.checks[0].endpoint, "http://localhost:8080/one-endpoint");
    assert!(observation.checks[0].visible);
    assert!(observation.checks[0].success);
    assert_eq!(
        observation.checks[0].diagnostics,
        "endpoint visible and probe succeeded".to_string()
    );
}

#[test]
fn run_onewf_bead_quick_check_marks_probe_failure_for_fail_endpoint() {
    let plan_result = build_onewf_bead_quick_plan(&OnewfBeadQuickInput {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-quick-2".to_string(),
        endpoint: "http://localhost:8080/one-endpoint?fail=true".to_string(),
    });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let observation_result = run_onewf_bead_quick_check(&plan);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    assert_eq!(observation.checks.len(), 1);
    assert!(observation.checks[0].visible);
    assert!(!observation.checks[0].success);
    assert_eq!(observation.checks[0].diagnostics, "endpoint probe failed");
}

#[test]
fn evaluate_onewf_bead_quick_result_generates_ordered_report_and_pass_decision() {
    let check = OnewfBeadQuickCheck {
        endpoint: "http://localhost:8080/one-endpoint".to_string(),
        visible: true,
        success: true,
        diagnostics: "endpoint visible and probe succeeded".to_string(),
        timestamp: Utc::now(),
    };
    let observation = OnewfBeadQuickObservation {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-quick-3".to_string(),
        checks: vec![check],
    };

    let report_result = evaluate_onewf_bead_quick_result(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else { return };

    let stage_order = report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>();
    assert_eq!(
        stage_order,
        vec![
            OnewfBeadQuickStageName::EndpointVisibility,
            OnewfBeadQuickStageName::EndpointProbe,
            OnewfBeadQuickStageName::FinalDecision,
        ]
    );
    assert_eq!(report.decision, OnewfBeadQuickDecision::Pass);
    assert_eq!(validate_onewf_bead_quick_report(&report), Ok(()));
}

#[test]
fn evaluate_onewf_bead_quick_result_fails_when_endpoint_not_visible() {
    let check = OnewfBeadQuickCheck {
        endpoint: "http://localhost:8080/one-endpoint/hidden".to_string(),
        visible: false,
        success: false,
        diagnostics: "endpoint not visible".to_string(),
        timestamp: Utc::now(),
    };
    let observation = OnewfBeadQuickObservation {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-quick-4".to_string(),
        checks: vec![check],
    };

    let report_result = evaluate_onewf_bead_quick_result(&observation);
    assert!(report_result.is_err());
    assert_eq!(
        report_result,
        Err(OnewfBeadQuickError::InvalidReport("single-endpoint visibility contract violated"))
    );
}

#[test]
fn validate_onewf_bead_quick_report_rejects_non_monotonic_timestamps() {
    let base = Utc::now();
    let report = OnewfBeadQuickReport {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-quick-5".to_string(),
        checks: vec![OnewfBeadQuickCheck {
            endpoint: "http://localhost:8080/one-endpoint".to_string(),
            visible: true,
            success: true,
            diagnostics: "ok".to_string(),
            timestamp: base,
        }],
        stages: vec![
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::EndpointVisibility,
                status: OnewfBeadQuickStageStatus::Passed,
                diagnostics: "visible".to_string(),
                timestamp: base,
            },
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::EndpointProbe,
                status: OnewfBeadQuickStageStatus::Passed,
                diagnostics: "probe passed".to_string(),
                timestamp: base - Duration::milliseconds(1),
            },
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::FinalDecision,
                status: OnewfBeadQuickStageStatus::Passed,
                diagnostics: "gate passed".to_string(),
                timestamp: base,
            },
        ],
        decision: OnewfBeadQuickDecision::Pass,
    };

    assert_eq!(
        validate_onewf_bead_quick_report(&report),
        Err(OnewfBeadQuickError::InvalidReport("non-monotonic stage timestamps"))
    );
}

#[test]
fn validate_onewf_bead_quick_report_rejects_decision_mismatch() {
    let base = Utc::now();
    let report = OnewfBeadQuickReport {
        workflow_id: "workflow-1".to_string(),
        bead_id: "bead-quick-6".to_string(),
        checks: vec![OnewfBeadQuickCheck {
            endpoint: "http://localhost:8080/one-endpoint".to_string(),
            visible: true,
            success: false,
            diagnostics: "probe failed".to_string(),
            timestamp: base,
        }],
        stages: vec![
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::EndpointVisibility,
                status: OnewfBeadQuickStageStatus::Passed,
                diagnostics: "visible".to_string(),
                timestamp: base,
            },
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::EndpointProbe,
                status: OnewfBeadQuickStageStatus::Failed,
                diagnostics: "probe failed".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::FinalDecision,
                status: OnewfBeadQuickStageStatus::Passed,
                diagnostics: "gate passed".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
        ],
        decision: OnewfBeadQuickDecision::Pass,
    };

    assert_eq!(
        validate_onewf_bead_quick_report(&report),
        Err(OnewfBeadQuickError::InvalidReport("decision mismatch"))
    );
}

fn make_valid_src_kes_report() -> SrcKesReport {
    let plan_result = build_src_kes_plan(&SrcKesInput { service_name: "src-kes-api".to_string() });
    let plan = match plan_result {
        Ok(value) => value,
        Err(_) => SrcKesPlan {
            service_name: "src-kes-api".to_string(),
            framework: "scotty".to_string(),
            resource: "user".to_string(),
            routes: register_user_routes(),
        },
    };
    let base = Utc::now();

    SrcKesReport {
        plan,
        runtime_started: true,
        deterministic_behavior: true,
        stages: vec![
            SrcKesStageReport {
                stage: SrcKesStageName::PlanBuild,
                status: SrcKesStageStatus::Passed,
                diagnostics: "plan built".to_string(),
                timestamp: base,
            },
            SrcKesStageReport {
                stage: SrcKesStageName::RuntimeStart,
                status: SrcKesStageStatus::Passed,
                diagnostics: "runtime started".to_string(),
                timestamp: base + Duration::milliseconds(1),
            },
            SrcKesStageReport {
                stage: SrcKesStageName::RouteContract,
                status: SrcKesStageStatus::Passed,
                diagnostics: "routes registered".to_string(),
                timestamp: base + Duration::milliseconds(2),
            },
            SrcKesStageReport {
                stage: SrcKesStageName::CrudContract,
                status: SrcKesStageStatus::Passed,
                diagnostics: "crud behavior valid".to_string(),
                timestamp: base + Duration::milliseconds(3),
            },
            SrcKesStageReport {
                stage: SrcKesStageName::FinalDecision,
                status: SrcKesStageStatus::Passed,
                diagnostics: "contract passed".to_string(),
                timestamp: base + Duration::milliseconds(4),
            },
        ],
        decision: SrcKesDecision::Pass,
    }
}

#[test]
fn build_src_kes_plan_sets_scotty_contract() {
    let result = build_src_kes_plan(&SrcKesInput { service_name: "  src-kes-api  ".to_string() });
    assert!(result.is_ok());
    let Ok(plan) = result else { return };

    assert_eq!(plan.service_name, "src-kes-api");
    assert_eq!(plan.framework, "scotty");
    assert_eq!(plan.resource, "user");
    assert_eq!(plan.routes, register_user_routes());
}

#[test]
fn src_kes_plan_and_route_contract_are_deterministic_for_same_input() {
    let input = SrcKesInput { service_name: "src-kes-api".to_string() };

    let first_result = build_src_kes_plan(&input);
    let second_result = build_src_kes_plan(&input);
    assert!(first_result.is_ok());
    assert!(second_result.is_ok());

    let Ok(first_plan) = first_result else { return };
    let Ok(second_plan) = second_result else { return };

    assert_eq!(first_plan, second_plan);
    assert_eq!(first_plan.routes, register_user_routes());
}

#[test]
fn register_user_routes_includes_exact_crud_contract() {
    let routes = register_user_routes();

    assert_eq!(
        routes,
        vec![
            SrcKesRouteContract {
                method: SrcKesRouteMethod::Post,
                path: "/users".to_string(),
                success_status: 201,
            },
            SrcKesRouteContract {
                method: SrcKesRouteMethod::Get,
                path: "/users/:id".to_string(),
                success_status: 200,
            },
            SrcKesRouteContract {
                method: SrcKesRouteMethod::Put,
                path: "/users/:id".to_string(),
                success_status: 200,
            },
            SrcKesRouteContract {
                method: SrcKesRouteMethod::Delete,
                path: "/users/:id".to_string(),
                success_status: 204,
            },
        ]
    );
}

#[test]
fn start_src_kes_server_rejects_resource_contract_mismatch() {
    let plan_result = build_src_kes_plan(&SrcKesInput { service_name: "src-kes-api".to_string() });
    assert!(plan_result.is_ok());
    let Ok(mut plan) = plan_result else { return };
    plan.resource = "account".to_string();

    assert_eq!(start_src_kes_server(&plan), Err(SrcKesError::InvalidFieldFormat("resource")));
}

#[test]
fn src_kes_user_crud_operations_report_user_not_found_for_missing_ids() {
    let state = SrcKesServiceState::default();

    assert_eq!(
        run_user_read(&state, "user-missing"),
        Err(SrcKesError::UserNotFound("user-missing".to_string()))
    );
    assert_eq!(
        run_user_update(
            &state,
            "user-missing",
            &UserUpdateRequest { name: "Ada".to_string(), email: "ada@example.com".to_string() },
        ),
        Err(SrcKesError::UserNotFound("user-missing".to_string()))
    );
    assert_eq!(
        run_user_delete(&state, "user-missing"),
        Err(SrcKesError::UserNotFound("user-missing".to_string()))
    );
}

#[test]
fn src_kes_user_crud_flow_is_deterministic() {
    let initial = SrcKesServiceState::default();

    let create_result = run_user_create(
        &initial,
        &UserCreateRequest { name: "Ada".to_string(), email: "ADA@Example.com".to_string() },
    );
    assert!(create_result.is_ok());
    let Ok((created_state, created_user)) = create_result else {
        return;
    };
    assert_eq!(created_user.id, "user-ada-example-com");

    let read_result = run_user_read(&created_state, "user-ada-example-com");
    assert_eq!(read_result, Ok(created_user.clone()));

    let update_result = run_user_update(
        &created_state,
        "user-ada-example-com",
        &UserUpdateRequest {
            name: "Ada Lovelace".to_string(),
            email: "ada.lovelace@example.com".to_string(),
        },
    );
    assert!(update_result.is_ok());
    let Ok((updated_state, updated_user)) = update_result else {
        return;
    };
    assert_eq!(updated_user.id, "user-ada-example-com");
    assert_eq!(updated_user.email, "ada.lovelace@example.com");

    let delete_result = run_user_delete(&updated_state, "user-ada-example-com");
    assert!(delete_result.is_ok());
    let Ok(deleted_state) = delete_result else { return };
    assert_eq!(deleted_state.users.len(), 0);
    assert_eq!(
        run_user_read(&deleted_state, "user-ada-example-com"),
        Err(SrcKesError::UserNotFound("user-ada-example-com".to_string()))
    );
}

#[test]
fn run_user_create_rejects_invalid_payload_and_duplicate_user() {
    let initial = SrcKesServiceState::default();
    let invalid_result = run_user_create(
        &initial,
        &UserCreateRequest { name: "Ada".to_string(), email: "not-an-email".to_string() },
    );
    assert_eq!(invalid_result, Err(SrcKesError::InvalidFieldFormat("email")));

    let first_create_result = run_user_create(
        &initial,
        &UserCreateRequest { name: "Ada".to_string(), email: "ada@example.com".to_string() },
    );
    assert!(first_create_result.is_ok());
    let Ok((created_state, _)) = first_create_result else {
        return;
    };

    let duplicate_result = run_user_create(
        &created_state,
        &UserCreateRequest { name: "Ada 2".to_string(), email: "ADA@example.com".to_string() },
    );
    assert_eq!(
        duplicate_result,
        Err(SrcKesError::DuplicateUserId("user-ada-example-com".to_string()))
    );
}

#[test]
fn validate_src_kes_report_rejects_decision_mismatch() {
    let mut report = make_valid_src_kes_report();
    report.decision = SrcKesDecision::Fail;

    assert_eq!(
        validate_src_kes_report(&report),
        Err(SrcKesError::InvalidReport("decision mismatch"))
    );
}

#[test]
fn validate_src_kes_report_rejects_non_monotonic_timestamps() {
    let mut report = make_valid_src_kes_report();
    report.stages[2].timestamp = report.stages[1].timestamp - Duration::milliseconds(1);

    assert_eq!(
        validate_src_kes_report(&report),
        Err(SrcKesError::InvalidReport("non-monotonic stage timestamps"))
    );
}

#[test]
fn build_src_kes_plan_rejects_invalid_service_name_inputs() {
    assert_eq!(
        build_src_kes_plan(&SrcKesInput { service_name: "   ".to_string() }),
        Err(SrcKesError::EmptyField("service_name"))
    );

    assert_eq!(
        build_src_kes_plan(&SrcKesInput { service_name: "a".repeat(65) }),
        Err(SrcKesError::FieldTooLong("service_name", 64))
    );

    let invalid_content = format!("src{}kes-api", '\u{0007}');
    assert_eq!(
        build_src_kes_plan(&SrcKesInput { service_name: invalid_content }),
        Err(SrcKesError::InvalidFieldContent("service_name"))
    );

    assert_eq!(
        build_src_kes_plan(&SrcKesInput { service_name: "src-kes\napi".to_string() }),
        Err(SrcKesError::InvalidFieldContent("service_name"))
    );

    let boundary_name = "a".repeat(64);
    assert!(build_src_kes_plan(&SrcKesInput { service_name: boundary_name }).is_ok());
}

#[test]
fn start_src_kes_server_rejects_framework_and_route_contract_mismatch() {
    let plan_result = build_src_kes_plan(&SrcKesInput { service_name: "src-kes-api".to_string() });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else { return };

    let mut bad_framework = plan.clone();
    bad_framework.framework = "axum".to_string();
    assert_eq!(
        start_src_kes_server(&bad_framework),
        Err(SrcKesError::InvalidFieldFormat("framework"))
    );

    let mut bad_routes = plan;
    bad_routes.routes = vec![];
    assert_eq!(start_src_kes_server(&bad_routes), Err(SrcKesError::InvalidRouteContract));
}

#[test]
fn run_user_crud_rejects_invalid_user_id_format() {
    let state = SrcKesServiceState::default();

    assert_eq!(
        run_user_read(&state, "user invalid"),
        Err(SrcKesError::InvalidFieldFormat("user_id"))
    );
    assert_eq!(
        run_user_update(
            &state,
            "user invalid",
            &UserUpdateRequest { name: "Ada".to_string(), email: "ada@example.com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldFormat("user_id"))
    );
    assert_eq!(
        run_user_delete(&state, "user invalid"),
        Err(SrcKesError::InvalidFieldFormat("user_id"))
    );
}

#[test]
fn run_user_create_and_update_reject_invalid_payload_edges() {
    let initial = SrcKesServiceState::default();
    let invalid_name = format!("Ada{}Lovelace", '\u{0007}');
    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: invalid_name, email: "ada@example.com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldContent("name"))
    );

    let long_local = "a".repeat(100);
    let long_email = format!("{}@x.io", long_local);
    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: long_email },
        ),
        Err(SrcKesError::FieldTooLong("user_id", 96))
    );

    let created_result = run_user_create(
        &initial,
        &UserCreateRequest { name: "Ada".to_string(), email: "ada@example.com".to_string() },
    );
    assert!(created_result.is_ok());
    let Ok((created_state, _)) = created_result else {
        return;
    };

    assert_eq!(
        run_user_update(
            &created_state,
            "user-ada-example-com",
            &UserUpdateRequest { name: " ".to_string(), email: "ada@example.com".to_string() },
        ),
        Err(SrcKesError::EmptyField("name"))
    );
    assert_eq!(
        run_user_update(
            &created_state,
            "user-ada-example-com",
            &UserUpdateRequest { name: "Ada".to_string(), email: "ada@@example.com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldFormat("email"))
    );
}

#[test]
fn run_user_create_rejects_malformed_email_shapes() {
    let initial = SrcKesServiceState::default();

    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "a..da@example.com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldFormat("email"))
    );

    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "ada@example..com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldFormat("email"))
    );

    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "ada@-example.com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldFormat("email"))
    );
}

#[test]
fn run_user_create_rejects_control_character_injection() {
    let initial = SrcKesServiceState::default();

    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest {
                name: "Ada\nLovelace".to_string(),
                email: "ada@example.com".to_string()
            },
        ),
        Err(SrcKesError::InvalidFieldContent("name"))
    );

    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "ada\t@example.com".to_string() },
        ),
        Err(SrcKesError::InvalidFieldContent("email"))
    );
}

#[test]
fn run_user_create_enforces_user_id_length_boundary() {
    let initial = SrcKesServiceState::default();
    let max_local = "a".repeat(86);
    let max_email = format!("{}@x.io", max_local);
    let max_create =
        run_user_create(&initial, &UserCreateRequest { name: "Ada".to_string(), email: max_email });
    assert!(max_create.is_ok());

    let overflow_local = "a".repeat(87);
    let overflow_email = format!("{}@x.io", overflow_local);
    assert_eq!(
        run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: overflow_email },
        ),
        Err(SrcKesError::FieldTooLong("user_id", 96))
    );
}

#[test]
fn validate_src_kes_report_rejects_runtime_and_determinism_flags() {
    let mut runtime_missing = make_valid_src_kes_report();
    runtime_missing.runtime_started = false;
    assert_eq!(
        validate_src_kes_report(&runtime_missing),
        Err(SrcKesError::InvalidReport("runtime not started"))
    );

    let mut non_deterministic = make_valid_src_kes_report();
    non_deterministic.deterministic_behavior = false;
    assert_eq!(
        validate_src_kes_report(&non_deterministic),
        Err(SrcKesError::InvalidReport("deterministic behavior violated"))
    );
}

#[test]
fn validate_src_kes_report_rejects_plan_and_stage_contract_errors() {
    let mut bad_framework = make_valid_src_kes_report();
    bad_framework.plan.framework = "axum".to_string();
    assert_eq!(
        validate_src_kes_report(&bad_framework),
        Err(SrcKesError::InvalidReport("framework must be scotty"))
    );

    let mut bad_resource = make_valid_src_kes_report();
    bad_resource.plan.resource = "account".to_string();
    assert_eq!(
        validate_src_kes_report(&bad_resource),
        Err(SrcKesError::InvalidReport("resource must be user"))
    );

    let mut bad_routes = make_valid_src_kes_report();
    bad_routes.plan.routes = vec![];
    assert_eq!(validate_src_kes_report(&bad_routes), Err(SrcKesError::InvalidRouteContract));

    let mut bad_stage_count = make_valid_src_kes_report();
    let _ = bad_stage_count.stages.pop();
    assert_eq!(
        validate_src_kes_report(&bad_stage_count),
        Err(SrcKesError::InvalidReport("unexpected stage count"))
    );

    let mut bad_stage_order = make_valid_src_kes_report();
    bad_stage_order.stages.swap(0, 1);
    assert_eq!(
        validate_src_kes_report(&bad_stage_order),
        Err(SrcKesError::InvalidReport("invalid stage order"))
    );

    let mut empty_diagnostics = make_valid_src_kes_report();
    empty_diagnostics.stages[1].diagnostics = "   ".to_string();
    assert_eq!(
        validate_src_kes_report(&empty_diagnostics),
        Err(SrcKesError::InvalidReport("empty stage diagnostics"))
    );
}

#[test]
fn validate_src_kes_report_accepts_fail_decision_when_stage_fails() {
    let mut report = make_valid_src_kes_report();
    report.stages[3].status = SrcKesStageStatus::Failed;
    report.decision = SrcKesDecision::Fail;

    assert_eq!(validate_src_kes_report(&report), Ok(()));
}

#[test]
fn build_test_trace_final_plan_rejects_empty_fields() {
    let result = build_test_trace_final_plan(&TestTraceFinalInput {
        workflow_id: "   ".to_string(),
        trace_id: "trace-001".to_string(),
        stage_name: "final".to_string(),
    });

    assert_eq!(result, Err(TestTraceFinalError::EmptyField("workflow_id")));
}

#[test]
fn build_test_trace_final_plan_rejects_oversized_inputs() {
    let result = build_test_trace_final_plan(&TestTraceFinalInput {
        workflow_id: "w".repeat(MAX_TEST_TRACE_FINAL_WORKFLOW_ID_LEN + 1),
        trace_id: "trace-001".to_string(),
        stage_name: "final".to_string(),
    });

    assert_eq!(
        result,
        Err(
            TestTraceFinalError::FieldTooLong("workflow_id", MAX_TEST_TRACE_FINAL_WORKFLOW_ID_LEN,)
        )
    );
}

#[test]
fn build_test_trace_final_plan_rejects_invalid_control_characters() {
    let result = build_test_trace_final_plan(&TestTraceFinalInput {
        workflow_id: "wf-001".to_string(),
        trace_id: "trace\u{0007}-001".to_string(),
        stage_name: "final".to_string(),
    });

    assert_eq!(result, Err(TestTraceFinalError::InvalidFieldContent("trace_id")));
}

#[test]
fn collect_test_trace_final_observation_emits_ordered_checks_and_monotonic_timestamps() {
    let plan_result = build_test_trace_final_plan(&TestTraceFinalInput {
        workflow_id: "wf-001".to_string(),
        trace_id: "trace-001".to_string(),
        stage_name: "final".to_string(),
    });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else {
        return;
    };

    let observation_result = collect_test_trace_final_observation(&plan);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    assert_eq!(observation.checks.len(), 3);
    assert_eq!(observation.checks[0].check, TestTraceFinalCheckName::PlanContract);
    assert_eq!(observation.checks[1].check, TestTraceFinalCheckName::TraceCollection);
    assert_eq!(observation.checks[2].check, TestTraceFinalCheckName::FinalGateSignal);
    assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
    assert!(observation.checks.windows(2).all(|pair| pair[0].timestamp <= pair[1].timestamp));
}

#[test]
fn evaluate_test_trace_final_report_preserves_stage_order_and_validates_timestamps() {
    let plan_result = build_test_trace_final_plan(&TestTraceFinalInput {
        workflow_id: "wf-002".to_string(),
        trace_id: "trace-002".to_string(),
        stage_name: "final".to_string(),
    });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else {
        return;
    };

    let observation_result = collect_test_trace_final_observation(&plan);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    let report_result = evaluate_test_trace_final_report(&observation);
    assert!(report_result.is_ok());
    let Ok(report) = report_result else {
        return;
    };

    assert_eq!(report.stages.len(), 3);
    assert_eq!(report.stages[0].stage, TestTraceFinalStageName::PlanContract);
    assert_eq!(report.stages[1].stage, TestTraceFinalStageName::TraceCollection);
    assert_eq!(report.stages[2].stage, TestTraceFinalStageName::FinalDecision);
    assert!(report.stages.windows(2).all(|pair| pair[0].timestamp <= pair[1].timestamp));
    assert_eq!(validate_test_trace_final_report(&report), Ok(()));
}

#[test]
fn validate_test_trace_final_report_rejects_decision_mismatch() {
    let plan_result = build_test_trace_final_plan(&TestTraceFinalInput {
        workflow_id: "wf-003".to_string(),
        trace_id: "trace-003".to_string(),
        stage_name: "final".to_string(),
    });
    assert!(plan_result.is_ok());
    let Ok(plan) = plan_result else {
        return;
    };

    let observation_result = collect_test_trace_final_observation(&plan);
    assert!(observation_result.is_ok());
    let Ok(observation) = observation_result else {
        return;
    };

    let report_result = evaluate_test_trace_final_report(&observation);
    assert!(report_result.is_ok());
    let Ok(mut report) = report_result else {
        return;
    };
    report.decision = TestTraceFinalDecision::Fail;

    assert_eq!(
        validate_test_trace_final_report(&report),
        Err(TestTraceFinalError::InvalidReport("decision mismatch"))
    );
}
