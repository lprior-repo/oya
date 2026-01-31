//! # Centralized Documentation
//!
//! Document transformation, chunking, and indexing for OYA.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use core::error::{Error, Result};

// Chunking modules
pub mod chunk;
pub mod document;

// Transformer modules
pub mod analyze;
pub mod assign;
pub mod config;
pub mod discover;
pub mod filter;
pub mod graph;
pub mod highlight;
pub mod index;
pub mod llms;
pub mod scrape;
pub mod search;
pub mod similarity;
pub mod transform;
pub mod types;
pub mod validate;

// Supporting modules
pub mod chunking_adapter;
pub mod features;
