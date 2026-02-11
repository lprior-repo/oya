//! Standalone test for clippy unwrap hook
//! This should fail with the current workspace clippy configuration

fn main() {
    // Test 1: Basic unwrap on None
    let maybe_value: Option<i32> = None;
    let _value = maybe_value.unwrap();
    println!("This will panic at runtime");

    // Test 2: expect with message
    let maybe_string: Option<String> = None;
    let _string = maybe_string.expect("This should have a value");

    // Test 3: Result unwrap
    let result: Result<String, &str> = Err("error");
    let _value = result.unwrap();

    println!("All these unwraps should be caught by clippy");
}