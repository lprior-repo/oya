#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

mod bead;
mod error;
mod lifecycle;
mod model;
mod pr;
mod workspace;

pub use bead::{BeadId, BeadIdError, BeadStatus, BeadStatusError};
pub use error::{FailureCategory, FailureClass, LifecycleError};
pub use lifecycle::{BeadData, CancelState, LifecycleResult, LifecycleState, Phase};
pub use model::{Model, ModelError};
pub use pr::{PrInfo, PrNumber, PrNumberError};
pub use workspace::{BookmarkName, BookmarkNameError, WorkspaceName, WorkspaceNameError};

const MAX_BEAD_ID_LEN: usize = 64;
const MAX_MODEL_LEN: usize = 128;
