//! Swarm database module (stub)
//!
//! Provides persistent storage for swarm state using Sled.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database operation failed: {0}")]
    OperationFailed(String),

    #[error("Record not found: {0}")]
    NotFound(String),
}

pub struct SwarmDatabase {
    connected: bool,
}

impl SwarmDatabase {
    pub fn new(_path: &str) -> Result<Self, DbError> {
        Ok(Self { connected: true })
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}
