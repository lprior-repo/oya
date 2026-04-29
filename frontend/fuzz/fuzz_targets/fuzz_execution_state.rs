//! Fuzz target for deserializing ExecutionState from arbitrary bytes

#![no_main]

use libfuzzer_sys::fuzz_target;
use oya_frontend::graph::ExecutionState;
use serde_json;

fuzz_target!(|data: &[u8]| {
    // Try to deserialize arbitrary bytes as ExecutionState
    // This should either succeed with a valid state or fail with a serde error

    let result = serde_json::from_str::<ExecutionState>(std::str::from_utf8(data).unwrap_or(""));

    // The result should be either:
    // 1. Ok(ExecutionState) - valid lowercase string
    // 2. Err(serde_json::Error) - invalid JSON or invalid state string

    // We don't assert anything here - fuzzing is about finding panics
    // If this doesn't panic, the fuzz target passed this iteration
    let _ = result;
});
