//! # OYA Web Server
//!
//! Axum-based REST API with Tower middleware.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use error::{AppError, ErrorResponse};
pub use server::run_server;

pub mod actors;
pub mod error;
pub mod routes;
pub mod server;
