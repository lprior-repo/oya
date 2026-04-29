#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Phase {
    Discovery,
    Implementation,
    Review,
    Deployment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitCriteria {
    pub tests_written: bool,
    pub tests_passing: bool,
    pub code_reviewed: bool,
    pub documentation_updated: bool,
}

impl ExitCriteria {
    pub fn for_discovery() -> Self {
        Self {
            tests_written: true,
            tests_passing: true,
            code_reviewed: true,
            documentation_updated: true,
        }
    }
    
    pub fn for_implementation() -> Self {
        Self {
            tests_written: true,
            tests_passing: true,
            code_reviewed: true,
            documentation_updated: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectMetadata {
    pub completed_phases: HashSet<Phase>,
    pub current_phase: Option<Phase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseCompletionError {
    ExitCriteriaNotMet { unmet: Vec<String> },
    PhaseAlreadyCompleted { phase: Phase },
    CannotRevertCompletedPhase { phase: Phase },
    CannotSkipPhase { current: Phase, attempted: Phase },
    CurrentPhaseNotSet,
}

impl PhaseCompletionError {
    pub fn message(&self) -> String {
        match self {
            Self::ExitCriteriaNotMet { unmet } => {
                format!("Exit criteria not met for phase: {}", unmet.join(", "))
            }
            Self::PhaseAlreadyCompleted { phase } => {
                format!("Phase {:?} has already been completed", phase)
            }
            Self::CannotRevertCompletedPhase { phase } => {
                format!("Cannot revert completed phase: {:?}", phase)
            }
            Self::CannotSkipPhase { current, attempted } => {
                format!("Cannot skip from {:?} to {:?}", current, attempted)
            }
            Self::CurrentPhaseNotSet => "Current phase is not set".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExitCriteriaStatus {
    pub all_met: bool,
    pub unmet: Vec<String>,
}

impl ExitCriteriaStatus {
    pub fn all_met(&self) -> bool {
        self.all_met
    }
    
    pub fn unmet_criteria(&self) -> &Vec<String> {
        &self.unmet
    }
}

pub fn check_exit_criteria(criteria: &ExitCriteria) -> ExitCriteriaStatus {
    let mut unmet = Vec::new();
    
    if !criteria.tests_written {
        unmet.push("tests_written".to_string());
    }
    if !criteria.tests_passing {
        unmet.push("tests_passing".to_string());
    }
    if !criteria.code_reviewed {
        unmet.push("code_reviewed".to_string());
    }
    if !criteria.documentation_updated {
        unmet.push("documentation_updated".to_string());
    }
    
    ExitCriteriaStatus {
        all_met: unmet.is_empty(),
        unmet,
    }
}

pub async fn complete_phase(
    metadata: &mut ProjectMetadata,
    phase: Phase,
    criteria: ExitCriteria,
) -> Result<(), PhaseCompletionError> {
    let status = check_exit_criteria(&criteria);
    
    if !status.all_met() {
        return Err(PhaseCompletionError::ExitCriteriaNotMet {
            unmet: status.unmet,
        });
    }
    
    if metadata.completed_phases.contains(&phase) {
        return Err(PhaseCompletionError::PhaseAlreadyCompleted { phase: phase.clone() });
    }
    
    if metadata.current_phase.as_ref() != Some(&phase) {
        if let Some(current) = &metadata.current_phase {
            return Err(PhaseCompletionError::CannotSkipPhase {
                current: current.clone(),
                attempted: phase,
            });
        } else {
            return Err(PhaseCompletionError::CurrentPhaseNotSet);
        }
    }
    
    metadata.completed_phases.insert(phase.clone());
    metadata.current_phase = None;
    
    Ok(())
}

impl ProjectMetadata {
    pub fn transition_to_phase(&mut self, phase: Phase) -> Result<(), PhaseCompletionError> {
        if self.completed_phases.contains(&phase) {
            return Err(PhaseCompletionError::CannotRevertCompletedPhase { phase });
        }
        self.current_phase = Some(phase);
        Ok(())
    }
    
    pub fn transition_to_next_phase(&mut self, phase: Phase) -> Result<(), PhaseCompletionError> {
        self.transition_to_phase(phase)
    }
}

fn create_test_project_metadata() -> ProjectMetadata {
    ProjectMetadata {
        completed_phases: HashSet::new(),
        current_phase: Some(Phase::Discovery),
    }
}

#[test]
fn test_phase_completes_when_all_exit_criteria_met() {
    let mut metadata = create_test_project_metadata();
    let criteria = ExitCriteria::for_discovery();
    
    let result = complete_phase(&mut metadata, Phase::Discovery, criteria);
    
    assert!(result.is_ok());
    assert!(metadata.completed_phases.contains(&Phase::Discovery));
    assert!(metadata.current_phase.is_none());
}

#[test]
fn test_phase_does_not_complete_when_criteria_not_met() {
    let mut metadata = create_test_project_metadata();
    let incomplete_criteria = ExitCriteria {
        tests_written: false,
        tests_passing: false,
        code_reviewed: false,
        documentation_updated: false,
    };
    
    let result = complete_phase(&mut metadata, Phase::Discovery, incomplete_criteria);
    
    assert!(result.is_err());
    let error = result.expect_err("should fail with incomplete criteria");
    assert!(matches!(error, PhaseCompletionError::ExitCriteriaNotMet { .. }));
    assert!(!metadata.completed_phases.contains(&Phase::Discovery));
}

#[test]
fn test_cannot_complete_phase_twice_without_reset() {
    let mut metadata = create_test_project_metadata();
    let criteria = ExitCriteria::for_discovery();
    
    let first_completion = complete_phase(&mut metadata, Phase::Discovery, criteria.clone());
    assert!(first_completion.is_ok());
    
    let second_completion = complete_phase(&mut metadata, Phase::Discovery, criteria);
    assert!(second_completion.is_err());
    let error = second_completion.expect_err("should fail on duplicate completion");
    assert!(matches!(error, PhaseCompletionError::PhaseAlreadyCompleted { .. }));
}

#[test]
fn test_phase_completion_is_irreversible() {
    let mut metadata = create_test_project_metadata();
    let criteria = ExitCriteria::for_discovery();
    
    complete_phase(&mut metadata, Phase::Discovery, criteria).ok();
    
    let result = metadata.transition_to_phase(Phase::Discovery);
    assert!(result.is_err());
    let error = result.expect_err("should fail to revert phase");
    assert!(matches!(error, PhaseCompletionError::CannotRevertCompletedPhase { .. }));
}

#[test]
fn test_error_message_is_clear_when_criteria_missing() {
    let mut metadata = create_test_project_metadata();
    let incomplete_criteria = ExitCriteria {
        tests_written: true,
        tests_passing: false,
        code_reviewed: true,
        documentation_updated: true,
    };
    
    let result = complete_phase(&mut metadata, Phase::Discovery, incomplete_criteria);
    
    assert!(result.is_err());
    let error = result.expect_err("should provide clear error");
    assert!(error.message().contains("tests_passing"));
}

#[test]
fn test_exit_criteria_check_returns_detailed_status() {
    let criteria = ExitCriteria {
        tests_written: true,
        tests_passing: true,
        code_reviewed: false,
        documentation_updated: true,
    };
    
    let status = check_exit_criteria(&criteria);
    
    assert!(!status.all_met());
    assert!(status.unmet_criteria().contains(&"code_reviewed".to_string()));
}

#[test]
fn test_multiple_phases_can_be_completed_in_sequence() {
    let mut metadata = create_test_project_metadata();
    
    let discovery_criteria = ExitCriteria::for_discovery();
    let implementation_criteria = ExitCriteria::for_implementation();
    
    let discovery_result = complete_phase(&mut metadata, Phase::Discovery, discovery_criteria);
    assert!(discovery_result.is_ok());
    
    metadata.current_phase = Some(Phase::Implementation);
    
    let implementation_result = complete_phase(&mut metadata, Phase::Implementation, implementation_criteria);
    assert!(implementation_result.is_ok());
    
    assert!(metadata.completed_phases.contains(&Phase::Discovery));
    assert!(metadata.completed_phases.contains(&Phase::Implementation));
}

#[test]
fn test_cannot_skip_phase() {
    let mut metadata = create_test_project_metadata();
    metadata.current_phase = Some(Phase::Discovery);
    
    let implementation_criteria = ExitCriteria::for_implementation();
    let result = complete_phase(&mut metadata, Phase::Implementation, implementation_criteria);
    
    assert!(result.is_err());
    let error = result.expect_err("should fail when skipping phase");
    assert!(matches!(error, PhaseCompletionError::CannotSkipPhase { .. }));
}

#[test]
fn test_project_metadata_tracks_completed_phases() {
    let mut metadata = ProjectMetadata {
        completed_phases: HashSet::new(),
        current_phase: Some(Phase::Discovery),
    };
    
    let criteria = ExitCriteria::for_discovery();
    complete_phase(&mut metadata, Phase::Discovery, criteria).ok();
    
    assert_eq!(metadata.completed_phases.len(), 1);
    assert!(metadata.completed_phases.contains(&Phase::Discovery));
}

#[test]
fn test_current_phase_transitions_correctly() {
    let mut metadata = ProjectMetadata {
        completed_phases: HashSet::new(),
        current_phase: Some(Phase::Discovery),
    };
    
    let criteria = ExitCriteria::for_discovery();
    complete_phase(&mut metadata, Phase::Discovery, criteria).ok();
    
    assert!(metadata.current_phase.is_none());
    
    metadata.transition_to_next_phase(Phase::Implementation).ok();
    assert_eq!(metadata.current_phase, Some(Phase::Implementation));
}

#[test]
fn test_exit_criteria_all_required_fields_checked() {
    let criteria_with_all = ExitCriteria {
        tests_written: true,
        tests_passing: true,
        code_reviewed: true,
        documentation_updated: true,
    };
    
    let status = check_exit_criteria(&criteria_with_all);
    assert!(status.all_met());
    assert!(status.unmet_criteria().is_empty());
}
