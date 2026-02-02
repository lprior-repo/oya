//! Interaction module for user input handling
//!
//! This module provides pure functional interaction handlers for canvas events.

pub mod hover;

pub use hover::{HitTestResult, contains_point};
