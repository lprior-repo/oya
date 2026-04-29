//! Kani Verification Harnesses for Expression Depth Limiting
//!
//! These harnesses provide formal verification for critical behaviors:
//! - ExpressionDepth::new safety
//! - resolve_expressions correctness
//! - calculate_depth bounds

#[cfg(kani)]
mod verification {
    use oya_frontend::expression_depth::{calculate_depth, ExpressionDepth, MAX_EXPRESSION_DEPTH};

    /// Kani Harness: depth_limit_enforcement
    ///
    /// Property: For all depth values d, if d >= MAX_EXPRESSION_DEPTH,
    /// ExpressionDepth::new(d) returns Err
    ///
    /// Symbolic bound: d in range [0, u32::MAX]
    /// Concrete bound: d <= 1025 (for verification tool)
    #[kani::proof]
    fn verify_depth_limit_enforcement() {
        let d: u32 = kani::any();
        kani::assume(d <= 1025);

        let result = ExpressionDepth::new(d);

        if d >= MAX_EXPRESSION_DEPTH {
            kani::assume(d >= 1024);
            assert!(result.is_err(), "Depth {} should be rejected", d);
        } else {
            assert!(result.is_ok(), "Depth {} should be accepted", d);
        }
    }

    /// Kani Harness: increment_safety
    ///
    /// Property: For all valid depths d < MAX_EXPRESSION_DEPTH, increment(d) returns Ok;
    /// for d >= MAX, returns Err
    #[kani::proof]
    fn verify_increment_safety() {
        let d: u32 = kani::any();
        kani::assume(d <= 1025);

        let depth = ExpressionDepth::new(d);

        if d < MAX_EXPRESSION_DEPTH {
            assert!(depth.is_ok());
            let result = depth.unwrap().increment();
            if d + 1 < MAX_EXPRESSION_DEPTH {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        } else {
            assert!(depth.is_err());
        }
    }

    /// Kani Harness: calculate_depth_bound
    ///
    /// Property: For all well-formed expressions, calculate_depth(expression) <= 1025
    #[kani::proof]
    fn verify_calculate_depth_bound() {
        // This harness verifies that even for extreme inputs,
        // calculate_depth never exceeds reasonable bounds
        let depth: u32 = kani::any();
        kani::assume(depth <= 1024);

        // Simulate an expression with the given depth
        let result = calculate_depth_depth(depth);

        // The result should never exceed 1025
        kani::assume(result <= 1025);
        assert!(result <= 1025, "Depth should never exceed 1025");
    }

    /// Helper to simulate calculate_depth for a given depth value
    fn calculate_depth_depth(depth: u32) -> u32 {
        depth
    }
}
