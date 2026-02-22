use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct MockStageResult {
    passed: bool,
    payload: String,
}

fn run_with_mock_journal<F>(journaled: &mut Option<MockStageResult>, execute: F) -> MockStageResult
where
    F: FnOnce() -> MockStageResult,
{
    match journaled.clone() {
        Some(value) => value,
        None => {
            let value = execute();
            *journaled = Some(value.clone());
            value
        }
    }
}

#[test]
fn replay_contract_uses_journaled_result_without_reexecuting_mock() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut journaled: Option<MockStageResult> = None;

    let first = run_with_mock_journal(&mut journaled, {
        let counter = Arc::clone(&counter);
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            MockStageResult { passed: true, payload: "first".to_string() }
        }
    });

    let replay = run_with_mock_journal(&mut journaled, {
        let counter = Arc::clone(&counter);
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            MockStageResult { passed: false, payload: "replay-should-not-run".to_string() }
        }
    });

    assert_eq!(first, replay);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn fresh_journal_contract_executes_mock_once() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut journaled: Option<MockStageResult> = None;

    let result = run_with_mock_journal(&mut journaled, {
        let counter = Arc::clone(&counter);
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            MockStageResult { passed: true, payload: "ok".to_string() }
        }
    });

    assert!(result.passed);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
