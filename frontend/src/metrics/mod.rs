mod errors;
mod model;
mod report;
mod store;

#[cfg(test)]
mod tests;

pub use errors::MetricsError;
pub use model::{
    CategoryStats, MetricsStore, MetricsSummary, QualityGateIteration, QualityGateSession,
    ScenarioValidationMetrics, SessionStatus, SpecValidationMetrics, SuggestionDecision,
    SuggestionDecisionMetrics, SuggestionKey,
};
