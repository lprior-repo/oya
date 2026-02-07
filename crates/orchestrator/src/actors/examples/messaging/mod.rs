//! Message Passing Patterns Example
//!
//! This module demonstrates the three core message passing patterns in ractor:
//!
//! - **Call**: Request-response pattern with `call!()` and `call_t!()`
//! - **Cast**: Fire-and-forget pattern with `cast!()`
//! - **Send**: Async message passing with `send_message()`
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐    call/     ┌─────────────┐
//! │   Client    │ ─────────────> │  Calculator │
//! │             │ <───────────── │             │
//! │             │      cast     │             │
//! │             │ ─────────────>│             │
//! └─────────────┘               └─────────────┘
//!
//!     send                           ┌─────────────┐
//!    (async) ──────────────────────> │   Logger    │
//!                                    │             │
//!                                    └─────────────┘
//! ```
//!
//! # Patterns
//!
//! ## 1. Call Pattern (Request-Response)
//!
//! Use `call!()` when you need a response back:
//! ```ignore
//! let result = call!(actor, CalculatorMessage::Add { a: 1, b: 2 })?;
//! ```
//!
//! ## 2. Cast Pattern (Fire-and-Forget)
//!
//! Use `cast!()` when you don't need a response:
//! ```ignore
//! cast!(actor, CalculatorMessage::Reset)?;
//! ```
//!
//! ## 3. Send Pattern (Async Message Passing)
//!
//! Use `send_message()` for async delivery without waiting:
//! ```ignore
//! actor.send_message(LoggerMessage::Log { msg: "Hello".to_string() })?;
//! ```
//!
//! # Functional Rust Properties
//!
//! - **Zero panics**: No unwrap, expect, or panic! macros
//! - **Zero unwraps**: All errors handled via Result
//! - **Pure functions**: State transitions are functional
//! - **Type safety**: Compile-time message type checking

pub mod calculator;
pub mod logger;

// Re-export main types
pub use calculator::{
    CalculatorActor, CalculatorError, CalculatorMessage, CalculatorResult, CalculatorState,
    CalculatorStats,
};
pub use logger::{LogEntry, LogLevel, LoggerActor, LoggerMessage, LoggerState};
