//! Integration tests demonstrating all three message passing patterns
//!
//! This test file shows:
//! - **call()**: Request-response pattern with CalculatorActor
//! - **cast()**: Fire-and-forget pattern with CalculatorActor
//! - **send()**: Async message passing with LoggerActor
//!
//! # Test Structure
//!
//! Each test demonstrates a specific pattern and verifies:
//! 1. The message is sent correctly
//! 2. The actor processes the message
//! 3. The response (if any) is received correctly

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::time::Duration;
use tokio::time::timeout;

use orchestrator::actors::examples::messaging::{
    CalculatorActor, CalculatorError, CalculatorMessage, CalculatorStats, LogEntry, LogLevel,
    LoggerActor, LoggerMessage,
};
use ractor::{Actor, ActorRef};

//==============================================================================
// Test Helpers
//==============================================================================

/// Helper to spawn a calculator actor
async fn spawn_calculator()
-> Result<(ActorRef<CalculatorMessage>, ractor::ActorHandle), Box<dyn std::error::Error>> {
    let (actor, handle) = Actor::spawn(None, CalculatorActor::new(), ()).await?;
    Ok((actor, handle))
}

/// Helper to spawn a logger actor
async fn spawn_logger()
-> Result<(ActorRef<LoggerMessage>, ractor::ActorHandle), Box<dyn std::error::Error>> {
    let (actor, handle) = Actor::spawn(None, LoggerActor::new(), ()).await?;
    Ok((actor, handle))
}

//==============================================================================
// CALL PATTERN TESTS
//==============================================================================

#[tokio::test]
async fn call_pattern_request_response_arithmetic() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A calculator actor
    let (calculator, calc_handle) = spawn_calculator().await?;

    // WHEN: Sending a request using call pattern (expecting response)
    let result = timeout(Duration::from_millis(100), async {
        ractor::call!(
            calculator,
            CalculatorMessage::Add {
                a: 15,
                b: 27,
                reply: ractor::RpcReplyPort::new()
            },
            Some(Duration::from_millis(50))
        )
    })
    .await??;

    // THEN: Should receive response with result
    assert_eq!(result, Ok(42));

    // Cleanup
    calculator.stop(None);
    calc_handle.await?;

    Ok(())
}

#[tokio::test]
async fn call_pattern_error_handling_division_by_zero() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A calculator actor
    let (calculator, calc_handle) = spawn_calculator().await?;

    // WHEN: Attempting to divide by zero using call pattern
    let result = timeout(Duration::from_millis(100), async {
        ractor::call!(
            calculator,
            CalculatorMessage::Divide {
                a: 100,
                b: 0,
                reply: ractor::RpcReplyPort::new()
            },
            Some(Duration::from_millis(50))
        )
    })
    .await??;

    // THEN: Should receive error response
    assert_eq!(result, Err(CalculatorError::DivisionByZero));

    // Cleanup
    calculator.stop(None);
    calc_handle.await?;

    Ok(())
}

#[tokio::test]
async fn call_pattern_get_statistics() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A calculator actor
    let (calculator, calc_handle) = spawn_calculator().await?;

    // Perform some operations
    ractor::call!(
        calculator,
        CalculatorMessage::Add {
            a: 1,
            b: 2,
            reply: ractor::RpcReplyPort::new()
        },
        Some(Duration::from_millis(50))
    )?;

    ractor::call!(
        calculator,
        CalculatorMessage::Multiply {
            a: 3,
            b: 4,
            reply: ractor::RpcReplyPort::new()
        },
        Some(Duration::from_millis(50))
    )?;

    // WHEN: Querying statistics using call pattern
    let stats: CalculatorStats = timeout(Duration::from_millis(100), async {
        ractor::call!(
            calculator,
            CalculatorMessage::GetStats {
                reply: ractor::RpcReplyPort::new()
            },
            Some(Duration::from_millis(50))
        )
    })
    .await??;

    // THEN: Should receive correct statistics
    assert_eq!(stats.additions, 1);
    assert_eq!(stats.multiplications, 1);
    assert_eq!(stats.operation_count, 2);

    // Cleanup
    calculator.stop(None);
    calc_handle.await?;

    Ok(())
}

//==============================================================================
// CAST PATTERN TESTS
//==============================================================================

#[tokio::test]
async fn cast_pattern_fire_and_forget_state_change() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A calculator actor
    let (calculator, calc_handle) = spawn_calculator().await?;

    // WHEN: Setting value using cast pattern (fire-and-forget, no response expected)
    ractor::cast!(calculator, CalculatorMessage::SetValue { value: 123 })?;

    // Give actor time to process
    tokio::time::sleep(Duration::from_millis(10)).await;

    // THEN: Value should be updated (verify with call pattern)
    let value = timeout(Duration::from_millis(100), async {
        ractor::call!(
            calculator,
            CalculatorMessage::GetValue {
                reply: ractor::RpcReplyPort::new()
            },
            Some(Duration::from_millis(50))
        )
    })
    .await??;

    assert_eq!(value, 123);

    // Cleanup
    calculator.stop(None);
    calc_handle.await?;

    Ok(())
}

#[tokio::test]
async fn cast_pattern_multiple_commands() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A calculator actor
    let (calculator, calc_handle) = spawn_calculator().await?;

    // WHEN: Sending multiple cast commands (fire-and-forget)
    ractor::cast!(calculator, CalculatorMessage::SetValue { value: 10 })?;
    ractor::cast!(calculator, CalculatorMessage::Increment { amount: 5 })?;
    ractor::cast!(calculator, CalculatorMessage::Increment { amount: 3 })?;
    ractor::cast!(calculator, CalculatorMessage::Decrement { amount: 2 })?;

    // Give actor time to process all commands
    tokio::time::sleep(Duration::from_millis(50)).await;

    // THEN: Final value should be 10 + 5 + 3 - 2 = 16
    let value = timeout(Duration::from_millis(100), async {
        ractor::call!(
            calculator,
            CalculatorMessage::GetValue {
                reply: ractor::RpcReplyPort::new()
            },
            Some(Duration::from_millis(50))
        )
    })
    .await??;

    assert_eq!(value, 16);

    // Cleanup
    calculator.stop(None);
    calc_handle.await?;

    Ok(())
}

//==============================================================================
// SEND PATTERN TESTS
//==============================================================================

#[tokio::test]
async fn send_pattern_async_message_passing() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A logger actor
    let (logger, log_handle) = spawn_logger().await?;

    // WHEN: Sending messages using send pattern (async, no waiting)
    logger.send_message(LoggerMessage::Log {
        msg: "First message".to_string(),
    })?;

    logger.send_message(LoggerMessage::LogWithLevel {
        level: LogLevel::Warning,
        msg: "Warning message".to_string(),
    })?;

    logger.send_message(LoggerMessage::LogWithLevel {
        level: LogLevel::Error,
        msg: "Error message".to_string(),
    })?;

    // Give actor time to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // THEN: All messages should be logged
    let (tx, rx) = tokio::sync::oneshot::channel();
    logger.send_message(LoggerMessage::GetAll { reply: tx })?;

    let entries = timeout(Duration::from_millis(100), rx).await??;

    assert_eq!(entries.len(), 3);

    // Verify log levels
    let levels: Vec<LogLevel> = entries.iter().map(|e| e.level()).collect();
    assert_eq!(
        levels,
        vec![LogLevel::Info, LogLevel::Warning, LogLevel::Error]
    );

    // Cleanup
    logger.stop(None);
    log_handle.await?;

    Ok(())
}

#[tokio::test]
async fn send_pattern_concurrent_logging() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A logger actor
    let (logger, log_handle) = spawn_logger().await?;

    // WHEN: Sending many messages concurrently using send pattern
    let mut tasks = Vec::new();
    for i in 0..50 {
        let logger_clone = logger.clone();
        let task = tokio::spawn(async move {
            logger_clone.send_message(LoggerMessage::Log {
                msg: format!("Concurrent message {}", i),
            })
        });
        tasks.push(task);
    }

    // Wait for all sends to complete
    for task in tasks {
        task.await??;
    }

    // Give actor time to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // THEN: All messages should be logged
    let (tx, rx) = tokio::sync::oneshot::channel();
    logger.send_message(LoggerMessage::GetAll { reply: tx })?;

    let entries = timeout(Duration::from_millis(100), rx).await??;

    assert_eq!(entries.len(), 50);

    // Cleanup
    logger.stop(None);
    log_handle.await?;

    Ok(())
}

//==============================================================================
// INTEGRATION TEST: COMBINING ALL PATTERNS
//==============================================================================

#[tokio::test]
async fn integration_combined_patterns() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Calculator and logger actors
    let (calculator, calc_handle) = spawn_calculator().await?;
    let (logger, log_handle) = spawn_logger().await?;

    // WHEN: Using all three patterns together

    // 1. Use SEND pattern to log operation start
    logger.send_message(LoggerMessage::LogWithLevel {
        level: LogLevel::Info,
        msg: "Starting calculation".to_string(),
    })?;

    // 2. Use CALL pattern to perform arithmetic and get result
    let result = ractor::call!(
        calculator,
        CalculatorMessage::Multiply {
            a: 6,
            b: 7,
            reply: ractor::RpcReplyPort::new()
        },
        Some(Duration::from_millis(50))
    )?;

    // 3. Use SEND pattern to log result
    logger.send_message(LoggerMessage::LogWithLevel {
        level: LogLevel::Info,
        msg: format!("Calculation result: {:?}", result),
    })?;

    // 4. Use CAST pattern to reset calculator
    ractor::cast!(calculator, CalculatorMessage::Reset)?;

    // 5. Use CALL pattern to verify reset
    let value = ractor::call!(
        calculator,
        CalculatorMessage::GetValue {
            reply: ractor::RpcReplyPort::new()
        },
        Some(Duration::from_millis(50))
    )?;

    // 6. Use SEND pattern to log completion
    logger.send_message(LoggerMessage::LogWithLevel {
        level: LogLevel::Info,
        msg: format!("Calculator reset to {}", value),
    })?;

    // Give actors time to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // THEN: Verify all operations completed successfully
    assert_eq!(result, Ok(42));
    assert_eq!(value, 0);

    // Verify log entries
    let (tx, rx) = tokio::sync::oneshot::channel();
    logger.send_message(LoggerMessage::GetAll { reply: tx })?;

    let entries = timeout(Duration::from_millis(100), rx).await??;

    assert_eq!(entries.len(), 3);
    assert!(entries[0].message().contains("Starting calculation"));
    assert!(entries[1].message().contains("Calculation result"));
    assert!(entries[2].message().contains("Calculator reset"));

    // Cleanup
    calculator.stop(None);
    logger.stop(None);
    calc_handle.await?;
    log_handle.await?;

    Ok(())
}

//==============================================================================
// PATTERN COMPARISON TESTS
//==============================================================================

#[tokio::test]
async fn comparison_call_vs_cast_vs_send() -> Result<(), Box<dyn std::error::Error>> {
    // This test demonstrates when to use each pattern

    let (calculator, calc_handle) = spawn_calculator().await?;
    let (logger, log_handle) = spawn_logger().await?;

    // ═══════════════════════════════════════════════════════════════════════
    // CALL PATTERN: Use when you need a response
    // ═══════════════════════════════════════════════════════════════════════
    let _result = ractor::call!(
        calculator,
        CalculatorMessage::Add {
            a: 10,
            b: 20,
            reply: ractor::RpcReplyPort::new()
        },
        Some(Duration::from_millis(50))
    )?;
    // ✓ We got the result back synchronously

    // ═══════════════════════════════════════════════════════════════════════
    // CAST PATTERN: Use when you don't need a response (state mutation)
    // ═══════════════════════════════════════════════════════════════════════
    ractor::cast!(calculator, CalculatorMessage::SetValue { value: 100 })?;
    // ✓ Command sent, we don't wait for confirmation

    // ═══════════════════════════════════════════════════════════════════════
    // SEND PATTERN: Use for async delivery without blocking
    // ═══════════════════════════════════════════════════════════════════════
    logger.send_message(LoggerMessage::Log {
        msg: "Async log message".to_string(),
    })?;
    // ✓ Message sent asynchronously, non-blocking

    // Give actors time to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify state changed (even though we used cast)
    let value = ractor::call!(
        calculator,
        CalculatorMessage::GetValue {
            reply: ractor::RpcReplyPort::new()
        },
        Some(Duration::from_millis(50))
    )?;
    assert_eq!(value, 100);

    // Cleanup
    calculator.stop(None);
    logger.stop(None);
    calc_handle.await?;
    log_handle.await?;

    Ok(())
}
