use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceManagerService, DeviceType, PhysicalDevice,
};
use locardx_drive_eraser::{
    detect_device_capabilities, generate_drive_erase_plan, DriveEraseFailureReason,
    DriveEraseRequest, DriveEraserService, ExecutionMode, MockDeviceRegistry,
    PhysicalDeviceSnapshot,
};
use locardx_security::SafetyEngine;
use std::sync::Arc;

fn setup_test_service() -> (DriveEraserService, MockDeviceRegistry) {
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
    let service = DriveEraserService::new(db, audit, safety, provider);
    (service, mock_registry)
}

#[tokio::test]
async fn test_planning_with_real_hardware_mode_is_forbidden() {
    let (service, _registry) = setup_test_service();

    let res = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::RealHardware),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await;

    assert!(res.is_err());
    match res.unwrap_err() {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(
                msg.contains("Real hardware execution is permanently disabled in Step 10A"),
                "Expected invariant message, got: {}",
                msg
            );
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }
}

#[tokio::test]
async fn test_execution_with_real_hardware_mode_is_forbidden() {
    let (service, _registry) = setup_test_service();

    // First create a simulated plan
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

    // Attempt to execute with RealHardware mode
    let res = service
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            "conf-challenge-real",
            "op-real-001",
            r"\\.\PhysicalDrive1",
            true,
            "session-token",
            ExecutionMode::RealHardware,
            None,
            None,
        )
        .await;

    assert!(res.is_err());
    match res.unwrap_err() {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(
                msg.contains("CRITICAL INVARIANT: Real hardware execution is permanently disabled in Step 10A"),
                "Expected critical invariant message, got: {}",
                msg
            );
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }
}

#[test]
fn test_planner_direct_rejection_of_real_hardware_mode() {
    let device = PhysicalDevice {
        device_id: r"\\.\PhysicalDrive1".to_string(),
        display_name: "Test Seagate HDD".to_string(),
        vendor: Some("Seagate".to_string()),
        model: Some("ST2000DM008".to_string()),
        serial_number: Some("S123".to_string()),
        device_type: DeviceType::Hdd,
        capacity_bytes: 2_000_000_000_000,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::FixedDataDevice,
        volumes: vec![],
    };
    let snapshot = PhysicalDeviceSnapshot::from_device(&device, 512);
    let caps = detect_device_capabilities(&device);

    let res = generate_drive_erase_plan(snapshot, caps, None, ExecutionMode::RealHardware);

    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::RealHardwareExecutionDisabled(msg) => {
            assert!(
                msg.contains("disabled in Step 10A") || msg.contains("Simulation"),
                "Expected disabled message, got: {}",
                msg
            );
        }
        other => panic!("Expected RealHardwareExecutionDisabled, got {:?}", other),
    }
}
