use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService, DeviceType,
};
use locardx_drive_eraser::{
    CapabilityState, DefaultDriveHardwareProvider, DeviceLockRegistry, DriveCapabilities,
    DriveCapabilitiesAssessment, DriveCapability, DriveEraseFailureReason, DriveErasePlan,
    DriveEraseProgress, DriveEraseStatus, DriveEraserService, DriveExecutionState,
    DriveHardwareExecutor, DriveSanitizationMethod, DriveSanitizer, DriveVerificationPlan,
    DriveVerificationResult, DriveVerificationStrategy, ExclusiveDriveHandle, ExecutionMode,
    HardwareExecutionPermit, MockDeviceRegistry, PhysicalDeviceSnapshot, RealHardwareExecutionGate,
    RealHardwareSanitizer, SimulatedHddOverwrite,
};
use locardx_security::{RiskLevel, SafetyEngine};
use locardx_verification::sanitization::VerificationOutcome;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ============================================================================
// MOCK PLATFORM EXECUTOR IMPLEMENTATION
// ============================================================================

struct MockHandle {
    device_id: String,
    valid: Arc<AtomicBool>,
}

impl ExclusiveDriveHandle for MockHandle {
    fn device_id(&self) -> &str {
        &self.device_id
    }
    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct MockDriveHardwareExecutor {
    fail_lock: AtomicBool,
    fail_disconnect: AtomicBool,
    fail_io: AtomicBool,
    fail_verification: AtomicBool,
    ambiguous_verification: AtomicBool,
    executed_methods: Mutex<Vec<DriveSanitizationMethod>>,
    total_bytes_written: AtomicU64,
}

impl DriveHardwareExecutor for MockDriveHardwareExecutor {
    fn acquire_exclusive_access(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ExclusiveDriveHandle>, DriveEraseFailureReason> {
        if self.fail_lock.load(Ordering::SeqCst) {
            return Err(DriveEraseFailureReason::ExclusiveAccessFailed(format!(
                "Failed to acquire exclusive lock on device '{}'",
                device_id
            )));
        }
        if self.fail_disconnect.load(Ordering::SeqCst) {
            return Err(DriveEraseFailureReason::DeviceDisconnected(format!(
                "Device '{}' vanished before exclusive handle opened",
                device_id
            )));
        }
        Ok(Box::new(MockHandle {
            device_id: device_id.to_string(),
            valid: Arc::new(AtomicBool::new(true)),
        }))
    }

    fn execute_sanitization(
        &self,
        handle: &mut Box<dyn ExclusiveDriveHandle>,
        permit: &HardwareExecutionPermit,
        _is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        if !handle.is_valid() {
            return Err(DriveEraseFailureReason::DeviceDisconnected(
                "Device handle invalid during execution".to_string(),
            ));
        }
        if self.fail_io.load(Ordering::SeqCst) {
            return Err(DriveEraseFailureReason::IoError(
                "Win32 write error 0x5: Access Denied / Bad Sector".to_string(),
            ));
        }

        self.executed_methods.lock().unwrap().push(permit.method());
        let bytes = permit.device_snapshot().capacity_bytes;
        self.total_bytes_written.fetch_add(bytes, Ordering::SeqCst);

        if let Some(cb) = on_progress {
            cb(DriveEraseProgress {
                operation_id: permit.operation_id().to_string(),
                percentage: 100.0,
                bytes_processed: bytes,
                total_bytes: bytes,
                current_pass: 1,
                total_passes: 1,
                current_stage: "Mock sector overwrite complete".to_string(),
                elapsed_seconds: 0.1,
                eta_seconds: None,
            });
        }

        Ok((bytes, 0.1))
    }

    fn verify_sanitization(
        &self,
        handle: &mut Box<dyn ExclusiveDriveHandle>,
        _permit: &HardwareExecutionPermit,
    ) -> Result<DriveVerificationResult, DriveEraseFailureReason> {
        if !handle.is_valid() {
            return Err(DriveEraseFailureReason::DeviceDisconnected(
                "Device handle invalid during verification".to_string(),
            ));
        }
        if self.fail_verification.load(Ordering::SeqCst) {
            return Ok(DriveVerificationResult {
                outcome: VerificationOutcome::VerificationFailed,
                strategy: DriveVerificationStrategy::FullDeviceReadVerify,
                details: "Sector 0x120 contains non-null pattern 0xAA".to_string(),
                verified_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        if self.ambiguous_verification.load(Ordering::SeqCst) {
            return Ok(DriveVerificationResult {
                outcome: VerificationOutcome::UnableToVerify,
                strategy: DriveVerificationStrategy::FullDeviceReadVerify,
                details: "Read command timed out on verification pass".to_string(),
                verified_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        Ok(DriveVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: DriveVerificationStrategy::FullDeviceReadVerify,
            details: "100% readback verified 0x00 pattern across all LBAs".to_string(),
            verified_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

// ============================================================================
// TEST HELPERS
// ============================================================================

fn setup_test_service(
    mock_executor: Arc<dyn DriveHardwareExecutor>,
) -> (DriveEraserService, MockDeviceRegistry, Arc<AuditService>) {
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
    let service = DriveEraserService::new(db, audit.clone(), safety, provider)
        .with_executor(mock_executor)
        .with_execution_gate(Arc::new(RealHardwareExecutionGate::with_enabled(true)));
    (service, mock_registry, audit)
}

fn create_sample_plan(
    device_id: &str,
    is_system: bool,
    is_boot: bool,
    mode: ExecutionMode,
    method: DriveSanitizationMethod,
) -> DriveErasePlan {
    let mock = MockDeviceRegistry::new_standard_test_set();
    let dev_opt = mock
        .discover_devices()
        .unwrap()
        .into_iter()
        .find(|d| d.device_id.eq_ignore_ascii_case(device_id));
    let (
        serial_number,
        capacity_bytes,
        sector_size,
        vendor,
        model,
        media_type,
        partition_count,
        volume_labels,
    ) = if let Some(ref d) = dev_opt {
        let labels = d.volumes.iter().filter_map(|v| v.label.clone()).collect();
        (
            d.serial_number.clone(),
            d.capacity_bytes,
            512,
            d.vendor.clone(),
            d.model.clone(),
            d.device_type,
            d.volumes.len(),
            labels,
        )
    } else {
        (
            Some("SERIAL-TEST-123".to_string()),
            1_000_000_000_000,
            512,
            Some("Crucial".to_string()),
            Some("CT1000MX500SSD1".to_string()),
            DeviceType::Ssd,
            1,
            vec!["Data".to_string()],
        )
    };

    let snapshot = PhysicalDeviceSnapshot {
        device_id: device_id.to_string(),
        display_name: "Test Physical Disk".to_string(),
        vendor,
        model,
        serial_number: serial_number.clone(),
        media_type,
        capacity_bytes,
        sector_size,
        physical_sector_size: None,
        bus_type: None,
        is_system,
        is_boot,
        is_removable: false,
        is_read_only: false,
        exists: true,
        classification: if is_system {
            DeviceClassification::SystemDevice
        } else if is_boot {
            DeviceClassification::BootDevice
        } else {
            DeviceClassification::FixedDataDevice
        },
        partition_count,
        volume_labels,
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut map = HashMap::new();
    map.insert(DriveCapability::SequentialWrite, CapabilityState::Supported);
    map.insert(DriveCapability::FullDeviceRead, CapabilityState::Supported);
    map.insert(DriveCapability::SectorAccess, CapabilityState::Supported);

    let mut method_support = HashMap::new();
    method_support.insert(
        DriveSanitizationMethod::Nist80088ClearZero,
        CapabilityState::Supported,
    );
    method_support.insert(
        DriveSanitizationMethod::Dod522022M,
        CapabilityState::Supported,
    );

    let assessment = DriveCapabilitiesAssessment {
        overall_state: CapabilityState::Supported,
        capabilities: DriveCapabilities {
            supported_capabilities: vec![
                DriveCapability::SequentialWrite,
                DriveCapability::FullDeviceRead,
                DriveCapability::SectorAccess,
            ],
            interface_bus: "SATA".to_string(),
            sector_size: 512,
            is_rotational: false,
            supports_crypto_erase: false,
            supports_firmware_sanitize: false,
            supports_overwrite: true,
        },
        capability_states: map,
        method_support,
        assessment_notes: vec!["Mock verified assessment".to_string()],
    };

    DriveErasePlan {
        plan_id: format!("plan-{}", uuid::Uuid::new_v4()),
        physical_device_id: device_id.to_string(),
        display_name: "Test Physical Disk".to_string(),
        vendor: snapshot.vendor.clone(),
        model: snapshot.model.clone(),
        serial_number,
        media_type,
        capacity_bytes,
        sector_size,
        method,
        passes: 1,
        risk_level: RiskLevel::High,
        capabilities: assessment.capabilities,
        verification_plan: DriveVerificationPlan {
            strategy: DriveVerificationStrategy::FullDeviceReadVerify,
            sample_percentage: Some(100.0),
            requirements: "Full LBA readback verification".to_string(),
            limitations: vec![],
        },
        limitations: vec![],
        device_snapshot: snapshot,
        execution_mode: mode,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

// ============================================================================
// A. PERMIT TESTS
// ============================================================================

#[test]
fn test_permit_valid_consumed_on_execution() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::new(
        mock_registry.clone(),
    )));

    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let permit = gate
        .request_permit(
            &plan,
            "op-permit-1",
            r"\\.\PhysicalDrive1",
            true,
            hw_provider.as_ref(),
        )
        .unwrap();

    assert!(!permit.is_consumed());

    let sanitizer = RealHardwareSanitizer::with_components(
        mock_executor.clone(),
        Arc::new(DeviceLockRegistry::new()),
        Some(hw_provider.clone()),
    );

    let res = sanitizer.execute_and_verify(&permit, None, None);
    assert!(res.is_ok());
    let (bytes, elapsed, verification) = res.unwrap();
    assert_eq!(bytes, plan.capacity_bytes);
    assert!(elapsed > 0.0);
    assert_eq!(verification.outcome, VerificationOutcome::Verified);

    // Must now be marked consumed
    assert!(permit.is_consumed());

    // Second execution with the same permit MUST fail closed
    let second_res = sanitizer.execute_and_verify(&permit, None, None);
    assert!(second_res.is_err());
    match second_res.unwrap_err() {
        DriveEraseFailureReason::PermitAlreadyConsumed(_) => {}
        other => panic!("Expected PermitAlreadyConsumed, got {:?}", other),
    }
}

#[test]
fn test_permit_expired_rejected() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::new(
        mock_registry.clone(),
    )));

    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mut permit = gate
        .request_permit(
            &plan,
            "op-expired-1",
            r"\\.\PhysicalDrive1",
            true,
            hw_provider.as_ref(),
        )
        .unwrap();

    // Set TTL to 0 seconds and sleep 10ms to ensure expiration
    permit = permit.with_ttl(0);
    thread::sleep(Duration::from_millis(10));

    let sanitizer = RealHardwareSanitizer::with_components(
        mock_executor,
        Arc::new(DeviceLockRegistry::new()),
        Some(hw_provider),
    );

    let res = sanitizer.execute_and_verify(&permit, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::PermitExpired(msg) => {
            assert!(msg.contains("expired"));
        }
        other => panic!("Expected PermitExpired, got {:?}", other),
    }
}

#[test]
fn test_permit_binding_mismatch_rejected() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock_registry));
    let gate = RealHardwareExecutionGate::with_enabled(true);

    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );

    let permit = gate
        .request_permit(&plan, "op-orig", r"\\.\PhysicalDrive1", true, &hw_provider)
        .unwrap();

    // 1. Operation ID mismatch
    let res_op =
        permit.validate_binding("op-mismatch", permit.plan_id(), permit.physical_device_id());
    assert!(res_op.is_err());
    match res_op.unwrap_err() {
        DriveEraseFailureReason::PermitBindingMismatch(msg) => {
            assert!(msg.contains("Operation ID mismatch"));
        }
        other => panic!("Expected PermitBindingMismatch, got {:?}", other),
    }

    // 2. Plan ID mismatch
    let res_plan = permit.validate_binding(
        permit.operation_id(),
        "plan-mismatch",
        permit.physical_device_id(),
    );
    assert!(res_plan.is_err());
    match res_plan.unwrap_err() {
        DriveEraseFailureReason::PermitBindingMismatch(msg) => {
            assert!(msg.contains("Plan ID mismatch"));
        }
        other => panic!("Expected PermitBindingMismatch, got {:?}", other),
    }

    // 3. Device ID mismatch
    let res_dev = permit.validate_binding(
        permit.operation_id(),
        permit.plan_id(),
        r"\\.\PhysicalDrive2",
    );
    assert!(res_dev.is_err());
    match res_dev.unwrap_err() {
        DriveEraseFailureReason::PermitBindingMismatch(msg) => {
            assert!(msg.contains("Device ID mismatch"));
        }
        other => panic!("Expected PermitBindingMismatch, got {:?}", other),
    }
}

// ============================================================================
// B. SAFETY TESTS
// ============================================================================

#[test]
fn test_safety_system_and_boot_rejected() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock_registry));
    let gate = RealHardwareExecutionGate::with_enabled(true);

    // System device
    let plan_sys = create_sample_plan(
        r"\\.\PhysicalDrive0",
        true,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
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
        DriveEraseFailureReason::SystemOrBootDeviceProtected(_) => {}
        other => panic!("Expected SystemOrBootDeviceProtected, got {:?}", other),
    }

    // Boot device
    let plan_boot = create_sample_plan(
        r"\\.\PhysicalDrive6",
        false,
        true,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
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
        DriveEraseFailureReason::SystemOrBootDeviceProtected(_) => {}
        other => panic!("Expected SystemOrBootDeviceProtected, got {:?}", other),
    }
}

#[test]
fn test_safety_logical_volume_rejected() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock_registry));
    let gate = RealHardwareExecutionGate::with_enabled(true);

    let plan_vol = create_sample_plan(
        r"C:\",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let res = gate.request_permit(&plan_vol, "op-vol", r"C:\", true, &hw_provider);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::LogicalVolumeTargetRejected(_) => {}
        other => panic!("Expected LogicalVolumeTargetRejected, got {:?}", other),
    }
}

#[test]
fn test_safety_missing_device_rejected() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = DefaultDriveHardwareProvider::new(Arc::new(mock_registry));
    let gate = RealHardwareExecutionGate::with_enabled(true);

    let plan_missing = create_sample_plan(
        r"\\.\PhysicalDrive99",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let res = gate.request_permit(
        &plan_missing,
        "op-missing",
        r"\\.\PhysicalDrive99",
        true,
        &hw_provider,
    );
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::DeviceNotFound(_) => {}
        other => panic!("Expected DeviceNotFound, got {:?}", other),
    }
}

#[test]
fn test_safety_write_protected_device_rejected() {
    let snapshot = PhysicalDeviceSnapshot {
        device_id: r"\\.\PhysicalDrive1".to_string(),
        display_name: "Read-only device".to_string(),
        vendor: None,
        model: None,
        serial_number: Some("SERIAL-RO".to_string()),
        media_type: DeviceType::Usb,
        capacity_bytes: 1_000_000_000,
        sector_size: 512,
        physical_sector_size: None,
        bus_type: None,
        is_system: false,
        is_boot: false,
        is_removable: true,
        is_read_only: false, // Originally not read-only
        exists: true,
        classification: DeviceClassification::RemovableDevice,
        partition_count: 1,
        volume_labels: vec![],
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut live = snapshot.clone();
    live.is_read_only = true; // Mutated to read-only
    let res = snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("read-only"));
}

#[test]
fn test_safety_toctou_mutation_rejected() {
    let snapshot = PhysicalDeviceSnapshot {
        device_id: r"\\.\PhysicalDrive1".to_string(),
        display_name: "Mutating Device".to_string(),
        vendor: Some("VendorA".to_string()),
        model: Some("ModelA".to_string()),
        serial_number: Some("SERIAL-AAA".to_string()),
        media_type: DeviceType::Hdd,
        capacity_bytes: 2_000_000_000_000,
        sector_size: 512,
        physical_sector_size: None,
        bus_type: None,
        is_system: false,
        is_boot: false,
        is_removable: false,
        is_read_only: false,
        exists: true,
        classification: DeviceClassification::FixedDataDevice,
        partition_count: 1,
        volume_labels: vec![],
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // 1. Capacity changed
    let mut live_cap = snapshot.clone();
    live_cap.capacity_bytes = 1_000_000_000_000;
    assert!(snapshot.detect_mutation(&live_cap).is_err());

    // 2. Serial changed
    let mut live_ser = snapshot.clone();
    live_ser.serial_number = Some("SERIAL-MUTATED".to_string());
    assert!(snapshot.detect_mutation(&live_ser).is_err());

    // 3. Partition count changed
    let mut live_part = snapshot.clone();
    live_part.partition_count = 3;
    assert!(snapshot.detect_mutation(&live_part).is_err());

    // 4. Device vanished
    let mut live_vanished = snapshot.clone();
    live_vanished.exists = false;
    assert!(snapshot.detect_mutation(&live_vanished).is_err());
}

// ============================================================================
// C. EXECUTION TESTS
// ============================================================================

#[test]
fn test_execution_simulation_never_reaches_real_executor() {
    let sanitizer = RealHardwareSanitizer::new();
    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
        DriveSanitizationMethod::Nist80088ClearZero,
    );

    let res = sanitizer.simulate_erasure(&plan, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::RealHardwareExecutionDisabled(msg) => {
            assert!(msg.contains("cannot perform simulation"));
        }
        other => panic!("Expected RealHardwareExecutionDisabled, got {:?}", other),
    }
}

#[test]
fn test_execution_real_mode_never_reaches_simulation_backend() {
    let sim = SimulatedHddOverwrite;
    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );

    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::new(mock_registry)));
    let permit = gate
        .request_permit(
            &plan,
            "op-sim-reject",
            r"\\.\PhysicalDrive1",
            true,
            hw_provider.as_ref(),
        )
        .unwrap();

    let res = sim.execute(&permit, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::PreExecutionCheckFailed(msg) => {
            assert!(msg.contains("Simulation backend only supports permits"));
        }
        other => panic!("Expected PreExecutionCheckFailed, got {:?}", other),
    }
}

#[test]
fn test_execution_exclusive_access_failure_stops_execution() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    mock_executor.fail_lock.store(true, Ordering::SeqCst);

    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::new(
        mock_registry.clone(),
    )));

    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let permit = gate
        .request_permit(
            &plan,
            "op-lock-fail",
            r"\\.\PhysicalDrive1",
            true,
            hw_provider.as_ref(),
        )
        .unwrap();

    let sanitizer = RealHardwareSanitizer::with_components(
        mock_executor,
        Arc::new(DeviceLockRegistry::new()),
        Some(hw_provider),
    );

    let res = sanitizer.execute_and_verify(&permit, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::ExclusiveAccessFailed(msg) => {
            assert!(msg.contains("exclusive lock"));
        }
        other => panic!("Expected ExclusiveAccessFailed, got {:?}", other),
    }

    // Permit must NOT be consumed if exclusive access acquisition fails!
    assert!(!permit.is_consumed());
}

#[test]
fn test_execution_device_disconnect_stops_execution() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    mock_executor.fail_disconnect.store(true, Ordering::SeqCst);

    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::new(
        mock_registry.clone(),
    )));

    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let permit = gate
        .request_permit(
            &plan,
            "op-disc-fail",
            r"\\.\PhysicalDrive1",
            true,
            hw_provider.as_ref(),
        )
        .unwrap();

    let sanitizer = RealHardwareSanitizer::with_components(
        mock_executor,
        Arc::new(DeviceLockRegistry::new()),
        Some(hw_provider),
    );

    let res = sanitizer.execute_and_verify(&permit, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::DeviceDisconnected(_) => {}
        other => panic!("Expected DeviceDisconnected, got {:?}", other),
    }
}

#[test]
fn test_execution_platform_io_error_produces_structured_failure() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    mock_executor.fail_io.store(true, Ordering::SeqCst);

    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let hw_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::new(
        mock_registry.clone(),
    )));

    let plan = create_sample_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
        DriveSanitizationMethod::Nist80088ClearZero,
    );
    let gate = RealHardwareExecutionGate::with_enabled(true);
    let permit = gate
        .request_permit(
            &plan,
            "op-io-fail",
            r"\\.\PhysicalDrive1",
            true,
            hw_provider.as_ref(),
        )
        .unwrap();

    let sanitizer = RealHardwareSanitizer::with_components(
        mock_executor,
        Arc::new(DeviceLockRegistry::new()),
        Some(hw_provider),
    );

    let res = sanitizer.execute_and_verify(&permit, None, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::IoError(msg) => {
            assert!(msg.contains("Win32 write error"));
        }
        other => panic!("Expected IoError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_execution_post_sanitization_verification_failure() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    mock_executor
        .fail_verification
        .store(true, Ordering::SeqCst);

    let (service, _registry, _audit) = setup_test_service(mock_executor);

    // 1. Plan erasure with RealHardware execution mode
    let plan = service
        .plan_drive_erasure(
            locardx_drive_eraser::DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: Some("Nist80088ClearZero".to_string()),
                execution_mode: Some(ExecutionMode::RealHardware),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    let res = service
        .execute_drive_erasure_with_gate(
            &plan.plan_id,
            "conf-1",
            "op-verif-fail",
            r"\\.\PhysicalDrive1",
            true,
            "session-token-1",
            ExecutionMode::RealHardware,
            None,
            None,
        )
        .await;

    assert!(res.is_ok());
    let erase_res = res.unwrap();
    assert_eq!(erase_res.status, DriveEraseStatus::VerificationFailed);
    assert_eq!(
        erase_res.verification.outcome,
        VerificationOutcome::VerificationFailed
    );
    assert!(erase_res.failure_reason.is_some());
}

#[tokio::test]
async fn test_execution_ambiguous_verification_produces_unable_to_verify() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    mock_executor
        .ambiguous_verification
        .store(true, Ordering::SeqCst);

    let (service, _registry, _audit) = setup_test_service(mock_executor);

    let plan = service
        .plan_drive_erasure(
            locardx_drive_eraser::DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: Some("Nist80088ClearZero".to_string()),
                execution_mode: Some(ExecutionMode::RealHardware),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    let res = service
        .execute_drive_erasure_with_gate(
            &plan.plan_id,
            "conf-2",
            "op-verif-ambig",
            r"\\.\PhysicalDrive1",
            true,
            "session-token-2",
            ExecutionMode::RealHardware,
            None,
            None,
        )
        .await;

    assert!(res.is_ok());
    let erase_res = res.unwrap();
    assert_eq!(erase_res.status, DriveEraseStatus::UnableToVerify);
    assert_eq!(
        erase_res.verification.outcome,
        VerificationOutcome::UnableToVerify
    );
}

// ============================================================================
// D. STATE MACHINE TESTS
// ============================================================================

#[test]
fn test_state_machine_strict_progression() {
    // Valid forward linear progression
    assert!(DriveExecutionState::Planned.can_transition_to(DriveExecutionState::Authorized));
    assert!(
        DriveExecutionState::Authorized.can_transition_to(DriveExecutionState::PreExecutionCheck)
    );
    assert!(DriveExecutionState::PreExecutionCheck
        .can_transition_to(DriveExecutionState::ExecutionReady));
    assert!(DriveExecutionState::ExecutionReady.can_transition_to(DriveExecutionState::Executing));
    assert!(
        DriveExecutionState::Executing.can_transition_to(DriveExecutionState::ExecutionComplete)
    );
    assert!(
        DriveExecutionState::ExecutionComplete.can_transition_to(DriveExecutionState::Verifying)
    );
    assert!(DriveExecutionState::Verifying.can_transition_to(DriveExecutionState::Completed));
    assert!(
        DriveExecutionState::Verifying.can_transition_to(DriveExecutionState::VerificationFailed)
    );
    assert!(DriveExecutionState::Verifying.can_transition_to(DriveExecutionState::UnableToVerify));

    // Cancellation / Failure allowed from active stages
    assert!(DriveExecutionState::Authorized.can_transition_to(DriveExecutionState::Cancelled));
    assert!(DriveExecutionState::Executing.can_transition_to(DriveExecutionState::Failed));

    // Terminal states CANNOT transition to anything
    assert!(!DriveExecutionState::Completed.can_transition_to(DriveExecutionState::Executing));
    assert!(!DriveExecutionState::Completed.can_transition_to(DriveExecutionState::Planned));
    assert!(!DriveExecutionState::Failed.can_transition_to(DriveExecutionState::Executing));
    assert!(!DriveExecutionState::Cancelled.can_transition_to(DriveExecutionState::Completed));
    assert!(
        !DriveExecutionState::VerificationFailed.can_transition_to(DriveExecutionState::Completed)
    );
    assert!(!DriveExecutionState::UnableToVerify.can_transition_to(DriveExecutionState::Completed));

    // Skipping stages is strictly forbidden
    assert!(!DriveExecutionState::Planned.can_transition_to(DriveExecutionState::Executing));
    assert!(!DriveExecutionState::Authorized.can_transition_to(DriveExecutionState::Completed));
    assert!(!DriveExecutionState::ExecutionReady.can_transition_to(DriveExecutionState::Completed));
}

// ============================================================================
// E. CONCURRENCY TESTS
// ============================================================================

#[test]
fn test_concurrency_same_device_locked() {
    let registry = Arc::new(DeviceLockRegistry::new());
    let target = r"\\.\PhysicalDrive1";

    let lock1 = registry.acquire_lock(target);
    assert!(lock1.is_ok());

    // Concurrent second lock on SAME device fails closed
    let lock2 = registry.acquire_lock(target);
    assert!(lock2.is_err());
    match lock2.unwrap_err() {
        DriveEraseFailureReason::ExclusiveAccessFailed(msg) => {
            assert!(msg.contains("is already being sanitized"));
        }
        other => panic!("Expected ExclusiveAccessFailed, got {:?}", other),
    }

    // Dropping lock1 releases target
    drop(lock1);
    let lock3 = registry.acquire_lock(target);
    assert!(lock3.is_ok());
}

#[test]
fn test_concurrency_distinct_devices_allowed() {
    let registry = Arc::new(DeviceLockRegistry::new());

    let lock1 = registry.acquire_lock(r"\\.\PhysicalDrive1");
    let lock2 = registry.acquire_lock(r"\\.\PhysicalDrive2");

    assert!(lock1.is_ok());
    assert!(lock2.is_ok());
}

// ============================================================================
// F. AUDIT TRAIL TESTS
// ============================================================================

#[tokio::test]
async fn test_audit_trail_recorded_without_sensitive_sector_leakage() {
    let mock_executor = Arc::new(MockDriveHardwareExecutor::default());
    let (service, _registry, audit) = setup_test_service(mock_executor);

    let plan = service
        .plan_drive_erasure(
            locardx_drive_eraser::DriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: Some("Nist80088ClearZero".to_string()),
                execution_mode: Some(ExecutionMode::RealHardware),
                session_token: None,
            },
            Some("investigator-1".to_string()),
        )
        .await
        .unwrap();

    let res = service
        .execute_drive_erasure_with_gate(
            &plan.plan_id,
            "conf-audit",
            "op-audit-1",
            r"\\.\PhysicalDrive1",
            true,
            "session-token-3",
            ExecutionMode::RealHardware,
            None,
            None,
        )
        .await;

    assert!(res.is_ok());
    let erase_res = res.unwrap();
    assert_eq!(erase_res.status, DriveEraseStatus::Completed);
    assert!(erase_res.audit_references.len() >= 3);

    // Check emitted audit events in AuditService
    let events = audit.list_events(50).unwrap();
    let action_types: Vec<String> = events.iter().map(|e| e.event_type.clone()).collect();

    assert!(action_types.contains(&"DRIVE_ERASURE_EXECUTION_STARTED".to_string()));
    assert!(action_types.contains(&"DRIVE_ERASURE_EXECUTION_COMPLETED".to_string()));
    assert!(action_types.contains(&"DRIVE_ERASURE_VERIFICATION_COMPLETED".to_string()));

    // Verify no raw sector buffer leaks in audit payloads
    for ev in events {
        assert!(!ev.details.contains("0x00000000000000000000000000000000"));
        assert!(!ev.details.contains("sector_buffer"));
    }
}
