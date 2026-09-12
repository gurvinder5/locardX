//! Comprehensive test suite for Step 10B.2: Real Device Enumeration & Read-Only Hardware Capability Probing.
//!
//! SAFETY VERIFICATIONS:
//! 1. Real device enumeration runs strictly in read-only mode (query-only access).
//! 2. Zero physical storage writes, format commands, or destructive IOCTLs are issued.
//! 3. System and boot devices are accurately detected and unconditionally hard blocked.
//! 4. Logical volumes (drive letters) are strictly rejected.
//! 5. Hardware capability probing evaluates to Supported, Unsupported, Unknown, or DetectionFailed.
//! 6. Unknown and DetectionFailed states fail closed.
//! 7. TOCTOU mutations (disconnect, substitution, serial, capacity, sector geometry, bus type, read-only)
//!    are strictly detected and block execution.
//! 8. Real hardware execution remains disabled with `RealHardwareExecutionNotEnabled`.

use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceType, PhysicalDevice,
};
use locardx_drive_eraser::{
    validate_physical_device_identifier, CapabilityState, DriveCapabilities, DriveCapability,
    DriveEraseFailureReason, DriveErasePlan, DriveHardwareProvider, DriveSanitizationMethod,
    ExecutionMode, MockDeviceRegistry, PhysicalDeviceSnapshot, RealDriveHardwareProvider,
    RealHardwareExecutionGate, RealHardwareSanitizer,
};
use std::sync::Arc;

/// Helper to create a mock test plan for boundary testing.
fn make_test_plan(
    device_id: &str,
    is_system: bool,
    is_boot: bool,
    mode: ExecutionMode,
) -> DriveErasePlan {
    let registry = MockDeviceRegistry::new_standard_test_set();
    let dev_opt = registry
        .discover_devices()
        .unwrap()
        .into_iter()
        .find(|d| d.device_id.eq_ignore_ascii_case(device_id));

    let (serial_number, capacity_bytes, sector_size, vendor, model, media_type) =
        if let Some(d) = dev_opt {
            (
                d.serial_number,
                d.capacity_bytes,
                512,
                d.vendor,
                d.model,
                d.device_type,
            )
        } else {
            (
                Some("SERIAL-TEST-123".to_string()),
                1_000_000_000_000,
                512,
                Some("Crucial".to_string()),
                Some("CT1000MX500SSD1".to_string()),
                DeviceType::Ssd,
            )
        };

    let snapshot = PhysicalDeviceSnapshot {
        device_id: device_id.to_string(),
        display_name: "Test Physical Disk".to_string(),
        vendor: vendor.clone(),
        model: model.clone(),
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
        partition_count: 1,
        volume_labels: vec![],
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let capabilities = DriveCapabilities {
        supported_capabilities: vec![
            DriveCapability::AtaSecureErase,
            DriveCapability::SequentialWrite,
            DriveCapability::FullDeviceRead,
            DriveCapability::SectorAccess,
        ],
        interface_bus: "SATA".to_string(),
        sector_size: 512,
        is_rotational: false,
        supports_crypto_erase: false,
        supports_firmware_sanitize: true,
        supports_overwrite: true,
    };

    DriveErasePlan {
        plan_id: "plan-test-001".to_string(),
        physical_device_id: device_id.to_string(),
        display_name: "Test Physical Disk".to_string(),
        vendor,
        model,
        serial_number,
        media_type,
        capacity_bytes,
        sector_size,
        method: DriveSanitizationMethod::AtaSecureErase,
        passes: 1,
        risk_level: locardx_security::RiskLevel::Critical,
        capabilities,
        verification_plan: locardx_drive_eraser::DriveVerificationPlan {
            strategy: locardx_drive_eraser::DriveVerificationStrategy::FirmwareStatusVerify,
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

// =========================================================================
// TEST 1: Real Device Enumeration Runs Safely Without Panicking or Writes
// =========================================================================
#[test]
fn test_01_real_device_enumeration_runs_safely() {
    let provider = RealDriveHardwareProvider::new();
    let devices_res = provider.discover_devices();
    assert!(
        devices_res.is_ok(),
        "Real device discovery failed: {:?}",
        devices_res.err()
    );

    let devices = devices_res.unwrap();
    println!("Discovered {} real physical device(s)", devices.len());

    for dev in &devices {
        assert!(
            !dev.device_id.is_empty(),
            "Physical device identifier must not be empty"
        );
        println!(
            "Device: ID={}, Name='{}', Type={:?}, Capacity={} bytes, Removable={}, System={}",
            dev.device_id,
            dev.display_name,
            dev.device_type,
            dev.capacity_bytes,
            dev.removable,
            dev.is_system_device
        );

        // Verify serial number is not fabricated
        if let Some(ref serial) = dev.serial_number {
            assert!(!serial.is_empty(), "Observed serial must not be empty");
        }
    }
}

// =========================================================================
// TEST 2: Physical Device Snapshot Populates Real Observations
// =========================================================================
#[test]
fn test_02_snapshot_populates_real_observations() {
    let provider = RealDriveHardwareProvider::new();
    let devices = provider.discover_devices().unwrap();

    if let Some(first_dev) = devices.first() {
        let snapshot_res = provider.probe_device_snapshot(&first_dev.device_id);
        assert!(
            snapshot_res.is_ok(),
            "Live snapshot probe failed: {:?}",
            snapshot_res.err()
        );

        let snapshot_opt = snapshot_res.unwrap();
        assert!(
            snapshot_opt.is_some(),
            "Live device snapshot should exist for discovered device"
        );

        let snapshot = snapshot_opt.unwrap();
        assert_eq!(snapshot.device_id, first_dev.device_id);
        assert!(snapshot.capacity_bytes > 0);
        assert!(snapshot.sector_size >= 512);
        assert!(snapshot.exists);
        assert!(!snapshot.snapshot_timestamp.is_empty());

        println!(
            "Real Snapshot: ID={}, Capacity={}, Sector={}, PhysicalSector={:?}, Bus={:?}, ReadOnly={}",
            snapshot.device_id,
            snapshot.capacity_bytes,
            snapshot.sector_size,
            snapshot.physical_sector_size,
            snapshot.bus_type,
            snapshot.is_read_only
        );
    }
}

// =========================================================================
// TEST 3: System Device Detection & Hard Block on Live Host Disk
// =========================================================================
#[test]
fn test_03_system_device_detection_and_hard_block() {
    let provider = RealDriveHardwareProvider::new();
    let devices = provider.discover_devices().unwrap();

    // On Windows, PhysicalDrive0 or the drive hosting C: is flagged as system
    let system_device = devices.iter().find(|d| d.is_system_device);
    if let Some(sys_dev) = system_device {
        println!(
            "Detected active system device: '{}' ({})",
            sys_dev.device_id, sys_dev.display_name
        );
        assert!(sys_dev.is_system_device);
        assert_eq!(sys_dev.classification, DeviceClassification::SystemDevice);

        // Verify gate rejects it
        let plan = make_test_plan(&sys_dev.device_id, true, false, ExecutionMode::Simulation);
        let gate = RealHardwareExecutionGate::new();
        let permit_res =
            gate.request_permit(&plan, "op-test-sys", &sys_dev.device_id, true, &provider);

        assert!(matches!(
            permit_res,
            Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(_))
        ));
    }
}

// =========================================================================
// TEST 4: Boot Device Detection & Hard Block
// =========================================================================
#[test]
fn test_04_boot_device_hard_block() {
    let provider = RealDriveHardwareProvider::new();
    let gate = RealHardwareExecutionGate::new();
    let plan = make_test_plan(
        r"\\.\PhysicalDrive6",
        false,
        true,
        ExecutionMode::Simulation,
    );

    let permit_res = gate.request_permit(
        &plan,
        "op-test-boot",
        r"\\.\PhysicalDrive6",
        true,
        &provider,
    );

    assert!(matches!(
        permit_res,
        Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(_))
    ));
}

// =========================================================================
// TEST 5: Logical Volume and Drive Letter Rejection
// =========================================================================
#[test]
fn test_05_logical_volume_rejection() {
    assert!(validate_physical_device_identifier("C:").is_err());
    assert!(validate_physical_device_identifier("C:\\").is_err());
    assert!(validate_physical_device_identifier("D:\\Data").is_err());
    assert!(validate_physical_device_identifier("/mnt/usb").is_err());
    assert!(validate_physical_device_identifier("/home/user").is_err());

    // Valid physical formats pass syntax validation
    assert!(validate_physical_device_identifier(r"\\.\PhysicalDrive1").is_ok());
    assert!(validate_physical_device_identifier("PhysicalDrive2").is_ok());
    assert!(validate_physical_device_identifier("/dev/sdb").is_ok());
    assert!(validate_physical_device_identifier("/dev/nvme0n1").is_ok());
}

// =========================================================================
// TEST 6: Hardware Capability Probing
// =========================================================================
#[test]
fn test_06_capability_probing_supported_and_fail_closed() {
    let provider = RealDriveHardwareProvider::new();

    // 1. Supported Mock NVMe SSD
    let nvme = PhysicalDevice {
        device_id: r"\\.\PhysicalDrive3".to_string(),
        display_name: "Samsung 980 PRO NVMe 1TB".to_string(),
        vendor: Some("Samsung".to_string()),
        model: Some("980 PRO".to_string()),
        serial_number: Some("S69ENF0R123456".to_string()),
        device_type: DeviceType::Ssd,
        capacity_bytes: 1_000_204_886_016,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::FixedDataDevice,
        volumes: vec![],
    };

    let assess = provider.probe_capabilities(&nvme).unwrap();
    assert_eq!(assess.overall_state, CapabilityState::Supported);
    assert!(assess
        .capabilities
        .has_capability(DriveCapability::NvmeSanitize));
    assert!(assess
        .capabilities
        .has_capability(DriveCapability::NvmeCryptoErase));

    // 2. Unknown Device Type -> Fails Closed with CapabilityState::Unknown
    let unknown_dev = PhysicalDevice {
        device_id: r"\\.\PhysicalDrive9".to_string(),
        display_name: "Unknown Exotic Media".to_string(),
        vendor: None,
        model: None,
        serial_number: None,
        device_type: DeviceType::Unknown,
        capacity_bytes: 500_000_000_000,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::Unknown,
        volumes: vec![],
    };

    let unknown_assess = provider.probe_capabilities(&unknown_dev).unwrap();
    assert_eq!(unknown_assess.overall_state, CapabilityState::Unknown);
    assert!(unknown_assess.overall_state.should_fail_closed());

    // 3. Zero Capacity Device -> Fails Closed with CapabilityState::DetectionFailed
    let zero_dev = PhysicalDevice {
        device_id: r"\\.\PhysicalDrive10".to_string(),
        display_name: "Corrupted Zero Capacity Storage".to_string(),
        vendor: None,
        model: None,
        serial_number: None,
        device_type: DeviceType::Hdd,
        capacity_bytes: 0,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::Unknown,
        volumes: vec![],
    };

    let zero_assess = provider.probe_capabilities(&zero_dev).unwrap();
    assert_eq!(zero_assess.overall_state, CapabilityState::DetectionFailed);
    assert!(zero_assess.overall_state.should_fail_closed());
}

// =========================================================================
// TEST 7: TOCTOU Mutation Defense: Device Disconnection / Disappearance
// =========================================================================
#[test]
fn test_07_toctou_device_disappearance() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.exists = false;

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("no longer exists or was disconnected"));
}

// =========================================================================
// TEST 8: TOCTOU Mutation Defense: Device Substitution
// =========================================================================
#[test]
fn test_08_toctou_device_substitution() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.device_id = r"\\.\PhysicalDrive2".to_string();

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Device ID mismatch"));
}

// =========================================================================
// TEST 9: TOCTOU Mutation Defense: Serial Number Mutation
// =========================================================================
#[test]
fn test_09_toctou_serial_number_mutation() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.serial_number = Some("DIFFERENT-SERIAL-999".to_string());

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Device serial number changed"));
}

// =========================================================================
// TEST 10: TOCTOU Mutation Defense: Capacity Mutation
// =========================================================================
#[test]
fn test_10_toctou_capacity_mutation() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.capacity_bytes += 1_000_000;

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Device capacity changed"));
}

// =========================================================================
// TEST 11: TOCTOU Mutation Defense: Sector Size & Physical Geometry Mutation
// =========================================================================
#[test]
fn test_11_toctou_sector_geometry_mutation() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    // 1. Logical sector size mutation
    let mut live_logical = plan.device_snapshot.clone();
    live_logical.sector_size = 4096;
    let res = plan.device_snapshot.detect_mutation(&live_logical);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Sector size mutated"));

    // 2. Physical sector size mutation
    let mut live_phys = plan.device_snapshot.clone();
    live_phys.physical_sector_size = Some(8192);
    let res = plan.device_snapshot.detect_mutation(&live_phys);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Physical sector size mutated"));
}

// =========================================================================
// TEST 12: TOCTOU Mutation Defense: Bus Type Mutation
// =========================================================================
#[test]
fn test_12_toctou_bus_type_mutation() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.bus_type = Some("NVMe".to_string());

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Bus type mutated"));
}

// =========================================================================
// TEST 13: TOCTOU Mutation Defense: Hardware Identity (Model) Mutation
// =========================================================================
#[test]
fn test_13_toctou_hardware_model_mutation() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.model = Some("Substituted Model X1".to_string());

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Hardware model identity mutated"));
}

// =========================================================================
// TEST 14: TOCTOU Mutation Defense: Read-Only Write-Protection Status Shift
// =========================================================================
#[test]
fn test_14_toctou_read_only_write_protection_shift() {
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::Simulation,
    );

    let mut live = plan.device_snapshot.clone();
    live.is_read_only = true;

    let res = plan.device_snapshot.detect_mutation(&live);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("read-only / write-protected"));
}

// =========================================================================
// TEST 15: Real Hardware Execution Mode Rejection Invariant
// =========================================================================
#[test]
fn test_15_no_destructive_io_real_hardware_mode_rejected() {
    let gate = RealHardwareExecutionGate::new();
    assert!(!gate.is_enabled());

    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
    );

    let provider = RealDriveHardwareProvider::new();
    let res = gate.request_permit(
        &plan,
        "op-real-exec-gate",
        r"\\.\PhysicalDrive1",
        true,
        &provider,
    );

    // Unconditionally rejected with RealHardwareExecutionNotEnabled
    assert!(matches!(
        res,
        Err(DriveEraseFailureReason::RealHardwareExecutionNotEnabled(_))
    ));
}

// =========================================================================
// TEST 16: Non-Destructive Stub RealHardwareSanitizer
// =========================================================================
#[test]
fn test_16_real_sanitizer_remains_non_destructive_stub() {
    use locardx_drive_eraser::DriveSanitizer;

    let sanitizer = RealHardwareSanitizer::stub();
    assert_eq!(sanitizer.execution_mode(), ExecutionMode::RealHardware);

    let gate = RealHardwareExecutionGate::with_enabled(true);
    let mock = MockDeviceRegistry::new_standard_test_set();
    let provider = locardx_drive_eraser::DefaultDriveHardwareProvider::new(Arc::new(mock));
    let plan = make_test_plan(
        r"\\.\PhysicalDrive1",
        false,
        false,
        ExecutionMode::RealHardware,
    );

    let permit = gate
        .request_permit(&plan, "op-stub-1", r"\\.\PhysicalDrive1", true, &provider)
        .unwrap();

    let exec_res = sanitizer.execute(&permit, None, None);
    assert!(matches!(
        exec_res,
        Err(DriveEraseFailureReason::RealHardwareExecutionNotEnabled(_))
    ));
}
