//! Health Indicators Usage Examples
//!
//! This file demonstrates how to use the health status indicators
//! in the OYA Zellij plugin.

use oya_zellij::ui::health_indicators::{
    format_health, overall_health, HealthChangeEvent, HealthIndicatorConfig, HealthStatus,
    HealthTracker,
};

fn main() {
    println!("=== Health Status Indicators Demo ===\n");

    // Example 1: Basic health status display
    println!("1. Basic Health Status Display:");
    println!("   Healthy:   {}", HealthStatus::Healthy.format());
    println!("   Unhealthy: {}", HealthStatus::Unhealthy.format());
    println!("   Unknown:   {}", HealthStatus::Unknown.format());
    println!();

    // Example 2: Health from scores
    println!("2. Health from Scores:");
    let scores = vec![0.95, 0.87, 0.92];
    let health = overall_health(&scores);
    println!("   Scores: {:?} -> {}", scores, health.format());
    println!();

    // Example 3: Configurable formatting
    println!("3. Configurable Formatting:");
    let status = HealthStatus::Healthy;

    println!("   Compact:  {}", format_health(status, &HealthIndicatorConfig::compact()));
    println!("   Detailed: {}", format_health(status, &HealthIndicatorConfig::detailed()));
    println!("   Minimal:  {}", format_health(status, &HealthIndicatorConfig::minimal()));
    println!(
        "   Fixed(8): {}",
        format_health(status, &HealthIndicatorConfig::fixed_width(8))
    );
    println!();

    // Example 4: Health tracking
    println!("4. Health Change Tracking:");
    let mut tracker = HealthTracker::new(HealthStatus::Unknown);

    println!("   Initial: {}", tracker.current().format());

    if let Some(event) = tracker.update(HealthStatus::Healthy) {
        println!(
            "   Changed: {} → {} ({})",
            event.previous.format(),
            event.current.format(),
            if event.is_improvement() {
                "improvement"
            } else if event.is_degradation() {
                "degradation"
            } else {
                "transition"
            }
        );
    }

    if let Some(event) = tracker.update(HealthStatus::Unhealthy) {
        println!(
            "   Changed: {} → {} ({})",
            event.previous.format(),
            event.current.format(),
            if event.is_improvement() {
                "improvement"
            } else if event.is_degradation() {
                "degradation"
            } else {
                "transition"
            }
        );
    }

    println!(
        "   Total degradations: {}, improvements: {}",
        tracker.degradation_count(),
        tracker.improvement_count()
    );
    println!();

    // Example 5: Real-time agent monitoring
    println!("5. Real-time Agent Monitoring:");
    let agent_scores = vec![0.92, 0.88, 0.45, 0.76];
    println!("   Agent Health Scores:");
    for (i, score) in agent_scores.iter().enumerate() {
        let status = HealthStatus::from_score(*score);
        println!(
            "     Agent {}: {:.0}% - {}",
            i + 1,
            score * 100.0,
            status.format_compact()
        );
    }
    println!(
        "   Overall: {}",
        overall_health(&agent_scores).format()
    );
    println!();

    println!("=== Demo Complete ===");
}
