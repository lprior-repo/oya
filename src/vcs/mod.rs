#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod jj_backend;

pub use jj_backend::{JjBackend, RebaseStats, VcsError};
