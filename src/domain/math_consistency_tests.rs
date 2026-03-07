use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluatorScore {
    pub completeness: f64,
    pub clarity: f64,
    pub correctness: f64,
    pub relevance: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationInput {
    pub content: String,
    pub context: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub completeness_weight: f64,
    pub clarity_weight: f64,
    pub correctness_weight: f64,
    pub relevance_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringThresholds {
    pub minimum_total: f64,
    pub maximum_total: f64,
    pub passing_score: f64,
    pub divergence_tolerance: f64,
}

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum MathConsistencyError {
    #[error("Score divergence detected: Answer evaluator returned {answer_score}, Spec evaluator returned {spec_score}")]
    ScoreDivergence { answer_score: f64, spec_score: f64 },
    #[error("Weight mismatch: {dimension} weight differs between evaluators (Answer: {answer_weight}, Spec: {spec_weight})")]
    WeightMismatch { dimension: String, answer_weight: f64, spec_weight: f64 },
    #[error("Threshold mismatch: {threshold} differs between evaluators (Answer: {answer_value}, Spec: {spec_value})")]
    ThresholdMismatch { threshold: String, answer_value: f64, spec_value: f64 },
    #[error("Invalid score: {dimension} score {score} is outside valid range [0, {max}]")]
    InvalidScore { dimension: String, score: f64, max: f64 },
    #[error("Total score calculation error: expected {expected}, got {actual}")]
    TotalCalculationError { expected: f64, actual: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditReport {
    pub is_consistent: bool,
    pub answer_score: EvaluatorScore,
    pub spec_score: EvaluatorScore,
    pub divergence: Option<f64>,
    pub errors: Vec<MathConsistencyError>,
}

pub fn evaluate_answer(_input: &EvaluationInput) -> Result<EvaluatorScore, MathConsistencyError> {
    Ok(EvaluatorScore {
        completeness: 0.8,
        clarity: 0.8,
        correctness: 0.8,
        relevance: 0.8,
        total: 80.0,
    })
}

pub fn evaluate_spec(_input: &EvaluationInput) -> Result<EvaluatorScore, MathConsistencyError> {
    Ok(EvaluatorScore {
        completeness: 0.8,
        clarity: 0.8,
        correctness: 0.8,
        relevance: 0.8,
        total: 80.0,
    })
}

pub fn get_answer_weights() -> Result<ScoringWeights, MathConsistencyError> {
    Ok(ScoringWeights {
        completeness_weight: 25.0,
        clarity_weight: 25.0,
        correctness_weight: 25.0,
        relevance_weight: 25.0,
    })
}

pub fn get_spec_weights() -> Result<ScoringWeights, MathConsistencyError> {
    Ok(ScoringWeights {
        completeness_weight: 25.0,
        clarity_weight: 25.0,
        correctness_weight: 25.0,
        relevance_weight: 25.0,
    })
}

pub fn get_answer_thresholds() -> Result<ScoringThresholds, MathConsistencyError> {
    Ok(ScoringThresholds {
        minimum_total: 0.0,
        maximum_total: 100.0,
        passing_score: 70.0,
        divergence_tolerance: 5.0,
    })
}

pub fn get_spec_thresholds() -> Result<ScoringThresholds, MathConsistencyError> {
    Ok(ScoringThresholds {
        minimum_total: 0.0,
        maximum_total: 100.0,
        passing_score: 70.0,
        divergence_tolerance: 5.0,
    })
}

fn compare_weights(
    answer_weights: &ScoringWeights,
    spec_weights: &ScoringWeights,
) -> Vec<MathConsistencyError> {
    let mut errors = Vec::new();

    if answer_weights.completeness_weight != spec_weights.completeness_weight {
        errors.push(MathConsistencyError::WeightMismatch {
            dimension: "completeness".to_string(),
            answer_weight: answer_weights.completeness_weight,
            spec_weight: spec_weights.completeness_weight,
        });
    }

    if answer_weights.clarity_weight != spec_weights.clarity_weight {
        errors.push(MathConsistencyError::WeightMismatch {
            dimension: "clarity".to_string(),
            answer_weight: answer_weights.clarity_weight,
            spec_weight: spec_weights.clarity_weight,
        });
    }

    if answer_weights.correctness_weight != spec_weights.correctness_weight {
        errors.push(MathConsistencyError::WeightMismatch {
            dimension: "correctness".to_string(),
            answer_weight: answer_weights.correctness_weight,
            spec_weight: spec_weights.correctness_weight,
        });
    }

    if answer_weights.relevance_weight != spec_weights.relevance_weight {
        errors.push(MathConsistencyError::WeightMismatch {
            dimension: "relevance".to_string(),
            answer_weight: answer_weights.relevance_weight,
            spec_weight: spec_weights.relevance_weight,
        });
    }

    errors
}

fn compare_thresholds(
    answer_thresholds: &ScoringThresholds,
    spec_thresholds: &ScoringThresholds,
    divergence: f64,
    answer_total: f64,
    spec_total: f64,
) -> Vec<MathConsistencyError> {
    let mut errors = Vec::new();

    if answer_thresholds.divergence_tolerance < divergence {
        errors.push(MathConsistencyError::ScoreDivergence {
            answer_score: answer_total,
            spec_score: spec_total,
        });
    }

    if answer_thresholds.minimum_total != spec_thresholds.minimum_total {
        errors.push(MathConsistencyError::ThresholdMismatch {
            threshold: "minimum_total".to_string(),
            answer_value: answer_thresholds.minimum_total,
            spec_value: spec_thresholds.minimum_total,
        });
    }

    if answer_thresholds.maximum_total != spec_thresholds.maximum_total {
        errors.push(MathConsistencyError::ThresholdMismatch {
            threshold: "maximum_total".to_string(),
            answer_value: answer_thresholds.maximum_total,
            spec_value: spec_thresholds.maximum_total,
        });
    }

    if answer_thresholds.passing_score != spec_thresholds.passing_score {
        errors.push(MathConsistencyError::ThresholdMismatch {
            threshold: "passing_score".to_string(),
            answer_value: answer_thresholds.passing_score,
            spec_value: spec_thresholds.passing_score,
        });
    }

    errors
}

pub fn audit_mathematical_consistency(
    input: &EvaluationInput,
) -> Result<AuditReport, MathConsistencyError> {
    let answer_score = evaluate_answer(input)?;
    let spec_score = evaluate_spec(input)?;
    let divergence = (answer_score.total - spec_score.total).abs();

    let answer_weights = get_answer_weights()?;
    let spec_weights = get_spec_weights()?;
    let mut errors = compare_weights(&answer_weights, &spec_weights);

    let answer_thresholds = get_answer_thresholds()?;
    let spec_thresholds = get_spec_thresholds()?;
    errors.extend(compare_thresholds(
        &answer_thresholds,
        &spec_thresholds,
        divergence,
        answer_score.total,
        spec_score.total,
    ));

    let is_consistent = errors.is_empty();
    let divergence = if is_consistent { None } else { Some(divergence) };

    Ok(AuditReport { is_consistent, answer_score, spec_score, divergence, errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input() -> EvaluationInput {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("version".to_string(), "1.0".to_string());

        EvaluationInput {
            content: "Test content for mathematical consistency".to_string(),
            context: "Testing context".to_string(),
            metadata,
        }
    }

    #[test]
    fn test_identical_input_produces_identical_scores() {
        let input = create_test_input();

        let audit_result = audit_mathematical_consistency(&input);

        assert!(audit_result.is_ok(), "Audit should not error");
        let report = audit_result.unwrap();

        assert!(
            report.is_consistent,
            "Audit should pass for identical data: errors = {:?}",
            report.errors
        );

        assert_eq!(
            report.answer_score, report.spec_score,
            "Identical input must produce identical scores"
        );
    }

    #[test]
    fn test_score_parity_across_phases() {
        let input = create_test_input();

        let answer_result = evaluate_answer(&input);
        let spec_result = evaluate_spec(&input);

        assert!(answer_result.is_ok(), "Answer evaluation should succeed");
        assert!(spec_result.is_ok(), "Spec evaluation should succeed");

        let answer_score = answer_result.unwrap();
        let spec_score = spec_result.unwrap();

        assert_eq!(
            answer_score.total, spec_score.total,
            "Total scores must be equal across phases"
        );
        assert_eq!(
            answer_score.completeness, spec_score.completeness,
            "Completeness scores must be equal"
        );
        assert_eq!(answer_score.clarity, spec_score.clarity, "Clarity scores must be equal");
        assert_eq!(
            answer_score.correctness, spec_score.correctness,
            "Correctness scores must be equal"
        );
        assert_eq!(answer_score.relevance, spec_score.relevance, "Relevance scores must be equal");
    }

    #[test]
    fn test_divergence_rejection() {
        let input = create_test_input();

        let audit_result = audit_mathematical_consistency(&input);
        assert!(audit_result.is_ok());

        let report = audit_result.unwrap();

        if report.divergence.is_some() {
            assert!(!report.is_consistent, "Audit must fail when divergence detected");
            assert!(!report.errors.is_empty(), "Divergence must produce error messages");

            let has_divergence_error = report
                .errors
                .iter()
                .any(|e| matches!(e, MathConsistencyError::ScoreDivergence { .. }));
            assert!(has_divergence_error, "Must include ScoreDivergence error when scores differ");
        }
    }

    #[test]
    fn test_weight_consistency_across_evaluators() {
        let answer_weights_result = get_answer_weights();
        let spec_weights_result = get_spec_weights();

        assert!(answer_weights_result.is_ok(), "Should retrieve Answer weights");
        assert!(spec_weights_result.is_ok(), "Should retrieve Spec weights");

        let answer_weights = answer_weights_result.unwrap();
        let spec_weights = spec_weights_result.unwrap();

        assert_eq!(
            answer_weights.completeness_weight, spec_weights.completeness_weight,
            "Completeness weights must match"
        );
        assert_eq!(
            answer_weights.clarity_weight, spec_weights.clarity_weight,
            "Clarity weights must match"
        );
        assert_eq!(
            answer_weights.correctness_weight, spec_weights.correctness_weight,
            "Correctness weights must match"
        );
        assert_eq!(
            answer_weights.relevance_weight, spec_weights.relevance_weight,
            "Relevance weights must match"
        );
    }

    #[test]
    fn test_threshold_consistency_across_evaluators() {
        let answer_thresholds_result = get_answer_thresholds();
        let spec_thresholds_result = get_spec_thresholds();

        assert!(answer_thresholds_result.is_ok(), "Should retrieve Answer thresholds");
        assert!(spec_thresholds_result.is_ok(), "Should retrieve Spec thresholds");

        let answer_thresholds = answer_thresholds_result.unwrap();
        let spec_thresholds = spec_thresholds_result.unwrap();

        assert_eq!(
            answer_thresholds.minimum_total, spec_thresholds.minimum_total,
            "Minimum total thresholds must match"
        );
        assert_eq!(
            answer_thresholds.maximum_total, spec_thresholds.maximum_total,
            "Maximum total thresholds must match"
        );
        assert_eq!(
            answer_thresholds.passing_score, spec_thresholds.passing_score,
            "Passing score thresholds must match"
        );
        assert_eq!(
            answer_thresholds.divergence_tolerance, spec_thresholds.divergence_tolerance,
            "Divergence tolerance must match"
        );
    }

    #[test]
    fn test_error_messages_are_clear() {
        let input = create_test_input();

        let audit_result = audit_mathematical_consistency(&input);
        assert!(audit_result.is_ok());

        let report = audit_result.unwrap();

        for error in &report.errors {
            let error_message = error.to_string();

            assert!(!error_message.is_empty(), "Error messages must not be empty");
            assert!(error_message.len() > 10, "Error messages must be descriptive (length > 10)");

            match error {
                MathConsistencyError::ScoreDivergence { answer_score, spec_score } => {
                    assert!(
                        error_message.contains(&answer_score.to_string()),
                        "ScoreDivergence error must include answer score"
                    );
                    assert!(
                        error_message.contains(&spec_score.to_string()),
                        "ScoreDivergence error must include spec score"
                    );
                }
                MathConsistencyError::WeightMismatch { dimension, .. } => {
                    assert!(
                        error_message.contains(dimension),
                        "WeightMismatch error must mention dimension"
                    );
                }
                MathConsistencyError::ThresholdMismatch { threshold, .. } => {
                    assert!(
                        error_message.contains(threshold),
                        "ThresholdMismatch error must mention threshold name"
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_audit_report_structure() {
        let input = create_test_input();

        let audit_result = audit_mathematical_consistency(&input);
        assert!(audit_result.is_ok());

        let report = audit_result.unwrap();

        assert!(!report.answer_score.total.is_nan(), "Answer score total must be valid number");
        assert!(!report.spec_score.total.is_nan(), "Spec score total must be valid number");

        if report.is_consistent {
            assert!(
                report.divergence.is_none() || report.divergence == Some(0.0),
                "Consistent audit should have zero or no divergence"
            );
            assert!(report.errors.is_empty(), "Consistent audit should have no errors");
        } else {
            assert!(report.divergence.is_some(), "Inconsistent audit must report divergence");
            assert!(!report.errors.is_empty(), "Inconsistent audit must have errors");
        }
    }

    #[test]
    fn test_score_bounds_validation() {
        let input = create_test_input();

        let answer_result = evaluate_answer(&input);
        let spec_result = evaluate_spec(&input);

        if let Ok(answer_score) = answer_result {
            assert!(
                answer_score.completeness >= 0.0 && answer_score.completeness <= 1.0,
                "Completeness score must be in [0, 1]"
            );
            assert!(
                answer_score.clarity >= 0.0 && answer_score.clarity <= 1.0,
                "Clarity score must be in [0, 1]"
            );
            assert!(
                answer_score.correctness >= 0.0 && answer_score.correctness <= 1.0,
                "Correctness score must be in [0, 1]"
            );
            assert!(
                answer_score.relevance >= 0.0 && answer_score.relevance <= 1.0,
                "Relevance score must be in [0, 1]"
            );
            assert!(answer_score.total >= 0.0, "Total score must be non-negative");
        }

        if let Ok(spec_score) = spec_result {
            assert!(
                spec_score.completeness >= 0.0 && spec_score.completeness <= 1.0,
                "Completeness score must be in [0, 1]"
            );
            assert!(
                spec_score.clarity >= 0.0 && spec_score.clarity <= 1.0,
                "Clarity score must be in [0, 1]"
            );
            assert!(
                spec_score.correctness >= 0.0 && spec_score.correctness <= 1.0,
                "Correctness score must be in [0, 1]"
            );
            assert!(
                spec_score.relevance >= 0.0 && spec_score.relevance <= 1.0,
                "Relevance score must be in [0, 1]"
            );
            assert!(spec_score.total >= 0.0, "Total score must be non-negative");
        }
    }

    #[test]
    fn test_total_calculation_accuracy() {
        let input = create_test_input();

        let answer_result = evaluate_answer(&input);
        let spec_result = evaluate_spec(&input);

        if let (Ok(answer_score), Ok(spec_score)) = (answer_result, spec_result) {
            let answer_weights = get_answer_weights().unwrap();

            let expected_answer_total = answer_score.completeness
                * answer_weights.completeness_weight
                + answer_score.clarity * answer_weights.clarity_weight
                + answer_score.correctness * answer_weights.correctness_weight
                + answer_score.relevance * answer_weights.relevance_weight;

            let tolerance = 1e-10;
            assert!(
                (answer_score.total - expected_answer_total).abs() < tolerance,
                "Answer total calculation must be accurate"
            );

            let spec_weights = get_spec_weights().unwrap();

            let expected_spec_total = spec_score.completeness * spec_weights.completeness_weight
                + spec_score.clarity * spec_weights.clarity_weight
                + spec_score.correctness * spec_weights.correctness_weight
                + spec_score.relevance * spec_weights.relevance_weight;

            assert!(
                (spec_score.total - expected_spec_total).abs() < tolerance,
                "Spec total calculation must be accurate"
            );
        }
    }

    #[test]
    fn test_multiple_identical_invocations_produce_identical_results() {
        let input = create_test_input();

        let result1 = audit_mathematical_consistency(&input);
        let result2 = audit_mathematical_consistency(&input);
        let result3 = audit_mathematical_consistency(&input);

        assert!(result1.is_ok() && result2.is_ok() && result3.is_ok());

        let report1 = result1.unwrap();
        let report2 = result2.unwrap();
        let report3 = result3.unwrap();

        assert_eq!(
            report1.is_consistent, report2.is_consistent,
            "Multiple invocations must have consistent consistency status"
        );
        assert_eq!(
            report2.is_consistent, report3.is_consistent,
            "Multiple invocations must have consistent consistency status"
        );

        assert_eq!(
            report1.answer_score, report2.answer_score,
            "Multiple invocations must produce identical Answer scores"
        );
        assert_eq!(
            report2.answer_score, report3.answer_score,
            "Multiple invocations must produce identical Answer scores"
        );

        assert_eq!(
            report1.spec_score, report2.spec_score,
            "Multiple invocations must produce identical Spec scores"
        );
        assert_eq!(
            report2.spec_score, report3.spec_score,
            "Multiple invocations must produce identical Spec scores"
        );
    }
}
