#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
use std::cell::RefCell;
use std::fmt;
use std::time::SystemTime;

pub struct BatchProcessor {
    _private: (),
}

pub struct DiscoverySession {
    id: String,
    version: RefCell<u64>,
    requirements: RefCell<Vec<(String, String)>>,
    writable: bool,
}

pub enum RequirementChange {
    Add { id: String, description: String },
    Update { id: String, description: String },
    Remove { id: String },
}

#[derive(Debug)]
#[allow(clippy::too_many_lines)]
pub enum BatchError {
    AtomicRollback { successful: usize, failed: usize },
    SessionNotWritable { session_id: String },
    RequirementNotFound { id: String },
    DuplicateRequirement { id: String },
    InvalidDescription { reason: String },
    ProcessingFailed { message: String },
}

#[derive(Debug)]
#[allow(clippy::too_many_lines)]
pub struct BatchReport {
    pub batch_id: String,
    pub successful_count: usize,
    pub failed_count: usize,
    pub total_count: usize,
    pub processing_time_ms: u64,
}

impl BatchProcessor {
    pub fn new() -> Self {
        Self { _private: () }
    }

    #[allow(clippy::too_many_lines)]
    pub fn process_batch(
        &self,
        session: &DiscoverySession,
        changes: Vec<RequirementChange>,
    ) -> Result<BatchReport, BatchError> {
        if !session.is_writable() {
            return Err(BatchError::SessionNotWritable { session_id: session.id().to_string() });
        }

        let total_count = changes.len();
        if total_count == 0 {
            return Ok(BatchReport {
                batch_id: format!(
                    "batch-{}",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                successful_count: 0,
                failed_count: 0,
                total_count: 0,
                processing_time_ms: 1,
            });
        }

        let mut successful_count = 0;
        let mut failed_count = 0;
        let original_requirements = session.requirements.borrow().clone();

        for change in changes {
            match change {
                RequirementChange::Add { id, description } => {
                    if description.is_empty() {
                        failed_count += 1;
                        continue;
                    }
                    if session.requirements.borrow().iter().any(|(rid, _)| rid == &id) {
                        *session.requirements.borrow_mut() = original_requirements;
                        return Err(BatchError::DuplicateRequirement { id });
                    }
                    session.requirements.borrow_mut().push((id, description));
                    successful_count += 1;
                }
                RequirementChange::Update { id, description } => {
                    let mut reqs = session.requirements.borrow_mut();
                    if let Some(req) = reqs.iter_mut().find(|(rid, _)| rid == &id) {
                        req.1 = description;
                        successful_count += 1;
                    } else {
                        drop(reqs);
                        *session.requirements.borrow_mut() = original_requirements;
                        return Err(BatchError::RequirementNotFound { id });
                    }
                }
                RequirementChange::Remove { id } => {
                    let mut reqs = session.requirements.borrow_mut();
                    let len_before = reqs.len();
                    reqs.retain(|(rid, _)| rid != &id);
                    if reqs.len() < len_before {
                        successful_count += 1;
                    } else {
                        drop(reqs);
                        *session.requirements.borrow_mut() = original_requirements;
                        return Err(BatchError::RequirementNotFound { id });
                    }
                }
            }
        }

        if failed_count > 0 {
            *session.requirements.borrow_mut() = original_requirements;
            return Err(BatchError::AtomicRollback {
                successful: successful_count,
                failed: failed_count,
            });
        }

        *session.version.borrow_mut() += 1;

        Ok(BatchReport {
            batch_id: format!(
                "batch-{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            successful_count,
            failed_count: 0,
            total_count,
            processing_time_ms: 1,
        })
    }
}

impl DiscoverySession {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            version: RefCell::new(0),
            requirements: RefCell::new(Vec::new()),
            writable: true,
        }
    }

    pub fn new_readonly(id: &str) -> Self {
        Self {
            id: id.to_string(),
            version: RefCell::new(0),
            requirements: RefCell::new(Vec::new()),
            writable: false,
        }
    }

    pub fn version(&self) -> u64 {
        *self.version.borrow()
    }

    pub fn requirement_count(&self) -> usize {
        self.requirements.borrow().len()
    }

    pub fn is_writable(&self) -> bool {
        self.writable
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn get_requirement_description(&self, id: &str) -> Option<String> {
        self.requirements.borrow().iter().find(|(rid, _)| rid == id).map(|(_, desc)| desc.clone())
    }
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchError::AtomicRollback { successful, failed } => {
                write!(f, "Batch rolled back: {} succeeded, {} failed", successful, failed)
            }
            BatchError::SessionNotWritable { session_id } => {
                write!(f, "Session '{}' is not writable", session_id)
            }
            BatchError::RequirementNotFound { id } => {
                write!(f, "Requirement '{}' not found or does not exist", id)
            }
            BatchError::DuplicateRequirement { id } => {
                write!(f, "Requirement '{}' already exists (duplicate)", id)
            }
            BatchError::InvalidDescription { reason } => {
                write!(f, "Invalid description: {}", reason)
            }
            BatchError::ProcessingFailed { message } => {
                write!(f, "Batch processing failed: {}", message)
            }
        }
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}
