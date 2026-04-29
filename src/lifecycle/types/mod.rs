#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

mod bead;
mod error;
mod evidence;
mod gate;
mod lifecycle;
mod model;
mod pr;
mod repair;
mod repo;
mod run;
mod run_state;
mod timeout;
mod workspace;

pub use bead::{BeadId, BeadIdError, BeadStatus, BeadStatusError};
pub use error::{FailureCategory, FailureClass, LifecycleError};
pub use evidence::{
    EvidenceChecksum, EvidenceChecksumError, EvidenceEnvelope, EvidenceEnvelopeError,
    EvidenceEnvelopeParts, EvidenceKind, EvidenceMetadata, EvidenceRecordId, EvidenceRecordIdError,
};
pub use gate::{GateFailureCategory, GateId, GateIdError, GateModel};
pub use lifecycle::{BeadData, CancelState, LifecycleResult, LifecycleState, Phase};
pub use model::{Model, ModelError};
pub use oya_contracts::CompensationDiagnostic;
pub use pr::{PrInfo, PrNumber, PrNumberError};
pub use repair::{
    BeadRepairBudget, GateRepairBudget, MutationScopeViolation, RepairBudget, RepairBudgetError,
    RepairMutationKind, RepairMutationScope,
};
pub use repo::{RepoSlug, RepoSlugError};
pub use run::{RunId, RunIdError};
pub use run_state::{RunEvent, RunPhase, RunState, RunStateTransitionError};
pub use timeout::{TimeoutError, TimeoutSeconds};
pub use workspace::{BookmarkName, BookmarkNameError, WorkspaceName, WorkspaceNameError};

const MAX_BEAD_ID_LEN: usize = 64;
const MAX_MODEL_LEN: usize = 128;
const MAX_RUN_ID_LEN: usize = 96;
