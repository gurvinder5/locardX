use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceType, Filesystem, FilesystemType,
    LogicalVolume, PhysicalDevice,
};
use locardx_drive_eraser::hardware::DeviceLockRegistry;
use locardx_drive_eraser::models::{
    DriveEraseFailureReason, DriveSanitizationMethod, ExecutionMode, HardwareExecutionPermit,
    IdentityRevalidationOutcome, PhysicalDeviceSnapshot,
};
use locardx_drive_eraser::platform::privileges::verify_administrative_privileges;
use locardx_drive_eraser::target::{
    inspect_physical_device, is_safe_external_erase_target, validate_physical_device_identifier,
};
use std::sync::Arc;

struct MockHardwareRegistry {
    devices: Vec<PhysicalDevice>,
}

impl MockHardwareRegistry {
    fn new(devices: Vec<PhysicalDevice>) -> Self {
        Self { devices }
    }
}

impl DeviceDiscoveryProvider for MockHardwareRegistry {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, locardx_common::LocardError> {
        Ok(self.devices.clone())
    }
}

fn create_test_system_drive() -> PhysicalDevice {
    PhysicalDevice {
        device_id: r"\\.\PhysicalDrive0".to_string(),
        display_name: "NVMe SKhynix 512GB (Internal OS Drive)".to_string(),
        vendor: Some("SKhynix".to_string()),
        model: Some("HFM512GD3JX013N".to_string()),
        serial_number: Some("HYNIXOS998811".to_string()),
        device_type: DeviceType::Ssd,
        capacity_bytes: 512_110_190_592,
        removable: false,
        read_only: false,
        is_system_device: true,
        classification: DeviceClassification::SystemDevice,
        volumes: vec![
            LogicalVolume {
                volume_id: "vol-efi".to_string(),
                mount_point: None,
                label: Some("SYSTEM".to_string()),
                filesystem: Filesystem {
                    fs_type: FilesystemType::Fat32,
                    label: Some("SYSTEM".to_string()),
                    read_only: false,
                },
                capacity_bytes: 100_000_000,
                free_bytes: 50_000_000,
                is_system_volume: true,
                is_boot_volume: true,
                read_only: false,
            },
            LogicalVolume {
                volume_id: "vol-c".to_string(),
                mount_point: Some("C:\\".to_string()),
                label: Some("Windows".to_string()),
                filesystem: Filesystem {
                    fs_type: FilesystemType::Ntfs,
                    label: Some("Windows".to_string()),
                    read_only: false,
                },
                capacity_bytes: 500_000_000_000,
                free_bytes: 200_000_000_000,
                is_system_volume: true,
                is_boot_volume: false,
                read_only: false,
            },
        ],
    }
}

fn create_test_usb_flash_drive() -> PhysicalDevice {
    PhysicalDevice {
        device_id: r"\\.\PhysicalDrive1".to_string(),
        display_name: "SanDisk Ultra 3.0 USB Flash Drive".to_string(),
        vendor: Some("SanDisk".to_string()),
        model: Some("Ultra 3.0".to_string()),
        serial_number: Some("000000000502".to_string()),
        device_type: DeviceType::Usb,
        capacity_bytes: 32_000_000_000,
        removable: true,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::RemovableDevice,
        volumes: vec![LogicalVolume {
            volume_id: "vol-e".to_string(),
            mount_point: Some("E:\\".to_string()),
            label: Some("USB_DATA".to_string()),
            filesystem: Filesystem {
                fs_type: FilesystemType::ExFat,
                label: Some("USB_DATA".to_string()),
                read_only: false,
            },
            capacity_bytes: 32_000_000_000,
            free_bytes: 20_000_000_000,
            is_system_volume: false,
            is_boot_volume: false,
            read_only: false,
        }],
    }
}

fn create_test_sd_card() -> PhysicalDevice {
    PhysicalDevice {
        device_id: r"\\.\PhysicalDrive2".to_string(),
        display_name: "MassStorageClass 2402 SD Card Reader".to_string(),
        vendor: Some("MassStorageClass".to_string()),
        model: Some("2402".to_string()),
        serial_number: Some("000000000502".to_string()),
        device_type: DeviceType::MemoryCard,
        capacity_bytes: 7_985_823_744,
        removable: true,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::RemovableDevice,
        volumes: vec![LogicalVolume {
            volume_id: "vol-f".to_string(),
            mount_point: Some("F:\\".to_string()),
            label: Some("SD_MEDIA".to_string()),
            filesystem: Filesystem {
                fs_type: FilesystemType::Fat32,
                label: Some("SD_MEDIA".to_string()),
                read_only: false,
            },
            capacity_bytes: 7_985_823_744,
            free_bytes: 7_000_000_000,
            is_system_volume: false,
            is_boot_volume: false,
            read_only: false,
        }],
    }
}

fn create_test_internal_fixed_drive() -> PhysicalDevice {
    PhysicalDevice {
        device_id: r"\\.\PhysicalDrive3".to_string(),
        display_name: "Seagate BarraCuda 2TB Internal HDD".to_string(),
        vendor: Some("Seagate".to_string()),
        model: Some("ST2000DM008".to_string()),
        serial_number: Some("W1E89XYZ".to_string()),
        device_type: DeviceType::Hdd,
        capacity_bytes: 2_000_398_934_016,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::FixedDataDevice,
        volumes: vec![LogicalVolume {
            volume_id: "vol-d".to_string(),
            mount_point: Some("D:\\".to_string()),
            label: Some("Archive".to_string()),
            filesystem: Filesystem {
                fs_type: FilesystemType::Ntfs,
                label: Some("Archive".to_string()),
                read_only: false,
            },
            capacity_bytes: 2_000_000_000_000,
            free_bytes: 1_000_000_000_000,
            is_system_volume: false,
            is_boot_volume: false,
            read_only: false,
        }],
    }
}

// -------------------------------------------------------------------------
// TEST 1: Internal system drive PhysicalDrive0 is rejected for erasure
// -------------------------------------------------------------------------
#[test]
fn test_1_internal_system_drive_physicaldrive0_rejected() {
    let sys_dev = create_test_system_drive();
    let auth_res = is_safe_external_erase_target(&sys_dev);
    assert!(auth_res.is_err(), "PhysicalDrive0 must be rejected");
    match auth_res.unwrap_err() {
        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
            assert!(msg.contains("active System Device") || msg.contains("PhysicalDrive0"));
        }
        other => panic!("Expected SystemOrBootDeviceProtected, got: {:?}", other),
    }

    let registry: Arc<dyn DeviceDiscoveryProvider> =
        Arc::new(MockHardwareRegistry::new(vec![sys_dev]));
    let inspect_res = inspect_physical_device(r"\\.\PhysicalDrive0", &registry, 512);
    assert!(inspect_res.is_err(), "inspect_physical_device must reject PhysicalDrive0");
}

// -------------------------------------------------------------------------
// TEST 2: Windows boot drive hosting C:\ is rejected for erasure
// -------------------------------------------------------------------------
#[test]
fn test_2_windows_boot_drive_c_rejected() {
    let mut dev = create_test_usb_flash_drive();
    // Simulate malicious/erroneous metadata where USB drive claims to host C:\
    dev.volumes.push(LogicalVolume {
        volume_id: "vol-c-fake".to_string(),
        mount_point: Some("C:\\".to_string()),
        label: Some("FakeWin".to_string()),
        filesystem: Filesystem {
            fs_type: FilesystemType::Ntfs,
            label: Some("FakeWin".to_string()),
            read_only: false,
        },
        capacity_bytes: 10_000_000_000,
        free_bytes: 5_000_000_000,
        is_system_volume: false,
        is_boot_volume: false,
        read_only: false,
    });

    let res = is_safe_external_erase_target(&dev);
    assert!(res.is_err(), "Device hosting C:\\ must be hard-blocked");
    match res.unwrap_err() {
        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
            assert!(msg.contains("C:"), "Error message must indicate C: protection: {}", msg);
        }
        other => panic!("Expected SystemOrBootDeviceProtected, got: {:?}", other),
    }
}

// -------------------------------------------------------------------------
// TEST 3: Logical volume C:\ is rejected with LogicalVolumeTargetRejected
// -------------------------------------------------------------------------
#[test]
fn test_3_logical_volume_c_rejected() {
    let targets = ["C:", "C:\\", "c:", "c:\\"];
    for target in targets {
        let res = validate_physical_device_identifier(target);
        assert!(res.is_err(), "Target '{}' must be rejected", target);
        match res.unwrap_err() {
            DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
                assert!(msg.contains("logical volume"), "Message must explain logical volume: {}", msg);
            }
            other => panic!("Expected LogicalVolumeTargetRejected for '{}', got: {:?}", target, other),
        }
    }
}

// -------------------------------------------------------------------------
// TEST 4: Logical volume D:\ is rejected with LogicalVolumeTargetRejected
// -------------------------------------------------------------------------
#[test]
fn test_4_logical_volume_d_rejected() {
    let targets = ["D:", "D:\\", "d:", "d:\\", "E:\\"];
    for target in targets {
        let res = validate_physical_device_identifier(target);
        assert!(res.is_err(), "Target '{}' must be rejected", target);
        match res.unwrap_err() {
            DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
                assert!(msg.contains("logical volume"), "Message must explain logical volume: {}", msg);
            }
            other => panic!("Expected LogicalVolumeTargetRejected for '{}', got: {:?}", target, other),
        }
    }
}

// -------------------------------------------------------------------------
// TEST 5: External USB drive is authorized for erasure
// -------------------------------------------------------------------------
#[test]
fn test_5_external_usb_drive_authorized() {
    let usb_dev = create_test_usb_flash_drive();
    let res = is_safe_external_erase_target(&usb_dev);
    assert!(res.is_ok(), "External USB flash drive must be authorized for erasure: {:?}", res);

    let registry: Arc<dyn DeviceDiscoveryProvider> =
        Arc::new(MockHardwareRegistry::new(vec![usb_dev.clone()]));
    let inspect_res = inspect_physical_device(r"\\.\PhysicalDrive1", &registry, 512);
    assert!(inspect_res.is_ok(), "inspect_physical_device must succeed for external USB: {:?}", inspect_res);
}

// -------------------------------------------------------------------------
// TEST 6: External SD card is authorized for erasure
// -------------------------------------------------------------------------
#[test]
fn test_6_external_sd_card_authorized() {
    let sd_dev = create_test_sd_card();
    let res = is_safe_external_erase_target(&sd_dev);
    assert!(res.is_ok(), "External SD card must be authorized for erasure: {:?}", res);

    let registry: Arc<dyn DeviceDiscoveryProvider> =
        Arc::new(MockHardwareRegistry::new(vec![sd_dev.clone()]));
    let inspect_res = inspect_physical_device(r"\\.\PhysicalDrive2", &registry, 512);
    assert!(inspect_res.is_ok(), "inspect_physical_device must succeed for external SD card: {:?}", inspect_res);
}

// -------------------------------------------------------------------------
// TEST 7: External drive with identical strong identity but missing serial passes with VerifiedWithDiscrepancy
// -------------------------------------------------------------------------
#[test]
fn test_7_external_drive_missing_serial_verified_with_discrepancy() {
    let planned_dev = create_test_sd_card();
    let planned_snap = PhysicalDeviceSnapshot::from_device(&planned_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let mut live_dev = planned_dev.clone();
    live_dev.serial_number = None; // Bridge omitted serial number on live re-probe
    let live_snap = PhysicalDeviceSnapshot::from_device(&live_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let outcome = planned_snap.revalidate_identity(&live_snap);
    assert!(outcome.is_verified(), "Must be verified despite missing serial: {:?}", outcome);
    match outcome {
        IdentityRevalidationOutcome::VerifiedWithDiscrepancy(discrepancies) => {
            assert!(!discrepancies.is_empty());
            assert!(discrepancies[0].contains("serial number discrepancy"));
        }
        other => panic!("Expected VerifiedWithDiscrepancy, got: {:?}", other),
    }

    assert!(planned_snap.detect_mutation(&live_snap).is_ok());
}

// -------------------------------------------------------------------------
// TEST 8: External drive with identical strong identity but generic USB bridge serial discrepancy passes with VerifiedWithDiscrepancy and logs warning
// -------------------------------------------------------------------------
#[test]
fn test_8_external_drive_usb_bridge_serial_discrepancy_verified_with_discrepancy() {
    let mut planned_dev = create_test_sd_card();
    planned_dev.serial_number = Some("GENERIC_USB_CARD_READER".to_string());
    let planned_snap = PhysicalDeviceSnapshot::from_device(&planned_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let mut live_dev = planned_dev.clone();
    live_dev.serial_number = Some("000000000502".to_string()); // Live bridge returned synthetic numeric serial
    let live_snap = PhysicalDeviceSnapshot::from_device(&live_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let outcome = planned_snap.revalidate_identity(&live_snap);
    assert!(outcome.is_verified(), "Must be verified for external media serial shift: {:?}", outcome);
    match outcome {
        IdentityRevalidationOutcome::VerifiedWithDiscrepancy(discrepancies) => {
            assert!(discrepancies.iter().any(|d| d.contains("serial number discrepancy")));
        }
        other => panic!("Expected VerifiedWithDiscrepancy, got: {:?}", other),
    }

    assert!(planned_snap.detect_mutation(&live_snap).is_ok());
}

// -------------------------------------------------------------------------
// TEST 9: Internal fixed drive with serial discrepancy is strictly rejected (IdentityRevalidationOutcome::Failed)
// -------------------------------------------------------------------------
#[test]
fn test_9_internal_fixed_drive_serial_discrepancy_strictly_rejected() {
    let fixed_dev = create_test_internal_fixed_drive();
    let planned_snap = PhysicalDeviceSnapshot::from_device(&fixed_dev, 512)
        .with_observations(Some(4096), Some("SATA".to_string()), false);

    let mut live_dev = fixed_dev.clone();
    live_dev.serial_number = Some("DIFFERENT_SERIAL_SUBSTITUTION".to_string());
    let live_snap = PhysicalDeviceSnapshot::from_device(&live_dev, 512)
        .with_observations(Some(4096), Some("SATA".to_string()), false);

    let outcome = planned_snap.revalidate_identity(&live_snap);
    assert!(outcome.is_failed(), "Internal fixed drive with serial discrepancy MUST fail: {:?}", outcome);
    match outcome {
        IdentityRevalidationOutcome::Failed(msg) => {
            assert!(msg.contains("internal fixed storage") || msg.contains("serial number changed"));
        }
        other => panic!("Expected Failed, got: {:?}", other),
    }

    assert!(planned_snap.detect_mutation(&live_snap).is_err());
}

// -------------------------------------------------------------------------
// TEST 10: Capacity change on live re-probe triggers TOCTOU failure
// -------------------------------------------------------------------------
#[test]
fn test_10_capacity_change_triggers_toctou_failure() {
    let usb_dev = create_test_usb_flash_drive();
    let planned_snap = PhysicalDeviceSnapshot::from_device(&usb_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let mut live_dev = usb_dev.clone();
    live_dev.capacity_bytes = 64_000_000_000; // Capacity doubled (different media inserted)
    let live_snap = PhysicalDeviceSnapshot::from_device(&live_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let outcome = planned_snap.revalidate_identity(&live_snap);
    assert!(outcome.is_failed(), "Capacity change MUST fail TOCTOU revalidation: {:?}", outcome);
    match outcome {
        IdentityRevalidationOutcome::Failed(msg) => {
            assert!(msg.contains("capacity changed"), "Message must explain capacity mutation: {}", msg);
        }
        other => panic!("Expected Failed, got: {:?}", other),
    }
}

// -------------------------------------------------------------------------
// TEST 11: Bus type change on live re-probe triggers TOCTOU failure
// -------------------------------------------------------------------------
#[test]
fn test_11_bus_type_change_triggers_toctou_failure() {
    let usb_dev = create_test_usb_flash_drive();
    let planned_snap = PhysicalDeviceSnapshot::from_device(&usb_dev, 512)
        .with_observations(Some(512), Some("USB".to_string()), false);

    let live_snap = PhysicalDeviceSnapshot::from_device(&usb_dev, 512)
        .with_observations(Some(512), Some("NVMe".to_string()), false); // Bus changed from USB to NVMe

    let outcome = planned_snap.revalidate_identity(&live_snap);
    assert!(outcome.is_failed(), "Bus type change MUST fail TOCTOU revalidation: {:?}", outcome);
    match outcome {
        IdentityRevalidationOutcome::Failed(msg) => {
            assert!(msg.contains("Bus type mutated"), "Message must explain bus mutation: {}", msg);
        }
        other => panic!("Expected Failed, got: {:?}", other),
    }
}

// -------------------------------------------------------------------------
// TEST 12: Device disconnection triggers failure
// -------------------------------------------------------------------------
#[test]
fn test_12_device_disconnection_triggers_failure() {
    let usb_dev = create_test_usb_flash_drive();
    let planned_snap = PhysicalDeviceSnapshot::from_device(&usb_dev, 512);

    let mut disconnected_snap = planned_snap.clone();
    disconnected_snap.exists = false;

    let outcome = planned_snap.revalidate_identity(&disconnected_snap);
    assert!(outcome.is_failed(), "Disconnected device MUST fail: {:?}", outcome);
    match outcome {
        IdentityRevalidationOutcome::Failed(msg) => {
            assert!(msg.contains("no longer exists") || msg.contains("disconnected"));
        }
        other => panic!("Expected Failed, got: {:?}", other),
    }
}

// -------------------------------------------------------------------------
// TEST 13: Non-elevated execution fails gracefully with clear privilege error
// -------------------------------------------------------------------------
#[test]
fn test_13_non_elevated_execution_privilege_check() {
    let priv_check = verify_administrative_privileges();
    if let Err(err) = priv_check {
        match err {
            DriveEraseFailureReason::PreExecutionCheckFailed(msg) => {
                assert!(msg.contains("Administrative privileges required"));
            }
            other => panic!("Expected PreExecutionCheckFailed, got: {:?}", other),
        }
    }
}

// -------------------------------------------------------------------------
// TEST 14: Hardware permit single-use is enforced
// -------------------------------------------------------------------------
#[test]
fn test_14_hardware_permit_single_use_enforced() {
    let usb_dev = create_test_usb_flash_drive();
    let snap = PhysicalDeviceSnapshot::from_device(&usb_dev, 512);

    let permit = HardwareExecutionPermit::issue_for_test(
        "op-test-14".to_string(),
        "plan-test-14".to_string(),
        usb_dev.device_id.clone(),
        snap,
        DriveSanitizationMethod::BlockZeroOverwrite,
        ExecutionMode::RealHardware,
    );

    assert!(!permit.is_consumed());

    // First consumption MUST succeed
    let consume1 = permit.consume();
    assert!(consume1.is_ok(), "First permit consumption must succeed");
    assert!(permit.is_consumed());

    // Second consumption MUST fail closed with PermitAlreadyConsumed
    let consume2 = permit.consume();
    assert!(consume2.is_err(), "Second permit consumption must fail closed");
    match consume2.unwrap_err() {
        DriveEraseFailureReason::PermitAlreadyConsumed(id) => {
            assert_eq!(id, permit.permit_id());
        }
        other => panic!("Expected PermitAlreadyConsumed, got: {:?}", other),
    }
}

// -------------------------------------------------------------------------
// TEST 15: Hardware permit TTL expiration is enforced
// -------------------------------------------------------------------------
#[test]
fn test_15_hardware_permit_ttl_expiration_enforced() {
    let usb_dev = create_test_usb_flash_drive();
    let snap = PhysicalDeviceSnapshot::from_device(&usb_dev, 512);

    // Permit with 0-second TTL (immediately expired)
    let permit = HardwareExecutionPermit::issue_for_test(
        "op-test-15".to_string(),
        "plan-test-15".to_string(),
        usb_dev.device_id.clone(),
        snap,
        DriveSanitizationMethod::BlockZeroOverwrite,
        ExecutionMode::RealHardware,
    )
    .with_ttl(0);

    std::thread::sleep(std::time::Duration::from_millis(10));

    let val_res = permit.validate_not_expired();
    assert!(val_res.is_err(), "Permit with expired TTL must be rejected");
    match val_res.unwrap_err() {
        DriveEraseFailureReason::PermitExpired(msg) => {
            assert!(msg.contains("expired") || msg.contains("TTL"));
        }
        other => panic!("Expected PermitExpired, got: {:?}", other),
    }
}

// -------------------------------------------------------------------------
// TEST 16: Concurrency lock prevents simultaneous erasure of the same physical device
// -------------------------------------------------------------------------
#[test]
fn test_16_concurrency_lock_prevents_simultaneous_erasure() {
    let registry = DeviceLockRegistry::new();
    let target = r"\\.\PhysicalDrive1";

    assert!(!registry.is_locked(target));

    // First lock acquisition must succeed
    let guard1 = registry.acquire_lock(target);
    assert!(guard1.is_ok(), "First exclusive lock must succeed");
    assert!(registry.is_locked(target));

    // Second concurrent lock acquisition on same device MUST fail
    let guard2 = registry.acquire_lock(target);
    assert!(guard2.is_err(), "Concurrent lock on same device must fail closed");
    match guard2.unwrap_err() {
        DriveEraseFailureReason::ExclusiveAccessFailed(msg) => {
            assert!(msg.contains("already being sanitized"));
        }
        other => panic!("Expected ExclusiveAccessFailed, got: {:?}", other),
    }

    // Drop guard1 and verify target can now be locked again
    drop(guard1);
    assert!(!registry.is_locked(target), "Lock must be released upon guard drop");

    let guard3 = registry.acquire_lock(target);
    assert!(guard3.is_ok(), "Re-acquiring lock after release must succeed");
}
