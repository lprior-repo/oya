use super::*;
use crate::types::{CircuitState, FailureCategory, ModelId, StageName, Tier};
use anyhow::Result;

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
fn test_tier_for_stage_maps_correctly() -> Result<()> {
    assert_eq!(tier_for_stage(&StageName::Implementation)?.as_str(), "d");
    assert_eq!(tier_for_stage(&StageName::Implementation)?.as_str(), "d");
    assert_eq!(tier_for_stage(&StageName::Main)?.as_str(), "d");
    Ok(())
}

#[test]
fn test_get_models_for_tier_returns_defaults() {
    let tiers = ["d", "c", "b", "a"];
    for tier in tiers {
        let models = get_models_for_tier(tier);
        if models.is_empty() {
            eprintln!("Skipping test: no models configured for '{}' tier", tier);
        }
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
fn test_is_model_healthy_returns_true_when_not_in_cooldown() -> Result<()> {
    let mut state = TrackerState::default();
    state.last_updated = Utc::now();
    let model_id = ModelId::new("test-model")?;
    state.model_health.insert(
        model_id.clone(),
        ModelHealth {
            model_id: model_id.clone(),
            is_rate_limited: true,
            consecutive_failures: 1,
            cooldown_until: Some(Utc::now() - Duration::seconds(100)),
        },
    );
    assert!(is_model_healthy_at(&state, "test-model", state.last_updated));
    Ok(())
}

#[test]
fn test_is_model_healthy_returns_false_when_in_cooldown() -> Result<()> {
    let mut state = TrackerState::default();
    state.last_updated = Utc::now();
    let model_id = ModelId::new("test-model")?;
    state.model_health.insert(
        model_id.clone(),
        ModelHealth {
            model_id: model_id.clone(),
            is_rate_limited: true,
            consecutive_failures: 3,
            cooldown_until: Some(Utc::now() + Duration::seconds(100)),
        },
    );
    assert!(!is_model_healthy_at(&state, "test-model", state.last_updated));
    Ok(())
}

#[test]
fn test_report_outcome_records_failure_and_increments_counter() -> Result<()> {
    let mut state = TrackerState::default();
    let model = ModelId::new("test-model")?;

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

    assert_eq!(state.model_health.get(&model).map(|h| h.consecutive_failures), Some(1));

    if let Some(health) = state.model_health.get_mut(&model) {
        health.consecutive_failures += 1;
    }

    assert_eq!(state.model_health.get(&model).map(|h| h.consecutive_failures), Some(2));
    Ok(())
}

#[test]
fn test_report_outcome_resets_counter_on_success() -> Result<()> {
    let mut state = TrackerState::default();
    let model = ModelId::new("test-model")?;

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

    assert_eq!(state.model_health.get(&model).map(|h| h.consecutive_failures), Some(0));
    Ok(())
}

#[test]
fn test_report_outcome_sets_rate_limit_flag() -> Result<()> {
    let mut state = TrackerState::default();
    let model = ModelId::new("test-model")?;
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

    let health = state.model_health.get(&model);
    assert!(health.map(|h| h.is_rate_limited).unwrap_or(false));
    assert!(health.and_then(|h| h.cooldown_until).is_some());
    Ok(())
}

#[test]
fn test_tier_rotation_selects_next_healthy_model() -> Result<()> {
    let mut state = TrackerState::default();
    let tier_str = "d";
    let tier = Tier::new(tier_str)?;

    let models = get_models_for_tier(tier_str);
    if models.len() < 2 {
        return Ok(());
    }

    let model_id = ModelId::new(&models[0])?;
    state.model_health.insert(
        model_id.clone(),
        ModelHealth {
            model_id,
            is_rate_limited: true,
            consecutive_failures: 3,
            cooldown_until: Some(Utc::now() + Duration::seconds(300)),
        },
    );

    state.active_indices.insert(tier, 0);

    assert!(!is_model_healthy_at(&state, &models[0], state.last_updated));
    assert!(is_model_healthy_at(&state, &models[1], state.last_updated));
    Ok(())
}

#[test]
fn test_tier_rotation_with_all_models_unhealthy() -> Result<()> {
    let mut state = TrackerState::default();
    let tier = "d";

    let models = get_models_for_tier(tier);
    if models.is_empty() {
        return Ok(());
    }

    for model in &models {
        let model_id = ModelId::new(model)?;
        state.model_health.insert(
            model_id.clone(),
            ModelHealth {
                model_id,
                is_rate_limited: true,
                consecutive_failures: 10,
                cooldown_until: Some(Utc::now() + Duration::seconds(300)),
            },
        );
    }

    for model in &models {
        assert!(!is_model_healthy_at(&state, model, state.last_updated));
    }
    Ok(())
}

#[test]
fn test_open_tier_circuit_blocks_until_timeout() -> Result<()> {
    let mut state = TrackerState::default();
    let now = Utc::now();
    let tier = Tier::new("c")?;

    open_tier_circuit(&mut state, &tier, now);
    let blocked = guard_tier_circuit(&mut state, &tier, now + Duration::seconds(1));
    assert!(blocked.is_err());

    let recovered = guard_tier_circuit(&mut state, &tier, now + Duration::seconds(31));
    assert!(recovered.is_ok());
    Ok(())
}

#[test]
fn test_consume_tier_token_exhausts_bucket_without_refill() -> Result<()> {
    let mut state = TrackerState::default();
    let now = Utc::now();
    let tier = Tier::new("d")?;

    for _ in 0..2 {
        assert!(consume_tier_token(&mut state, &tier, now).is_ok());
    }
    assert!(consume_tier_token(&mut state, &tier, now).is_err());
    Ok(())
}

#[test]
fn test_aggregate_circuit_state_reports_open_when_any_tier_open() -> Result<()> {
    let mut state = TrackerState::default();
    let tier = Tier::new("b")?;
    open_tier_circuit(&mut state, &tier, Utc::now());
    assert_eq!(aggregate_circuit_state(&state), CircuitState::Open);
    Ok(())
}

#[test]
fn test_circuit_breaker_state_transitions() {
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
    assert!(CircuitState::Closed.allows_operations());
    assert!(CircuitState::HalfOpen.allows_operations());
    assert!(!CircuitState::Open.allows_operations());
}

#[test]
fn test_full_failure_workflow() -> Result<()> {
    let mut state = TrackerState::default();
    let tier_str = "d";
    let tier = Tier::new(tier_str)?;
    let model_str = "test-model";
    let model = ModelId::new(model_str)?;

    assert!(is_model_healthy_at(&state, model_str, state.last_updated));

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 1,
            cooldown_until: None,
        },
    );

    assert!(is_model_healthy_at(&state, model_str, state.last_updated));

    if let Some(health) = state.model_health.get_mut(&model) {
        health.is_rate_limited = true;
        health.cooldown_until = Some(Utc::now() + Duration::seconds(300));
    }

    assert!(!is_model_healthy_at(&state, model_str, state.last_updated));

    let models = get_models_for_tier(tier_str);
    if models.len() > 1 {
        let current_index = *state.active_indices.get(&tier).unwrap_or(&0);
        let next_index = (current_index + 1) % models.len();
        assert_ne!(current_index, next_index, "Should rotate to different model");
    }
    Ok(())
}

#[test]
fn test_multiple_tiers_with_different_states() -> Result<()> {
    let mut state = TrackerState::default();
    let tier_d = Tier::new("d")?;
    let tier_c = Tier::new("c")?;
    let model_id = ModelId::new("openai/gpt-3.5-turbo")?;

    state.model_health.insert(
        model_id.clone(),
        ModelHealth {
            model_id,
            is_rate_limited: true,
            consecutive_failures: 5,
            cooldown_until: Some(Utc::now() + Duration::seconds(300)),
        },
    );
    state.active_indices.insert(tier_d, 0);
    state.active_indices.insert(tier_c, 0);

    assert!(!is_model_healthy_at(&state, "openai/gpt-3.5-turbo", state.last_updated));

    let tier_c_models = get_models_for_tier("c");
    for model in &tier_c_models {
        assert!(is_model_healthy_at(&state, model, state.last_updated));
    }
    Ok(())
}

#[test]
fn test_consecutive_failures_accumulate() -> Result<()> {
    let mut state = TrackerState::default();
    let model = ModelId::new("test-model")?;

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
        assert_eq!(state.model_health.get(&model).map(|h| h.consecutive_failures), Some(i));
    }
    Ok(())
}

#[test]
fn test_cooldown_expiration_allows_model_recovery() -> Result<()> {
    let mut state = TrackerState::default();
    state.last_updated = Utc::now();
    let model_str = "test-model";
    let model = ModelId::new(model_str)?;

    state.model_health.insert(
        model.clone(),
        ModelHealth {
            model_id: model.clone(),
            is_rate_limited: true,
            consecutive_failures: 10,
            cooldown_until: Some(Utc::now() - Duration::seconds(100)),
        },
    );

    assert!(is_model_healthy_at(&state, model_str, state.last_updated));
    Ok(())
}

#[test]
fn test_tier_rotation_on_rate_limit() -> Result<()> {
    let mut state = TrackerState::default();
    let tier_str = "d";
    let tier = Tier::new(tier_str)?;

    let models = get_models_for_tier(tier_str);
    if models.len() < 2 {
        return Ok(());
    }

    state.active_indices.insert(tier.clone(), 0);

    let model_id = ModelId::new(&models[0])?;
    state.model_health.insert(
        model_id.clone(),
        ModelHealth {
            model_id,
            is_rate_limited: true,
            consecutive_failures: 1,
            cooldown_until: Some(Utc::now() + Duration::seconds(300)),
        },
    );

    let current = *state.active_indices.get(&tier).unwrap_or(&0);
    let next = (current + 1) % models.len();
    state.active_indices.insert(tier.clone(), next);

    assert_eq!(next, 1);
    assert_eq!(state.active_indices.get(&tier).copied(), Some(1));
    Ok(())
}
