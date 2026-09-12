use locardx_device_manager::DeviceDiscoveryProvider;
use locardx_drive_eraser::{
    inspect_physical_device, validate_physical_device_identifier, verify_live_device_integrity,
    DriveEraseFailureReason, MockDeviceRegistry,
};
use std::sync::Arc;

#[test]
fn test_logical_volume_and_path_rejections() {
    let invalid_targets = vec![
        "C:",
        "C:\\",
        "c:\\windows",
        "D:\\Data",
        "E:/Storage",
        "/mnt/forensic",
        "/home/investigator",
        "\\\\?\\Volume{b4e05459-0000-0000-0000-100000000000}\\",
    ];

    for target in invalid_targets {
        let result = validate_physical_device_identifier(target);
        assert!(
            result.is_err(),
            "Target '{}' must be rejected as logical volume / path",
            target
        );
        match result.unwrap_err() {
            DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
                assert!(
                    msg.contains("logical volume") || msg.contains("physical storage device"),
                    "Expected logical volume rejection error message, got: {}",
                    msg
                );
            }
            other => panic!("Expected LogicalVolumeTargetRejected, got {:?}", other),
        }
    }
}

#[test]
fn test_valid_physical_device_identifiers() {
    let valid_targets = vec![
        (r"\\.\PhysicalDrive0", r"\\.\PhysicalDrive0"),
        (r"\\.\PhysicalDrive1", r"\\.\PhysicalDrive1"),
        ("PhysicalDrive2", r"\\.\PhysicalDrive2"),
        ("/dev/sda", "/dev/sda"),
        ("/dev/sdb", "/dev/sdb"),
        ("/dev/nvme0n1", "/dev/nvme0n1"),
    ];

    for (input, expected_normalized) in valid_targets {
        let result = validate_physical_device_identifier(input);
        assert!(result.is_ok(), "Input '{}' should be accepted", input);
        assert_eq!(result.unwrap(), expected_normalized);
    }
}

#[test]
fn test_system_and_boot_device_hard_blocks() {
    let registry =
        Arc::new(MockDeviceRegistry::new_standard_test_set()) as Arc<dyn DeviceDiscoveryProvider>;

    // 1. System Device PhysicalDrive0 must be unconditionally blocked
    let res_sys = inspect_physical_device(r"\\.\PhysicalDrive0", &registry, 512);
    assert!(res_sys.is_err());
    match res_sys.unwrap_err() {
        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
            assert!(msg.contains("System or Boot"));
        }
        other => panic!("Expected SystemOrBootDeviceProtected, got {:?}", other),
    }

    // 2. UEFI Boot Device PhysicalDrive6 must be unconditionally blocked
    let res_boot = inspect_physical_device(r"\\.\PhysicalDrive6", &registry, 512);
    assert!(res_boot.is_err());
    match res_boot.unwrap_err() {
        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
            assert!(msg.contains("System or Boot"));
        }
        other => panic!("Expected SystemOrBootDeviceProtected, got {:?}", other),
    }

    // 3. Normal Data HDD PhysicalDrive1 must succeed inspection
    let res_data = inspect_physical_device(r"\\.\PhysicalDrive1", &registry, 512);
    assert!(res_data.is_ok());
    let (dev, snapshot) = res_data.unwrap();
    assert_eq!(dev.device_id, r"\\.\PhysicalDrive1");
    assert_eq!(snapshot.device_id, r"\\.\PhysicalDrive1");
    assert!(!snapshot.is_system);
    assert!(!snapshot.is_boot);
}

#[test]
fn test_non_existent_device_rejection() {
    let registry =
        Arc::new(MockDeviceRegistry::new_standard_test_set()) as Arc<dyn DeviceDiscoveryProvider>;
    let res = inspect_physical_device(r"\\.\PhysicalDrive99", &registry, 512);
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::DeviceNotFound(msg) => {
            assert!(msg.contains("PhysicalDrive99"));
        }
        other => panic!("Expected DeviceNotFound, got {:?}", other),
    }
}

#[test]
fn test_toctou_mutation_serial_number_changed() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let registry = Arc::new(mock_registry.clone()) as Arc<dyn DeviceDiscoveryProvider>;

    // Initial snapshot of PhysicalDrive9
    let (_, snapshot) = inspect_physical_device(r"\\.\PhysicalDrive9", &registry, 512).unwrap();
    assert_eq!(
        snapshot.serial_number,
        Some("SERIAL-ORIGINAL-123".to_string())
    );

    // Verify initial integrity check passes
    assert!(verify_live_device_integrity(&snapshot, &registry).is_ok());

    // Attacker or hardware swap changes serial number
    mock_registry.mutate_device(r"\\.\PhysicalDrive9", |dev| {
        dev.serial_number = Some("SERIAL-ATTACKER-REPLACED-999".to_string());
    });

    // Re-probing must fail closed with DeviceMutated
    let re_probe = verify_live_device_integrity(&snapshot, &registry);
    assert!(re_probe.is_err());
    match re_probe.unwrap_err() {
        DriveEraseFailureReason::DeviceMutated(detail) => {
            assert!(
                detail.to_lowercase().contains("serial number changed"),
                "Expected serial number changed in error, got: {}",
                detail
            );
        }
        other => panic!("Expected DeviceMutated, got {:?}", other),
    }
}

#[test]
fn test_toctou_mutation_capacity_changed() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let registry = Arc::new(mock_registry.clone()) as Arc<dyn DeviceDiscoveryProvider>;

    let (_, snapshot) = inspect_physical_device(r"\\.\PhysicalDrive9", &registry, 512).unwrap();

    // Hardware swap changes capacity
    mock_registry.mutate_device(r"\\.\PhysicalDrive9", |dev| {
        dev.capacity_bytes = 500_000_000_000;
    });

    let re_probe = verify_live_device_integrity(&snapshot, &registry);
    assert!(re_probe.is_err());
    match re_probe.unwrap_err() {
        DriveEraseFailureReason::DeviceMutated(detail) => {
            assert!(
                detail.to_lowercase().contains("capacity changed"),
                "Expected capacity changed in error, got: {}",
                detail
            );
        }
        other => panic!("Expected DeviceMutated, got {:?}", other),
    }
}

#[test]
fn test_toctou_mutation_device_disconnected() {
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let registry = Arc::new(mock_registry.clone()) as Arc<dyn DeviceDiscoveryProvider>;

    let (_, snapshot) = inspect_physical_device(r"\\.\PhysicalDrive9", &registry, 512).unwrap();

    // Drive physically disconnected before execution
    mock_registry.disconnect_device(r"\\.\PhysicalDrive9");

    let re_probe = verify_live_device_integrity(&snapshot, &registry);
    assert!(re_probe.is_err());
    match re_probe.unwrap_err() {
        DriveEraseFailureReason::DeviceNotFound(msg) => {
            assert!(msg.contains("disconnected or no longer present"));
        }
        other => panic!("Expected DeviceNotFound, got {:?}", other),
    }
}
