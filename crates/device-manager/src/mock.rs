use crate::models::{
    DeviceClassification, DeviceType, Filesystem, FilesystemType, LogicalVolume, PhysicalDevice,
};
use crate::traits::DeviceDiscoveryProvider;
use locardx_common::LocardError;
use std::sync::Mutex;

/// Mock implementation of `DeviceDiscoveryProvider` for deterministic testing.
pub struct MockDeviceProvider {
    devices: Mutex<Result<Vec<PhysicalDevice>, LocardError>>,
}

impl MockDeviceProvider {
    /// Creates a mock provider with realistic forensic workstation devices:
    /// - PhysicalDrive0 (System NVMe SSD with C: and D: volumes)
    /// - PhysicalDrive1 (Removable USB drive with E: exFAT volume)
    pub fn new_default() -> Self {
        let default_devices = vec![
            PhysicalDevice {
                device_id: "\\\\.\\PhysicalDrive0".to_string(),
                display_name: "Samsung SSD 980 PRO 1TB".to_string(),
                vendor: Some("Samsung".to_string()),
                model: Some("SSD 980 PRO 1TB".to_string()),
                serial_number: Some("S5GXNF0T123456K".to_string()),
                device_type: DeviceType::Ssd,
                capacity_bytes: 1_000_204_886_016, // ~1 TB
                removable: false,
                read_only: false,
                is_system_device: true,
                classification: DeviceClassification::SystemDevice,
                volumes: vec![
                    LogicalVolume {
                        volume_id: "\\\\?\\Volume{a1b2c3d4-e5f6-0001-0000-000000000001}\\"
                            .to_string(),
                        mount_point: Some("C:".to_string()),
                        label: Some("Windows-OS".to_string()),
                        filesystem: Filesystem {
                            fs_type: FilesystemType::Ntfs,
                            label: Some("Windows-OS".to_string()),
                            read_only: false,
                        },
                        capacity_bytes: 512_000_000_000,
                        free_bytes: 230_000_000_000,
                        is_system_volume: true,
                        is_boot_volume: true,
                        read_only: false,
                    },
                    LogicalVolume {
                        volume_id: "\\\\?\\Volume{a1b2c3d4-e5f6-0001-0000-000000000002}\\"
                            .to_string(),
                        mount_point: Some("D:".to_string()),
                        label: Some("Data".to_string()),
                        filesystem: Filesystem {
                            fs_type: FilesystemType::Ntfs,
                            label: Some("Data".to_string()),
                            read_only: false,
                        },
                        capacity_bytes: 488_204_886_016,
                        free_bytes: 190_000_000_000,
                        is_system_volume: false,
                        is_boot_volume: false,
                        read_only: false,
                    },
                ],
            },
            PhysicalDevice {
                device_id: "\\\\.\\PhysicalDrive1".to_string(),
                display_name: "SanDisk Ultra USB 3.0".to_string(),
                vendor: Some("SanDisk".to_string()),
                model: Some("Ultra USB 3.0".to_string()),
                serial_number: Some("4C530001230912117181".to_string()),
                device_type: DeviceType::Usb,
                capacity_bytes: 64_000_000_000,
                removable: true,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::RemovableDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "\\\\?\\Volume{b2c3d4e5-f6a1-0002-0000-000000000001}\\".to_string(),
                    mount_point: Some("E:".to_string()),
                    label: Some("EVIDENCE_USB".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::ExFat,
                        label: Some("EVIDENCE_USB".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 64_000_000_000,
                    free_bytes: 48_000_000_000,
                    is_system_volume: false,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
        ];

        Self {
            devices: Mutex::new(Ok(default_devices)),
        }
    }

    /// Creates an empty mock provider (simulating a system with no detected drives).
    pub fn new_empty() -> Self {
        Self {
            devices: Mutex::new(Ok(Vec::new())),
        }
    }

    /// Creates a mock provider that simulates a device discovery error.
    pub fn new_error(error: LocardError) -> Self {
        Self {
            devices: Mutex::new(Err(error)),
        }
    }

    /// Creates a mock provider with explicit devices.
    pub fn with_devices(devices: Vec<PhysicalDevice>) -> Self {
        Self {
            devices: Mutex::new(Ok(devices)),
        }
    }

    /// Updates the mock devices dynamically for testing refresh behavior.
    pub fn set_devices(&self, devices: Vec<PhysicalDevice>) {
        let mut lock = self.devices.lock().unwrap();
        *lock = Ok(devices);
    }
}

impl DeviceDiscoveryProvider for MockDeviceProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        let lock = self.devices.lock().unwrap();
        lock.clone()
    }
}
