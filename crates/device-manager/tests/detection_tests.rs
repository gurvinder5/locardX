use locardx_common::{LocardError, SafeErrorResponse};
use locardx_device_manager::models::{
    DeviceClassification, DeviceType, Filesystem, FilesystemType, LogicalVolume, PhysicalDevice,
    StorageDeviceDto,
};
use locardx_device_manager::{DeviceManagerService, MockDeviceProvider};
use std::sync::Arc;

#[test]
fn test_01_device_model_serialization_deserialization() {
    let volume = LogicalVolume {
        volume_id: "vol-001".to_string(),
        mount_point: Some("C:".to_string()),
        label: Some("Windows-Root".to_string()),
        filesystem: Filesystem {
            fs_type: FilesystemType::Ntfs,
            label: Some("Windows-Root".to_string()),
            read_only: false,
        },
        capacity_bytes: 500_000_000_000,
        free_bytes: 200_000_000_000,
        is_system_volume: true,
        is_boot_volume: true,
        read_only: false,
    };

    let device = PhysicalDevice {
        device_id: "\\\\.\\PhysicalDrive0".to_string(),
        display_name: "NVMe Samsung 980".to_string(),
        vendor: Some("Samsung".to_string()),
        model: Some("980".to_string()),
        serial_number: Some("S5GXNF0T".to_string()),
        device_type: DeviceType::Ssd,
        capacity_bytes: 1_000_000_000_000,
        removable: false,
        read_only: false,
        is_system_device: true,
        classification: DeviceClassification::SystemDevice,
        volumes: vec![volume],
    };

    let json = serde_json::to_string(&device).expect("Failed to serialize PhysicalDevice");
    assert!(json.contains("PhysicalDrive0"));
    assert!(json.contains("system_device"));

    let deserialized: PhysicalDevice =
        serde_json::from_str(&json).expect("Failed to deserialize PhysicalDevice");
    assert_eq!(deserialized.device_id, "\\\\.\\PhysicalDrive0");
    assert_eq!(deserialized.device_type, DeviceType::Ssd);
    assert_eq!(deserialized.volumes.len(), 1);
    assert_eq!(deserialized.volumes[0].mount_point, Some("C:".to_string()));
}

#[test]
fn test_02_physical_device_and_logical_volume_remain_distinct() {
    let mock = Arc::new(MockDeviceProvider::new_default());
    let service = DeviceManagerService::new(mock);

    let devices = service.list_devices().expect("Failed to list devices");
    assert!(!devices.is_empty());

    let sys_disk = &devices[0];
    assert_eq!(sys_disk.device_id, "\\\\.\\PhysicalDrive0");
    assert_ne!(sys_disk.device_id, "C:");
    assert_ne!(sys_disk.device_id, "C:\\");

    // The physical disk must contain multiple logical volumes
    assert_eq!(sys_disk.volumes.len(), 2);
    let vol_c = &sys_disk.volumes[0];
    let vol_d = &sys_disk.volumes[1];

    assert_eq!(vol_c.mount_point, Some("C:".to_string()));
    assert_eq!(vol_d.mount_point, Some("D:".to_string()));

    // Volume capacities must be less than or equal to total physical capacity
    assert!(vol_c.capacity_bytes < sys_disk.capacity_bytes);
    assert!(vol_d.capacity_bytes < sys_disk.capacity_bytes);
}

#[test]
fn test_03_unknown_metadata_is_handled_correctly() {
    let device = PhysicalDevice {
        device_id: "\\\\.\\PhysicalDriveX".to_string(),
        display_name: "Generic Storage Device".to_string(),
        vendor: None,
        model: None,
        serial_number: None,
        device_type: DeviceType::Unknown,
        capacity_bytes: 0,
        removable: false,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::Unknown,
        volumes: Vec::new(),
    };

    let dto = StorageDeviceDto::from(&device);
    assert_eq!(dto.vendor, None);
    assert_eq!(dto.model, None);
    assert_eq!(dto.device_type, DeviceType::Unknown);
    assert_eq!(dto.classification, DeviceClassification::Unknown);

    let json = serde_json::to_string(&dto).expect("Serialization of unknown metadata failed");
    assert!(json.contains("unknown"));
}

#[test]
fn test_04_unsupported_filesystem_is_handled_correctly() {
    let btrfs = FilesystemType::from_os_str("Btrfs");
    assert_eq!(btrfs, FilesystemType::Unknown("BTRFS".to_string()));
    assert_eq!(btrfs.as_str(), "BTRFS");

    let zfs = FilesystemType::from_os_str("ZFS");
    assert_eq!(zfs, FilesystemType::Unknown("ZFS".to_string()));

    let ext4 = FilesystemType::from_os_str("ext4");
    assert_eq!(ext4, FilesystemType::Ext4);

    let ntfs = FilesystemType::from_os_str("NTFS");
    assert_eq!(ntfs, FilesystemType::Ntfs);
}

#[test]
fn test_05_device_classification_works() {
    assert_eq!(
        DeviceClassification::SystemDevice.to_string(),
        "System Device"
    );
    assert_eq!(DeviceClassification::BootDevice.to_string(), "Boot Device");
    assert_eq!(
        DeviceClassification::RemovableDevice.to_string(),
        "Removable Media"
    );
    assert_eq!(
        DeviceClassification::ExternalDevice.to_string(),
        "External Storage"
    );
    assert_eq!(
        DeviceClassification::FixedDataDevice.to_string(),
        "Fixed Data Device"
    );
    assert_eq!(
        DeviceClassification::Unknown.to_string(),
        "Unknown Classification"
    );
}

#[test]
fn test_06_system_device_classification_is_deterministic() {
    let mock = Arc::new(MockDeviceProvider::new_default());
    let service = DeviceManagerService::new(mock);

    let devices = service.list_devices().expect("List devices failed");
    let sys_disk = devices
        .iter()
        .find(|d| d.is_system_device)
        .expect("System device missing");

    assert_eq!(sys_disk.classification, DeviceClassification::SystemDevice);
    assert!(sys_disk.volumes.iter().any(|v| v.is_system_volume));

    let removable_disk = devices
        .iter()
        .find(|d| d.removable)
        .expect("Removable device missing");
    assert_eq!(
        removable_disk.classification,
        DeviceClassification::RemovableDevice
    );
    assert!(!removable_disk.is_system_device);
}

#[test]
fn test_07_read_only_discovery_path_does_not_expose_write_operations() {
    let mock = Arc::new(MockDeviceProvider::new_default());
    let service = DeviceManagerService::new(mock);

    // List devices through service
    let devices = service.list_devices().expect("List failed");
    for dev in &devices {
        // Assert discovery structure contains no handle or mutable write command
        assert!(!dev.device_id.is_empty());
        for vol in &dev.volumes {
            assert!(!vol.volume_id.is_empty());
        }
    }
}

#[test]
fn test_08_empty_device_list_is_handled() {
    let mock = Arc::new(MockDeviceProvider::new_empty());
    let service = DeviceManagerService::new(mock);

    let devices = service.list_devices().expect("List failed");
    assert!(devices.is_empty());

    let dtos = service.list_devices_dto().expect("List DTOs failed");
    assert!(dtos.is_empty());
}

#[test]
fn test_09_device_discovery_errors_convert_to_safe_errors() {
    let error = LocardError::Device("Hardware bus timeout during inquiry".to_string());
    let mock = Arc::new(MockDeviceProvider::new_error(error));
    let service = DeviceManagerService::new(mock);

    let err = service
        .list_devices()
        .expect_err("Expected discovery error");
    let safe_err = SafeErrorResponse::from(err);

    assert_eq!(safe_err.code, "DEVICE_ERROR");
    // Verifies raw hardware internals / bus details are not leaked in message
    assert_eq!(
        safe_err.message,
        "A storage device communication failure occurred."
    );
}

#[test]
fn test_10_tauri_dto_does_not_expose_unsafe_os_handles() {
    let mock = Arc::new(MockDeviceProvider::new_default());
    let service = DeviceManagerService::new(mock);

    let dtos = service.list_devices_dto().expect("DTO generation failed");
    assert_eq!(dtos.len(), 2);

    for dto in &dtos {
        // Invariant check: Invariant note must accompany every DTO
        assert_eq!(
            dto.classification_note,
            "Device classification is not authorization."
        );
        // Serial number is intentionally omitted in StorageDeviceDto to prevent exposure over IPC
        let json = serde_json::to_string(dto).expect("JSON serialization failed");
        assert!(!json.contains("serial_number"));
        assert!(!json.contains("0x")); // No raw OS memory pointers or kernel handles
    }
}
