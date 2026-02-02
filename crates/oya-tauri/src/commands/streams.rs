//! Stream-related Tauri commands
//!
//! High-throughput streaming for process output, logs, and real-time data.
//! Optimized for 120fps rendering with virtual scrolling support.

use crate::error::{AppError, AppResult};
use crate::state::{AppState, StreamStatus};
use oya_shared::{StreamChunk, StreamEnded};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use ulid::Ulid;

/// Start a process stream
///
/// Spawns a process and streams its output via Tauri events.
/// Returns immediately with the stream ID - output arrives via events.
#[tauri::command]
pub async fn start_process_stream(
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
    bead_id: Option<String>,
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> AppResult<String> {
    let stream_id = Ulid::new().to_string();

    // Register stream
    state.register_stream(stream_id.clone(), bead_id);

    // Build command
    let mut cmd = Command::new(&command);
    cmd.args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    // Spawn process
    let mut child = cmd.spawn().map_err(|e| {
        state.remove_stream(&stream_id);
        AppError::Stream(format!("Failed to spawn process: {e}"))
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Clone for async tasks
    let stream_id_stdout = stream_id.clone();
    let stream_id_stderr = stream_id.clone();
    let stream_id_wait = stream_id.clone();
    let app_stdout = app.clone();
    let app_stderr = app.clone();
    let app_wait = app.clone();
    let state_wait = Arc::clone(&state);

    // Stream stdout
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut offset = 0u64;

            while let Ok(Some(line)) = lines.next_line().await {
                let chunk = StreamChunk::new(
                    stream_id_stdout.clone(),
                    format!("{line}\n").into_bytes(),
                    offset,
                );
                offset += chunk.data.len() as u64;

                // Emit to frontend
                let _ = app_stdout.emit("stream-chunk", &chunk);
            }
        });
    }

    // Stream stderr
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut offset = 0u64;

            while let Ok(Some(line)) = lines.next_line().await {
                let chunk = StreamChunk::new(
                    stream_id_stderr.clone(),
                    format!("[stderr] {line}\n").into_bytes(),
                    offset,
                );
                offset += chunk.data.len() as u64;

                let _ = app_stderr.emit("stream-chunk", &chunk);
            }
        });
    }

    // Wait for process completion
    tokio::spawn(async move {
        let exit_code = match child.wait().await {
            Ok(status) => status.code(),
            Err(_) => None,
        };

        state_wait.update_stream_status(&stream_id_wait, StreamStatus::Ended);

        let ended = StreamEnded {
            stream_id: stream_id_wait.clone(),
            exit_code,
        };

        let _ = app_wait.emit("stream-ended", &ended);
    });

    Ok(stream_id)
}

/// Stop a stream
///
/// Marks the stream as ended. For process streams, this does not kill the process.
#[tauri::command]
pub async fn stop_stream(stream_id: String, state: State<'_, Arc<AppState>>) -> AppResult<()> {
    if state.get_stream(&stream_id).is_none() {
        return Err(AppError::Stream(format!("Stream not found: {stream_id}")));
    }

    state.update_stream_status(&stream_id, StreamStatus::Ended);
    Ok(())
}

/// Get stream status
#[tauri::command]
pub async fn get_stream_status(
    stream_id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<StreamStatusResponse> {
    let info = state
        .get_stream(&stream_id)
        .ok_or_else(|| AppError::Stream(format!("Stream not found: {stream_id}")))?;

    Ok(StreamStatusResponse {
        stream_id: info.id,
        bead_id: info.bead_id,
        status: match info.status {
            StreamStatus::Active => "active",
            StreamStatus::Paused => "paused",
            StreamStatus::Ended => "ended",
        },
    })
}

/// List active streams
#[tauri::command]
pub fn list_active_streams(state: State<'_, Arc<AppState>>) -> ActiveStreamsResponse {
    ActiveStreamsResponse {
        count: state.active_stream_count(),
    }
}

/// Stream status response
#[derive(Serialize)]
pub struct StreamStatusResponse {
    pub stream_id: String,
    pub bead_id: Option<String>,
    pub status: &'static str,
}

/// Active streams response
#[derive(Serialize)]
pub struct ActiveStreamsResponse {
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_id_generation() {
        let id1 = Ulid::new().to_string();
        let id2 = Ulid::new().to_string();

        // ULIDs should be unique
        assert_ne!(id1, id2);

        // ULIDs should be 26 characters
        assert_eq!(id1.len(), 26);
    }
}
