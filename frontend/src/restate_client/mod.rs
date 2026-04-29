//! Restate Client Module
//!
//! Integration with Restate's introspection API for OYA Frontend.
//!
//! API endpoints:
//! - Admin API: localhost:9070 (PATCH /invocations/*)
//! - SQL Query: localhost:9070/query (POST with SQL body)

pub mod client;
pub mod queries;
pub mod types;

pub use client::{ClientError, RestateClient, RestateClientConfig};
pub use queries::SqlQueries;
pub use types::*;
pub use types::{InvocationAction, InvocationActionResponse};
