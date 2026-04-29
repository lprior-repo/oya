//! Toast Notification System — Unit & Property Tests
//!
//! Separated from `src/ui/toast.rs` to keep the production file under 300 lines.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, TimeZone, Utc};
use oya_frontend::ui::toast::{
    is_expired, Toast, ToastDuration, ToastError, ToastId, ToastSeverity, ToastStoreState,
};
use proptest::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use uuid::Uuid;

// ── Helpers ───────────────────────────────────────────────────────────

/// A deterministic timestamp for repeatable tests.
fn fixed_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

/// Builds a `Toast` with deterministic `created_at`.
fn make_toast(
    id: ToastId,
    message: &str,
    severity: ToastSeverity,
    auto_dismiss_at: Option<DateTime<Utc>>,
) -> Toast {
    Toast { id, message: message.to_string(), severity, created_at: fixed_time(), auto_dismiss_at }
}

/// Constructs a default 3 000 ms `ToastDuration` directly.
fn default_dur() -> ToastDuration {
    ToastDuration(Duration::from_millis(3_000))
}

// ══════════════════════════════════════════════════════════════════════
// B1–B6 : ToastStoreState::new
// ══════════════════════════════════════════════════════════════════════

#[test]
fn store_new_returns_empty_store_when_capacity_is_valid() {
    let result = ToastStoreState::new(5);
    let state = result.unwrap();
    assert_eq!(state.toasts.len(), 0);
    assert_eq!(state.capacity, 5);
}

#[test]
fn store_new_accepts_min_valid_capacity_of_one() {
    let result = ToastStoreState::new(1);
    let state = result.unwrap();
    assert_eq!(state.toasts.len(), 0);
    assert_eq!(state.capacity, 1);
}

#[test]
fn store_new_accepts_max_valid_capacity_of_twenty() {
    let result = ToastStoreState::new(20);
    let state = result.unwrap();
    assert_eq!(state.toasts.len(), 0);
    assert_eq!(state.capacity, 20);
}

#[test]
fn store_new_returns_invalid_capacity_when_capacity_is_zero() {
    let result = ToastStoreState::new(0);
    assert_eq!(result, Err(ToastError::InvalidCapacity(0)));
}

#[test]
fn store_new_returns_invalid_capacity_when_capacity_is_21() {
    let result = ToastStoreState::new(21);
    assert_eq!(result, Err(ToastError::InvalidCapacity(21)));
}

#[test]
fn store_new_returns_invalid_capacity_when_capacity_is_usize_max() {
    let result = ToastStoreState::new(usize::MAX);
    assert_eq!(result, Err(ToastError::InvalidCapacity(usize::MAX)));
}

// ══════════════════════════════════════════════════════════════════════
// B7–B17 : ToastStoreState::push
// ══════════════════════════════════════════════════════════════════════

#[test]
fn push_prepends_new_toast_at_index_zero() {
    let toast_a = make_toast(Uuid::new_v4(), "First", ToastSeverity::Info, None);
    let store = ToastStoreState { toasts: vec![toast_a], capacity: 3 };
    let result = store.push("Second".into(), ToastSeverity::Info, default_dur());
    let s = result.unwrap();
    assert_eq!(s.toasts.len(), 2);
    assert_eq!(s.toasts[0].message, "Second");
    assert_eq!(s.toasts[1].message, "First");
    assert_eq!(s.toasts[0].severity, ToastSeverity::Info);
}

#[test]
fn push_creates_correct_toast_for_each_severity_variant() {
    let store = ToastStoreState { toasts: vec![], capacity: 4 };
    let s = store.push("success-msg".into(), ToastSeverity::Success, default_dur()).unwrap();
    let s = s.push("error-msg".into(), ToastSeverity::Error, default_dur()).unwrap();
    let s = s.push("info-msg".into(), ToastSeverity::Info, default_dur()).unwrap();
    let s = s.push("warning-msg".into(), ToastSeverity::Warning, default_dur()).unwrap();
    assert_eq!(s.toasts.len(), 4);
    assert_eq!(s.toasts[0].severity, ToastSeverity::Warning);
    assert_eq!(s.toasts[1].severity, ToastSeverity::Info);
    assert_eq!(s.toasts[2].severity, ToastSeverity::Error);
    assert_eq!(s.toasts[3].severity, ToastSeverity::Success);
    assert_eq!(s.toasts[0].message, "warning-msg");
    assert_eq!(s.toasts[1].message, "info-msg");
    assert_eq!(s.toasts[2].message, "error-msg");
    assert_eq!(s.toasts[3].message, "success-msg");
}

#[test]
fn push_returns_empty_message_error_when_message_is_empty() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let result = store.push("".into(), ToastSeverity::Success, default_dur());
    assert_eq!(result, Err(ToastError::EmptyMessage));
}

#[test]
fn push_returns_empty_message_error_when_message_is_whitespace() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let result = store.push("   \t\n  ".into(), ToastSeverity::Success, default_dur());
    assert_eq!(result, Err(ToastError::EmptyMessage));
}

#[test]
fn push_returns_invalid_duration_error_when_duration_is_zero() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let zero_dur = ToastDuration(Duration::from_millis(0));
    let result = store.push("Hello".into(), ToastSeverity::Info, zero_dur);
    assert_eq!(result, Err(ToastError::InvalidDuration(0)));
}

#[test]
fn push_returns_invalid_duration_error_when_duration_exceeds_30s() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let over_dur = ToastDuration(Duration::from_millis(30_001));
    let result = store.push("Hello".into(), ToastSeverity::Info, over_dur);
    assert_eq!(result, Err(ToastError::InvalidDuration(30_001)));
}

#[test]
fn push_with_max_valid_duration_succeeds() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let max_dur = ToastDuration(Duration::from_millis(30_000));
    let result = store.push("Max duration".into(), ToastSeverity::Info, max_dur);
    let s = result.unwrap();
    assert_eq!(s.toasts.len(), 1);
    let toast = &s.toasts[0];
    let expected = toast.created_at + chrono::Duration::milliseconds(30_000);
    assert_eq!(toast.auto_dismiss_at, Some(expected));
}

#[test]
fn push_evicts_oldest_toast_when_at_capacity() {
    let toast_a = make_toast(Uuid::new_v4(), "A", ToastSeverity::Info, None);
    let toast_b = make_toast(Uuid::new_v4(), "B", ToastSeverity::Info, None);
    let store = ToastStoreState { toasts: vec![toast_a, toast_b], capacity: 2 };
    let result = store.push("Third".into(), ToastSeverity::Warning, default_dur());
    let s = result.unwrap();
    assert_eq!(s.toasts.len(), 2);
    assert_eq!(s.toasts[0].message, "Third");
    assert_eq!(s.toasts[1].message, "A");
}

#[test]
fn push_returns_error_without_side_effects_when_message_empty() {
    let original = make_toast(Uuid::new_v4(), "Original", ToastSeverity::Info, None);
    let store_a = ToastStoreState { toasts: vec![original.clone()], capacity: 3 };
    let result_err = store_a.push("".into(), ToastSeverity::Success, default_dur());
    assert_eq!(result_err, Err(ToastError::EmptyMessage));
    let store_b = ToastStoreState { toasts: vec![original], capacity: 3 };
    let s = store_b.push("Valid".into(), ToastSeverity::Success, default_dur()).unwrap();
    assert_eq!(s.toasts.len(), 2);
    assert_eq!(s.toasts[1].message, "Original");
}

#[test]
fn push_sets_auto_dismiss_at_to_created_at_plus_duration() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let dur = ToastDuration(Duration::from_millis(5_000));
    let result = store.push("Test".into(), ToastSeverity::Info, dur);
    let s = result.unwrap();
    let toast = &s.toasts[0];
    let expected = toast.created_at + chrono::Duration::milliseconds(5_000);
    assert_eq!(toast.auto_dismiss_at, Some(expected));
}

#[test]
fn push_preserves_newest_first_ordering_across_multiple_pushes() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let s = store.push("Alpha".into(), ToastSeverity::Info, default_dur()).unwrap();
    let s = s.push("Beta".into(), ToastSeverity::Info, default_dur()).unwrap();
    let s = s.push("Gamma".into(), ToastSeverity::Info, default_dur()).unwrap();
    assert_eq!(s.toasts.len(), 3);
    assert_eq!(s.toasts[0].message, "Gamma");
    assert_eq!(s.toasts[1].message, "Beta");
    assert_eq!(s.toasts[2].message, "Alpha");
}

// ══════════════════════════════════════════════════════════════════════
// B18–B21 : ToastStoreState::dismiss
// ══════════════════════════════════════════════════════════════════════

#[test]
fn dismiss_removes_toast_with_matching_id() {
    let id_a = Uuid::new_v4();
    let id_b = Uuid::new_v4();
    let id_c = Uuid::new_v4();
    let t_a = make_toast(id_a, "A", ToastSeverity::Info, None);
    let t_b = make_toast(id_b, "B", ToastSeverity::Info, None);
    let t_c = make_toast(id_c, "C", ToastSeverity::Info, None);
    let store = ToastStoreState { toasts: vec![t_a, t_b, t_c], capacity: 5 };
    let result = store.dismiss(id_b);
    assert_eq!(result.toasts.len(), 2);
    assert_eq!(result.toasts[0].id, id_a);
    assert_eq!(result.toasts[1].id, id_c);
}

#[test]
fn dismiss_is_noop_when_id_not_found() {
    let id_a = Uuid::new_v4();
    let t_a = make_toast(id_a, "A", ToastSeverity::Info, None);
    let store = ToastStoreState { toasts: vec![t_a], capacity: 5 };
    let result = store.dismiss(Uuid::new_v4());
    assert_eq!(result.toasts.len(), 1);
    assert_eq!(result.toasts[0].id, id_a);
}

#[test]
fn dismiss_only_toast_produces_empty_store() {
    let id_a = Uuid::new_v4();
    let t_a = make_toast(id_a, "A", ToastSeverity::Success, None);
    let store = ToastStoreState { toasts: vec![t_a], capacity: 3 };
    let result = store.dismiss(id_a);
    assert_eq!(result.toasts.len(), 0);
    assert_eq!(result.capacity, 3);
}

#[test]
fn dismiss_preserves_relative_ordering_of_remaining_toasts() {
    let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    let toasts: Vec<Toast> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| make_toast(*id, &format!("T{i}"), ToastSeverity::Info, None))
        .collect();
    let store = ToastStoreState { toasts, capacity: 5 };
    let result = store.dismiss(ids[2]);
    assert_eq!(result.toasts.len(), 4);
    assert_eq!(result.toasts[0].id, ids[0]);
    assert_eq!(result.toasts[1].id, ids[1]);
    assert_eq!(result.toasts[2].id, ids[3]);
    assert_eq!(result.toasts[3].id, ids[4]);
}

// ══════════════════════════════════════════════════════════════════════
// B22–B23 : ToastStoreState::clear_all
// ══════════════════════════════════════════════════════════════════════

#[test]
fn clear_all_empties_toasts_and_preserves_capacity() {
    let t_a = make_toast(Uuid::new_v4(), "A", ToastSeverity::Info, None);
    let t_b = make_toast(Uuid::new_v4(), "B", ToastSeverity::Info, None);
    let store = ToastStoreState { toasts: vec![t_a, t_b], capacity: 3 };
    let result = store.clear_all();
    assert_eq!(result.toasts.len(), 0);
    assert_eq!(result.capacity, 3);
}

#[test]
fn clear_all_on_empty_store_is_idempotent() {
    let store = ToastStoreState { toasts: vec![], capacity: 5 };
    let result = store.clear_all();
    assert_eq!(result.toasts.len(), 0);
    assert_eq!(result.capacity, 5);
}

// ══════════════════════════════════════════════════════════════════════
// B24–B26 : is_expired (free function)
// ══════════════════════════════════════════════════════════════════════

#[test]
fn is_expired_returns_true_when_now_equals_or_exceeds_dismiss_time() {
    let dismiss_at = fixed_time();
    let toast = make_toast(Uuid::new_v4(), "Test", ToastSeverity::Info, Some(dismiss_at));
    assert!(is_expired(&toast, dismiss_at));
    let later = dismiss_at + chrono::Duration::milliseconds(1);
    assert!(is_expired(&toast, later));
}

#[test]
fn is_expired_returns_false_when_auto_dismiss_is_none() {
    let toast = make_toast(Uuid::new_v4(), "Sticky", ToastSeverity::Error, None);
    assert!(!is_expired(&toast, fixed_time()));
}

#[test]
fn is_expired_returns_false_when_now_is_before_dismiss_time() {
    let dismiss_at = fixed_time();
    let toast = make_toast(Uuid::new_v4(), "Test", ToastSeverity::Info, Some(dismiss_at));
    let before = dismiss_at - chrono::Duration::milliseconds(1);
    assert!(!is_expired(&toast, before));
}

// ══════════════════════════════════════════════════════════════════════
// B27–B30 : ToastStoreState::evict_expired
// ══════════════════════════════════════════════════════════════════════

#[test]
fn evict_expired_removes_all_expired_toasts_and_keeps_valid() {
    let now = fixed_time();
    let past = now - chrono::Duration::seconds(10);
    let future = now + chrono::Duration::seconds(10);
    let t_expired_a = make_toast(Uuid::new_v4(), "Expired-A", ToastSeverity::Info, Some(past));
    let t_valid_b = make_toast(Uuid::new_v4(), "Valid-B", ToastSeverity::Info, Some(future));
    let t_persistent = make_toast(Uuid::new_v4(), "Persistent", ToastSeverity::Warning, None);
    let t_expired_d = make_toast(Uuid::new_v4(), "Expired-D", ToastSeverity::Error, Some(past));
    let store = ToastStoreState {
        toasts: vec![t_expired_a, t_valid_b, t_persistent, t_expired_d],
        capacity: 5,
    };
    let result = store.evict_expired(now);
    assert_eq!(result.toasts.len(), 2);
    assert_eq!(result.toasts[0].message, "Valid-B");
    assert_eq!(result.toasts[1].message, "Persistent");
}

#[test]
fn evict_expired_removes_all_when_every_toast_is_expired() {
    let now = fixed_time();
    let past = now - chrono::Duration::seconds(10);
    let t1 = make_toast(Uuid::new_v4(), "E1", ToastSeverity::Info, Some(past));
    let t2 = make_toast(Uuid::new_v4(), "E2", ToastSeverity::Info, Some(past));
    let t3 = make_toast(Uuid::new_v4(), "E3", ToastSeverity::Info, Some(past));
    let store = ToastStoreState { toasts: vec![t1, t2, t3], capacity: 5 };
    let result = store.evict_expired(now);
    assert_eq!(result.toasts.len(), 0);
    assert_eq!(result.capacity, 5);
}

#[test]
fn evict_expired_preserves_relative_ordering_of_survivors() {
    let now = fixed_time();
    let past = now - chrono::Duration::seconds(10);
    let future = now + chrono::Duration::seconds(10);
    let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    let toasts: Vec<Toast> = (0..5)
        .map(|i| {
            let dismiss = if i % 2 == 0 { Some(past) } else { Some(future) };
            make_toast(ids[i], &format!("T{i}"), ToastSeverity::Info, dismiss)
        })
        .collect();
    let store = ToastStoreState { toasts, capacity: 5 };
    let result = store.evict_expired(now);
    assert_eq!(result.toasts.len(), 2);
    assert_eq!(result.toasts[0].id, ids[1]);
    assert_eq!(result.toasts[1].id, ids[3]);
}

#[test]
fn evict_expired_is_noop_when_no_toasts_expired() {
    let now = fixed_time();
    let future = now + chrono::Duration::seconds(10);
    let t_a = make_toast(Uuid::new_v4(), "A", ToastSeverity::Info, Some(future));
    let t_b = make_toast(Uuid::new_v4(), "B", ToastSeverity::Info, None);
    let store = ToastStoreState { toasts: vec![t_a, t_b], capacity: 5 };
    let result = store.evict_expired(now);
    assert_eq!(result.toasts.len(), 2);
    assert_eq!(result.toasts[0].message, "A");
    assert_eq!(result.toasts[1].message, "B");
}

// ══════════════════════════════════════════════════════════════════════
// B31–B35 : ToastDuration
// ══════════════════════════════════════════════════════════════════════

#[test]
fn toast_duration_returns_invalid_duration_when_zero() {
    let result = ToastDuration::new(Duration::from_millis(0));
    assert_eq!(result, Err(ToastError::InvalidDuration(0)));
}

#[test]
fn toast_duration_returns_invalid_duration_when_exceeds_30s() {
    let result = ToastDuration::new(Duration::from_millis(30_001));
    assert_eq!(result, Err(ToastError::InvalidDuration(30_001)));
}

#[test]
fn toast_duration_accepts_1ms_min_boundary() {
    let result = ToastDuration::new(Duration::from_millis(1));
    let dur = result.unwrap();
    assert_eq!(dur.inner(), Duration::from_millis(1));
}

#[test]
fn toast_duration_accepts_max_valid_30s() {
    let result = ToastDuration::new(Duration::from_millis(30_000));
    let dur = result.unwrap();
    assert_eq!(dur.inner(), Duration::from_millis(30_000));
}

#[test]
fn toast_duration_default_is_3000ms() {
    let dur = ToastDuration::default();
    assert_eq!(dur.inner(), Duration::from_millis(3_000));
}

// ══════════════════════════════════════════════════════════════════════
// Proptests
// ══════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn proptest_capacity_invariant_holds_after_every_push(
        capacity in 1usize..=20,
        messages in prop::collection::vec("[a-zA-Z]{1,10}", 0..=25),
    ) {
        let store = ToastStoreState::new(capacity).unwrap();
        let final_store = messages.into_iter().try_fold(
            store,
            |s, msg| s.push(msg, ToastSeverity::Info, default_dur()),
        ).unwrap();
        prop_assert!(final_store.toasts.len() <= final_store.capacity);
    }

    #[test]
    fn proptest_message_rejection_invariant(ws in "[ \t\n\r]{0,50}") {
        let store = ToastStoreState { toasts: vec![], capacity: 5 };
        let result = store.push(ws, ToastSeverity::Info, default_dur());
        prop_assert_eq!(result, Err(ToastError::EmptyMessage));
    }

    #[test]
    fn proptest_duration_rejects_invalid_bounds(
        ms in prop_oneof![Just(0u64), 30001u64..=100_000u64]
    ) {
        let result = ToastDuration::new(Duration::from_millis(ms));
        prop_assert_eq!(result, Err(ToastError::InvalidDuration(ms)));
    }

    #[test]
    fn proptest_duration_accepts_valid_range(ms in 1u64..=30_000) {
        let dur = ToastDuration::new(Duration::from_millis(ms)).unwrap();
        prop_assert_eq!(dur.inner(), Duration::from_millis(ms));
    }

    #[test]
    fn proptest_ordering_invariant_across_push(
        messages in prop::collection::vec("[a-zA-Z]{1,10}", 1usize..=10),
    ) {
        let capacity = messages.len();
        let store = ToastStoreState::new(capacity).unwrap();
        let final_store = messages.iter().try_fold(
            store,
            |s, msg| s.push(msg.clone(), ToastSeverity::Info, default_dur()),
        ).unwrap();
        let actual: Vec<&str> = final_store.toasts.iter().map(|t| t.message.as_str()).collect();
        let expected: Vec<&str> = messages.iter().rev().map(|s| s.as_str()).collect();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn proptest_is_expired_monotonic(offset_ms in -1000i64..=1000i64) {
        let base = fixed_time();
        let toast = make_toast(Uuid::new_v4(), "Test", ToastSeverity::Info, Some(base));
        let now = base + chrono::Duration::milliseconds(offset_ms);
        let expired_now = is_expired(&toast, now);
        let later = now + chrono::Duration::milliseconds(1);
        let expired_later = is_expired(&toast, later);
        prop_assert!(!expired_now || expired_later);
    }

    #[test]
    fn proptest_id_uniqueness_across_pushes(
        messages in prop::collection::vec("[a-zA-Z]{1,10}", 1usize..=20),
    ) {
        let capacity = messages.len();
        let store = ToastStoreState::new(capacity).unwrap();
        let final_store = messages.into_iter().try_fold(
            store,
            |s, msg| s.push(msg, ToastSeverity::Info, default_dur()),
        ).unwrap();
        let ids: Vec<Uuid> = final_store.toasts.iter().map(|t| t.id).collect();
        let unique_count = ids.iter().collect::<HashSet<_>>().len();
        prop_assert_eq!(ids.len(), unique_count);
    }
}
