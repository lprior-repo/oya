use std::collections::HashMap;

use super::model::{MetricsStore, MetricsSummary, SessionStatus};

impl MetricsStore {
    #[must_use]
    pub fn get_summary(&self) -> MetricsSummary {
        let Ok(data_guard) = self.data.read() else {
            return MetricsSummary::default();
        };
        let data = &*data_guard;

        let total_sessions = data.sessions.len();
        let passed_sessions =
            data.sessions.iter().filter(|s| s.status == SessionStatus::Passed).count();
        let failed_sessions =
            data.sessions.iter().filter(|s| s.status == SessionStatus::Failed).count();
        let escalated_sessions = data.sessions.iter().filter(|s| s.escalated).count();

        let passed_sessions_refs: Vec<_> =
            data.sessions.iter().filter(|s| s.status == SessionStatus::Passed).collect();

        let avg_iterations = if passed_sessions_refs.is_empty() {
            0.0
        } else {
            let total_iterations: usize =
                passed_sessions_refs.iter().map(|s| s.iterations.len()).sum();
            #[allow(clippy::cast_precision_loss)]
            {
                total_iterations as f64 / passed_sessions_refs.len() as f64
            }
        };

        let total_duration_ms: u64 = passed_sessions_refs.iter().map(|s| s.total_duration_ms).sum();
        let avg_duration_minutes = if passed_sessions_refs.is_empty() {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                total_duration_ms as f64 / passed_sessions_refs.len() as f64 / 60000.0
            }
        };

        let spec_scores: Vec<f64> =
            data.spec_validations.iter().map(|v| f64::from(v.overall_score)).collect();
        let avg_spec_score = if spec_scores.is_empty() {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                spec_scores.iter().sum::<f64>() / spec_scores.len() as f64
            }
        };

        let mut failure_counts: HashMap<String, usize> = HashMap::new();
        for session in &data.sessions {
            for iteration in &session.iterations {
                if !iteration.overall_passed {
                    if let Some(category) = &iteration.failure_category {
                        *failure_counts.entry(category.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut failures: Vec<_> = failure_counts.into_iter().collect();
        failures.sort_by_key(|b| std::cmp::Reverse(b.1));

        MetricsSummary {
            total_sessions,
            passed_sessions,
            failed_sessions,
            escalated_sessions,
            avg_iterations_to_pass: avg_iterations,
            avg_duration_minutes,
            most_common_failure_categories: failures,
            avg_spec_score,
        }
    }

    /// Export a metrics report.
    ///
    /// # Errors
    /// Returns an error if export format is unsupported.
    pub fn export_report(&self, format: &str) -> Result<String, Box<dyn std::error::Error>> {
        let summary = self.get_summary();

        match format {
            "json" => Ok(serde_json::to_string_pretty(&summary)?),
            "text" => Ok(Self::format_text_report(&summary)),
            _ => Err("Unsupported format. Use 'json' or 'text'".into()),
        }
    }

    fn format_text_report(summary: &MetricsSummary) -> String {
        let failures_str = if summary.most_common_failure_categories.is_empty() {
            "  (none)".to_string()
        } else {
            summary
                .most_common_failure_categories
                .iter()
                .map(|(cat, count)| format!("    - {cat} ({count} times)"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let total = summary.total_sessions;
        let passed_pct = if total > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                summary.passed_sessions as f64 / total as f64 * 100.0
            }
        } else {
            0.0
        };
        let failed_pct = if total > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                summary.failed_sessions as f64 / total as f64 * 100.0
            }
        } else {
            0.0
        };
        let escalated_pct = if total > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                summary.escalated_sessions as f64 / total as f64 * 100.0
            }
        } else {
            0.0
        };

        format!(
            "
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  QUALITY GATE METRICS REPORT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Sessions:
    Total: {}
    Passed: {} ({passed_pct:.1}%)
    Failed: {} ({failed_pct:.1}%)
    Escalated: {} ({escalated_pct:.1}%)

  Performance:
    Avg iterations to pass: {:.2}
    Avg duration: {:.2} minutes
    Avg spec quality score: {:.1}/100

  Common Failure Categories:
{}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        ",
            summary.total_sessions,
            summary.passed_sessions,
            summary.failed_sessions,
            summary.escalated_sessions,
            summary.avg_iterations_to_pass,
            summary.avg_duration_minutes,
            summary.avg_spec_score,
            failures_str
        )
    }
}
