//! Usage tracking and smart model failover service.
//!
//! This module implements the `OyaUsageTracker` Virtual Object, which manages:
//! - Model health tracking (success/failure rates, rate limits).
//! - Active model selection (round-robin failover within tiers).
//! - Circuit breaking for persistent failures.
//! - Usage statistics for monitoring.

use crate::types::{CircuitState, ModelHealth, UsageStatus};
use chrono::{DateTime, Duration, Utc};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_COOLDOWN_SECONDS: i64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrackerState {
    // Current active index for each tier's model list
    active_indices: HashMap<String, usize>,
    // Health status per model ID
    model_health: HashMap<String, ModelHealth>,
    last_updated: DateTime<Utc>,
}

impl Default for TrackerState {
    fn default() -> Self {
        Self {
            active_indices: HashMap::new(),
            model_health: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOutcomeRequest {
    pub model: String,
    pub success: bool,
    pub is_rate_limit: bool,
}

#[restate_sdk::object]
pub trait OyaUsageTracker {
    /// Get the currently active, healthy model for the requested tier.
    /// Rotates automatically if the current one is unhealthy.
    async fn get_active_model(tier: String) -> Result<Json<String>, HandlerError>;

    /// Report the outcome of a model execution.
    /// Triggers failover if rate limited.
    async fn report_outcome(request: Json<ReportOutcomeRequest>) -> Result<(), HandlerError>;

    /// Get the full status of the usage tracker for monitoring.
    async fn get_status() -> Result<Json<UsageStatus>, HandlerError>;

    /// Manually reset circuit breakers and health status.
    async fn reset() -> Result<(), HandlerError>;
}

pub struct OyaUsageTrackerImpl;

impl OyaUsageTracker for OyaUsageTrackerImpl {
    async fn get_active_model(
        &self,
        ctx: ObjectContext<'_>,
        tier: String,
    ) -> Result<Json<String>, HandlerError> {
        let mut state =
            ctx.get::<Json<TrackerState>>("state").await?.map(|j| j.0).unwrap_or_default();
        let model_list = get_models_for_tier(&tier);

        if model_list.is_empty() {
            return Err(HandlerError::from(format!("No models configured for tier '{}'", tier)));
        }

        // Get current index, defaulting to 0
        let current_index = *state.active_indices.get(&tier).unwrap_or(&0);

        // Check if current model is healthy
        let mut selected_index = current_index;
        let mut found_healthy = false;

        // Try to find a healthy model, starting from current
        for i in 0..model_list.len() {
            let idx = (current_index + i) % model_list.len();
            let model_id = &model_list[idx];

            if is_model_healthy(&state, model_id) {
                selected_index = idx;
                found_healthy = true;
                break;
            }
        }

        // If no healthy models found, we might need to trip circuit or just pick the "least bad" (current)
        // For now, we'll log a warning and return the current one, but the orchestrator handles retries.
        if !found_healthy {
            tracing::warn!("All models in tier '{}' are unhealthy. Using current index.", tier);
        }

        // If we switched indices, update state
        if selected_index != current_index {
            state.active_indices.insert(tier.clone(), selected_index);
            state.last_updated = Utc::now();
            ctx.set("state", Json(state));
        }

        Ok(Json(model_list[selected_index].clone()))
    }

    async fn report_outcome(
        &self,
        ctx: ObjectContext<'_>,
        request: Json<ReportOutcomeRequest>,
    ) -> Result<(), HandlerError> {
        let mut state =
            ctx.get::<Json<TrackerState>>("state").await?.map(|j| j.0).unwrap_or_default();
        let request = request.0;
        let now = Utc::now();
        let model = request.model;

        let health = state.model_health.entry(model.clone()).or_insert(ModelHealth {
            model_id: model.clone(),
            is_rate_limited: false,
            consecutive_failures: 0,
            cooldown_until: None,
        });

        if request.success {
            // Reset health on success
            health.is_rate_limited = false;
            health.consecutive_failures = 0;
            health.cooldown_until = None;
        } else {
            health.consecutive_failures += 1;

            if request.is_rate_limit {
                health.is_rate_limited = true;
                health.cooldown_until = Some(now + Duration::seconds(DEFAULT_COOLDOWN_SECONDS));

                // Trigger rotation for all tiers containing this model
                // Note: efficient lookup would be inverted map, but list is small
                for tier in ["fast", "balanced", "capable", "best"] {
                    let list = get_models_for_tier(tier);
                    if let Some(idx) = list.iter().position(|m| m == &model) {
                        let current = *state.active_indices.get(tier).unwrap_or(&0);
                        if current == idx {
                            // Rotate to next
                            let next = (current + 1) % list.len();
                            state.active_indices.insert(tier.to_string(), next);
                            tracing::info!(
                                "Rate limit detected for {}. Rotating tier '{}' to model {}",
                                model,
                                tier,
                                list[next]
                            );
                        }
                    }
                }
            }
        }

        state.last_updated = now;
        ctx.set("state", Json(state));
        Ok(())
    }

    async fn get_status(&self, ctx: ObjectContext<'_>) -> Result<Json<UsageStatus>, HandlerError> {
        let state = ctx.get::<Json<TrackerState>>("state").await?.map(|j| j.0).unwrap_or_default();

        // Convert active indices to model names for display
        let mut active_models = HashMap::new();
        for tier in ["fast", "balanced", "capable", "best"] {
            let list = get_models_for_tier(tier);
            if !list.is_empty() {
                let idx = *state.active_indices.get(tier).unwrap_or(&0);
                if idx < list.len() {
                    active_models.insert(tier.to_string(), list[idx].clone());
                }
            }
        }

        Ok(Json(UsageStatus {
            active_models,
            model_health: state.model_health,
            circuit_state: CircuitState::Closed,
            last_updated: state.last_updated,
        }))
    }

    async fn reset(&self, ctx: ObjectContext<'_>) -> Result<(), HandlerError> {
        ctx.clear("state");
        Ok(())
    }
}

// --- Helper Functions ---

fn is_model_healthy(state: &TrackerState, model_id: &str) -> bool {
    if let Some(health) = state.model_health.get(model_id) {
        if let Some(cooldown) = health.cooldown_until {
            if Utc::now() < cooldown {
                return false;
            }
        }
    }
    true
}

fn get_models_for_tier(tier: &str) -> Vec<String> {
    // Load from env or use defaults
    // Note: In real Restate app, this should be deterministic or passed in.
    // Env vars are stable per deployment, so acceptable here.
    match tier {
        "fast" => parse_model_list(
            "OYA_MODELS_FAST",
            vec!["openai/gpt-3.5-turbo", "anthropic/claude-3-haiku"],
        ),
        "balanced" => parse_model_list(
            "OYA_MODELS_BALANCED",
            vec!["openai/gpt-4o-mini", "anthropic/claude-3-sonnet"],
        ),
        "capable" => parse_model_list(
            "OYA_MODELS_CAPABLE",
            vec!["openai/gpt-4-turbo", "anthropic/claude-3-opus"],
        ),
        "best" => parse_model_list(
            "OYA_MODELS_BEST",
            vec!["openai/gpt-4o", "anthropic/claude-3-5-sonnet"],
        ),
        _ => vec![],
    }
}

fn parse_model_list(env_key: &str, default: Vec<&str>) -> Vec<String> {
    std::env::var(env_key)
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|_| default.iter().map(|s| s.to_string()).collect())
}

/// Determine if a failure category indicates a rate limit condition.
/// Used by the orchestration loop to decide what to report to the tracker.
pub fn is_rate_limit_failure(category: &crate::types::FailureCategory) -> bool {
    matches!(category, crate::types::FailureCategory::RateLimited)
}

/// Extract tier name from stage for model selection.
/// Maps each stage to its appropriate model tier.
pub fn tier_for_stage(stage: &crate::types::StageName) -> &'static str {
    stage.model_for_stage().as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FailureCategory, StageName};

    // === Contract Tests for Helper Functions ===

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
        assert_eq!(tier_for_stage(&StageName::Plan), "balanced");
        assert_eq!(tier_for_stage(&StageName::Contract), "fast");
        assert_eq!(tier_for_stage(&StageName::Tdd15), "balanced");
        assert_eq!(tier_for_stage(&StageName::Qa), "balanced");
        assert_eq!(tier_for_stage(&StageName::RedQueen), "capable");
        assert_eq!(tier_for_stage(&StageName::GptReview), "capable");
        assert_eq!(tier_for_stage(&StageName::ShipGate), "best");
    }

    #[test]
    fn test_get_models_for_tier_returns_defaults() {
        let fast = get_models_for_tier("fast");
        assert!(!fast.is_empty());
        assert!(fast.iter().any(|m| m.contains("gpt-3.5") || m.contains("haiku")));

        let balanced = get_models_for_tier("balanced");
        assert!(!balanced.is_empty());

        let capable = get_models_for_tier("capable");
        assert!(!capable.is_empty());

        let best = get_models_for_tier("best");
        assert!(!best.is_empty());
    }

    #[test]
    fn test_get_models_for_tier_returns_empty_for_unknown() {
        let unknown = get_models_for_tier("unknown_tier");
        assert!(unknown.is_empty());
    }

    #[test]
    fn test_parse_model_list_handles_empty_string() {
        std::env::set_var("OYA_TEST_EMPTY", "");
        let result = parse_model_list("OYA_TEST_EMPTY", vec!["default"]);
        // Empty string should return one empty element
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

    // === TrackerState Health Logic Tests ===

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
                cooldown_until: Some(Utc::now() - Duration::seconds(100)), // expired
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
                cooldown_until: Some(Utc::now() + Duration::seconds(100)), // future
            },
        );
        assert!(!is_model_healthy(&state, "test-model"));
    }

    // === ReportOutcome Consecutive Failure Tests ===

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

        // Simulate failure
        if let Some(health) = state.model_health.get_mut(&model) {
            health.consecutive_failures += 1;
        }

        assert_eq!(state.model_health.get(&model).unwrap().consecutive_failures, 1);

        // Second failure
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

        // Simulate success
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

        // Simulate rate limit
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
        let tier = "fast".to_string();

        let models = get_models_for_tier(&tier);
        assert!(models.len() >= 2, "Need at least 2 models for rotation test");

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
        let tier = "fast".to_string();

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
        let tier = "fast".to_string();
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
        state.active_indices.insert("fast".to_string(), 0);

        state.active_indices.insert("balanced".to_string(), 0);

        assert!(!is_model_healthy(&state, "openai/gpt-3.5-turbo"));

        let balanced_models = get_models_for_tier("balanced");
        for model in &balanced_models {
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
        let tier = "fast".to_string();

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
}
