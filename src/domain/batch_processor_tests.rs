use crate::domain::batch_processor::{
    BatchError, BatchProcessor, DiscoverySession, RequirementChange,
};

#[test]
fn test_happy_path_processes_valid_batch_atomically() {
    let session = DiscoverySession::new("test-session-1");
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add {
            id: "req-1".to_string(),
            description: "First requirement".to_string(),
        },
        RequirementChange::Add {
            id: "req-2".to_string(),
            description: "Second requirement".to_string(),
        },
    ];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.successful_count, 2);
    assert_eq!(report.failed_count, 0);
    assert_eq!(session.version(), 1); // Only increments by 1 per batch
}

#[test]
fn test_batch_atomicity_rollback_on_partial_failure() {
    let session = DiscoverySession::new("test-session-2");
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add {
            id: "req-1".to_string(),
            description: "Valid requirement".to_string(),
        },
        RequirementChange::Add { id: "req-2".to_string(), description: "".to_string() },
        RequirementChange::Add {
            id: "req-3".to_string(),
            description: "Another valid requirement".to_string(),
        },
    ];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_err());
    match result.unwrap_err() {
        BatchError::AtomicRollback { successful, failed } => {
            assert_eq!(successful, 2); // Two valid requirements processed
            assert_eq!(failed, 1);
            assert_eq!(session.requirement_count(), 0);
        }
        _ => panic!("Expected AtomicRollback error"),
    }
    assert_eq!(session.version(), 0); // No version increment on rollback
}

#[test]
fn test_version_increments_once_per_batch_not_per_item() {
    let session = DiscoverySession::new("test-session-3");
    let initial_version = session.version();
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add { id: "req-1".to_string(), description: "First".to_string() },
        RequirementChange::Add { id: "req-2".to_string(), description: "Second".to_string() },
        RequirementChange::Update {
            id: "req-1".to_string(),
            description: "Updated first".to_string(),
        },
    ];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    assert_eq!(session.version(), initial_version + 1);
}

#[test]
fn test_error_messages_are_clear_and_specific() {
    let session = DiscoverySession::new("test-session-4");
    let processor = BatchProcessor::new();

    let changes = vec![RequirementChange::Update {
        id: "nonexistent-req".to_string(),
        description: "Updated".to_string(),
    }];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_message = error.to_string();

    assert!(error_message.contains("nonexistent-req"));
    assert!(error_message.contains("not found") || error_message.contains("does not exist"));
}

#[test]
fn test_high_volume_batch_processing() {
    let session = DiscoverySession::new("test-session-5");
    let processor = BatchProcessor::new();

    let changes: Vec<RequirementChange> = (1..=100)
        .map(|i| RequirementChange::Add {
            id: format!("req-{}", i),
            description: format!("Requirement number {}", i),
        })
        .collect();

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.successful_count, 100);
    assert_eq!(session.version(), 1);
    assert_eq!(session.requirement_count(), 100);
}

#[test]
fn test_empty_batch_is_handled_gracefully() {
    let session = DiscoverySession::new("test-session-6");
    let initial_version = session.version();
    let processor = BatchProcessor::new();

    let changes = vec![];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    assert_eq!(session.version(), initial_version);
}

#[test]
fn test_no_orphaned_items_on_failure() {
    let session = DiscoverySession::new("test-session-7");
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add { id: "req-1".to_string(), description: "Valid".to_string() },
        RequirementChange::Add { id: "req-2".to_string(), description: "Valid".to_string() },
        RequirementChange::Add { id: "req-3".to_string(), description: "".to_string() },
        RequirementChange::Add {
            id: "req-4".to_string(),
            description: "Would be orphaned".to_string(),
        },
    ];

    let initial_count = session.requirement_count();
    let result = processor.process_batch(&session, changes);

    assert!(result.is_err());
    assert_eq!(session.requirement_count(), initial_count);
}

#[test]
fn test_batch_command_variants() {
    let session = DiscoverySession::new("test-session-8");
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add { id: "req-1".to_string(), description: "Add this".to_string() },
        RequirementChange::Update { id: "req-1".to_string(), description: "Update it".to_string() },
        RequirementChange::Remove { id: "req-1".to_string() },
    ];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    assert_eq!(session.requirement_count(), 0);
}

#[test]
fn test_duplicate_add_rejected_with_clear_error() {
    let session = DiscoverySession::new("test-session-9");
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add { id: "req-1".to_string(), description: "First".to_string() },
        RequirementChange::Add { id: "req-1".to_string(), description: "Duplicate".to_string() },
    ];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_message = error.to_string();

    assert!(error_message.contains("req-1"));
    assert!(error_message.contains("duplicate") || error_message.contains("already exists"));
    assert_eq!(session.requirement_count(), 0);
}

#[test]
fn test_remove_nonexistent_rejected_with_clear_error() {
    let session = DiscoverySession::new("test-session-10");
    let processor = BatchProcessor::new();

    let changes = vec![RequirementChange::Remove { id: "nonexistent".to_string() }];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_message = error.to_string();

    assert!(error_message.contains("nonexistent"));
    assert!(error_message.contains("not found") || error_message.contains("does not exist"));
}

#[test]
fn test_session_must_be_writable() {
    let session = DiscoverySession::new_readonly("test-session-11");
    let processor = BatchProcessor::new();

    let changes =
        vec![RequirementChange::Add { id: "req-1".to_string(), description: "Test".to_string() }];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_err());
    match result.unwrap_err() {
        BatchError::SessionNotWritable { session_id } => {
            assert_eq!(session_id, "test-session-11");
        }
        _ => panic!("Expected SessionNotWritable error"),
    }
}

#[test]
fn test_batch_processor_returns_batch_report() {
    let session = DiscoverySession::new("test-session-12");
    let processor = BatchProcessor::new();

    let changes = vec![
        RequirementChange::Add { id: "req-1".to_string(), description: "First".to_string() },
        RequirementChange::Add { id: "req-2".to_string(), description: "Second".to_string() },
    ];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.batch_id.starts_with("batch-"));
    assert_eq!(report.total_count, 2);
    assert!(report.processing_time_ms > 0);
}

#[test]
fn test_large_description_handled_correctly() {
    let session = DiscoverySession::new("test-session-13");
    let processor = BatchProcessor::new();

    let large_description = "x".repeat(10000);
    let changes = vec![RequirementChange::Add {
        id: "req-1".to_string(),
        description: large_description.clone(),
    }];

    let result = processor.process_batch(&session, changes);

    assert!(result.is_ok());
    assert_eq!(session.get_requirement_description("req-1"), Some(large_description));
}
