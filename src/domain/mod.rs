#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
pub mod batch_processor;
#[cfg(test)]
pub mod batch_processor_tests;
#[cfg(test)]
pub mod math_consistency_tests;
