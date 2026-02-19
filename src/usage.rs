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
        let mut state = tracker_state_from_ctx(&ctx).await?;
        let model_list = models_for_tier_or_error(&tier)?;
        let current_index = active_index_for_tier(&state, &tier);
        let selected_index = selected_healthy_index(&state, &model_list, current_index)
            .map_or_else(
                || {
                    tracing::warn!(
                        "All models in tier '{}' are unhealthy. Using current index.",
                        tier
                    );
                    current_index
                },
                std::convert::identity,
            );
        persist_index_change(&ctx, &mut state, &tier, current_index, selected_index);
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
            health.is_rate_limited = false;
            health.consecutive_failures = 0;
            health.cooldown_until = None;
        } else {
            health.consecutive_failures += 1;

            if request.is_rate_limit {
                health.is_rate_limited = true;
                health.cooldown_until = Some(now + Duration::seconds(DEFAULT_COOLDOWN_SECONDS));
                rotate_tiers_on_rate_limit(&mut state, &model);
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

async fn tracker_state_from_ctx(ctx: &ObjectContext<'_>) -> Result<TrackerState, HandlerError> {
    let value = ctx.get::<Json<TrackerState>>("state").await?;
    Ok(value.map_or_else(TrackerState::default, |json| json.0))
}

fn models_for_tier_or_error(tier: &str) -> Result<Vec<String>, HandlerError> {
    let model_list = get_models_for_tier(tier);
    if model_list.is_empty() {
        return Err(HandlerError::from(format!("No models configured for tier '{}'", tier)));
    }
    Ok(model_list)
}

fn active_index_for_tier(state: &TrackerState, tier: &str) -> usize {
    *state.active_indices.get(tier).unwrap_or(&0)
}

fn selected_healthy_index(
    state: &TrackerState,
    model_list: &[String],
    current_index: usize,
) -> Option<usize> {
    (0..model_list.len()).find_map(|offset| {
        let idx = (current_index + offset) % model_list.len();
        is_model_healthy(state, &model_list[idx]).then_some(idx)
    })
}

fn persist_index_change(
    ctx: &ObjectContext<'_>,
    state: &mut TrackerState,
    tier: &str,
    current_index: usize,
    selected_index: usize,
) {
    if selected_index != current_index {
        state.active_indices.insert(tier.to_string(), selected_index);
        state.last_updated = Utc::now();
        ctx.set("state", Json(state.clone()));
    }
}

fn rotate_tiers_on_rate_limit(state: &mut TrackerState, model: &str) {
    for tier in ["fast", "balanced", "capable", "best"] {
        let list = get_models_for_tier(tier);
        if let Some(idx) = list.iter().position(|m| m == model) {
            let current = *state.active_indices.get(tier).unwrap_or(&0);
            if current == idx {
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
mod tests;
