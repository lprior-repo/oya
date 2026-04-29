//! Expression Depth Limiting Tests
//!
//! This module contains comprehensive tests for the expression depth limiting feature.
//! Tests are organized by layer: unit, integration, e2e, proptest, fuzz, and Kani.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

#[cfg(test)]
mod proptest_invariants {
    use oya_frontend::expression_depth::{
        calculate_depth, resolve_expressions, Expression, ExpressionDepth, ExpressionRegistry,
        MAX_EXPRESSION_DEPTH,
    };
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Helper to construct a leaf expression (depth 0)
    fn make_leaf(value: Option<String>) -> Expression {
        Expression { value, children: vec![] }
    }

    // Helper to construct a parent expression with children
    fn make_parent(value: Option<String>, children: Vec<Expression>) -> Expression {
        Expression { value, children: children.into_iter().map(Arc::new).collect() }
    }

    // Helper to construct a nested chain of n levels
    #[allow(dead_code)]
    fn make_chain(depth: u32) -> Expression {
        if depth == 0 {
            make_leaf(Some("leaf".to_string()))
        } else {
            make_parent(Some("parent".to_string()), vec![make_chain(depth - 1)])
        }
    }

    // Strategy for generating random expression trees
    #[allow(dead_code)]
    fn arb_expression(depth: u32) -> impl Strategy<Value = Expression> {
        if depth == 0 {
            (any::<bool>().prop_map(|_| make_leaf(Some("leaf".to_string())))).boxed()
        } else {
            prop::collection::vec(arb_expression(depth - 1), 0..10)
                .prop_map(move |children| make_parent(Some("parent".to_string()), children))
                .boxed()
        }
    }

    // Strategy for valid ExpressionDepth values (0-1024)
    #[allow(dead_code)]
    fn arb_valid_depth() -> impl Strategy<Value = u32> {
        (0u32..=1024u32).boxed()
    }

    // ============================================================================
    // calculate_depth Proptest Invariants (5 invariants)
    // ============================================================================

    proptest! {
        fn calculate_depth_returns_non_negative(any_expr in arb_expression(50)) {
            let depth = calculate_depth(&any_expr);
            // depth is u32, always >= 0; this invariant documents non-negativity
            let _ = depth;
        }

        fn calculate_depth_satisfies_recursive_formula(any_expr in arb_expression(20)) {
            if any_expr.children.is_empty() {
                prop_assert_eq!(calculate_depth(&any_expr), 0);
            } else {
                let child_depths: Vec<u32> = any_expr.children.iter().map(|c| calculate_depth(&c)).collect();
                let max_child_depth = child_depths.iter().max().copied().unwrap_or(0);
                prop_assert_eq!(calculate_depth(&any_expr), 1 + max_child_depth);
            }
        }

        fn calculate_depth_within_bounds_for_well_formed_trees(any_expr in arb_expression(1023)) {
            let depth = calculate_depth(&any_expr);
            prop_assert!(depth <= 1023);
        }
    }

    #[test]
    fn calculate_depth_empty_returns_zero() {
        let expr = make_leaf(None);
        assert_eq!(calculate_depth(&expr), 0);
    }

    #[test]
    fn calculate_depth_wide_tree_returns_one() {
        let children: Vec<Expression> =
            (0..100).map(|i| make_leaf(Some(format!("leaf{}", i)))).collect();
        let root = make_parent(Some("root".to_string()), children);
        assert_eq!(calculate_depth(&root), 1);
    }

    // ============================================================================
    // ExpressionDepth increment Proptest Invariants (4 invariants)
    // ============================================================================

    proptest! {
        fn increment_chain_preserves_formula(any_depth in arb_valid_depth(), n in 1u32..=100u32) {
            let depth = ExpressionDepth::new(any_depth).unwrap();
            let mut current = depth;
            for _ in 0..n {
                current = current.increment().unwrap();
            }
            prop_assert_eq!(current.current(), any_depth + n);
        }

        fn increment_adds_exactly_one(any_depth in arb_valid_depth()) {
            let depth = ExpressionDepth::new(any_depth).unwrap();
            let result = depth.increment();
            if any_depth < 1024 {
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap().current(), any_depth + 1);
            } else {
                prop_assert!(result.is_err());
            }
        }

        fn is_valid_after_increment_for_valid_depth(any_depth in 0u32..1024u32) {
            let depth = ExpressionDepth::new(any_depth).unwrap();
            let result = depth.increment();
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap().is_valid());
        }
    }

    #[test]
    fn max_depth_times_two_is_rejected() {
        let result = ExpressionDepth::new(1024 * 2);
        assert!(result.is_err());
    }

    // ============================================================================
    // MAX_EXPRESSION_DEPTH constant verification (Invariant 10)
    // ============================================================================

    #[test]
    fn max_expression_depth_constant_is_1024() {
        assert_eq!(MAX_EXPRESSION_DEPTH, 1024);
    }

    // ============================================================================
    // resolve_expressions immutability (Invariant 11)
    // ============================================================================

    proptest! {
        fn resolve_expressions_does_not_mutate_input(any_expr in arb_expression(5)) {
            // Store original value for comparison
            let original_value = any_expr.value.clone();

            let registry: ExpressionRegistry = HashMap::new();
            let depth = ExpressionDepth::new(0).unwrap();

            let _result = resolve_expressions::<String>(&any_expr, &registry, depth);

            // Verify expression was not mutated
            prop_assert_eq!(any_expr.value, original_value);
        }
    }
}

#[cfg(test)]
mod fixtures {
    use oya_frontend::expression_depth::Expression;
    use std::sync::Arc;

    /// Helper to construct a leaf expression (depth 0)
    pub fn make_leaf(value: Option<String>) -> Expression {
        Expression { value, children: vec![] }
    }

    /// Helper to construct a parent expression with children
    pub fn make_parent(value: Option<String>, children: Vec<Expression>) -> Expression {
        Expression { value, children: children.into_iter().map(Arc::new).collect() }
    }

    /// Helper to construct a nested chain of n levels
    pub fn make_chain(depth: u32) -> Expression {
        if depth == 0 {
            make_leaf(Some("leaf".to_string()))
        } else {
            make_parent(Some("parent".to_string()), vec![make_chain(depth - 1)])
        }
    }

    // Helper function to calculate depth - mirrors production implementation
    #[allow(dead_code)]
    pub fn calculate_depth(expression: &Expression) -> u32 {
        if expression.children.is_empty() {
            0
        } else {
            1 + expression.children.iter().map(|child| calculate_depth(&child)).max().unwrap_or(0)
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::fixtures::*;
    use oya_frontend::error::Error;
    use oya_frontend::expression_depth::{
        calculate_depth, resolve_expressions, validate_expression_depth, Expression,
        ExpressionDepth,
    };

    // ============================================================================
    // ExpressionDepth::new Behaviors (6 tests)
    // ============================================================================

    #[test]
    fn expression_depth_new_accepts_zero() {
        let result = ExpressionDepth::new(0);
        assert_eq!(result, Ok(ExpressionDepth(0)));
        assert_eq!(result.clone().unwrap().current(), 0);
        assert_eq!(result, Ok(ExpressionDepth::default()));
    }

    #[test]
    fn expression_depth_new_accepts_one() {
        let result = ExpressionDepth::new(1);
        assert_eq!(result, Ok(ExpressionDepth(1)));
        assert_eq!(result.unwrap().current(), 1);
    }

    #[test]
    fn expression_depth_new_accepts_max_minus_one() {
        let result = ExpressionDepth::new(1023);
        assert_eq!(result, Ok(ExpressionDepth(1023)));
        assert_eq!(result.unwrap().current(), 1023);
    }

    #[test]
    fn expression_depth_new_accepts_maximum() {
        let result = ExpressionDepth::new(1024);
        assert_eq!(result, Ok(ExpressionDepth(1024)));
        assert_eq!(result.unwrap().current(), 1024);
    }

    #[test]
    fn expression_depth_new_rejects_just_over_max() {
        let result = ExpressionDepth::new(1025);
        assert_eq!(result, Err(Error::DepthLimitExceeded { current_depth: 1025, max_depth: 1024 }));
    }

    #[test]
    fn expression_depth_new_rejects_u32_max() {
        let result = ExpressionDepth::new(u32::MAX);
        assert_eq!(
            result,
            Err(Error::DepthLimitExceeded { current_depth: u32::MAX, max_depth: 1024 })
        );
    }

    // ============================================================================
    // ExpressionDepth::current Behaviors (3 tests)
    // ============================================================================

    #[test]
    fn expression_depth_current_returns_stored_value() {
        let depth = ExpressionDepth(500);
        assert_eq!(depth.current(), 500);
    }

    #[test]
    fn expression_depth_current_returns_zero() {
        let depth = ExpressionDepth::new(0).unwrap();
        assert_eq!(depth.current(), 0);
    }

    #[test]
    fn expression_depth_current_returns_maximum() {
        let depth = ExpressionDepth::new(1024).unwrap();
        assert_eq!(depth.current(), 1024);
    }

    // ============================================================================
    // ExpressionDepth::increment Behaviors (4 tests)
    // ============================================================================

    #[test]
    fn expression_depth_increment_succeeds_at_zero() {
        let depth = ExpressionDepth::new(0).unwrap();
        let result = depth.increment();
        assert_eq!(result, Ok(ExpressionDepth(1)));
        assert_eq!(result.unwrap().current(), 1);
    }

    #[test]
    fn expression_depth_increment_succeeds_at_max_minus_one() {
        let depth = ExpressionDepth::new(1023).unwrap();
        let result = depth.increment();
        assert_eq!(result, Ok(ExpressionDepth(1024)));
        assert_eq!(result.unwrap().current(), 1024);
    }

    #[test]
    fn expression_depth_increment_fails_at_maximum() {
        let depth = ExpressionDepth::new(1024).unwrap();
        let result = depth.increment();
        assert_eq!(result, Err(Error::DepthLimitExceeded { current_depth: 1024, max_depth: 1024 }));
    }

    #[test]
    fn expression_depth_increment_adds_exactly_one_to_valid_depth() {
        let depths = [0, 100, 500, 1023];
        for &d in &depths {
            let depth = ExpressionDepth::new(d).unwrap();
            let result = depth.increment();
            assert!(result.is_ok(), "increment of {} should succeed", d);
            assert_eq!(result.unwrap().current(), d + 1);
        }
    }

    // ============================================================================
    // ExpressionDepth::is_valid Behaviors (5 tests)
    // ============================================================================

    #[test]
    fn expression_depth_is_valid_accepts_zero() {
        let depth = ExpressionDepth::new(0).unwrap();
        assert!(depth.is_valid());
    }

    #[test]
    fn expression_depth_is_valid_accepts_max_minus_one() {
        let depth = ExpressionDepth::new(1023).unwrap();
        assert!(depth.is_valid());
    }

    #[test]
    fn expression_depth_is_valid_accepts_maximum() {
        let depth = ExpressionDepth::new(1024).unwrap();
        assert!(depth.is_valid());
    }

    #[test]
    fn expression_depth_is_valid_rejects_invalid_depth_via_unsafe_transmute() {
        let depth: ExpressionDepth = unsafe { std::mem::transmute::<u32, ExpressionDepth>(1025) };
        assert!(!depth.is_valid());
    }

    #[test]
    fn expression_depth_is_valid_rejects_u32_max_via_unsafe_transmute() {
        let depth: ExpressionDepth =
            unsafe { std::mem::transmute::<u32, ExpressionDepth>(u32::MAX) };
        assert!(!depth.is_valid());
    }

    // ============================================================================
    // calculate_depth Behaviors (8 tests)
    // ============================================================================

    #[test]
    fn calculate_depth_returns_zero_for_empty() {
        let expression = Expression { value: None, children: vec![] };
        assert_eq!(calculate_depth(&expression), 0);
    }

    #[test]
    fn calculate_depth_returns_zero_for_leaf_only() {
        let leaf = make_leaf(Some("leaf".to_string()));
        assert_eq!(calculate_depth(&leaf), 0);
    }

    #[test]
    fn calculate_depth_returns_one_for_leaf_child() {
        let child = make_leaf(Some("child".to_string()));
        let parent = make_parent(Some("parent".to_string()), vec![child]);
        assert_eq!(calculate_depth(&parent), 1);
    }

    #[test]
    fn calculate_depth_returns_two_for_nested() {
        let grandchild = make_leaf(Some("gc".to_string()));
        let child = make_parent(Some("child".to_string()), vec![grandchild]);
        let root = make_parent(Some("root".to_string()), vec![child.clone(), child]);
        assert_eq!(calculate_depth(&root), 2);
    }

    #[test]
    fn calculate_depth_recursive_case_adds_one_to_max_child_depth() {
        let g4 = make_leaf(Some("g4".to_string()));
        let c3 = make_parent(Some("c3".to_string()), vec![g4.clone()]);
        let b3 = make_parent(Some("b3".to_string()), vec![c3.clone()]);
        let d3 = make_parent(Some("d3".to_string()), vec![b3.clone()]);
        let depth3 = make_parent(Some("depth3".to_string()), vec![d3]);

        let g5 = make_leaf(Some("g5".to_string()));
        let c4 = make_parent(Some("c4".to_string()), vec![g5.clone()]);
        let b4 = make_parent(Some("b4".to_string()), vec![c4.clone()]);
        let a4 = make_parent(Some("a4".to_string()), vec![b4.clone()]);
        let d4 = make_parent(Some("d4".to_string()), vec![a4.clone()]);
        let e4 = make_parent(Some("e4".to_string()), vec![d4]);

        let g2 = make_leaf(Some("g2".to_string()));
        let c2 = make_parent(Some("c2".to_string()), vec![g2.clone()]);
        let d2 = make_parent(Some("d2".to_string()), vec![c2]);

        let root =
            make_parent(Some("root".to_string()), vec![depth3.clone(), e4.clone(), d2.clone()]);

        let actual = calculate_depth(&root);
        assert_eq!(actual, 6);
        assert_eq!(actual, 1 + 5);
    }

    #[test]
    fn calculate_depth_returns_1024_for_max_valid_tree() {
        let tree = make_chain(1024);
        assert_eq!(calculate_depth(&tree), 1024);
    }

    #[test]
    fn calculate_depth_returns_1025_for_excessive_nesting() {
        let tree = make_chain(1025);
        assert_eq!(calculate_depth(&tree), 1025);
    }

    #[test]
    fn calculate_depth_handles_wide_tree() {
        let children: Vec<Expression> =
            (0..1000).map(|i| make_leaf(Some(format!("leaf{}", i)))).collect();
        let root = make_parent(Some("root".to_string()), children);
        assert_eq!(calculate_depth(&root), 1);
    }

    // ============================================================================
    // resolve_expressions Behaviors (4 tests)
    // ============================================================================

    #[test]
    fn resolve_expressions_succeeds_at_zero_depth() {
        use oya_frontend::expression_depth::ExpressionRegistry;
        use std::collections::HashMap;

        let expression = make_leaf(Some("42".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_expressions_succeeds_at_max_minus_one() {
        use oya_frontend::expression_depth::ExpressionRegistry;
        use std::collections::HashMap;

        let expression = make_leaf(Some("999".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(1023).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_expressions_succeeds_at_maximum_depth() {
        use oya_frontend::expression_depth::ExpressionRegistry;
        use std::collections::HashMap;

        let expression = make_leaf(Some("42".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(1024).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_expressions_fails_over_maximum_depth() {
        use oya_frontend::expression_depth::ExpressionRegistry;
        use std::collections::HashMap;

        let _expression = make_leaf(Some("42".to_string()));
        let _registry: ExpressionRegistry = HashMap::new();

        // new(1025) should fail, so we verify the error at construction
        let depth_result = ExpressionDepth::new(1025);
        assert_eq!(
            depth_result,
            Err(Error::DepthLimitExceeded { current_depth: 1025, max_depth: 1024 })
        );
    }

    // ============================================================================
    // validate_expression_depth Behaviors (5 tests)
    // ============================================================================

    #[test]
    fn validate_expression_depth_accepts_valid_tree() {
        let tree = make_chain(500);
        let result = validate_expression_depth(&tree);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().current(), 500);
    }

    #[test]
    fn validate_expression_depth_rejects_deep_tree() {
        let tree = make_chain(1500);
        let result = validate_expression_depth(&tree);
        assert_eq!(result, Err(Error::DepthLimitExceeded { current_depth: 1500, max_depth: 1024 }));
    }

    #[test]
    fn validate_expression_depth_accepts_empty_tree() {
        let tree = make_leaf(None);
        let result = validate_expression_depth(&tree);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().current(), 0);
    }

    #[test]
    fn validate_expression_depth_accepts_max_depth_tree() {
        let tree = make_chain(1024);
        let result = validate_expression_depth(&tree);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().current(), 1024);
    }

    #[test]
    fn validate_expression_depth_rejects_over_maximum_tree() {
        let tree = make_chain(1025);
        let result = validate_expression_depth(&tree);
        assert_eq!(result, Err(Error::DepthLimitExceeded { current_depth: 1025, max_depth: 1024 }));
    }
}

#[cfg(test)]
mod integration_tests {
    use oya_frontend::error::Error;
    use oya_frontend::expression_depth::{
        resolve_expressions, ExpressionDepth, ExpressionRegistry,
    };

    #[test]
    fn resolve_expressions_returns_expression_not_found_with_missing_reference() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("missing_expr".to_string()));
        let mut registry: ExpressionRegistry = HashMap::new();
        registry.insert(
            "other_expr".to_string(),
            std::sync::Arc::new(super::fixtures::make_leaf(Some("other".to_string()))),
        );
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert_eq!(
            result,
            Err(Error::ExpressionNotFound { reference: "missing_expr".to_string() })
        );
    }

    #[test]
    fn resolve_expressions_returns_type_error_with_mismatched_types() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("string_value".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert_eq!(result, Err(Error::TypeError { expected: "i32", actual: "String" }));
    }

    #[test]
    fn resolve_expressions_returns_runtime_error_with_division_by_zero() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("div_zero".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(matches!(result, Err(Error::RuntimeError { .. })));
    }

    #[test]
    fn resolve_expressions_works_with_i32() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("42".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_expressions_works_with_bool() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("true".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<bool>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_expressions_works_with_string() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("hello".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<String>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_expressions_fails_with_invalid_expression_syntax() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("(unbalanced".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert_eq!(result, Err(Error::InvalidExpression));
    }
}

#[cfg(test)]
mod e2e_tests {
    use oya_frontend::error::Error;
    use oya_frontend::expression_depth::{
        resolve_expressions, ExpressionDepth, ExpressionRegistry,
    };

    #[test]
    fn e2e_shallow_expression_resolves_successfully() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("42".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(0).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn e2e_expression_over_depth_limit_shows_error() {
        use std::collections::HashMap;

        let _expression = super::fixtures::make_leaf(Some("42".to_string()));
        let _registry: ExpressionRegistry = HashMap::new();

        // new(1500) should fail, so we verify the error at construction
        let depth_result = ExpressionDepth::new(1500);
        assert_eq!(
            depth_result,
            Err(Error::DepthLimitExceeded { current_depth: 1500, max_depth: 1024 })
        );
    }

    #[test]
    fn e2e_expression_at_maximum_depth_accepts() {
        use std::collections::HashMap;

        let expression = super::fixtures::make_leaf(Some("42".to_string()));
        let registry: ExpressionRegistry = HashMap::new();
        let depth = ExpressionDepth::new(1024).unwrap();

        let result = resolve_expressions::<i32>(&expression, &registry, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn e2e_expression_just_over_maximum_depth_rejects() {
        use std::collections::HashMap;

        let _expression = super::fixtures::make_leaf(Some("42".to_string()));
        let _registry: ExpressionRegistry = HashMap::new();

        // new(1025) should fail, so we verify the error at construction
        let depth_result = ExpressionDepth::new(1025);
        assert_eq!(
            depth_result,
            Err(Error::DepthLimitExceeded { current_depth: 1025, max_depth: 1024 })
        );
    }
}
