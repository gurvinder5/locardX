use locardx_device_manager::{DeviceClassification, DeviceType, PhysicalDevice};
use locardx_drive_eraser::{
    detect_device_capabilities, generate_drive_erase_plan, DriveCapability,
    DriveEraseFailureReason, DriveSanitizationMethod, ExecutionMode, PhysicalDeviceSnapshot,
};

fn create_test_device(
    device_id: &str,
    media_type: DeviceType,
    capacity_bytes: u64,
    vendor: Option<&str>,
    model: Option<&str>,
) -> PhysicalDevice {
    PhysicalDevice {
        device_id: device_id.to_string(),
        display_name: format!("Test Device {}", device_id),
        vendor: vendor.map(|s| s.to_string()),
        model: model.map(|s| s.to_string()),
        serial_number: Some("TEST-SERIAL-123".to_string()),
        device_type: media_type,
        capacity_bytes,
        removable: media_type == DeviceType::Usb || media_type == DeviceType::MemoryCard,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::FixedDataDevice,
        volumes: vec![],
    }
}

#[test]
fn test_hdd_capability_detection_and_planning() {
    let device = create_test_device(
        r"\\.\PhysicalDrive1",
        DeviceType::Hdd,
        2_000_000_000_000,
        Some("Seagate"),
        Some("ST2000DM008"),
    );
    let snapshot = PhysicalDeviceSnapshot::from_device(&device, 512);
    let caps = detect_device_capabilities(&device);

    assert!(caps.has_capability(DriveCapability::SequentialWrite));
    assert!(caps.has_capability(DriveCapability::FullDeviceRead));
    assert!(caps.has_capability(DriveCapability::SectorAccess));
    assert!(caps.supports_overwrite);
    assert!(!caps.supports_crypto_erase);
    assert!(caps.is_rotational);

    // Default plan selection for HDD: NIST 800-88 Clear (Single pass zero)
    let plan = generate_drive_erase_plan(
        snapshot.clone(),
        caps.clone(),
        None,
        ExecutionMode::Simulation,
    )
    .unwrap();
    assert_eq!(plan.method, DriveSanitizationMethod::Nist80088ClearZero);
    assert_eq!(plan.passes, 1);

    // Multi-pass DoD 5220.22-M plan selection
    let plan_dod = generate_drive_erase_plan(
        snapshot.clone(),
        caps.clone(),
        Some("dod522022m"),
        ExecutionMode::Simulation,
    )
    .unwrap();
    assert_eq!(plan_dod.method, DriveSanitizationMethod::Dod522022M);
    assert_eq!(plan_dod.passes, 3);
}

#[test]
fn test_sata_ssd_capability_detection_and_planning() {
    let device = create_test_device(
        r"\\.\PhysicalDrive2",
        DeviceType::Ssd,
        1_000_000_000_000,
        Some("Crucial"),
        Some("CT1000MX500SSD1"),
    );
    let snapshot = PhysicalDeviceSnapshot::from_device(&device, 512);
    let caps = detect_device_capabilities(&device);

    assert!(caps.has_capability(DriveCapability::AtaSecureErase));
    assert!(caps.has_capability(DriveCapability::AtaSanitize));
    assert_eq!(caps.interface_bus, "SATA");

    // Default plan selection for SATA SSD: ATA Secure Erase
    let plan = generate_drive_erase_plan(
        snapshot.clone(),
        caps.clone(),
        None,
        ExecutionMode::Simulation,
    )
    .unwrap();
    assert_eq!(plan.method, DriveSanitizationMethod::AtaSecureErase);
    assert_eq!(plan.passes, 1);
}

#[test]
fn test_nvme_ssd_capability_detection_and_planning() {
    let device = create_test_device(
        r"\\.\PhysicalDrive3",
        DeviceType::Ssd,
        1_000_000_000_000,
        Some("Samsung"),
        Some("980 PRO NVMe"),
    );
    let snapshot = PhysicalDeviceSnapshot::from_device(&device, 4096);
    let caps = detect_device_capabilities(&device);

    assert!(caps.has_capability(DriveCapability::NvmeSanitize));
    assert!(caps.has_capability(DriveCapability::NvmeCryptoErase));
    assert!(caps.has_capability(DriveCapability::NvmeFormat));
    assert_eq!(caps.interface_bus, "NVMe");
    assert!(caps.supports_crypto_erase);

    // Default plan selection for NVMe SSD prefers NVMe Crypto Erase
    let plan = generate_drive_erase_plan(
        snapshot.clone(),
        caps.clone(),
        None,
        ExecutionMode::Simulation,
    )
    .unwrap();
    assert_eq!(plan.method, DriveSanitizationMethod::NvmeCryptoErase);
    assert_eq!(plan.passes, 1);
}

#[test]
fn test_usb_and_memory_card_planning() {
    let device_usb = create_test_device(
        r"\\.\PhysicalDrive4",
        DeviceType::Usb,
        64_000_000_000,
        Some("SanDisk"),
        Some("Ultra"),
    );
    let snapshot_usb = PhysicalDeviceSnapshot::from_device(&device_usb, 512);
    let caps_usb = detect_device_capabilities(&device_usb);

    assert!(caps_usb.has_capability(DriveCapability::RemovableMedia));
    assert_eq!(caps_usb.interface_bus, "USB");

    let plan_usb =
        generate_drive_erase_plan(snapshot_usb, caps_usb, None, ExecutionMode::Simulation).unwrap();
    assert_eq!(plan_usb.method, DriveSanitizationMethod::Nist80088ClearZero);
    assert!(plan_usb
        .limitations
        .iter()
        .any(|l| l.to_lowercase().contains("wear leveling")
            || l.to_lowercase().contains("removable")));

    let device_sd = create_test_device(
        r"\\.\PhysicalDrive5",
        DeviceType::MemoryCard,
        32_000_000_000,
        Some("Kingston"),
        Some("Canvas Select"),
    );
    let snapshot_sd = PhysicalDeviceSnapshot::from_device(&device_sd, 512);
    let caps_sd = detect_device_capabilities(&device_sd);

    let plan_sd =
        generate_drive_erase_plan(snapshot_sd, caps_sd, None, ExecutionMode::Simulation).unwrap();
    assert_eq!(plan_sd.method, DriveSanitizationMethod::Nist80088ClearZero);
}

#[test]
fn test_unknown_device_rejection() {
    let device_unknown = create_test_device(
        r"\\.\PhysicalDrive7",
        DeviceType::Unknown,
        8_000_000_000,
        None,
        None,
    );
    let snapshot_unknown = PhysicalDeviceSnapshot::from_device(&device_unknown, 512);
    let caps_unknown = detect_device_capabilities(&device_unknown);

    let res = generate_drive_erase_plan(
        snapshot_unknown,
        caps_unknown,
        None,
        ExecutionMode::Simulation,
    );
    assert!(res.is_err());
    match res.unwrap_err() {
        DriveEraseFailureReason::UnsupportedMedia(msg) => {
            assert!(msg.contains("unknown media type") || msg.contains("undetectable"));
        }
        other => panic!("Expected UnsupportedMedia, got {:?}", other),
    }
}
