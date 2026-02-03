#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

//! Oya UI - Interactive DAG visualization library.
//!
//! Provides functional, type-safe components for building interactive
//! graph visualizations with Leptos and Canvas API.
//!
//! # Modules
//!
//! - [`models`] - Data structures for nodes and graph elements
//! - [`interaction`] - Event handling, hover detection, and viewport transforms
//! - [`components`] - Leptos components for DAG rendering (to be implemented)

pub mod interaction;
pub mod models;
