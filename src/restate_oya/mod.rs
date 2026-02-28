mod handlers;
mod opencode;
mod trace;
mod types;

#[cfg(test)]
mod handlers_tests;

pub use handlers::serve;
pub use opencode::pipeline_prompt;
pub use types::{
    BeadSnapshot, BeadSyncRequest, CancelResponse, KeyRequest, LifecycleGateSnapshot,
    LifecycleRequest, LifecycleStatusSnapshot, LifecycleStepSnapshot, MemorySnapshot,
    PipelineRequest, StartRequest, StartResponse,
};
