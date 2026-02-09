//! Pure stage gate decision engine for recursive bead orchestration.

use oya_events::{
    BeadStateMachine, ExhaustionPolicy, RecursionPolicy, Severity, StageKind, StateMachineError,
};

const MAX_FEEDBACK_LEN: usize = 2048;

/// Stage execution output used by gate evaluation.
#[derive(Debug, Clone)]
pub struct StageOutput {
    /// Stage that produced this output.
    pub stage: StageKind,
    /// Whether stage execution succeeded.
    pub success: bool,
    /// Captured output (stdout/stderr/verdict text).
    pub output: String,
    /// Optional exit code.
    pub exit_code: Option<i32>,
    /// Stage duration in milliseconds.
    pub duration_ms: u64,
}

/// Gate decision after evaluating stage output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// Continue forward progression.
    Proceed { next_stage: StageKind },
    /// Reenter a previous stage.
    Reenter {
        stage: StageKind,
        feedback: String,
        severity: Severity,
    },
    /// Hard fail bead execution.
    Fail { reason: String },
    /// Retry limits exhausted.
    Exhausted { policy: ExhaustionPolicy },
}

/// Evaluates stage outputs using recursion policy constraints.
#[derive(Debug, Clone)]
pub struct StageGate {
    policy: RecursionPolicy,
}

impl StageGate {
    /// Create a new stage gate.
    pub fn new(policy: RecursionPolicy) -> Self {
        Self { policy }
    }

    /// Evaluate a stage output and produce a gate decision.
    pub fn evaluate(&self, machine: &BeadStateMachine, output: StageOutput) -> GateDecision {
        if output.success {
            let next = machine
                .current_stage()
                .next()
                .map_or(StageKind::Accept, |s| s);
            return GateDecision::Proceed { next_stage: next };
        }

        let feedback = build_feedback(output.output, output.exit_code);

        match output.stage {
            StageKind::Validate => {
                self.reentry_or_exhausted(machine, StageKind::Implement, feedback, Severity::Minor)
            }
            StageKind::Review => {
                let severity = parse_review_severity(&feedback);
                let target = BeadStateMachine::reentry_target_for_severity(severity);
                self.reentry_or_exhausted(machine, target, feedback, severity)
            }
            _ => GateDecision::Fail { reason: feedback },
        }
    }

    fn reentry_or_exhausted(
        &self,
        machine: &BeadStateMachine,
        target: StageKind,
        feedback: String,
        severity: Severity,
    ) -> GateDecision {
        let mut probe = machine.clone();
        let reentry_result = probe.reenter(target, feedback.clone(), severity);
        if reentry_result.is_err() {
            return GateDecision::Exhausted {
                policy: self.policy.on_exhaustion,
            };
        }

        let enter_result: Result<(), StateMachineError> = probe.enter_stage();
        if enter_result.is_err() {
            return GateDecision::Exhausted {
                policy: self.policy.on_exhaustion,
            };
        }

        GateDecision::Reenter {
            stage: target,
            feedback,
            severity,
        }
    }
}

fn parse_review_severity(output: &str) -> Severity {
    let text = output.to_ascii_lowercase();
    if text.contains("fundamental")
        || text.contains("wrong approach")
        || text.contains("misunderstood")
    {
        return Severity::Fundamental;
    }
    if text.contains("major") || text.contains("redesign") || text.contains("significant") {
        return Severity::Major;
    }
    Severity::Minor
}

fn build_feedback(output: String, exit_code: Option<i32>) -> String {
    let mut feedback = if output.is_empty() {
        String::from("stage failed")
    } else {
        output
    };

    if let Some(code) = exit_code {
        feedback.push_str(&format!(" [exit_code={code}]"));
    }

    if feedback.len() > MAX_FEEDBACK_LEN {
        feedback.chars().take(MAX_FEEDBACK_LEN).collect()
    } else {
        feedback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_events::BeadId;

    fn machine_at(stage: StageKind) -> BeadStateMachine {
        let mut machine = BeadStateMachine::new(BeadId::new());
        while machine.current_stage() != stage {
            if machine.advance().is_err() {
                break;
            }
        }
        machine
    }

    fn output(stage: StageKind, success: bool, text: &str, exit_code: Option<i32>) -> StageOutput {
        StageOutput {
            stage,
            success,
            output: text.to_string(),
            exit_code,
            duration_ms: 1,
        }
    }

    #[test]
    fn test_gate_success_advances_research_to_plan() {
        let machine = machine_at(StageKind::Research);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(&machine, output(StageKind::Research, true, "ok", Some(0)));
        assert_eq!(
            decision,
            GateDecision::Proceed {
                next_stage: StageKind::Plan
            }
        );
    }

    #[test]
    fn test_gate_success_advances_implement_to_review() {
        let machine = machine_at(StageKind::Implement);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(&machine, output(StageKind::Implement, true, "ok", Some(0)));
        assert_eq!(
            decision,
            GateDecision::Proceed {
                next_stage: StageKind::Review
            }
        );
    }

    #[test]
    fn test_gate_success_at_validate_advances_to_accept() {
        let machine = machine_at(StageKind::Validate);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(&machine, output(StageKind::Validate, true, "ok", Some(0)));
        assert_eq!(
            decision,
            GateDecision::Proceed {
                next_stage: StageKind::Accept
            }
        );
    }

    #[test]
    fn test_gate_success_full_forward_chain() {
        let machine = machine_at(StageKind::Plan);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(&machine, output(StageKind::Plan, true, "ok", Some(0)));
        assert_eq!(
            decision,
            GateDecision::Proceed {
                next_stage: StageKind::Implement
            }
        );
    }

    #[test]
    fn test_gate_review_reject_minor_to_implement() {
        let machine = machine_at(StageKind::Review);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Review, false, "fix nits", Some(1)),
        );
        assert!(matches!(
            decision,
            GateDecision::Reenter {
                stage: StageKind::Implement,
                severity: Severity::Minor,
                ..
            }
        ));
    }

    #[test]
    fn test_gate_review_reject_major_to_plan() {
        let machine = machine_at(StageKind::Review);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Review, false, "requires redesign", Some(1)),
        );
        assert!(matches!(
            decision,
            GateDecision::Reenter {
                stage: StageKind::Plan,
                severity: Severity::Major,
                ..
            }
        ));
    }

    #[test]
    fn test_gate_review_reject_fundamental_to_research() {
        let machine = machine_at(StageKind::Review);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Review, false, "wrong approach", Some(1)),
        );
        assert!(matches!(
            decision,
            GateDecision::Reenter {
                stage: StageKind::Research,
                severity: Severity::Fundamental,
                ..
            }
        ));
    }

    #[test]
    fn test_gate_validate_fail_to_implement() {
        let machine = machine_at(StageKind::Validate);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Validate, false, "test failed", Some(1)),
        );
        assert!(matches!(
            decision,
            GateDecision::Reenter {
                stage: StageKind::Implement,
                ..
            }
        ));
    }

    #[test]
    fn test_gate_implement_fail_is_hard_fail() {
        let machine = machine_at(StageKind::Implement);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Implement, false, "process crash", Some(101)),
        );
        assert!(matches!(decision, GateDecision::Fail { .. }));
    }

    #[test]
    fn test_gate_exhausted_when_bounds_exceeded() {
        let policy = RecursionPolicy {
            max_total_attempts: 1,
            max_stage_retries: 1,
            max_research_retries: 1,
            on_exhaustion: ExhaustionPolicy::Fail,
        };
        let mut machine = machine_at(StageKind::Review);
        let _ = machine.enter_stage();
        let gate = StageGate::new(policy);
        let decision = gate.evaluate(&machine, output(StageKind::Review, false, "minor", Some(1)));
        assert!(matches!(decision, GateDecision::Exhausted { .. }));
    }

    #[test]
    fn test_gate_exhausted_uses_policy() {
        let policy = RecursionPolicy {
            max_total_attempts: 1,
            max_stage_retries: 1,
            max_research_retries: 1,
            on_exhaustion: ExhaustionPolicy::ParkForHuman,
        };
        let mut machine = machine_at(StageKind::Review);
        let _ = machine.enter_stage();
        let gate = StageGate::new(policy);
        let decision = gate.evaluate(&machine, output(StageKind::Review, false, "minor", Some(1)));
        assert_eq!(
            decision,
            GateDecision::Exhausted {
                policy: ExhaustionPolicy::ParkForHuman
            }
        );
    }

    #[test]
    fn test_gate_empty_output_on_failure() {
        let machine = machine_at(StageKind::Implement);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(&machine, output(StageKind::Implement, false, "", None));
        assert!(matches!(decision, GateDecision::Fail { reason } if !reason.is_empty()));
    }

    #[test]
    fn test_gate_very_long_output_truncated_in_feedback() {
        let machine = machine_at(StageKind::Implement);
        let gate = StageGate::new(RecursionPolicy::default());
        let long_output = "a".repeat(4000);
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Implement, false, &long_output, None),
        );
        assert!(
            matches!(decision, GateDecision::Fail { reason } if reason.len() <= MAX_FEEDBACK_LEN)
        );
    }

    #[test]
    fn test_gate_preserves_exit_code_in_feedback() {
        let machine = machine_at(StageKind::Implement);
        let gate = StageGate::new(RecursionPolicy::default());
        let decision = gate.evaluate(
            &machine,
            output(StageKind::Implement, false, "boom", Some(42)),
        );
        assert!(
            matches!(decision, GateDecision::Fail { reason } if reason.contains("exit_code=42"))
        );
    }
}
