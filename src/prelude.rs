//! Prelude module - common imports for Intent CLI
//!
//! Import this module to get all common types and traits:
//! ```rust
//! use intent_core::prelude::*;
//! ```

// Re-export functional utilities
pub use itertools::Itertools;
pub use tap::{Pipe, Tap};

// Re-export error types
pub use crate::error::{IntentError, IntentResult};

// Re-export domain types
pub use crate::types::{
    HeaderName, HeaderValue, HttpMethod, IntentDuration, SpecName, StatusCode, Url,
};
