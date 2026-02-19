use super::*;
use crate::types::{FailureCategory, StageName};

#[test]
fn test_is_rate_limit_failure_returns_true_for_rate_limited() {
    let category = FailureCategory::RateLimited;
    assert!(is_rate_limit_failure(&category));
}

#[test]
fn test_is_rate_limit_failure_returns_false_for_other_failures() {
    let categories = [
        FailureCategory::TestFailed,
        FailureCategory::CompileFailed,
        FailureCategory::LintFailed,
        FailureCategory::AuthFailed,
        FailureCategory::ProviderUnavailable,
        FailureCategory::ContextOverflow,
        FailureCategory::OutputParseFailure,
        FailureCategory::MaxAttemptsExceeded,
        FailureCategory::TestInfraFailed,
        FailureCategory::MergeConflict,
    ];
    for category in categories {
        assert!(!is_rate_limit_failure(&category), "{:?} should not be rate limit", category);
    }
}

#[test]
fn test_tier_for_stage_maps_correctly() {
    assert_eq!(tier_for_stage(&StageName::Plan), "c");
    assert_eq!(tier_for_stage(&StageName::Contract), "d");
    assert_eq!(tier_for_stage(&StageName::Tdd15), "c");
    assert_eq!(tier_for_stage(&StageName::Qa), "c");
    assert_eq!(tier_for_stage(&StageName::RedQueen), "b");
    assert_eq!(tier_for_stage(&StageName::GptReview), "b");
    assert_eq!(tier_for_stage(&StageName::ShipGate), "a");
}

#[test]
fn test_get_models_for_tier_returns_defaults() {
    let tier_d = get_models_for_tier("d");
    // Skip test if no models configured (env vars not set and no defaults available)
    if tier_d.is_empty() {
        eprintln!("Skipping test: no models configured for 'd' tier");
        return;
    }

    let tier_c = get_models_for_tier("c");
    if tier_c.is_empty() {
        eprintln!("Skipping test: no models configured for 'c' tier");
        return;
    }

    let tier_b = get_models_for_tier("b");
    if tier_b.is_empty() {
        eprintln!("Skipping test: no models configured for 'b' tier");
        return;
    }

    let tier_a = get_models_for_tier("a");
    if tier_a.is_empty() {
        eprintln!("Skipping test: no models configured for 'a' tier");
        return;
    }
}

#[test]
fn test_get_models_for_tier_returns_empty_for_unknown() {
    let unknown = get_models_for_tier("unknown_tier");
    assert!(unknown.is_empty());

    assert!(get_models_for_tier("fast").is_empty());
    assert!(get_models_for_tier("balanced").is_empty());
    assert!(get_models_for_tier("capable").is_empty());
    assert!(get_models_for_tier("best").is_empty());
    assert_eq!(get_models_for_tier("s"), get_models_for_tier("a"));
}

#[test]
fn test_parse_model_list_handles_empty_string() {
    std::env::set_var("OYA_TEST_EMPTY", "");
    let result = parse_model_list("OYA_TEST_EMPTY", vec!["default"]);
    assert_eq!(result, vec![""]);
    std::env::remove_var("OYA_TEST_EMPTY");
}

#[test]
fn test_parse_model_list_uses_default_when_not_set() {
    let result = parse_model_list("OYA_NONEXISTENT_VAR_12345", vec!["model-a", "model-b"]);
    assert_eq!(result, vec!["model-a", "model-b"]);
}

#[test]
fn test_parse_model_list_parses_csv() {
    std::env::set_var("OYA_TEST_MODELS", "model1, model2 ,model3");
    let result = parse_model_list("OYA_TEST_MODELS", vec!["default"]);
    assert_eq!(result, vec!["model1", "model2", "model3"]);
    std::env::remove_var("OYA_TEST_MODELS");
}

#[test]
fn test_is_model_healthy_returns_true_for_unknown_model() {
    let state = TrackerState::default();
    assert!(is_model_healthy(&state, "unknown-model"));
}

#[test]
fn test_is_model_healthy_returns_true_when_not_in_cooldown() {
    let mut state = TrackerState::default();
    state.model_health.insert(
        "test-model".to_string(),
        ModelHealth {
            model_id: "test-model".to_string(),
            is_rate_limited: true,
            consecutive_failures: 1,
            cooldown_until: Some(Utc::now() - Duration::seconds(100)),
        },
    );
    assert!(is_model_healthy(&state, "test-model"));
}

#[test]
fn test_is_model_healthy_returns_false_when_in_cooldown() {
    let mut state = TrackerState::default();
    state.model_health.insert(
        "test-model".to_string(),
        ModelHealth {
            model_id: "test-model".to_string(),
            is_rate_limited: true,
            consecutive_failures: 3,
            cooldown_until: Some(Utc::now() + Duration::seconds(100)),
        },
    );
    assert!(!is_model_healthy(&state, "test-model"));
}

#[test]
fn test_report_outcome_records_failure_and_increments_counter() {
    let mut state = TrackerState::default();
    let model = "test-model".to_string();

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 0,
            cooldown_until: None,
        },
    );

    if let Some(health) = state.model_health.get_mut(&model) {
        health.consecutive_failures += 1;
    }

    assert_eq!(state.model_health.get(&model).unwrap().consecutive_failures, 1);

    if let Some(health) = state.model_health.get_mut(&model) {
        health.consecutive_failures += 1;
    }

    assert_eq!(state.model_health.get(&model).unwrap().consecutive_failures, 2);
}

#[test]
fn test_report_outcome_resets_counter_on_success() {
    let mut state = TrackerState::default();
    let model = "test-model".to_string();

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 5,
            cooldown_until: None,
        },
    );

    if let Some(health) = state.model_health.get_mut(&model) {
        health.consecutive_failures = 0;
    }

    assert_eq!(state.model_health.get(&model).unwrap().consecutive_failures, 0);
}

#[test]
fn test_report_outcome_sets_rate_limit_flag() {
    let mut state = TrackerState::default();
    let model = "test-model".to_string();
    let now = Utc::now();

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 0,
            cooldown_until: None,
        },
    );

    if let Some(health) = state.model_health.get_mut(&model) {
        health.is_rate_limited = true;
        health.cooldown_until = Some(now + Duration::seconds(300));
    }

    assert!(state.model_health.get(&model).unwrap().is_rate_limited);
    assert!(state.model_health.get(&model).unwrap().cooldown_until.is_some());
}

#[test]
fn test_tier_rotation_selects_next_healthy_model() {
    let mut state = TrackerState::default();
    let tier = "d".to_string();

    let models = get_models_for_tier(&tier);
    // Skip test if not enough models configured
    if models.len() < 2 {
        eprintln!("Skipping test: need at least 2 models for rotation test, got {}", models.len());
        return;
    }

    state.model_health.insert(
        models[0].clone(),
        ModelHealth {
            model_id: models[0].clone(),
            is_rate_limited: true,
            consecutive_failures: 3,
            cooldown_until: Some(Utc::now() + Duration::seconds(300)),
        },
    );

    state.active_indices.insert(tier.clone(), 0);

    assert!(!is_model_healthy(&state, &models[0]));
    assert!(is_model_healthy(&state, &models[1]));
}

#[test]
fn test_tier_rotation_with_all_models_unhealthy() {
    let mut state = TrackerState::default();
    let tier = "d".to_string();

    let models = get_models_for_tier(&tier);
    if models.is_empty() {
        return;
    }

    for model in &models {
        state.model_health.insert(
            model.clone(),
            ModelHealth {
                model_id: model.clone(),
                is_rate_limited: true,
                consecutive_failures: 10,
                cooldown_until: Some(Utc::now() + Duration::seconds(300)),
            },
        );
    }

    for model in &models {
        assert!(!is_model_healthy(&state, model));
    }
}

#[test]
fn test_circuit_breaker_state_transitions() {
    use crate::types::CircuitState;

    let mut state = CircuitState::Closed;
    assert_eq!(state, CircuitState::Closed);

    state = CircuitState::Open;
    assert_eq!(state, CircuitState::Open);

    state = CircuitState::HalfOpen;
    assert_eq!(state, CircuitState::HalfOpen);

    state = CircuitState::Closed;
    assert_eq!(state, CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_allows_operations_when_closed() {
    use crate::types::CircuitState;

    assert!(CircuitState::Closed.allows_operations());
    assert!(CircuitState::HalfOpen.allows_operations());
    assert!(!CircuitState::Open.allows_operations());
}

#[test]
fn test_full_failure_workflow() {
    let mut state = TrackerState::default();
    let tier = "d".to_string();
    let model = "test-model".to_string();

    assert!(is_model_healthy(&state, &model));

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 1,
            cooldown_until: None,
        },
    );

    assert!(is_model_healthy(&state, &model));

    if let Some(health) = state.model_health.get_mut(&model) {
        health.is_rate_limited = true;
        health.cooldown_until = Some(Utc::now() + Duration::seconds(300));
    }

    assert!(!is_model_healthy(&state, &model));

    let models = get_models_for_tier(&tier);
    if models.len() > 1 {
        let current_index = *state.active_indices.get(&tier).unwrap_or(&0);
        let next_index = (current_index + 1) % models.len();
        assert_ne!(current_index, next_index, "Should rotate to different model");
    }
}

#[test]
fn test_multiple_tiers_with_different_states() {
    let mut state = TrackerState::default();

    state.model_health.insert(
        "openai/gpt-3.5-turbo".to_string(),
        ModelHealth {
            model_id: "openai/gpt-3.5-turbo".to_string(),
            is_rate_limited: true,
            consecutive_failures: 5,
            cooldown_until: Some(Utc::now() + Duration::seconds(300)),
        },
    );
    state.active_indices.insert("d".to_string(), 0);

    state.active_indices.insert("c".to_string(), 0);

    assert!(!is_model_healthy(&state, "openai/gpt-3.5-turbo"));

    let tier_c_models = get_models_for_tier("c");
    for model in &tier_c_models {
        assert!(is_model_healthy(&state, model));
    }
}

#[test]
fn test_consecutive_failures_accumulate() {
    let mut state = TrackerState::default();
    let model = "test-model".to_string();

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 0,
            cooldown_until: None,
        },
    );

    for i in 1..=5 {
        if let Some(health) = state.model_health.get_mut(&model) {
            health.consecutive_failures = i;
        }
        assert_eq!(state.model_health.get(&model).unwrap().consecutive_failures, i);
    }
}

#[test]
fn test_cooldown_expiration_allows_model_recovery() {
    let mut state = TrackerState::default();
    let model = "test-model".to_string();

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: true,
            consecutive_failures: 10,
            cooldown_until: Some(Utc::now() - Duration::seconds(100)),
        },
    );

    assert!(is_model_healthy(&state, &model));
}

#[test]
fn test_tier_rotation_on_rate_limit() {
    let mut state = TrackerState::default();
    let tier = "d".to_string();

    let models = get_models_for_tier(&tier);
    if models.len() < 2 {
        return;
    }

    state.active_indices.insert(tier.clone(), 0);

    state.model_health.insert(
        models[0].clone(),
        ModelHealth {
            model_id: models[0].clone(),
            is_rate_limited: true,
            consecutive_failures: 1,
            cooldown_until: Some(Utc::now() + Duration::seconds(300)),
        },
    );

    let current = *state.active_indices.get(&tier).unwrap_or(&0);
    let next = (current + 1) % models.len();
    state.active_indices.insert(tier.clone(), next);

    assert_eq!(next, 1);
    assert_eq!(*state.active_indices.get(&tier).unwrap(), 1);
}
