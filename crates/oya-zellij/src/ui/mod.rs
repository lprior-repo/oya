//! UI components and visual indicators for the Zellij plugin
//!
//! This module contains reusable UI components for rendering
//! status indicators, health displays, and other visual elements.

pub mod bead_detail;
pub mod health_indicators;

pub use bead_detail::{BeadDetail, HistoryEntry};
pub use health_indicators::{
    format_health, overall_health, HealthChangeEvent, HealthIndicatorConfig, HealthStatus,
    HealthTracker,
};
