//! Usage tracking and smart model failover service.
//!
//! This module implements the `OyaUsageTracker` Virtual Object, which manages:
//! - Model health tracking (success/failure rates, rate limits).
//! - Active model selection (round-robin failover within tiers).
//! - Circuit breaking for persistent failures.
//! - Usage statistics for monitoring.

use crate::types::{
    load_model_tier_config, CircuitState, ModelHealth, ModelTierConfig, UsageStatus,
};
use chrono::{DateTime, Duration, Utc};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const DEFAULT_COOLDOWN_SECONDS: i64 = 300; // 5 minutes
const TIER_CIRCUIT_OPEN_SECONDS: i64 = 30;
const TIER_TOKEN_BUCKET_CAPACITY: f64 = 2.0;
const TIER_TOKEN_BUCKET_REFILL_PER_SEC: f64 = 0.2;

static MODEL_TIER_CONFIG: OnceLock<Option<ModelTierConfig>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerState {
    // Current active index for each tier's model list
    pub active_indices: HashMap<String, usize>,
    // Health status per model ID
    pub model_health: HashMap<String, ModelHealth>,
    // Tier-level breaker state when all models are overloaded
    #[serde(default)]
    pub tier_circuits: HashMap<String, TierCircuit>,
    // Tier-level token bucket to smooth bursts
    #[serde(default)]
    pub tier_limiters: HashMap<String, TierLimiter>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierCircuit {
    pub state: CircuitState,
    pub open_until: Option<DateTime<Utc>>,
}

impl Default for TierCircuit {
    fn default() -> Self {
        Self { state: CircuitState::Closed, open_until: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierLimiter {
    pub tokens: f64,
    pub last_refill: DateTime<Utc>,
}

impl Default for TierLimiter {
    fn default() -> Self {
        Self { tokens: TIER_TOKEN_BUCKET_CAPACITY, last_refill: DateTime::UNIX_EPOCH }
    }
}

impl Default for TrackerState {
    fn default() -> Self {
        Self {
            active_indices: HashMap::new(),
            model_health: HashMap::new(),
            tier_circuits: HashMap::new(),
            tier_limiters: HashMap::new(),
            last_updated: DateTime::UNIX_EPOCH,
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
#[name = "Oya"]
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
        let now = next_logical_time(&state);
        guard_tier_circuit(&mut state, &tier, now)?;
        consume_tier_token(&mut state, &tier, now)?;
        let model_list = models_for_tier_or_error(&tier)?;
        let current_index = active_index_for_tier(&state, &tier);
        let selected_index =
            if let Some(index) = selected_healthy_index(&state, &model_list, current_index, now) {
                index
            } else {
                open_tier_circuit(&mut state, &tier, now);
                state.last_updated = now;
                ctx.set("state", Json(state));
                let retry_after_ms = TIER_CIRCUIT_OPEN_SECONDS * 1_000;
                return Err(TerminalError::new(format!(
                    "all_models_rate_limited tier={} retry_after_ms={retry_after_ms}",
                    tier
                ))
                .into());
            };
        persist_index_change(&ctx, &mut state, &tier, current_index, selected_index);
        state.last_updated = now;
        ctx.set("state", Json(state.clone()));
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
        let now = next_logical_time(&state);
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
                open_circuit_for_overloaded_tiers(&mut state, &model, now);
            }
        }

        if request.success {
            close_circuit_for_model_tiers(&mut state, &model);
        }

        state.last_updated = now;
        ctx.set("state", Json(state));
        Ok(())
    }

    async fn get_status(&self, ctx: ObjectContext<'_>) -> Result<Json<UsageStatus>, HandlerError> {
        let state = ctx.get::<Json<TrackerState>>("state").await?.map(|j| j.0).unwrap_or_default();
        let circuit_state = aggregate_circuit_state(&state);

        // Convert active indices to model names for display
        let mut active_models = HashMap::new();
        for tier in ["d", "c", "b", "a", "s"] {
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
            circuit_state,
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
    now: DateTime<Utc>,
) -> Option<usize> {
    (0..model_list.len()).find_map(|offset| {
        let idx = (current_index + offset) % model_list.len();
        is_model_healthy_at(state, &model_list[idx], now).then_some(idx)
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
        state.last_updated = next_logical_time(state);
        ctx.set("state", Json(state.clone()));
    }
}

fn rotate_tiers_on_rate_limit(state: &mut TrackerState, model: &str) {
    for tier in ["d", "c", "b", "a", "s"] {
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

fn open_circuit_for_overloaded_tiers(state: &mut TrackerState, model: &str, now: DateTime<Utc>) {
    for tier in ["d", "c", "b", "a", "s"] {
        let models = get_models_for_tier(tier);
        if !models.iter().any(|entry| entry == model) {
            continue;
        }
        let all_unhealthy = models.iter().all(|entry| !is_model_healthy_at(state, entry, now));
        if all_unhealthy {
            open_tier_circuit(state, tier, now);
        }
    }
}

fn close_circuit_for_model_tiers(state: &mut TrackerState, model: &str) {
    for tier in ["d", "c", "b", "a", "s"] {
        let models = get_models_for_tier(tier);
        if models.iter().any(|entry| entry == model) {
            close_tier_circuit(state, tier);
        }
    }
}

fn close_tier_circuit(state: &mut TrackerState, tier: &str) {
    let circuit = state.tier_circuits.entry(tier.to_string()).or_default();
    circuit.state = CircuitState::Closed;
    circuit.open_until = None;
}

fn open_tier_circuit(state: &mut TrackerState, tier: &str, now: DateTime<Utc>) {
    let circuit = state.tier_circuits.entry(tier.to_string()).or_default();
    circuit.state = CircuitState::Open;
    circuit.open_until = Some(now + Duration::seconds(TIER_CIRCUIT_OPEN_SECONDS));
}

fn guard_tier_circuit(
    state: &mut TrackerState,
    tier: &str,
    now: DateTime<Utc>,
) -> Result<(), HandlerError> {
    let circuit = state.tier_circuits.entry(tier.to_string()).or_default();
    if circuit.state != CircuitState::Open {
        return Ok(());
    }
    let Some(open_until) = circuit.open_until else {
        return Ok(());
    };
    if now >= open_until {
        circuit.state = CircuitState::HalfOpen;
        circuit.open_until = None;
        return Ok(());
    }
    let retry_after_ms = (open_until - now).num_milliseconds().max(0);
    Err(TerminalError::new(format!(
        "tier_circuit_open tier={} retry_after_ms={retry_after_ms}",
        tier
    ))
    .into())
}

fn consume_tier_token(
    state: &mut TrackerState,
    tier: &str,
    now: DateTime<Utc>,
) -> Result<(), HandlerError> {
    let limiter = state.tier_limiters.entry(tier.to_string()).or_default();
    let elapsed_ms = (now - limiter.last_refill).num_milliseconds().max(0) as f64;
    let refill = (elapsed_ms / 1000.0) * TIER_TOKEN_BUCKET_REFILL_PER_SEC;
    limiter.tokens = (limiter.tokens + refill).min(TIER_TOKEN_BUCKET_CAPACITY);
    limiter.last_refill = now;

    if limiter.tokens < 1.0 {
        let retry_after_ms = tier_backoff_duration(limiter.tokens)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        return Err(TerminalError::new(format!(
            "tier_token_exhausted tier={tier} retry_after_ms={retry_after_ms}"
        ))
        .into());
    }

    limiter.tokens -= 1.0;
    Ok(())
}

/// Compute backoff duration needed for a tier token bucket to refill to 1 token.
///
/// Returns `None` when there are already sufficient tokens.
#[must_use]
pub fn tier_backoff_duration(tokens: f64) -> Option<std::time::Duration> {
    if tokens >= 1.0 {
        return None;
    }
    let deficit = (1.0 - tokens).max(0.0);
    let seconds = (deficit / TIER_TOKEN_BUCKET_REFILL_PER_SEC).ceil();
    Some(std::time::Duration::from_secs_f64(seconds))
}

fn aggregate_circuit_state(state: &TrackerState) -> CircuitState {
    let now = state.last_updated;
    if state.tier_circuits.values().any(|circuit| {
        circuit.state == CircuitState::Open
            && circuit.open_until.map(|open_until| now < open_until).unwrap_or(false)
    }) {
        return CircuitState::Open;
    }
    if state.tier_circuits.values().any(|circuit| circuit.state == CircuitState::HalfOpen) {
        return CircuitState::HalfOpen;
    }
    CircuitState::Closed
}

fn is_model_healthy_at(state: &TrackerState, model_id: &str, now: DateTime<Utc>) -> bool {
    if let Some(health) = state.model_health.get(model_id) {
        if let Some(cooldown) = health.cooldown_until {
            if now < cooldown {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
fn is_model_healthy(state: &TrackerState, model_id: &str) -> bool {
    is_model_healthy_at(state, model_id, state.last_updated)
}

fn next_logical_time(state: &TrackerState) -> DateTime<Utc> {
    let base = if state.last_updated == DateTime::UNIX_EPOCH {
        DateTime::UNIX_EPOCH
    } else {
        state.last_updated
    };
    base + Duration::seconds(1)
}

fn get_models_for_tier(tier: &str) -> Vec<String> {
    if let Some(models) = models_for_tier_from_env(tier) {
        return models;
    }

    let from_config = models_for_tier_from_file(tier);
    if !from_config.is_empty() {
        return from_config;
    }

    match tier {
        "d" => parse_model_list(
            "OYA_MODELS_D",
            vec!["openai/gpt-3.5-turbo", "anthropic/claude-3-haiku"],
        ),
        "c" => parse_model_list(
            "OYA_MODELS_C",
            vec!["openai/gpt-4o-mini", "anthropic/claude-3-sonnet"],
        ),
        "b" => {
            parse_model_list("OYA_MODELS_B", vec!["openai/gpt-4-turbo", "anthropic/claude-3-opus"])
        }
        "a" => {
            parse_model_list("OYA_MODELS_A", vec!["openai/gpt-4o", "anthropic/claude-3-5-sonnet"])
        }
        "s" => {
            parse_model_list("OYA_MODELS_S", vec!["openai/gpt-4o", "anthropic/claude-3-5-sonnet"])
        }
        _ => vec![],
    }
}

fn models_for_tier_from_env(tier: &str) -> Option<Vec<String>> {
    let env_key = match tier {
        "d" => Some("OYA_MODELS_D"),
        "c" => Some("OYA_MODELS_C"),
        "b" => Some("OYA_MODELS_B"),
        "a" => Some("OYA_MODELS_A"),
        "s" => Some("OYA_MODELS_S"),
        _ => None,
    }?;

    std::env::var(env_key).ok().map(|env_models| parse_csv_model_list(&env_models))
}

fn models_for_tier_from_file(tier: &str) -> Vec<String> {
    let Some(config) = model_tier_config_cached() else {
        return vec![];
    };

    config.tiers.get(tier).cloned().unwrap_or_default()
}

fn model_tier_config_cached() -> Option<ModelTierConfig> {
    MODEL_TIER_CONFIG.get_or_init(|| load_model_tier_config().ok()).clone()
}

fn parse_model_list(env_key: &str, default: Vec<&str>) -> Vec<String> {
    std::env::var(env_key)
        .map(|s| parse_csv_model_list(&s))
        .unwrap_or_else(|_| default.iter().map(|s| s.to_string()).collect())
}

fn parse_csv_model_list(raw: &str) -> Vec<String> {
    raw.split(',').map(|s| s.trim().to_string()).collect()
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
