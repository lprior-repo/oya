//! oya-tauri - High-performance Tauri backend for oya-ui
//!
//! This crate provides the desktop backend for the oya UI, featuring:
//!
//! - **Zero-copy IPC**: Uses rkyv for fast serialization (~10μs round-trip)
//! - **High-throughput streaming**: Ring buffers and chunked emission for 120fps
//! - **Async file I/O**: Non-blocking operations via tokio
//! - **Smart caching**: moka-based LRU cache with automatic expiration
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────────┐
//! │  oya-ui (WASM)  │◄───►│  oya-tauri (Rust)   │
//! │  Leptos         │     │  Tauri Commands     │
//! └─────────────────┘     └─────────────────────┘
//!         │                         │
//!         │ IPC (~10μs)             │ Async I/O
//!         ▼                         ▼
//! ┌─────────────────┐     ┌─────────────────────┐
//! │  Signal Layer   │     │  moka Cache + Disk  │
//! └─────────────────┘     └─────────────────────┘
//! ```
//!
//! # Performance Targets
//!
//! | Metric | Target |
//! |--------|--------|
//! | IPC round-trip | <1ms |
//! | Frame time | <8.3ms (120fps) |
//! | Text throughput | >100MB/s |
//! | Cache lookup | <0.1ms |

pub mod commands;
pub mod error;
pub mod state;
pub mod streaming;

pub use error::{AppError, AppResult};
pub use state::AppState;
