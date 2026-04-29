#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
use super::model::{SpecId, SpecVersion, SuggestionKey};
use super::{MetricsStore, SpecValidationMetrics, SuggestionDecision, SuggestionDecisionMetrics};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[test]
fn test_metrics_store_new() {
    let store = MetricsStore::new(Path::new("/tmp/test-metrics"));

    assert_eq!(store.base_path, PathBuf::from("/tmp/test-metrics/quality-metrics"));
}

#[test]
fn test_spec_validation_metrics() -> anyhow::Result<()> {
    let metrics = SpecValidationMetrics {
        timestamp: Utc::now(),
        spec_id: SpecId::new("test-spec").expect("test spec id is valid"),
        spec_version: SpecVersion::new("1.0.0").expect("test spec version is valid"),
        overall_score: 90,
        passed: true,
        category_scores: HashMap::new(),
        errors_count: 0,
        warnings_count: 1,
        duration_ms: 500,
    };

    let json = serde_json::to_string(&metrics)?;
    let deserialized: SpecValidationMetrics = serde_json::from_str(&json)?;

    assert_eq!(deserialized.spec_id, metrics.spec_id);
    assert_eq!(deserialized.overall_score, metrics.overall_score);

    Ok(())
}

#[test]
fn test_suggestion_decision_metrics_roundtrip() -> anyhow::Result<()> {
    let metrics = SuggestionDecisionMetrics {
        timestamp: Utc::now(),
        suggestion_key: SuggestionKey::new("add-timeout-guard"),
        decision: SuggestionDecision::Accepted,
        source: "single-apply".to_string(),
    };

    let json = serde_json::to_string(&metrics)?;
    let deserialized: SuggestionDecisionMetrics = serde_json::from_str(&json)?;

    assert_eq!(deserialized.suggestion_key, metrics.suggestion_key);
    assert_eq!(deserialized.decision, metrics.decision);

    Ok(())
}

#[test]
fn test_record_suggestion_decision_persists() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let store = MetricsStore::new(temp.path());
    let metrics = SuggestionDecisionMetrics {
        timestamp: Utc::now(),
        suggestion_key: SuggestionKey::new("add-compensation-branch"),
        decision: SuggestionDecision::Rejected,
        source: "bulk-clear".to_string(),
    };

    store
        .record_suggestion_decision(metrics.clone())
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let data = store.data.read().map_err(|err| anyhow::anyhow!("failed to read lock: {err}"))?;
    assert_eq!(data.suggestion_decisions.len(), 1);
    assert_eq!(data.suggestion_decisions[0].suggestion_key, metrics.suggestion_key);
    assert_eq!(data.suggestion_decisions[0].decision, metrics.decision);

    Ok(())
}
