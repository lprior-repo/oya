#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

const EPSILON: f64 = f64::EPSILON;

use thiserror::Error;

/// Error types for spring force calculations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SpringForceError {
    #[error("invalid stiffness parameter: {0} must be positive")]
    InvalidStiffness(f64),

    #[error("invalid rest length: {0} must be non-negative")]
    InvalidRestLength(f64),

    #[error("node positions are identical (zero-length edge)")]
    ZeroLengthEdge,
}

/// Represents a 2D position
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    x: f64,
    y: f64,
}

impl Position {
    /// Creates a new position
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculates the distance to another position
    #[must_use]
    pub fn distance(&self, other: &Self) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dx.hypot(dy)
    }

    /// Returns the normalized direction vector to another position
    ///
    /// # Errors
    ///
    /// Returns `SpringForceError::ZeroLengthEdge` if positions are identical
    pub fn direction_to(&self, other: &Self) -> Result<(f64, f64), SpringForceError> {
        let distance = self.distance(other);
        if distance < EPSILON {
            Err(SpringForceError::ZeroLengthEdge)
        } else {
            let dx = other.x - self.x;
            let dy = other.y - self.y;
            Ok((dx / distance, dy / distance))
        }
    }
}

/// Represents a force vector in 2D space
#[derive(Debug, Clone, PartialEq)]
pub struct Force {
    x: f64,
    y: f64,
}

impl Force {
    /// Creates a new force from components
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Returns the force components
    #[must_use]
    pub const fn components(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    /// Returns the magnitude of the force
    #[must_use]
    pub fn magnitude(&self) -> f64 {
        self.x.hypot(self.y)
    }
}

/// Spring force configuration using Hooke's law
#[derive(Debug, Clone, PartialEq)]
pub struct SpringForce {
    stiffness: f64,
    rest_length: f64,
}

impl SpringForce {
    /// Creates a new spring force with validation
    ///
    /// # Errors
    ///
    /// Returns `SpringForceError::InvalidStiffness` if stiffness is not positive
    /// Returns `SpringForceError::InvalidRestLength` if rest length is negative
    pub fn new(stiffness: f64, rest_length: f64) -> Result<Self, SpringForceError> {
        if stiffness <= 0.0 {
            Err(SpringForceError::InvalidStiffness(stiffness))
        } else if rest_length < 0.0 {
            Err(SpringForceError::InvalidRestLength(rest_length))
        } else {
            Ok(Self {
                stiffness,
                rest_length,
            })
        }
    }

    /// Calculates the spring force between two positions using Hooke's law
    ///
    /// # Hooke's Law
    ///
    /// F = -k * (d - L) * direction
    ///
    /// Where:
    /// - k is the stiffness coefficient
    /// - d is the current distance between nodes
    /// - L is the rest length
    /// - direction is the normalized vector from source to target
    ///
    /// # Returns
    ///
    /// A tuple of (`source_force`, `target_force`) where:
    /// - `source_force` is applied to the source node
    /// - `target_force` is the equal and opposite force applied to the target node
    ///
    /// # Errors
    ///
    /// Returns `SpringForceError::ZeroLengthEdge` if nodes are at identical positions
    pub fn calculate_force(
        &self,
        source: &Position,
        target: &Position,
    ) -> Result<(Force, Force), SpringForceError> {
        let distance = source.distance(target);
        let displacement = distance - self.rest_length;
        let force_magnitude = self.stiffness * displacement;

        let (dir_x, dir_y) = source.direction_to(target)?;

        let fx = force_magnitude * dir_x;
        let fy = force_magnitude * dir_y;

        // Newton's third law: equal and opposite forces
        // Source is pulled toward target (positive direction)
        let source_force = Force::new(fx, fy);
        // Target is pulled toward source (negative direction)
        let target_force = Force::new(-fx, -fy);

        Ok((source_force, target_force))
    }

    /// Applies spring forces to a collection of edges
    ///
    /// # Arguments
    ///
    /// * `edges` - Iterator of (`source_position`, `target_position`) tuples
    ///
    /// # Returns
    ///
    /// Iterator of (`source_force`, `target_force`) tuples
    pub fn apply_to_edges<'a, I>(
        &'a self,
        edges: I,
    ) -> impl Iterator<Item = Result<(Force, Force), SpringForceError>> + 'a
    where
        I: IntoIterator<Item = (&'a Position, &'a Position)> + 'a,
    {
        edges
            .into_iter()
            .map(|(source, target)| self.calculate_force(source, target))
    }

    /// Gets the stiffness coefficient
    #[must_use]
    pub const fn stiffness(&self) -> f64 {
        self.stiffness
    }

    /// Gets the rest length
    #[must_use]
    pub const fn rest_length(&self) -> f64 {
        self.rest_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let p1 = Position::new(10.0, 20.0);
        let p2 = Position::new(0.0, 1.0);
        let p3 = Position::new(1.0, 0.0);

        assert!((p1.distance(&p1) - 0.0).abs() < EPSILON);
        assert!((p2.distance(&p2) - 0.0).abs() < EPSILON);
        assert!((p3.distance(&p3) - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(3.0, 4.0);
        assert!((p1.distance(&p2) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_position_direction() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(1.0, 0.0);
        assert_eq!(p1.direction_to(&p2), Ok((1.0, 0.0)));

        let p3 = Position::new(0.0, 0.0);
        let p4 = Position::new(0.0, 1.0);
        assert_eq!(p3.direction_to(&p4), Ok((0.0, 1.0)));
    }

    #[test]
    fn test_position_direction_zero_length() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(0.0, 0.0);
        assert_eq!(p1.direction_to(&p2), Err(SpringForceError::ZeroLengthEdge));
    }

    #[test]
    fn test_force_components() {
        let force = Force::new(3.0, 4.0);
        assert_eq!(force.components(), (3.0, 4.0));
    }

    #[test]
    fn test_force_magnitude() {
        let force = Force::new(3.0, 4.0);
        assert!((force.magnitude() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_spring_force_creation_valid() {
        assert!(SpringForce::new(0.1, 50.0).is_ok());
        assert!(SpringForce::new(0.01, 100.0).is_ok());
    }

    #[test]
    fn test_spring_force_creation_invalid_stiffness() {
        assert_eq!(
            SpringForce::new(0.0, 50.0),
            Err(SpringForceError::InvalidStiffness(0.0))
        );
        assert_eq!(
            SpringForce::new(-0.1, 50.0),
            Err(SpringForceError::InvalidStiffness(-0.1))
        );
    }

    #[test]
    fn test_spring_force_creation_invalid_rest_length() {
        assert_eq!(
            SpringForce::new(0.1, -10.0),
            Err(SpringForceError::InvalidRestLength(-10.0))
        );
    }

    #[test]
    fn test_spring_force_at_rest() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;
        let source = Position::new(0.0, 0.0);
        let target = Position::new(50.0, 0.0);

        let (source_force, target_force) = spring.calculate_force(&source, &target)?;
        assert!((source_force.magnitude() - 0.0).abs() < EPSILON);
        assert!((target_force.magnitude() - 0.0).abs() < EPSILON);
        Ok(())
    }

    #[test]
    fn test_spring_force_stretched() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;
        let source = Position::new(0.0, 0.0);
        let target = Position::new(60.0, 0.0);

        let (source_force, target_force) = spring.calculate_force(&source, &target)?;
        // Force = 0.1 * (60 - 50) = 1.0
        let magnitude = (source_force.magnitude() - 1.0).abs();
        assert!(magnitude < EPSILON, "magnitude diff: {magnitude}");
        Ok(())
    }

    #[test]
    fn test_spring_force_compressed() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;
        let source = Position::new(0.0, 0.0);
        let target = Position::new(40.0, 0.0);

        let (source_force, target_force) = spring.calculate_force(&source, &target)?;
        // Force = 0.1 * (40 - 50) = -1.0
        let magnitude = (source_force.magnitude() - 1.0).abs();
        assert!(magnitude < EPSILON, "magnitude diff: {magnitude}");
        Ok(())
    }

    #[test]
    fn test_newton_third_law_equal_opposite() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;
        let source = Position::new(0.0, 0.0);
        let target = Position::new(60.0, 0.0);

        let (source_force, target_force) = spring.calculate_force(&source, &target)?;

        let (sx, sy) = source_force.components();
        let (tx, ty) = target_force.components();

        assert!((sx + tx).abs() < EPSILON);
        assert!((sy + ty).abs() < EPSILON);
        Ok(())
    }

    #[test]
    fn test_stiffness_affects_magnitude() -> Result<(), SpringForceError> {
        let spring1 = SpringForce::new(0.01, 50.0)?;
        let spring2 = SpringForce::new(0.1, 50.0)?;

        let source = Position::new(0.0, 0.0);
        let target = Position::new(60.0, 0.0);

        let (force1, _) = spring1.calculate_force(&source, &target)?;
        let (force2, _) = spring2.calculate_force(&source, &target)?;

        // Stiffer spring should have 10x the force
        let ratio = force2.magnitude() / force1.magnitude();
        assert!((ratio - 10.0).abs() < EPSILON);
        Ok(())
    }

    #[test]
    fn test_diagonal_force() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;
        let source = Position::new(0.0, 0.0);
        let target = Position::new(30.0, 40.0);

        let (source_force, _) = spring.calculate_force(&source, &target)?;

        let (sx, sy) = source_force.components();
        let magnitude = (sx * sx + sy * sy).sqrt();
        assert!((magnitude - 2.0).abs() < EPSILON);
        Ok(())
    }

    #[test]
    fn test_zero_length_edge_error() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;
        let source = Position::new(10.0, 10.0);
        let target = Position::new(10.0, 10.0);

        assert_eq!(
            spring.calculate_force(&source, &target),
            Err(SpringForceError::ZeroLengthEdge)
        );
        Ok(())
    }

    #[test]
    fn test_apply_to_edges() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.1, 50.0)?;

        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(60.0, 0.0);
        let p3 = Position::new(0.0, 0.0);
        let p4 = Position::new(40.0, 0.0);

        let edges = [(&p1, &p2), (&p3, &p4)];

        let results: Vec<_> = spring.apply_to_edges(edges).collect();

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());

        let (f1, _) = results[0].as_ref().map_err(|err| err.clone())?;
        let (f2, _) = results[1].as_ref().map_err(|err| err.clone())?;

        // First edge stretched, second compressed
        assert!((f1.magnitude() - 1.0).abs() < EPSILON);
        assert!((f2.magnitude() - 1.0).abs() < EPSILON);
        Ok(())
    }

    #[test]
    fn test_accessors() -> Result<(), SpringForceError> {
        let spring = SpringForce::new(0.05, 75.0)?;
        assert!((spring.stiffness() - 0.05).abs() < EPSILON);
        assert!((spring.rest_length() - 75.0).abs() < EPSILON);
        Ok(())
    }
}
