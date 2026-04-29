//! Value objects for domain types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunOutcome {
    Success,
    Failure,
}

impl RunOutcome {
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PositiveDuration {
    pub millis: u64,
}

impl PositiveDuration {
    #[must_use]
    pub const fn from_millis(millis: u64) -> Option<Self> {
        if millis == 0 {
            return None;
        }
        Some(Self { millis })
    }

    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.millis
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub category: super::NodeCategory,
    pub label: &'static str,
    pub icon: super::NodeIcon,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    mod positive_duration {
        use super::*;

        #[test]
        fn zero_is_none() {
            assert!(PositiveDuration::from_millis(0).is_none());
        }

        #[test]
        fn positive_is_some() {
            let d = PositiveDuration::from_millis(100);
            assert!(d.is_some());
            assert_eq!(d.unwrap().as_millis(), 100);
        }
    }

    mod run_outcome {
        use super::*;

        #[test]
        fn success_is_success() {
            assert!(RunOutcome::Success.is_success());
        }

        #[test]
        fn failure_is_not_success() {
            assert!(!RunOutcome::Failure.is_success());
        }
    }
}
