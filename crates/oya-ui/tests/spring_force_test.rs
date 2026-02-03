#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use oya_ui::layout::spring_force::{Force, Position, SpringForce, SpringForceError};
use proptest::prelude::*;

const EPSILON: f64 = 1e-9;

/// Tests that spring force never panics with valid inputs
#[test]
fn test_spring_force_never_panics() {
    // Test various stiffness and rest length combinations
    let stiffness_values = vec![0.01, 0.05, 0.1, 0.5, 1.0];
    let rest_lengths = vec![25.0, 50.0, 75.0, 100.0];

    for stiffness in &stiffness_values {
        for rest_length in &rest_lengths {
            let result = SpringForce::new(*stiffness, *rest_length);
            assert!(result.is_ok());
            let spring = result.unwrap();

            let p1 = Position::new(0.0, 0.0);
            let p2 = Position::new(60.0, 80.0);

            let force_result = spring.calculate_force(&p1, &p2);
            assert!(force_result.is_ok());
        }
    }
}

/// Property test: All valid spring parameters produce a valid struct
proptest! {
    #[test]
    fn prop_spring_force_valid_params(stiffness in 0.01..=0.1f64, rest_length in 25.0..=100.0f64) {
        let result = SpringForce::new(stiffness, rest_length);
        prop_assert!(result.is_ok());

        let spring = result.unwrap();
        prop_assert!((spring.stiffness() - stiffness).abs() < EPSILON);
        prop_assert!((spring.rest_length() - rest_length).abs() < EPSILON);
    }
}

/// Property test: Zero or negative stiffness always fails
proptest! {
    #[test]
    fn prop_invalid_stiffness_fails(stiffness in -1.0..=0.0f64) {
        let result = SpringForce::new(stiffness, 50.0);
        prop_assert!(matches!(result, Err(SpringForceError::InvalidStiffness(_))));
    }
}

/// Property test: Negative rest length always fails
proptest! {
    #[test]
    fn prop_invalid_rest_length_fails(rest_length in -100.0..=0.0f64) {
        let result = SpringForce::new(0.1, rest_length);
        prop_assert!(matches!(result, Err(SpringForceError::InvalidRestLength(_))));
    }
}

/// Property test: Non-identical positions always have valid distance
proptest! {
    #[test]
    fn prop_position_distance_always_valid(x1 in -1000.0..=1000.0f64, y1 in -1000.0..=1000.0f64, x2 in -1000.0..=1000.0f64, y2 in -1000.0..=1000.0f64) {
        // Skip identical positions
        prop_assume!(x1 != x2 || y1 != y2);

        let p1 = Position::new(x1, y1);
        let p2 = Position::new(x2, y2);

        prop_assert!(p1.is_ok());
        prop_assert!(p2.is_ok());

        let distance = p1.unwrap().distance(&p2.unwrap());
        prop_assert!(distance >= 0.0);
    }
}

/// Property test: Position direction is always normalized
proptest! {
    #[test]
    fn prop_direction_normalized(x1 in -1000.0..=1000.0f64, y1 in -1000.0..=1000.0f64, x2 in -1000.0..=1000.0f64, y2 in -1000.0..=1000.0f64) {
        // Skip identical positions
        prop_assume!((x1 - x2).abs() > EPSILON || (y1 - y2).abs() > EPSILON);

        let p1 = Position::new(x1, y1);
        let p2 = Position::new(x2, y2);

        let result = p1.direction_to(&p2);
        prop_assert!(result.is_ok());

        let (dx, dy) = result.unwrap();
        let magnitude = (dx * dx + dy * dy).sqrt();
        prop_assert!((magnitude - 1.0).abs() < 1e-6);
    }
}

/// Property test: Force magnitude increases linearly with displacement
proptest! {
    #[test]
    fn prop_force_linear_displacement(
        stiffness in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        distance in 50.0..=300.0f64
    ) {
        let spring = SpringForce::new(stiffness, rest_length).unwrap();
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(distance, 0.0);

        let result = spring.calculate_force(&p1, &p2);
        prop_assert!(result.is_ok());

        let (source_force, _) = result.unwrap();
        let expected_magnitude = stiffness * (distance - rest_length);
        prop_assert!((source_force.magnitude() - expected_magnitude).abs() < 1e-6);
    }
}

/// Property test: Newton's third law always holds
proptest! {
    #[test]
    fn prop_newton_third_law(
        stiffness in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        x1 in -1000.0..=1000.0f64,
        y1 in -1000.0..=1000.0f64,
        x2 in -1000.0..=1000.0f64,
        y2 in -1000.0..=1000.0f64
    ) {
        // Skip identical positions
        prop_assume!((x1 - x2).abs() > EPSILON || (y1 - y2).abs() > EPSILON);

        let spring = SpringForce::new(stiffness, rest_length).unwrap();
        let p1 = Position::new(x1, y1);
        let p2 = Position::new(x2, y2);

        let result = spring.calculate_force(&p1, &p2);
        prop_assert!(result.is_ok());

        let (source_force, target_force) = result.unwrap();
        let (sx, sy) = source_force.components();
        let (tx, ty) = target_force.components();

        // Forces should be equal and opposite
        prop_assert!((sx + tx).abs() < 1e-9);
        prop_assert!((sy + ty).abs() < 1e-9);
    }
}

/// Property test: At rest length, force is zero
proptest! {
    #[test]
    fn prop_force_zero_at_rest(
        stiffness in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        angle in 0.0f64..=6.28318530718f64 // 0 to 2π
    ) {
        let spring = SpringForce::new(stiffness, rest_length).unwrap();
        let p1 = Position::new(0.0, 0.0);

        let x = rest_length * angle.cos();
        let y = rest_length * angle.sin();
        let p2 = Position::new(x, y);

        let result = spring.calculate_force(&p1, &p2);
        prop_assert!(result.is_ok());

        let (source_force, target_force) = result.unwrap();
        prop_assert!(source_force.magnitude() < 1e-9);
        prop_assert!(target_force.magnitude() < 1e-9);
    }
}

/// Property test: Stiffness scales force linearly
proptest! {
    #[test]
    fn prop_stiffness_scales_force(
        k1 in 0.01..=0.1f64,
        k2 in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        distance in 100.0..=200.0f64
    ) {
        let spring1 = SpringForce::new(k1, rest_length).unwrap();
        let spring2 = SpringForce::new(k2, rest_length).unwrap();

        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(distance, 0.0);

        let result1 = spring1.calculate_force(&p1, &p2).unwrap();
        let result2 = spring2.calculate_force(&p1, &p2).unwrap();

        let mag1 = result1.0.magnitude();
        let mag2 = result2.0.magnitude();

        if k1 > 0.0 {
            let ratio = mag2 / mag1;
            let expected_ratio = k2 / k1;
            prop_assert!((ratio - expected_ratio).abs() < 1e-6);
        }
    }
}

/// Property test: Force direction always points toward target for stretched spring
proptest! {
    #[test]
    fn prop_force_direction_stretched(
        stiffness in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        distance in 60.0..=300.0f64
    ) {
        let spring = SpringForce::new(stiffness, rest_length).unwrap();
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(distance, 0.0);

        let result = spring.calculate_force(&p1, &p2);
        prop_assert!(result.is_ok());

        let (source_force, _) = result.unwrap();
        let (fx, _) = source_force.components();

        // Source should be pulled toward positive direction
        prop_assert!(fx > 0.0);
    }
}

/// Property test: Force direction always points toward source for compressed spring
proptest! {
    #[test]
    fn prop_force_direction_compressed(
        stiffness in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        distance in 10.0..=90.0f64
    ) {
        let spring = SpringForce::new(stiffness, rest_length).unwrap();
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(distance, 0.0);

        let result = spring.calculate_force(&p1, &p2);
        prop_assert!(result.is_ok());

        let (source_force, _) = result.unwrap();
        let (fx, _) = source_force.components();

        // Source should be pulled toward negative direction
        prop_assert!(fx < 0.0);
    }
}

/// Property test: apply_to_edges handles all valid edges
proptest! {
    #[test]
    fn prop_apply_to_edges_valid(
        stiffness in 0.01..=0.1f64,
        rest_length in 50.0..=100.0f64,
        edges in prop::collection::vec(
            (-1000.0f64..=1000.0f64, -1000.0f64..=1000.0f64, -1000.0f64..=1000.0f64, -1000.0f64..=1000.0f64),
            1..=10
        )
    ) {
        let spring = SpringForce::new(stiffness, rest_length).unwrap();

        let positions: Vec<_> = edges
            .into_iter()
            .filter_map(|(x1, y1, x2, y2)| {
                // Skip identical positions
                if (x1 - x2).abs() <= EPSILON && (y1 - y2).abs() <= EPSILON {
                    return None;
                }
                Some((
                    Position::new(x1, y1),
                    Position::new(x2, y2),
                ))
            })
            .collect();

        // This should never panic or produce errors for valid positions
        let results: Vec<_> = spring
            .apply_to_edges(positions.iter().map(|(p1, p2)| (p1, p2)))
            .collect();

        // All results should be Ok
        for result in results {
            prop_assert!(result.is_ok());
        }
    }
}

/// Test edge cases for numerical stability
#[test]
fn test_numerical_stability() {
    let spring = SpringForce::new(0.1, 50.0).unwrap();

    // Very small displacement
    let p1 = Position::new(0.0, 0.0);
    let p2 = Position::new(50.0001, 0.0);
    let result = spring.calculate_force(&p1, &p2);
    assert!(result.is_ok());
    assert!(result.unwrap().0.magnitude() < 1e-4);

    // Very large displacement
    let p1 = Position::new(0.0, 0.0);
    let p2 = Position::new(1_000_000.0, 0.0);
    let result = spring.calculate_force(&p1, &p2);
    assert!(result.is_ok());
    assert!(result.unwrap().0.magnitude() > 1.0);

    // Very small stiffness
    let spring_small = SpringForce::new(0.0001, 50.0).unwrap();
    let p1 = Position::new(0.0, 0.0);
    let p2 = Position::new(100.0, 0.0);
    let result = spring_small.calculate_force(&p1, &p2);
    assert!(result.is_ok());
    // Force = 0.0001 * 50 = 0.005
    assert!((result.unwrap().0.magnitude() - 0.005).abs() < 1e-6);

    // Very large stiffness
    let spring_large = SpringForce::new(10.0, 50.0).unwrap();
    let p1 = Position::new(0.0, 0.0);
    let p2 = Position::new(100.0, 0.0);
    let result = spring_large.calculate_force(&p1, &p2);
    assert!(result.is_ok());
    // Force = 10.0 * 50 = 500.0
    assert!((result.unwrap().0.magnitude() - 500.0).abs() < 1e-6);
}
