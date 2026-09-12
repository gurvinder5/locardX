use locardx_audit::AuditService;
use locardx_common::{TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_operation_manager::{
    CancellationToken, OperationManager, OperationProgress, OperationState, OperationType,
};
use locardx_verification::IntegrityService;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

struct TempTestFile {
    path: PathBuf,
}

impl TempTestFile {
    fn new(content: &[u8]) -> Self {
        let filename = format!("locardx_op_test_{}.tmp", Uuid::new_v4());
        let path = std::env::temp_dir().join(filename);
        let mut file = File::create(&path).expect("Failed to create temp file");
        file.write_all(content).expect("Failed to write temp file");
        file.flush().expect("Failed to flush temp file");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn setup_test_manager() -> (OperationManager, Arc<Database>, Arc<AuditService>) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to create in-memory database"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let integrity = Arc::new(IntegrityService::new(Arc::clone(&db), Arc::clone(&audit)));
    let manager = OperationManager::new(Arc::clone(&db), Arc::clone(&audit), integrity);
    (manager, db, audit)
}

#[test]
fn test_01_operation_type_executability_boundary() {
    // Read-only integrity operations, sanitization planning, and file/folder erasure are executable
    assert!(OperationType::IntegrityHash.is_executable());
    assert!(OperationType::IntegrityVerify.is_executable());
    assert!(OperationType::SanitizationPlanEvaluation.is_executable());
    assert!(OperationType::FileErasure.is_executable());
    assert!(OperationType::FolderErasure.is_executable());

    // Physical drive erasure and carving operations remain permanently disabled
    assert!(!OperationType::DriveErasure.is_executable());
    assert!(!OperationType::Recovery.is_executable());
    assert!(!OperationType::Unknown.is_executable());
}

#[test]
fn test_02_operation_state_machine_valid_transitions() {
    let valid_pairs = vec![
        (OperationState::Created, OperationState::Queued),
        (OperationState::Created, OperationState::Cancelled),
        (OperationState::Queued, OperationState::Running),
        (OperationState::Queued, OperationState::Cancelled),
        (OperationState::Running, OperationState::Completed),
        (OperationState::Running, OperationState::Failed),
        (OperationState::Running, OperationState::Cancelling),
        (OperationState::Cancelling, OperationState::Cancelled),
        (OperationState::Cancelling, OperationState::Failed),
    ];

    for (from, to) in valid_pairs {
        assert!(
            from.can_transition_to(to),
            "State {} should be permitted to transition to {}",
            from,
            to
        );
    }
}

#[test]
fn test_03_operation_state_machine_invalid_transitions() {
    let invalid_pairs = vec![
        // Terminal states cannot transition to anything
        (OperationState::Completed, OperationState::Running),
        (OperationState::Completed, OperationState::Created),
        (OperationState::Failed, OperationState::Running),
        (OperationState::Failed, OperationState::Queued),
        (OperationState::Cancelled, OperationState::Completed),
        (OperationState::Cancelled, OperationState::Running),
        // Illegal skipping
        (OperationState::Created, OperationState::Completed),
        (OperationState::Created, OperationState::Running),
        (OperationState::Queued, OperationState::Completed),
    ];

    for (from, to) in invalid_pairs {
        assert!(
            !from.can_transition_to(to),
            "State {} must NOT be permitted to transition to {}",
            from,
            to
        );
    }
}

#[test]
fn test_04_progress_model_validation() {
    let mut p = OperationProgress::new("Hashing", "Reading blocks");
    assert_eq!(p.percentage, None);

    // Valid percentage
    p = p
        .with_percentage(45.5)
        .expect("Valid percentage should succeed");
    assert_eq!(p.percentage, Some(45.5));

    // Bounds check
    assert!(p.clone().with_percentage(-0.1).is_err());
    assert!(p.clone().with_percentage(100.1).is_err());

    // Byte progress calculation
    let p_bytes = OperationProgress::new("Copying", "Writing stream").with_bytes(50, Some(100));
    assert_eq!(p_bytes.percentage, Some(50.0));
    assert_eq!(p_bytes.bytes_processed, Some(50));
    assert_eq!(p_bytes.total_bytes, Some(100));
}

#[test]
fn test_05_cancellation_token_mechanics() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
    assert!(token.check_cancelled().is_ok());

    token.cancel();
    assert!(token.is_cancelled());
    assert!(token.check_cancelled().is_err());
}

#[test]
fn test_06_create_and_query_operation_persistence() {
    let (manager, _, audit) = setup_test_manager();
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: "C:\\Data\\file.img".to_string(),
        display_name: "file.img".to_string(),
        size_bytes: Some(1024),
    };

    let op = manager
        .create_operation(
            OperationType::IntegrityHash,
            target.clone(),
            Some("admin".to_string()),
        )
        .expect("create_operation failed");

    assert_eq!(op.current_state, OperationState::Created);
    assert_eq!(op.actor_id.as_deref(), Some("admin"));
    assert_eq!(op.target.identifier, "C:\\Data\\file.img");

    // Query back from SQLite
    let fetched = manager
        .get_operation(&op.operation_id)
        .expect("get_operation failed");
    assert_eq!(fetched.operation_id, op.operation_id);
    assert_eq!(fetched.current_state, OperationState::Created);

    // Check audit trail
    let events = audit.list_events(10).expect("list_events failed");
    assert!(!events.is_empty());
    assert_eq!(events[0].event_type, "OPERATION_CREATED");
}

#[tokio::test]
async fn test_07_execute_integrity_hash_operation() {
    let (manager, _, audit) = setup_test_manager();
    let file = TempTestFile::new(b"forensic evidence contents");

    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: file.path().to_string_lossy().to_string(),
        display_name: "test_evidence.raw".to_string(),
        size_bytes: Some(26),
    };

    let op = manager
        .submit_and_run(
            OperationType::IntegrityHash,
            target,
            Some("investigator1".to_string()),
            None,
        )
        .await
        .expect("submit_and_run failed");

    // Yield to allow background tokio task to run
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let finished_op = manager
        .get_operation(&op.operation_id)
        .expect("get_operation failed");
    assert_eq!(finished_op.current_state, OperationState::Completed);
    assert!(finished_op.result_summary.is_some());
    assert!(finished_op
        .result_summary
        .unwrap()
        .contains("Calculated SHA-256"));

    // Verify audit chain
    let events = audit.list_events(10).expect("list_events failed");
    let event_types: Vec<String> = events.into_iter().map(|e| e.event_type).collect();
    assert!(event_types.contains(&"OPERATION_CREATED".to_string()));
    assert!(event_types.contains(&"OPERATION_STARTED".to_string()));
    assert!(event_types.contains(&"OPERATION_COMPLETED".to_string()));
}

#[tokio::test]
async fn test_08_destructive_operation_types_are_rejected() {
    let (manager, _, _) = setup_test_manager();
    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "Target Disk".to_string(),
        size_bytes: Some(1000000),
    };

    let res = manager
        .submit_and_run(
            OperationType::DriveErasure,
            target,
            Some("admin".to_string()),
            None,
        )
        .await;

    assert!(res.is_err());
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("disabled") || err_msg.contains("not implemented"));
}

#[tokio::test]
async fn test_09_cancellation_request() {
    let (manager, _, audit) = setup_test_manager();
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: "C:\\Large\\image.raw".to_string(),
        display_name: "image.raw".to_string(),
        size_bytes: Some(10000000),
    };

    let op = manager
        .create_operation(
            OperationType::IntegrityHash,
            target,
            Some("admin".to_string()),
        )
        .expect("create_operation failed");

    // Request cancellation of created operation
    manager
        .request_cancellation(&op.operation_id, Some("admin"))
        .await
        .expect("request_cancellation failed");

    let cancelled_op = manager
        .get_operation(&op.operation_id)
        .expect("get_operation failed");
    assert_eq!(cancelled_op.current_state, OperationState::Cancelled);

    let events = audit.list_events(10).expect("list_events failed");
    assert!(events
        .iter()
        .any(|e| e.event_type == "OPERATION_CANCELLATION_REQUESTED"));
}

#[test]
fn test_10_list_operations_with_filtering() {
    let (manager, _, _) = setup_test_manager();

    for i in 1..=3 {
        let target = TargetIdentity {
            target_type: TargetType::File,
            identifier: format!("C:\\File_{}.bin", i),
            display_name: format!("File_{}.bin", i),
            size_bytes: Some(100),
        };
        manager
            .create_operation(
                OperationType::IntegrityHash,
                target,
                Some("admin".to_string()),
            )
            .unwrap();
    }

    let all_ops = manager.list_operations(10, None, None).unwrap();
    assert_eq!(all_ops.len(), 3);

    let created_only = manager.list_operations(10, Some("Created"), None).unwrap();
    assert_eq!(created_only.len(), 3);

    let running_only = manager.list_operations(10, Some("Running"), None).unwrap();
    assert_eq!(running_only.len(), 0);
}

#[tokio::test]
async fn test_11_execute_integrity_verify_operation_matching() {
    let (manager, _, audit) = setup_test_manager();
    let file = TempTestFile::new(b"abc");
    let sha256_abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: file.path().to_string_lossy().to_string(),
        display_name: "abc.txt".to_string(),
        size_bytes: Some(3),
    };

    let params = serde_json::json!({
        "expected_digest": sha256_abc
    });

    let op = manager
        .submit_and_run(
            OperationType::IntegrityVerify,
            target,
            Some("operator2".to_string()),
            Some(params),
        )
        .await
        .expect("submit_and_run failed for verify");

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let finished_op = manager
        .get_operation(&op.operation_id)
        .expect("get_operation failed");
    assert_eq!(finished_op.current_state, OperationState::Completed);
    assert!(finished_op.result_summary.unwrap().contains("VERIFIED"));

    let events = audit.list_events(10).expect("list_events failed");
    let event_types: Vec<String> = events.into_iter().map(|e| e.event_type).collect();
    assert!(event_types.contains(&"OPERATION_COMPLETED".to_string()));
}

#[tokio::test]
async fn test_12_safety_interlock_blocks_execution_on_invalid_target() {
    use locardx_auth::AuthService;
    use locardx_device_manager::{DeviceDiscoveryProvider, DeviceManagerService, PhysicalDevice};
    use locardx_security::SafetyEngine;

    struct EmptyProvider;
    impl DeviceDiscoveryProvider for EmptyProvider {
        fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, locardx_common::LocardError> {
            Ok(vec![])
        }
    }

    let (mut manager, db, audit) = setup_test_manager();
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let dev_mgr = Arc::new(DeviceManagerService::new(Arc::new(EmptyProvider)));
    let safety = Arc::new(SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        dev_mgr,
    ));

    manager = manager.with_safety(safety);

    // Attempt to execute on a non-existent file
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: "C:\\non_existent_path_404_file.img".to_string(),
        display_name: "missing.img".to_string(),
        size_bytes: None,
    };

    let result = manager
        .submit_and_run(
            OperationType::IntegrityHash,
            target,
            Some("admin".to_string()),
            None,
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("Safety interlock rejected execution")
            || err.to_string().contains("INVALID_TARGET")
    );
}

#[tokio::test]
async fn test_13_simulated_drive_erasure_dispatch() {
    let (base_manager, db, audit) = setup_test_manager();
    let auth = Arc::new(locardx_auth::AuthService::new(
        Arc::clone(&db),
        Arc::clone(&audit),
    ));
    let mock_registry = locardx_drive_eraser::MockDeviceRegistry::new_standard_test_set();
    let provider = Arc::new(mock_registry);
    let device_manager = Arc::new(locardx_device_manager::DeviceManagerService::new(
        provider.clone(),
    ));
    let safety = Arc::new(locardx_security::SafetyEngine::new(
        db.clone(),
        audit.clone(),
        auth,
        device_manager,
    ));
    let drive_eraser = Arc::new(locardx_drive_eraser::DriveEraserService::new(
        db.clone(),
        audit,
        safety,
        provider,
    ));

    let manager = base_manager.with_drive_eraser(drive_eraser);

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: r"\\.\PhysicalDrive1".to_string(),
        display_name: "Seagate 2TB HDD".to_string(),
        size_bytes: Some(2_000_000_000_000),
    };

    let params = serde_json::json!({
        "requested_method": "Nist80088ClearZero",
        "confirmation_id": "conf-challenge-mgr-13",
        "typed_confirmation": r"\\.\PhysicalDrive1",
        "warning_acknowledged": true,
        "session_token": "token-test-13",
    });

    let op = manager
        .submit_and_run(
            OperationType::DriveErasure,
            target,
            Some("investigator-1".to_string()),
            Some(params),
        )
        .await
        .unwrap();

    assert_eq!(op.operation_type, OperationType::DriveErasure);
    assert_eq!(op.current_state, OperationState::Running);
}
