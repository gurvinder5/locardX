use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService, DeviceType, PhysicalDevice,
};
use locardx_drive_eraser::{
    assess_device_capabilities, CapabilityState, DefaultDriveHardwareProvider, DriveCapability,
    DriveEraseFailureReason, DriveErasePlan, DriveEraseStatus, DriveEraserService,
    DriveExecutionState, DriveSanitizationMethod, DriveSanitizer, DriveVerificationPlan,
    DriveVerificationStrategy, ExecutionMode, MockDeviceRegistry, PhysicalDeviceSnapshot,
    RealHardwareExecutionGate, RealHardwareSanitizer,
};
use locardx_security::{RiskLevel, SafetyEngine};
use std::sync::Arc;

#[allow(dead_code)]
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

fn create_sample_plan(
    device_id: &str,
    is_system: bool,
    is_boot: bool,
    mode: ExecutionMode,
) -> DriveErasePlan {
    let mock = MockDeviceRegistry::new_standard_test_set();
    let dev_opt = mock
        .discover_devices()
        .unwrap()
        .into_iter()
        .find(|d| d.device_id.eq_ignore_ascii_case(device_id));
    let (serial_number, capacity_bytes, sector_size, vendor, model) = if let Some(d) = dev_opt {
        (d.serial_number, d.capacity_bytes, 512, d.vendor, d.model)
    } else {
        (
            Some("SERIAL-TEST-123".to_string()),
            1_000_000_000_000,
            512,
            Some("Crucial".to_string()),
            Some("CT1000MX500SSD1".to_string()),
        )
    };

    let snapshot = PhysicalDeviceSnapshot {
        device_id: device_id.to_string(),
        display_name: "Test Physical Disk".to_string(),
        vendor,
        model,
        serial_number: serial_number.clone(),
        media_type: DeviceType::Ssd,
        capacity_bytes,
        sector_size,
        is_system,
        is_boot,
        is_removable: false,
        is_read_only: false,
        exists: true,
        physical_sector_size: None,
        bus_type: None,
        classification: if is_system {
            DeviceClassification::SystemDevice
        } else if is_boot {
            DeviceClassification::BootDevice
        } else {
            DeviceClassification::FixedDataDevice
        },
        partition_count: 1,
        volume_labels: vec![],
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let capabilities = locardx_drive_eraser::DriveCapabilities {
        supported_capabilities: vec![
            DriveCapability::AtaSecureErase,
            DriveCapability::SequentialWrite,
            DriveCapability::FullDeviceRead,
        ],
        interface_bus: "SATA".to_string(),
        sector_size: 512,
        is_rotational: false,
        supports_crypto_erase: false,
        supports_firmware_sanitize: true,
        supports_overwrite: true,
    };

    DriveErasePlan {
        plan_id: "plan-boundary-001".to_string(),
        physical_device_id: device_id.to_string(),
        display_name: "Test Physical Disk".to_string(),
        vendor: Some("Crucial".to_string()),
        model: Some("CT1000MX500".to_string()),
        serial_number,
        media_type: DeviceType::Ssd,
        capacity_bytes,
        sector_size,
        method: DriveSanitizationMethod::AtaSecureErase,
        passes: 1,
        risk_level: RiskLevel::Critical,
        capabilities,
        verification_plan: DriveVerificationPlan {
            strategy: DriveVerificationStrategy::FirmwareStatusVerify,
            sample_percentage: None,
            requirements: "Firmware status check".to_string(),
            limitations: vec![],
        },
        limitations: vec![],
        device_snapshot: snapshot,
        execution_mode: mode,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[test]
fn test_real_hardware_cannot_bypass_target_validation() {
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock));

    // 1. Logical drive C:\
    let plan_c = create_sample_plan(r"C:\", false, false, ExecutionMode::RealHardware);
    let res_c = gate.request_permit(&plan_c, "op-1", r"C:\", true, &hw_provider);
    assert!(res_c.is_err());
    match res_c.unwrap_err() {
        DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
            assert!(msg.to_lowercase().contains("logical volume"));
        }
        other => panic!("Expected LogicalVolumeTargetRejected, got {:?}", other),
    }

    // 2. Logical volume D:\Data
    let plan_d = create_sample_plan(r"D:\Data", false, false, ExecutionMode::RealHardware);
    let res_d = gate.request_permit(&plan_d, "op-2", r"D:\Data", true, &hw_provider);
    assert!(res_d.is_err());
    match res_d.unwrap_err() {
        DriveEraseFailureReason::LogicalVolumeTargetRejected(_) => {}
        other => panic!("Expected LogicalVolumeTargetRejected, got {:?}", other),
    }
}

#[test]
fn test_real_hardware_cannot_bypass_system_and_boot_protection() {
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock));

    // 1. System device
    let plan_sys = create_sample_plan(
        r"\\.\PhysicalDrive0",
        true,
        false,
        ExecutionMode::RealHardware,
    );
    let res_sys = gate.request_permit(
        &plan_sys,
        "op-sys",
        r"\\.\PhysicalDrive0",
        true,
        &hw_provider,
    );
    assert!(res_sys.is_err());
    match res_sys.unwrap_err() {
        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
            assert!(msg.contains("active system or boot device"));
        }
        other => panic!("Expected SystemOrBootDeviceProtected, got {:?}", other),
    }

    // 2. Boot device
    let plan_boot = create_sample_plan(
        r"\\.\PhysicalDrive6",
        false,
        true,
        ExecutionMode::RealHardware,
    );
    let res_boot = gate.request_permit(
        &plan_boot,
        "op-boot",
        r"\\.\PhysicalDrive6",
        true,
        &hw_provider,
    );
    assert!(res_boot.is_err());
    match res_boot.unwrap_err() {
        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
            assert!(msg.contains("active system or boot device"));
        }
        other => panic!("Expected SystemOrBootDeviceProtected, got {:?}", other),
    }
}

#[test]
fn test_real_hardware_cannot_bypass_authorization() {
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock));
    let plan = create_sample_plan(
        r"\\.\PhysicalDrive2",
        false,
        false,
        ExecutionMode::RealHardware,
    );

    // 1. Warning not acknowledged
    let res_no_ack = gate.request_permit(
        &plan,
        "op-auth-1",
        r"\\.\PhysicalDrive2",
        false,
        &hw_provider,
    );
    assert!(res_no_ack.is_err());
    match res_no_ack.unwrap_err() {
        DriveEraseFailureReason::SecurityViolation(msg) => {
            assert!(msg.contains("Destructive consequences warning"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 2. Typed confirmation mismatch
    let res_mismatch = gate.request_permit(
        &plan,
        "op-auth-2",
        r"\\.\PhysicalDrive99",
        true,
        &hw_provider,
    );
    assert!(res_mismatch.is_err());
    match res_mismatch.unwrap_err() {
        DriveEraseFailureReason::SecurityViolation(msg) => {
            assert!(msg.contains("does not match"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }
}

#[test]
fn test_real_hardware_cannot_bypass_toctou_validation() {
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock.clone()));

    // Target PhysicalDrive9 has serial "SERIAL-ORIGINAL-123"
    let mut plan = create_sample_plan(
        r"\\.\PhysicalDrive9",
        false,
        false,
        ExecutionMode::RealHardware,
    );
    plan.device_snapshot.serial_number = Some("SERIAL-ORIGINAL-123".to_string());
    plan.device_snapshot.capacity_bytes = 500_000_000_000;

    // Mutate live serial number in mock registry
    mock.mutate_device(r"\\.\PhysicalDrive9", |dev| {
        dev.serial_number = Some("SERIAL-MUTATED-999".to_string());
    });

    let res = gate.request_permit(
        &plan,
        "op-toctou",
        r"\\.\PhysicalDrive9",
        true,
        &hw_provider,
    );
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::DeviceMutated(msg) => {
            assert!(
                msg.contains("serial number changed"),
                "Expected serial mutation, got: {}",
                msg
            );
        }
        other => panic!("Expected DeviceMutated, got {:?}", other),
    }
}

#[test]
fn test_real_hardware_execution_mode_restriction_rejected_by_gate() {
    // Gate with default (hardware_execution_enabled = false)
    let gate = RealHardwareExecutionGate::new();
    assert!(!gate.is_enabled());

    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock));
    let plan = create_sample_plan(
        r"\\.\PhysicalDrive2",
        false,
        false,
        ExecutionMode::RealHardware,
    );

    let res = gate.request_permit(
        &plan,
        "op-gate-closed",
        r"\\.\PhysicalDrive2",
        true,
        &hw_provider,
    );
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::RealHardwareExecutionNotEnabled(msg) => {
            assert!(
                msg.contains("Real hardware execution gate is CLOSED in Step 10B.1"),
                "Got: {}",
                msg
            );
        }
        other => panic!("Expected RealHardwareExecutionNotEnabled, got {:?}", other),
    }
}

#[test]
fn test_hardware_execution_permit_single_use_and_binding() {
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock));
    let plan = create_sample_plan(
        r"\\.\PhysicalDrive2",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let permit = gate
        .request_permit(
            &plan,
            "op-permit-001",
            r"\\.\PhysicalDrive2",
            true,
            &hw_provider,
        )
        .unwrap();

    // 1. Check bindings
    assert_eq!(permit.operation_id(), "op-permit-001");
    assert_eq!(permit.plan_id(), "plan-boundary-001");
    assert_eq!(permit.physical_device_id(), r"\\.\PhysicalDrive2");
    assert_eq!(permit.execution_mode(), ExecutionMode::Simulation);
    assert!(!permit.is_consumed());

    // 2. Validate binding checks
    assert!(permit
        .validate_binding("op-permit-001", "plan-boundary-001", r"\\.\PhysicalDrive2")
        .is_ok());
    assert!(permit
        .validate_binding("op-other", "plan-boundary-001", r"\\.\PhysicalDrive2")
        .is_err());
    assert!(permit
        .validate_binding("op-permit-001", "plan-other", r"\\.\PhysicalDrive2")
        .is_err());
    assert!(permit
        .validate_binding("op-permit-001", "plan-boundary-001", r"\\.\PhysicalDrive3")
        .is_err());

    // 3. First consumption succeeds
    assert!(permit.consume().is_ok());
    assert!(permit.is_consumed());

    // 4. Second consumption fails closed
    let res_second = permit.consume();
    assert!(res_second.is_err());
    match res_second.unwrap_err() {
        DriveEraseFailureReason::PermitAlreadyConsumed(id) => {
            assert_eq!(id, permit.permit_id());
        }
        other => panic!("Expected PermitAlreadyConsumed, got {:?}", other),
    }
}

#[test]
fn test_real_sanitizer_is_non_destructive_stub() {
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock));
    let plan = create_sample_plan(
        r"\\.\PhysicalDrive2",
        false,
        false,
        ExecutionMode::RealHardware,
    );

    let permit = gate
        .request_permit(
            &plan,
            "op-stub-001",
            r"\\.\PhysicalDrive2",
            true,
            &hw_provider,
        )
        .unwrap();

    let sanitizer = RealHardwareSanitizer::stub();
    assert_eq!(sanitizer.execution_mode(), ExecutionMode::RealHardware);

    // Invoking real hardware sanitizer stub MUST unconditionally reject execution
    let res = sanitizer.execute(&permit, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::RealHardwareExecutionNotEnabled(msg) => {
            assert!(
                msg.contains("Real hardware backend execution on '\\\\.\\PhysicalDrive2' is disabled in Step 10B.1"),
                "Got: {}",
                msg
            );
        }
        other => panic!("Expected RealHardwareExecutionNotEnabled, got {:?}", other),
    }

    // In addition, permit was consumed by the attempt
    assert!(permit.is_consumed());
}

#[test]
fn test_capability_states_unknown_and_detection_failed_fail_closed() {
    // 1. Unknown device fails closed with CapabilityState::Unknown
    let unknown_dev = PhysicalDevice {
        device_id: r"\\.\PhysicalDrive7".to_string(),
        display_name: "Legacy Tape Drive".to_string(),
        vendor: None,
        model: None,
        serial_number: None,
        device_type: DeviceType::Unknown,
        capacity_bytes: 10_000_000_000,
        removable: true,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::RemovableDevice,
        volumes: vec![],
    };

    let assess_unknown = assess_device_capabilities(&unknown_dev);
    assert_eq!(assess_unknown.overall_state, CapabilityState::Unknown);
    assert!(assess_unknown.overall_state.should_fail_closed());
    assert_eq!(
        assess_unknown.method_state(DriveSanitizationMethod::Nist80088ClearZero),
        CapabilityState::Unknown
    );

    // 2. Corrupted device with 0 capacity fails closed with CapabilityState::DetectionFailed
    let corrupted_dev = PhysicalDevice {
        device_id: r"\\.\PhysicalDrive8".to_string(),
        display_name: "Corrupted Drive".to_string(),
        vendor: None,
        model: None,
        serial_number: None,
        device_type: DeviceType::Hdd,
        capacity_bytes: 0,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::FixedDataDevice,
        volumes: vec![],
    };

    let assess_corrupt = assess_device_capabilities(&corrupted_dev);
    assert_eq!(
        assess_corrupt.overall_state,
        CapabilityState::DetectionFailed
    );
    assert!(assess_corrupt.overall_state.should_fail_closed());
}

#[test]
fn test_interrupted_operation_never_becomes_completed() {
    // Test that an interrupted execution strictly produces Cancelled or Failed, never Completed
    let status_cancelled = DriveEraseStatus::from(DriveExecutionState::Cancelled);
    assert_eq!(status_cancelled, DriveEraseStatus::Cancelled);
    assert_ne!(status_cancelled, DriveEraseStatus::Completed);

    let status_failed = DriveEraseStatus::from(DriveExecutionState::Failed);
    assert_eq!(status_failed, DriveEraseStatus::Failed);
    assert_ne!(status_failed, DriveEraseStatus::Completed);

    // DriveExecutionState lifecycle sequence verification
    assert_eq!(DriveExecutionState::Planned.as_str(), "PLANNED");
    assert_eq!(DriveExecutionState::Authorized.as_str(), "AUTHORIZED");
    assert_eq!(
        DriveExecutionState::PreExecutionCheck.as_str(),
        "PRE_EXECUTION_CHECK"
    );
    assert_eq!(
        DriveExecutionState::ExecutionReady.as_str(),
        "EXECUTION_READY"
    );
    assert_eq!(DriveExecutionState::Executing.as_str(), "EXECUTING");
    assert_eq!(
        DriveExecutionState::ExecutionComplete.as_str(),
        "EXECUTION_COMPLETE"
    );
    assert_eq!(DriveExecutionState::Verifying.as_str(), "VERIFYING");
    assert_eq!(DriveExecutionState::Completed.as_str(), "COMPLETED");
    assert_eq!(DriveExecutionState::Failed.as_str(), "FAILED");
    assert_eq!(DriveExecutionState::Cancelled.as_str(), "CANCELLED");
    assert_eq!(DriveExecutionState::Unknown.as_str(), "UNKNOWN");
    assert_eq!(
        DriveExecutionState::VerificationFailed.as_str(),
        "VERIFICATION_FAILED"
    );
    assert_eq!(
        DriveExecutionState::UnableToVerify.as_str(),
        "UNABLE_TO_VERIFY"
    );
}
