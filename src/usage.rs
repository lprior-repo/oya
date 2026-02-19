//! Usage tracking and smart model failover service.
//!
//! This module implements the `OyaUsageTracker` Virtual Object, which manages:
//! - Model health tracking (success/failure rates, rate limits).
//! - Active model selection (round-robin failover within tiers).
//! - Circuit breaking for persistent failures.
//! - Usage statistics for monitoring.

use crate::types::{CircuitState, ModelHealth, ModelTier, UsageStatus};
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
    // Circuit breaker state per tier
    circuit_states: HashMap<String, CircuitState>,
    last_updated: DateTime<Utc>,
}

impl Default for TrackerState {
    fn default() -> Self {
        Self {
            active_indices: HashMap::new(),
            model_health: HashMap::new(),
            circuit_states: HashMap::new(),
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
    async fn get_active_model(tier: String) -> Result<String, HandlerError>;

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
    ) -> Result<String, HandlerError> {
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

        Ok(model_list[selected_index].clone())
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
            circuit_state: CircuitState::Closed, // TODO: Implement circuit logic if needed
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
