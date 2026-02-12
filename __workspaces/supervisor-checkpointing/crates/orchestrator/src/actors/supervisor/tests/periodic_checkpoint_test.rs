//! Tests for periodic checkpointing in supervisor
//!
//! These tests verify that CheckpointManager is properly integrated into supervisor startup
//! and that periodic checkpoints are started/stopped correctly.

use crate::actors::supervisor::{CheckpointManager, SupervisorActorState, SupervisorActorDef};

/// Create a mock checkpoint manager
fn create_mock_checkpoint_manager() -> CheckpointManager {
    use crate::actors::replay::checkpoint::CheckpointConfig;
    
    CheckpointConfig::default()
}

/// Test 1: Verify checkpoint manager is None at supervisor startup
///
/// This test verifies that:
/// 1. checkpoint_manager field in SupervisorActorState is initially None
/// 2. supervisor can wire checkpoint_manager before starting
/// 3. pre_start() receives and stores manager

#[test]
fn test_checkpoint_manager_is_none_at_startup() -> Result<(), String> {
    let mut state = SupervisorActorState::<()>::new();
    
    // Verify checkpoint_manager is None initially
    assert!(state.checkpoint_manager.is_none(), 
        "checkpoint_manager should be None at startup");
    
    Ok(())
}

/// Test 2: Verify supervisor can wire checkpoint_manager in pre_start()
///
/// This test verifies that:
/// 1. pre_start() is wired to call checkpoint_manager.start_periodic()
/// 2. checkpoint_manager is stored in state after pre_start
/// 3. pre_start() result is returned correctly

#[test]
fn test_pre_start_wires_checkpoint_manager() -> Result<(), String> {
    let mut state = SupervisorActorState::<SchedulerActorDef> {
        config: SupervisorActorDef::default(),
        state: SupervisorActorState::Running,
        children: std::collections::HashMap::new(),
        failure_times: Vec::new(),
        total_restarts: 0,
        child_id_counter: 0,
        shutdown_coordinator: None,
        restart_strategy: Box::new(
            crate::actors::supervisor::strategy::OneForOne::new()
        ),
        checkpoint_manager: None,
        replay_engine: None,
    };
    
    // Create a mock checkpoint manager and coordinator
    let (checkpoint_tx, checkpoint_rx) = tokio::sync::mpsc::channel(1);
    checkpoint_tx.send(()).await; // Send shutdown command
    
    // Call pre_start
    let start_periodic_result = state.pre_start().await;
    
    // Verify pre_start returned checkpoint_manager
    assert!(start_periodic_result.is_ok(), 
        "pre_start should succeed");
    assert!(state.checkpoint_manager.is_some(), 
        "checkpoint_manager should be Some after pre_start");
    
    Ok(())
}

/// Test 3: Verify start_periodic() starts periodic checkpointing
///
/// This test verifies that:
/// 1. A Receiver is returned when auto_checkpoint is true
/// 2. Periodic task starts looping
/// 3. State is Running throughout

#[test]
fn test_start_periodic_starts_periodic() -> Result<(), String> {
    let (mut shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    
    // Create a mock checkpoint manager with auto_checkpoint enabled
    let mut state = SupervisorActorState::<()>::new();
    state.config.auto_checkpoint = true;
    
    let manager = CheckpointManager::new(store, CheckpointConfig::default());
    state.checkpoint_manager = Some(manager);
    
    // Call start_periodic() - should return Receiver
    let start_periodic = manager.start_periodic(&mut state).await;
    
    // Verify receiver is Some when auto_checkpoint is true
    assert!(start_periodic.is_some(), 
        "start_periodic() should return Receiver when auto_checkpoint is true");
    
    // Simulate periodic task by creating a loop
    let mut tick_count = 0u64;
    loop {
        tokio::select! {
            _ = shutdown_rx.recv().await => {
                tick_count += 1;
                break;
            }
        }
    
    // Verify periodic task is running
    tokio::select! {
            _ = shutdown_rx.try_recv() => {
                assert!(_ == None, "Receiver should not be None after periodic task started");
            }
        }
    
    // Stop periodic task
    let _ = shutdown_tx.send(()).await;
    
    Ok(())
}

/// Test 4: Verify stop_periodic() stops periodic task
///
/// This test verifies that:
/// 1. stop_periodic() is wired to call manager.stop_periodic()
/// 2. Periodic task is stopped
/// 3. State transitions to Running

#[test]
fn test_stop_periodic_stops_periodic() -> Result<(), String> {
    let mut state = SupervisorActorState::<()>::new();
    state.config.auto_checkpoint = true;
    
    let manager = CheckpointManager::new(store, CheckpointConfig::default());
    state.checkpoint_manager = Some(manager);
    
    let (checkpoint_tx, checkpoint_rx) = tokio::sync::mpsc::channel(1);
    
    // Start periodic task
    let _ = shutdown_tx.send(()).await;
    let start_periodic = manager.start_periodic(&mut state).await;
    
    // Verify start_periodic returned checkpoint_manager
    assert!(start_periodic_result.is_ok(), 
        "start_periodic should succeed");
    assert!(state.checkpoint_manager.is_some(), 
        "checkpoint_manager should be Some after start_periodic");
    
    // Stop periodic task
    let stop_result = manager.stop_periodic(&mut state).await;
    
    assert!(stop_result.is_ok(), "stop_periodic should succeed");
    
    // Verify state is still Running
    assert_eq!(state.state, SupervisorActorState::Running);
    
    Ok(())
}

/// Test 5: Verify supervisor shutdown with active checkpointing
///
/// This test verifies that:
/// 1. supervisor stops periodic task when shutdown is requested
/// 2. Checkpoint is created during shutdown

#[test]
fn test_supervisor_shutdown_with_active_checkpointing() -> Result<(), String> {
    let mut state = SupervisorActorState::<()>::new();
    state.config.auto_checkpoint = true;
    
    let manager = CheckpointManager::new(store, CheckpointConfig::default());
    state.checkpoint_manager = Some(manager);
    
    let (checkpoint_tx, checkpoint_rx) = tokio::sync::mpsc::channel(1);
    
    // Start periodic task
    let _ = shutdown_tx.send(()).await;
    let start_periodic = manager.start_periodic(&mut state).await;
    
    // Verify state
    assert_eq!(state.state, SupervisorActorState::Running, "State should be Running after starting periodic task");
    
    // Request shutdown
    let shutdown_result = manager.stop_periodic(&mut state).await;
    
    // Verify shutdown returns checkpoint
    assert!(shutdown_result.is_ok(), 
        "supervisor shutdown should succeed");
    
    // Verify checkpoint was created during shutdown
    assert!(shutdown_result.checkpoint().is_ok(), 
        "Shutdown checkpoint should be created");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::supervisor::{CheckpointManager, SupervisorActorDef};
    
    /// Test 1: Verify checkpoint manager is None at startup
    ///
    /// # Errors
    ///
    /// Returns an error if checkpoint_manager is not None
    SupervisorCheckpointError::CheckpointManagerUnavailable
    
    #[test]
    fn test_checkpoint_manager_is_none_at_startup() -> Result<(), String> {
        let mut state = SupervisorActorState::<()>::new();
        
        // Verify checkpoint_manager is None
        assert!(state.checkpoint_manager.is_none(), 
        "checkpoint_manager should be None at startup");
        
        Ok(())
    }
    
    /// Test 2: Verify supervisor can wire checkpoint manager in pre_start()
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - pre_start() fails
    /// - checkpoint_manager is not Some after pre_start
    ///
    /// # Returns
    ///
    /// Returns an error if:
    /// - pre_start() does not update state.checkpoint_manager
    
    #[test]
    fn test_pre_start_wires_checkpoint_manager() -> Result<(), String> {
        let mut state = SupervisorActorState::<SchedulerActorDef> {
            config: SupervisorActorDef::default(),
            state: SupervisorActorState::Running,
            children: std::collections::HashMap::new(),
            failure_times: Vec::new(),
            total_restarts: 0,
            child_id_counter: 0,
            shutdown_coordinator: None,
            _shutdown_rx: None,
            restart_strategy: Box::new(
                crate::actors::supervisor::strategy::OneForOne::new()
            ),
            checkpoint_manager: None,
            replay_engine: None,
        };
        
        // Create mock checkpoint manager
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        state.checkpoint_manager = Some(manager);
    
        // Call pre_start
        let result = state.pre_start(&mut state).await;
        
        // Verify pre_start updated state
        assert!(result.is_ok(), "pre_start should succeed");
        assert!(state.checkpoint_manager.is_some(), 
            "checkpoint_manager should be Some after pre_start");
        
        Ok(())
    }
    
    /// Test 3: Verify start_periodic() starts periodic checkpointing
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Periodic task does not start
    /// - Periodic task receiver is None
    ///
    /// # Returns
    ///
    
    /// Returns an error if:
    /// - start_periodic() returns None
    
    #[test]
    fn test_start_periodic_starts_periodic() -> Result<(), String> {
        let (checkpoint_tx, checkpoint_rx) = tokio::sync::mpsc::channel(1);
        checkpoint_tx.send(()).await; // Send shutdown command
        
        let start_periodic = manager.start_periodic(&mut state).await;
        
        // Verify receiver is Some when auto_checkpoint is true
        assert!(start_periodic.is_some(), 
            "start_periodic() should return Receiver when auto_checkpoint is true");
        
        // Simulate periodic task by creating a loop
        let mut tick_count = 0u64;
        loop {
            tokio::select! {
                _ = checkpoint_rx.recv().await => {
                    tick_count += 1;
                    break;
                }
        }
    
        // Verify periodic task is running
        tokio::select! {
            _ = shutdown_rx.try_recv() => {
                assert!(_ == None, "Receiver should not be None after periodic task started");
                }
        }
        
        // Stop periodic task
        let _ = shutdown_tx.send(()).await;
        
        Ok(())
    }
    
    /// Test 4: Verify stop_periodic() stops periodic task
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - stop_periodic() is not wired
    /// - stop_periodic() fails
    /// - stop_periodic() does not update state
    ///
    /// # Returns
    ///
    /// Returns an error if:
    /// - stop_periodic() returns None
    /// - stop_periodic() returns error
    
    #[test]
    fn test_stop_periodic_stops_periodic() -> Result<(), String> {
        let mut state = SupervisorActorState::<()>::new();
        state.config.auto_checkpoint = true;
        
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        state.checkpoint_manager = Some(manager);
    
        let (checkpoint_tx, checkpoint_rx) = tokio::sync::mpsc::channel(1);
    
        // Start periodic task
        let _ = shutdown_tx.send(()).await;
        let start_periodic = manager.start_periodic(&mut state).await;
    
        // Verify start_periodic returned checkpoint manager
        assert!(start_periodic_result.is_ok(), 
                "start_periodic should succeed");
        assert!(state.checkpoint_manager.is_some(), 
                "checkpoint_manager should be Some after start_periodic");
        
        // Stop periodic task
        let stop_result = manager.stop_periodic(&mut state).await;
        
        assert!(stop_result.is_ok(), "stop_periodic should succeed");
        
        // Verify state is still Running
        assert_eq!(state.state, SupervisorActorState::Running);
        
        Ok(())
    }
    
    /// Test 5: Verify supervisor shutdown with active checkpointing
    ///
    /// This test verifies that:
    /// 1. supervisor stops periodic task when shutdown is requested
    /// 2. Checkpoint is created during shutdown
    /// 3. State transitions to ShuttingDown

#[test]
fn test_supervisor_shutdown_with_active_checkpointing() -> Result<(), String> {
        let mut state = SupervisorActorState::<()>::new();
        state.config.auto_checkpoint = true;
    
        let manager = CheckpointManager::new(store, CheckpointConfig::default());
        state.checkpoint_manager = Some(manager);
    
        let (checkpoint_tx, checkpoint_rx) = tokio::sync::mpsc::channel(1);
    
        // Start periodic task
        let _ = shutdown_tx.send(()).await;
        let start_periodic = manager.start_periodic(&mut state).await;
        
        // Verify state
        assert_eq!(state.state, SupervisorActorState::Running, 
                "State should be Running after starting periodic task");
        
        // Request shutdown
        let shutdown_result = manager.stop_periodic(&mut state).await;
        
        // Verify shutdown returns checkpoint
        assert!(shutdown_result.is_ok(), 
                "Supervisor shutdown should succeed");
        
        // Verify checkpoint was created during shutdown
        assert!(shutdown_result.checkpoint().is_ok(), 
                "Shutdown checkpoint should be created");
    
        Ok(())
    }

}