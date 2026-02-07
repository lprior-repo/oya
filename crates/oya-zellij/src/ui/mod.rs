//! UI components and visual indicators for the Zellij plugin
//!
//! This module contains reusable UI components for rendering
//! status indicators, health displays, and other visual elements.

pub mod health_indicators;

pub use health_indicators::{
    format_health, overall_health, HealthChangeEvent, HealthIndicatorConfig, HealthStatus,
    HealthTracker,
};
