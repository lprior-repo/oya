use oya::hello_world;

#[test]
fn hello_world_returns_expected_value() {
    assert_eq!(hello_world(), "hello world");
}

#[test]
fn hello_world_is_stable_across_calls() {
    assert_eq!(hello_world(), hello_world());
}

#[test]
fn hello_world_is_ascii() {
    assert!(hello_world().is_ascii());
}
