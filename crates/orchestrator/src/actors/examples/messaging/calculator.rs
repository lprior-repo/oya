//! Calculator Actor - Demonstrates call() and cast() patterns
//!
//! This actor shows:
//! - **call()** pattern: Request-response for arithmetic operations
//! - **cast()** pattern: Fire-and-forget for state mutations

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::fmt;

//==============================================================================
// Messages
//==============================================================================

/// Calculator messages supporting both call and cast patterns
#[derive(Debug)]
pub enum CalculatorMessage {
    // ═══════════════════════════════════════════════════════════════════════
    // QUERY MESSAGES (use call! for request-response)
    // ═══════════════════════════════════════════════════════════════════════
    /// Add two numbers
    Add {
        a: i64,
        b: i64,
        reply: RpcReplyPort<Result<i64, CalculatorError>>,
    },

    /// Subtract two numbers
    Subtract {
        a: i64,
        b: i64,
        reply: RpcReplyPort<Result<i64, CalculatorError>>,
    },

    /// Multiply two numbers
    Multiply {
        a: i64,
        b: i64,
        reply: RpcReplyPort<Result<i64, CalculatorError>>,
    },

    /// Divide two numbers
    Divide {
        a: i64,
        b: i64,
        reply: RpcReplyPort<Result<i64, CalculatorError>>,
    },

    /// Get the current value
    GetValue { reply: RpcReplyPort<i64> },

    /// Get calculator statistics
    GetStats {
        reply: RpcReplyPort<CalculatorStats>,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // COMMAND MESSAGES (use cast! or send_message for fire-and-forget)
    // ═══════════════════════════════════════════════════════════════════════
    /// Reset the calculator to zero
    Reset,

    /// Set the calculator to a specific value
    SetValue { value: i64 },

    /// Increment the current value
    Increment { amount: i64 },

    /// Decrement the current value
    Decrement { amount: i64 },
}

//==============================================================================
// Error Types
//==============================================================================

/// Calculator-specific errors
#[derive(Debug, Clone, PartialEq)]
pub enum CalculatorError {
    /// Division by zero attempted
    DivisionByZero,
    /// Overflow occurred
    Overflow,
    /// Invalid operation
    InvalidOperation(String),
}

impl fmt::Display for CalculatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "Division by zero"),
            Self::Overflow => write!(f, "Arithmetic overflow"),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for CalculatorError {}

//==============================================================================
// State & Statistics
//==============================================================================

/// Calculator state using persistent data structures
#[derive(Debug, Clone, PartialEq)]
pub struct CalculatorState {
    /// Current value
    value: i64,
    /// Number of operations performed
    operation_count: usize,
    /// Number of additions
    additions: usize,
    /// Number of subtractions
    subtractions: usize,
    /// Number of multiplications
    multiplications: usize,
    /// Number of divisions
    divisions: usize,
}

impl CalculatorState {
    /// Create a new calculator state
    pub fn new() -> Self {
        Self {
            value: 0,
            operation_count: 0,
            additions: 0,
            subtractions: 0,
            multiplications: 0,
            divisions: 0,
        }
    }

    /// Get the current value
    pub fn value(&self) -> i64 {
        self.value
    }

    /// Get the operation count
    pub fn operation_count(&self) -> usize {
        self.operation_count
    }
}

impl Default for CalculatorState {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculator statistics
#[derive(Debug, Clone, PartialEq)]
pub struct CalculatorStats {
    /// Current value
    pub value: i64,
    /// Total operations performed
    pub operation_count: usize,
    /// Number of additions
    pub additions: usize,
    /// Number of subtractions
    pub subtractions: usize,
    /// Number of multiplications
    pub multiplications: usize,
    /// Number of divisions
    pub divisions: usize,
}

//==============================================================================
// Actor Implementation
//==============================================================================

/// Calculator actor demonstrating call and cast patterns
pub struct CalculatorActor;

impl CalculatorActor {
    /// Create a new calculator actor
    pub fn new() -> Self {
        Self
    }
}

impl Default for CalculatorActor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Actor for CalculatorActor {
    type Msg = CalculatorMessage;
    type State = CalculatorState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(CalculatorState::new())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // ═════════════════════════════════════════════════════════════════
            // QUERY HANDLERS (request-response)
            // ═════════════════════════════════════════════════════════════════
            CalculatorMessage::Add { a, b, reply } => {
                state.operation_count += 1;
                state.additions += 1;

                let result = a.checked_add(b).ok_or(CalculatorError::Overflow);

                // Send reply back
                let _ = reply.send(result);
                Ok(())
            }

            CalculatorMessage::Subtract { a, b, reply } => {
                state.operation_count += 1;
                state.subtractions += 1;

                let result = a.checked_sub(b).ok_or(CalculatorError::Overflow);

                let _ = reply.send(result);
                Ok(())
            }

            CalculatorMessage::Multiply { a, b, reply } => {
                state.operation_count += 1;
                state.multiplications += 1;

                let result = a.checked_mul(b).ok_or(CalculatorError::Overflow);

                let _ = reply.send(result);
                Ok(())
            }

            CalculatorMessage::Divide { a, b, reply } => {
                state.operation_count += 1;
                state.divisions += 1;

                let result = if b == 0 {
                    Err(CalculatorError::DivisionByZero)
                } else {
                    a.checked_div(b).ok_or(CalculatorError::Overflow)
                };

                let _ = reply.send(result);
                Ok(())
            }

            CalculatorMessage::GetValue { reply } => {
                let _ = reply.send(state.value);
                Ok(())
            }

            CalculatorMessage::GetStats { reply } => {
                let stats = CalculatorStats {
                    value: state.value,
                    operation_count: state.operation_count,
                    additions: state.additions,
                    subtractions: state.subtractions,
                    multiplications: state.multiplications,
                    divisions: state.divisions,
                };
                let _ = reply.send(stats);
                Ok(())
            }

            // ═════════════════════════════════════════════════════════════════
            // COMMAND HANDLERS (fire-and-forget)
            // ═════════════════════════════════════════════════════════════════
            CalculatorMessage::Reset => {
                state.value = 0;
                Ok(())
            }

            CalculatorMessage::SetValue { value } => {
                state.value = value;
                Ok(())
            }

            CalculatorMessage::Increment { amount } => {
                state.value = state
                    .value
                    .checked_add(amount)
                    .ok_or_else(|| ActorProcessingErr::from("Overflow on increment".to_string()))?;
                Ok(())
            }

            CalculatorMessage::Decrement { amount } => {
                state.value = state.value.checked_sub(amount).ok_or_else(|| {
                    ActorProcessingErr::from("Underflow on decrement".to_string())
                })?;
                Ok(())
            }
        }
    }
}

//==============================================================================
// Helper Types
//==============================================================================

/// Result of a calculator operation
#[derive(Debug, Clone, PartialEq)]
pub enum CalculatorResult {
    /// Successful operation with result
    Success(i64),
    /// Failed operation
    Error(CalculatorError),
}

impl CalculatorResult {
    /// Convert from Result<i64, CalculatorError>
    pub fn from_result(result: Result<i64, CalculatorError>) -> Self {
        match result {
            Ok(value) => Self::Success(value),
            Err(err) => Self::Error(err),
        }
    }

    /// Check if the result is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if the result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get the value if successful
    pub fn value(&self) -> Option<i64> {
        match self {
            Self::Success(v) => Some(*v),
            Self::Error(_) => None,
        }
    }

    /// Get the error if failed
    pub fn error(&self) -> Option<&CalculatorError> {
        match self {
            Self::Success(_) => None,
            Self::Error(err) => Some(err),
        }
    }
}

//==============================================================================
// Tests
//==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ractor::{call, cast};

    /// Helper to spawn a calculator actor for testing
    async fn spawn_calculator()
    -> Result<(ActorRef<CalculatorMessage>, ractor::ActorHandle), ActorProcessingErr> {
        Actor::spawn(None, CalculatorActor::new(), ()).await
    }

    #[tokio::test]
    async fn call_pattern_add_two_numbers() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A calculator actor
        let (actor, handle) = spawn_calculator().await?;

        // When: Adding two numbers using call pattern
        let result = call!(
            actor,
            CalculatorMessage::Add {
                a: 10,
                b: 20,
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;

        // Then: Result should be correct
        assert_eq!(result, Ok(30));

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn call_pattern_divide_by_zero_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A calculator actor
        let (actor, handle) = spawn_calculator().await?;

        // When: Dividing by zero
        let result = call!(
            actor,
            CalculatorMessage::Divide {
                a: 10,
                b: 0,
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;

        // Then: Should return DivisionByZero error
        assert_eq!(result, Err(CalculatorError::DivisionByZero));

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn cast_pattern_reset_calculator() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A calculator actor with a value set
        let (actor, handle) = spawn_calculator().await?;
        cast!(actor, CalculatorMessage::SetValue { value: 42 })?;

        // Verify value is set
        let value = call!(
            actor,
            CalculatorMessage::GetValue {
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;
        assert_eq!(value, 42);

        // When: Resetting using cast pattern (fire-and-forget)
        cast!(actor, CalculatorMessage::Reset)?;

        // Give actor time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Then: Value should be reset to zero
        let value = call!(
            actor,
            CalculatorMessage::GetValue {
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;
        assert_eq!(value, 0);

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn cast_pattern_increment_value() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A calculator actor
        let (actor, handle) = spawn_calculator().await?;

        // Set initial value
        cast!(actor, CalculatorMessage::SetValue { value: 10 })?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // When: Incrementing using cast pattern
        cast!(actor, CalculatorMessage::Increment { amount: 5 })?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Then: Value should be incremented
        let value = call!(
            actor,
            CalculatorMessage::GetValue {
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;
        assert_eq!(value, 15);

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn call_pattern_get_statistics() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A calculator actor
        let (actor, handle) = spawn_calculator().await?;

        // Perform some operations
        call!(
            actor,
            CalculatorMessage::Add {
                a: 1,
                b: 2,
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;

        call!(
            actor,
            CalculatorMessage::Multiply {
                a: 3,
                b: 4,
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;

        cast!(actor, CalculatorMessage::Reset)?;

        // When: Getting statistics
        let stats = call!(
            actor,
            CalculatorMessage::GetStats {
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;

        // Then: Should track all operations
        assert_eq!(stats.additions, 1);
        assert_eq!(stats.multiplications, 1);
        assert_eq!(stats.operation_count, 2);

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn call_pattern_overflow_detection() -> Result<(), Box<dyn std::error::Error>> {
        // Given: A calculator actor
        let (actor, handle) = spawn_calculator().await?;

        // When: Overflowing addition
        let result = call!(
            actor,
            CalculatorMessage::Add {
                a: i64::MAX,
                b: 1,
                reply: ractor::RpcReplyPort::new(),
            },
            Some(tokio::time::Duration::from_millis(100))
        )?;

        // Then: Should return Overflow error
        assert_eq!(result, Err(CalculatorError::Overflow));

        // Cleanup
        actor.stop(None);
        handle.await?;

        Ok(())
    }

    #[test]
    fn calculator_error_display() {
        assert_eq!(
            CalculatorError::DivisionByZero.to_string(),
            "Division by zero"
        );
        assert_eq!(CalculatorError::Overflow.to_string(), "Arithmetic overflow");
        assert_eq!(
            CalculatorError::InvalidOperation("test".to_string()).to_string(),
            "Invalid operation: test"
        );
    }

    #[test]
    fn calculator_result_from_result() {
        let success = CalculatorResult::from_result(Ok(42));
        assert!(success.is_success());
        assert_eq!(success.value(), Some(42));

        let error = CalculatorResult::from_result(Err(CalculatorError::DivisionByZero));
        assert!(error.is_error());
        assert!(matches!(
            error.error(),
            Some(CalculatorError::DivisionByZero)
        ));
    }
}
