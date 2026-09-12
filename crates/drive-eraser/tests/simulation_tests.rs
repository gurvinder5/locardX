use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_database::Database;
use locardx_device_manager::DeviceManagerService;
use locardx_drive_eraser::{
    DriveEraseFailureReason, DriveEraseRequest, DriveEraseStatus, DriveEraserService,
    DriveSanitizationMethod, DriveVerificationStrategy, ExecutionMode, MockDeviceRegistry,
};
use locardx_security::SafetyEngine;
use locardx_verification::sanitization::VerificationOutcome;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

fn setup_test_service() -> (DriveEraserService, MockDeviceRegistry, Arc<Database>) {
    let db = Arc::new(Database::open(":memory:").unwrap());
    let audit = Arc::new(AuditService::new(db.clone()));
    let auth = Arc::new(AuthService::new(db.clone(), audit.clone()));
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let provider = Arc::new(mock_registry.clone());
    let device_manager = Arc::new(DeviceManagerService::new(provider.clone()));
    let safety = Arc::new(SafetyEngine::new(
        db.clone(),
        audit.clone(),
        auth.clone(),
        device_manager,
    ));
    let service = DriveEraserService::new(db.clone(), audit, safety, provider);
    (service, mock_registry, db)
}

#[tokio::test]
async fn test_simulated_hdd_erasure_end_to_end() {
    let (service, _registry, db) = setup_test_service();

    // 1. Plan erasure on PhysicalDrive1 (HDD)
    let plan = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::Simulation),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    assert_eq!(plan.physical_device_id, r"\\.\PhysicalDrive1");
    assert_eq!(plan.method, DriveSanitizationMethod::Nist80088ClearZero);
    assert_eq!(plan.passes, 1);

    // 2. Execute simulation with two-stage confirmation
    let op_id = "op-hdd-sim-001";
    let progress_count = Arc::new(AtomicU32::new(0));
    let pc = progress_count.clone();

    let result = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-123",
            op_id,
            r"\\.\PhysicalDrive1",
            true,
            "session-token-abc",
            ExecutionMode::Simulation,
            None,
            Some(&move |_progress| {
                pc.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .await
        .unwrap();

    // Verify completion
    assert_eq!(result.status, DriveEraseStatus::Completed);
    assert_eq!(result.execution_mode, ExecutionMode::Simulation);
    assert_eq!(result.method, DriveSanitizationMethod::Nist80088ClearZero);
    assert!(result.bytes_processed > 0);
    assert_eq!(result.verification.outcome, VerificationOutcome::Verified);
    assert!(progress_count.load(Ordering::SeqCst) > 0);

    // 3. Verify SQLite DB persistence via get_drive_erasure_result
    let saved_result = service.get_drive_erasure_result(op_id).unwrap().unwrap();
    assert_eq!(saved_result.operation_id, op_id);
    assert_eq!(saved_result.status, DriveEraseStatus::Completed);
    assert_eq!(saved_result.physical_device_id, r"\\.\PhysicalDrive1");

    // Check direct table query
    let count = db
        .with_conn(|conn| {
            let row_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM drive_erasure_records WHERE operation_id = ?1",
                rusqlite::params![op_id],
                |r| r.get(0),
            )?;
            Ok(row_count)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_simulated_nvme_crypto_erasure() {
    let (service, _registry, _db) = setup_test_service();

    let plan = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive3".to_string(),
                requested_method: Some("nvmecryptoerase".to_string()),
                execution_mode: Some(ExecutionMode::Simulation),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    assert_eq!(plan.method, DriveSanitizationMethod::NvmeCryptoErase);
    assert_eq!(
        plan.verification_plan.strategy,
        DriveVerificationStrategy::CryptoKeyDestructionCheck
    );

    let op_id = "op-nvme-sim-002";
    let result = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-nvme",
            op_id,
            "PhysicalDrive3", // without \\.\ prefix should also match
            true,
            "session-token-nvme",
            ExecutionMode::Simulation,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.status, DriveEraseStatus::Completed);
    assert_eq!(result.method, DriveSanitizationMethod::NvmeCryptoErase);
    assert_eq!(result.verification.outcome, VerificationOutcome::Verified);
}

#[tokio::test]
async fn test_simulated_sata_ssd_ata_secure_erase() {
    let (service, _registry, _db) = setup_test_service();

    let plan = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive2".to_string(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::Simulation),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    assert_eq!(plan.method, DriveSanitizationMethod::AtaSecureErase);

    let op_id = "op-sata-sim-003";
    let result = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-sata",
            op_id,
            r"\\.\PhysicalDrive2",
            true,
            "session-token-sata",
            ExecutionMode::Simulation,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.status, DriveEraseStatus::Completed);
    assert_eq!(result.method, DriveSanitizationMethod::AtaSecureErase);
    assert_eq!(result.verification.outcome, VerificationOutcome::Verified);
}

#[tokio::test]
async fn test_simulated_erasure_cooperative_cancellation() {
    let (service, _registry, _db) = setup_test_service();

    let plan = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::Simulation),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    let op_id = "op-cancel-sim-004";
    let cancel_flag = true;

    let result = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-cancel",
            op_id,
            r"\\.\PhysicalDrive1",
            true,
            "session-token-cancel",
            ExecutionMode::Simulation,
            Some(&move || cancel_flag),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.status, DriveEraseStatus::Cancelled);
    assert_eq!(
        result.failure_reason,
        Some(DriveEraseFailureReason::Cancelled)
    );

    let saved = service.get_drive_erasure_result(op_id).unwrap().unwrap();
    assert_eq!(saved.status, DriveEraseStatus::Cancelled);
}

#[tokio::test]
async fn test_two_stage_confirmation_safety_interlocks() {
    let (service, _registry, _db) = setup_test_service();

    let plan = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::Simulation),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    // 1. Wrong typed target confirmation
    let res_wrong = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-err",
            "op-err-1",
            "WrongPhysicalDrive99",
            true,
            "session-token",
            ExecutionMode::Simulation,
            None,
            None,
        )
        .await;
    assert!(res_wrong.is_err());
    match res_wrong.unwrap_err() {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("does not match physical device ID"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 2. Destructive consequences warning not acknowledged
    let res_unack = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-err",
            "op-err-2",
            r"\\.\PhysicalDrive1",
            false,
            "session-token",
            ExecutionMode::Simulation,
            None,
            None,
        )
        .await;
    assert!(res_unack.is_err());
    match res_unack.unwrap_err() {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("must be explicitly acknowledged"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 3. Attempt to plan erasure on System Disk PhysicalDrive0 fails immediately
    let res_sys = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive0".to_string(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::Simulation),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await;
    assert!(res_sys.is_err());
    match res_sys.unwrap_err() {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("System or Boot") || msg.contains("blocked"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }
}
