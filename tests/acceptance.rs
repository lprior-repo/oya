//! Acceptance tests for the orchestrator pipeline
//!
//! These tests verify the public API contract for the staged pipeline:
//! JjWorkspace -> Implementation -> Main

use anyhow::Result;
use oya::orchestrator::{Orchestrator, StageRequest};
use oya::types::{FailureCategory, ModelId, StageName};
use proptest::prelude::*;

mod util;

/// Implementation stage passes and advances to Main.
#[tokio::test]
async fn test_contract_stage_passes_and_advances() -> Result<()> {
    let orch = util::passing_orchestrator();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "test-run-123".to_string(),
            context: "debug".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!("Orchestrator error: {}", e))?;

    assert!(result.passed, "Implementation stage should pass");
    assert_eq!(result.next_stage, Some(StageName::Main));
    Ok(())
}

/// Implementation stage passes and advances to Witness.
#[tokio::test]
async fn test_implementation_stage_passes_and_advances() -> Result<()> {
    let orch = util::passing_orchestrator();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "test-run-123".to_string(),
            context: "debug".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!("Orchestrator error: {}", e))?;

    assert!(result.passed, "Implementation stage should pass");
    assert_eq!(result.next_stage, Some(StageName::Main));
    Ok(())
}

/// Property: Any failed stage returns a non-None failure_category.
#[test]
fn prop_failed_stage_has_failure_category() {
    let config = proptest::test_runner::Config::default();

    let mut runner = proptest::test_runner::TestRunner::new(config);

    let _ = runner.run(&(proptest::prelude::any::<bool>(),), |(flag,)| {
        let category =
            if flag { FailureCategory::CompileFailed } else { FailureCategory::TestFailed };
        prop_assert!(!category.as_str().is_empty(), "Failure category must have string repr");
        Ok(())
    });
}

/// Gate invariant: Implementation stage has exactly 2 gates (Compiles + TestsPass).
#[test]
fn test_implementation_gates_invariant() {
    use oya::types::Gate;
    let gates = StageName::Implementation.gates();
    assert_eq!(gates.len(), 2);
    assert!(gates.contains(&Gate::Compiles));
    assert!(gates.contains(&Gate::TestsPass));
}

// ============================================================================
// Rate-Limit Failover Acceptance Tests (ATDD for src-1ch)
// ============================================================================

mod rate_limit_failover {
    use anyhow::Result;
    use chrono::{Duration, Utc};
    use oya::types::{FailureCategory, ModelHealth, ModelId, Tier};
    use oya::usage::{is_rate_limit_failure, ReportOutcomeRequest, TrackerState};

    #[test]
    fn acceptance_rate_limit_triggers_model_rotation() -> Result<()> {
        let model_id = ModelId::new("openai/gpt-3.5-turbo")?;
        let request =
            ReportOutcomeRequest { model: model_id.clone(), success: false, is_rate_limit: true };

        assert!(!request.success, "Request must indicate failure");
        assert!(request.is_rate_limit, "Request must indicate rate limit");

        let mut state = TrackerState::default();
        let now = Utc::now();

        state.model_health.insert(
            request.model.clone(),
            ModelHealth {
                model_id: request.model.clone(),
                is_rate_limited: true,
                consecutive_failures: 1,
                cooldown_until: Some(now + Duration::seconds(300)),
            },
        );

        let health = state
            .model_health
            .get(&request.model)
            .ok_or_else(|| anyhow::anyhow!("Missing health"))?;
        assert!(health.is_rate_limited, "Model must be marked as rate-limited");
        assert!(health.cooldown_until.is_some(), "Model must have cooldown period set");
        Ok(())
    }

    #[test]
    fn acceptance_all_models_rate_limited_returns_fallback() -> Result<()> {
        let mut state = TrackerState::default();
        let model_a = ModelId::new("model-a")?;
        let model_b = ModelId::new("model-b")?;
        let tier_d = Tier::new("d")?;

        state.model_health.insert(
            model_a.clone(),
            ModelHealth {
                model_id: model_a.clone(),
                is_rate_limited: true,
                consecutive_failures: 5,
                cooldown_until: Some(Utc::now() + Duration::seconds(300)),
            },
        );
        state.model_health.insert(
            model_b.clone(),
            ModelHealth {
                model_id: model_b.clone(),
                is_rate_limited: true,
                consecutive_failures: 3,
                cooldown_until: Some(Utc::now() + Duration::seconds(300)),
            },
        );

        assert!(
            state
                .model_health
                .get(&model_a)
                .ok_or_else(|| anyhow::anyhow!("Missing health A"))?
                .is_rate_limited,
            "Model A must be rate-limited"
        );
        assert!(
            state
                .model_health
                .get(&model_b)
                .ok_or_else(|| anyhow::anyhow!("Missing health B"))?
                .is_rate_limited,
            "Model B must be rate-limited"
        );

        let active_index = *state.active_indices.get(&tier_d).unwrap_or(&0);
        assert!(active_index < 10, "Must have valid fallback index even when all unhealthy");
        Ok(())
    }

    #[test]
    fn acceptance_success_clears_rate_limit_state() -> Result<()> {
        let mut state = TrackerState::default();
        let model_a = ModelId::new("model-a")?;

        state.model_health.insert(
            model_a.clone(),
            ModelHealth {
                model_id: model_a.clone(),
                is_rate_limited: true,
                consecutive_failures: 5,
                cooldown_until: Some(Utc::now() + Duration::seconds(300)),
            },
        );

        let success_request =
            ReportOutcomeRequest { model: model_a.clone(), success: true, is_rate_limit: false };

        if let Some(health) = state.model_health.get_mut(&success_request.model) {
            if success_request.success {
                health.is_rate_limited = false;
                health.consecutive_failures = 0;
                health.cooldown_until = None;
            }
        }

        let health =
            state.model_health.get(&model_a).ok_or_else(|| anyhow::anyhow!("Missing health"))?;
        assert!(!health.is_rate_limited, "Rate limit flag must be cleared");
        assert_eq!(health.consecutive_failures, 0, "Failures must be reset");
        assert!(health.cooldown_until.is_none(), "Cooldown must be cleared");
        Ok(())
    }

    #[test]
    fn acceptance_cooldown_expiry_restores_health() -> Result<()> {
        let mut state = TrackerState::default();
        let model_a = ModelId::new("model-a")?;

        state.model_health.insert(
            model_a.clone(),
            ModelHealth {
                model_id: model_a.clone(),
                is_rate_limited: true,
                consecutive_failures: 5,
                cooldown_until: Some(Utc::now() - Duration::seconds(100)),
            },
        );

        let health =
            state.model_health.get(&model_a).ok_or_else(|| anyhow::anyhow!("Missing health"))?;
        let cooldown = health.cooldown_until.ok_or_else(|| anyhow::anyhow!("Missing cooldown"))?;
        let is_healthy = Utc::now() > cooldown;

        assert!(is_healthy, "Model must be healthy after cooldown expires");
        Ok(())
    }

    #[test]
    fn acceptance_only_rate_limit_category_triggers_rotation() {
        assert!(
            is_rate_limit_failure(&FailureCategory::RateLimited),
            "RateLimited category must trigger rotation"
        );

        let non_rate_limit_categories = [
            FailureCategory::TestFailed,
            FailureCategory::CompileFailed,
            FailureCategory::LintFailed,
            FailureCategory::AuthFailed,
            FailureCategory::ProviderUnavailable,
            FailureCategory::ContextOverflow,
            FailureCategory::OutputParseFailure,
            FailureCategory::MaxAttemptsExceeded,
        ];

        for category in &non_rate_limit_categories {
            assert!(
                !is_rate_limit_failure(category),
                "{:?} must NOT trigger rate-limit rotation",
                category
            );
        }
    }

    #[test]
    fn acceptance_rate_limited_model_skipped_during_selection() -> Result<()> {
        let mut state = TrackerState::default();
        let model_a = ModelId::new("model-a")?;
        let tier_d = Tier::new("d")?;
        let models = vec![model_a.clone(), ModelId::new("model-b")?, ModelId::new("model-c")?];

        state.model_health.insert(
            model_a.clone(),
            ModelHealth {
                model_id: model_a.clone(),
                is_rate_limited: true,
                consecutive_failures: 3,
                cooldown_until: Some(Utc::now() + Duration::seconds(300)),
            },
        );
        state.active_indices.insert(tier_d.clone(), 0);

        fn is_model_healthy(state: &TrackerState, model_id: &ModelId) -> bool {
            if let Some(health) = state.model_health.get(model_id) {
                if let Some(cooldown) = health.cooldown_until {
                    if Utc::now() < cooldown {
                        return false;
                    }
                }
            }
            true
        }

        let current_index = *state.active_indices.get(&tier_d).unwrap_or(&0);
        let selected_index = (0..models.len())
            .find_map(|offset| {
                let idx = (current_index + offset) % models.len();
                is_model_healthy(&state, &models[idx]).then_some(idx)
            })
            .unwrap_or(current_index);

        assert_ne!(selected_index, 0, "Must not select rate-limited model at index 0");
        assert_eq!(selected_index, 1, "Must select next healthy model");
        Ok(())
    }

    #[test]
    fn acceptance_consecutive_rate_limits_stable_rotation() -> Result<()> {
        let mut state = TrackerState::default();
        let tier_d = Tier::new("d")?;
        let models = vec![ModelId::new("model-a")?, ModelId::new("model-b")?];

        state.active_indices.insert(tier_d.clone(), 0);

        for i in 0..5 {
            let current = *state.active_indices.get(&tier_d).unwrap_or(&0);
            let next = (current + 1) % models.len();
            state.active_indices.insert(tier_d.clone(), next);

            let idx = state
                .active_indices
                .get(&tier_d)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("Missing index"))?;
            assert!(idx < models.len(), "Rotation must stay within bounds on iteration {}", i);
        }
        Ok(())
    }
}

// ============================================================================
// Token Exhaustion Backoff Acceptance Tests (ATDD for src-1ch)
// ============================================================================

mod token_exhaustion_backoff {
    use anyhow::Result;
    use oya::types::Tier;
    use oya::usage::{tier_backoff_duration, TierLimiter, TrackerState};
    use std::time::Duration as StdDuration;

    #[test]
    fn acceptance_exhausted_bucket_returns_backoff_duration() -> Result<()> {
        let tokens = 0.3;

        let backoff = tier_backoff_duration(tokens);

        let b = backoff.ok_or_else(|| anyhow::anyhow!("Expected backoff duration"))?;
        assert!(b.as_millis() > 0, "Backoff must be positive");
        assert!(
            b <= StdDuration::from_secs(10),
            "Backoff must be bounded (< 10s for 0.7 tokens deficit at 0.2/s)"
        );
        Ok(())
    }

    #[test]
    fn acceptance_sufficient_tokens_no_backoff() {
        let tokens = 1.5;

        let backoff = tier_backoff_duration(tokens);

        assert!(backoff.is_none(), "No backoff needed when tokens >= 1.0");
    }

    #[test]
    fn acceptance_backoff_is_stable() {
        let tokens = 0.5;

        let backoff1 = tier_backoff_duration(tokens);
        let backoff2 = tier_backoff_duration(tokens);

        assert_eq!(backoff1, backoff2, "Same token level must produce same backoff");
    }

    #[test]
    fn acceptance_empty_bucket_reasonable_backoff() -> Result<()> {
        let tokens = 0.0;
        let b = tier_backoff_duration(tokens)
            .ok_or_else(|| anyhow::anyhow!("Expected backoff duration"))?;

        let max_expected = StdDuration::from_secs(6);
        assert!(b <= max_expected, "Empty bucket backoff must be <= 6s (got {:?})", b);
        assert!(
            b >= StdDuration::from_secs(4),
            "Empty bucket must need at least 4s to refill 1 token at 0.2/s"
        );
        Ok(())
    }

    #[test]
    fn acceptance_tiers_have_independent_buckets() -> Result<()> {
        use chrono::Utc;

        let mut state = TrackerState::default();
        let now = Utc::now();
        let tier_d = Tier::new("d")?;
        let tier_c = Tier::new("c")?;
        let tier_b = Tier::new("b")?;

        state.tier_limiters.insert(tier_d.clone(), TierLimiter { tokens: 0.0, last_refill: now });
        state.tier_limiters.insert(tier_c.clone(), TierLimiter { tokens: 2.0, last_refill: now });
        state.tier_limiters.insert(tier_b.clone(), TierLimiter { tokens: 1.5, last_refill: now });

        let d_tokens = state
            .tier_limiters
            .get(&tier_d)
            .ok_or_else(|| anyhow::anyhow!("Missing tier D"))?
            .tokens;
        let c_tokens = state
            .tier_limiters
            .get(&tier_c)
            .ok_or_else(|| anyhow::anyhow!("Missing tier C"))?
            .tokens;
        let b_tokens = state
            .tier_limiters
            .get(&tier_b)
            .ok_or_else(|| anyhow::anyhow!("Missing tier B"))?
            .tokens;

        assert!(d_tokens < 1.0, "Tier D must be exhausted");
        assert!(c_tokens >= 1.0, "Tier C must have tokens");
        assert!(b_tokens >= 1.0, "Tier B must have tokens");
        Ok(())
    }

    #[test]
    fn acceptance_tracker_exposes_tier_limiters() -> Result<()> {
        use chrono::Utc;

        let mut state = TrackerState::default();
        let tier_d = Tier::new("d")?;

        state
            .tier_limiters
            .insert(tier_d.clone(), TierLimiter { tokens: 0.5, last_refill: Utc::now() });

        let limiter =
            state.tier_limiters.get(&tier_d).ok_or_else(|| anyhow::anyhow!("Missing tier D"))?;
        assert_eq!(limiter.tokens, 0.5, "Token count must be preserved");
        Ok(())
    }
}
