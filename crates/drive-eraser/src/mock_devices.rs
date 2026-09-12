use locardx_common::LocardError;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceType, Filesystem, FilesystemType,
    LogicalVolume, PhysicalDevice,
};
use std::sync::{Arc, RwLock};

/// Realistic mock physical storage device registry for simulation and testing.
#[derive(Clone)]
pub struct MockDeviceRegistry {
    devices: Arc<RwLock<Vec<PhysicalDevice>>>,
}

impl MockDeviceRegistry {
    pub fn new_standard_test_set() -> Self {
        let devices = vec![
            // 1. Windows System Disk (Host OS)
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive0".to_string(),
                display_name: "Samsung PM9A1 NVMe 512GB (System Disk)".to_string(),
                vendor: Some("Samsung".to_string()),
                model: Some("PM9A1".to_string()),
                serial_number: Some("S676NF0M100001".to_string()),
                device_type: DeviceType::Ssd,
                capacity_bytes: 512_110_190_592,
                removable: false,
                read_only: false,
                is_system_device: true,
                classification: DeviceClassification::SystemDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "vol-sys-0".to_string(),
                    mount_point: Some("C:\\".to_string()),
                    label: Some("Windows".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::Ntfs,
                        label: Some("Windows".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 500_000_000_000,
                    free_bytes: 250_000_000_000,
                    is_system_volume: true,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
            // 2. HDD Data Disk
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive1".to_string(),
                display_name: "Seagate Barracuda 2TB HDD".to_string(),
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
                    volume_id: "vol-data-1".to_string(),
                    mount_point: Some("D:\\".to_string()),
                    label: Some("DataStorage".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::Ntfs,
                        label: Some("DataStorage".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 2_000_000_000_000,
                    free_bytes: 1_200_000_000_000,
                    is_system_volume: false,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
            // 3. SATA SSD Data Disk
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive2".to_string(),
                display_name: "Crucial MX500 1TB SATA SSD".to_string(),
                vendor: Some("Crucial".to_string()),
                model: Some("CT1000MX500SSD1".to_string()),
                serial_number: Some("2104E489A123".to_string()),
                device_type: DeviceType::Ssd,
                capacity_bytes: 1_000_204_886_016,
                removable: false,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::FixedDataDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "vol-ssd-2".to_string(),
                    mount_point: Some("E:\\".to_string()),
                    label: Some("FastSSD".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::Ntfs,
                        label: Some("FastSSD".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 1_000_000_000_000,
                    free_bytes: 800_000_000_000,
                    is_system_volume: false,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
            // 4. NVMe SSD Data Disk
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive3".to_string(),
                display_name: "Samsung 980 PRO NVMe 1TB".to_string(),
                vendor: Some("Samsung".to_string()),
                model: Some("980 PRO".to_string()),
                serial_number: Some("S69ENF0R999999".to_string()),
                device_type: DeviceType::Ssd,
                capacity_bytes: 1_000_204_886_016,
                removable: false,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::FixedDataDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "vol-nvme-3".to_string(),
                    mount_point: Some("F:\\".to_string()),
                    label: Some("ScratchNVMe".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::Ntfs,
                        label: Some("ScratchNVMe".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 1_000_000_000_000,
                    free_bytes: 500_000_000_000,
                    is_system_volume: false,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
            // 5. USB Flash Drive
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive4".to_string(),
                display_name: "SanDisk Ultra USB 3.0 64GB".to_string(),
                vendor: Some("SanDisk".to_string()),
                model: Some("Ultra".to_string()),
                serial_number: Some("SDUSB64GB9988".to_string()),
                device_type: DeviceType::Usb,
                capacity_bytes: 64_000_000_000,
                removable: true,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::RemovableDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "vol-usb-4".to_string(),
                    mount_point: Some("G:\\".to_string()),
                    label: Some("USB_DRIVE".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::ExFat,
                        label: Some("USB_DRIVE".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 64_000_000_000,
                    free_bytes: 32_000_000_000,
                    is_system_volume: false,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
            // 6. SD / Memory Card
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive5".to_string(),
                display_name: "Kingston Canvas SD Card 32GB".to_string(),
                vendor: Some("Kingston".to_string()),
                model: Some("Canvas Select".to_string()),
                serial_number: Some("KSDCARD32GB".to_string()),
                device_type: DeviceType::MemoryCard,
                capacity_bytes: 32_000_000_000,
                removable: true,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::RemovableDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "vol-sd-5".to_string(),
                    mount_point: Some("H:\\".to_string()),
                    label: Some("SD_CARD".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::Fat32,
                        label: Some("SD_CARD".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 32_000_000_000,
                    free_bytes: 16_000_000_000,
                    is_system_volume: false,
                    is_boot_volume: false,
                    read_only: false,
                }],
            },
            // 7. UEFI Boot Disk
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive6".to_string(),
                display_name: "UEFI System Boot Media 16GB".to_string(),
                vendor: Some("Generic".to_string()),
                model: Some("BootStorage".to_string()),
                serial_number: Some("BOOTMEDIA001".to_string()),
                device_type: DeviceType::Usb,
                capacity_bytes: 16_000_000_000,
                removable: true,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::BootDevice,
                volumes: vec![LogicalVolume {
                    volume_id: "vol-boot-6".to_string(),
                    mount_point: None,
                    label: Some("ESP".to_string()),
                    filesystem: Filesystem {
                        fs_type: FilesystemType::Fat32,
                        label: Some("ESP".to_string()),
                        read_only: false,
                    },
                    capacity_bytes: 512_000_000,
                    free_bytes: 400_000_000,
                    is_system_volume: false,
                    is_boot_volume: true,
                    read_only: false,
                }],
            },
            // 8. Unknown Media Device
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive7".to_string(),
                display_name: "Generic Unclassified Storage 8GB".to_string(),
                vendor: None,
                model: None,
                serial_number: None,
                device_type: DeviceType::Unknown,
                capacity_bytes: 8_000_000_000,
                removable: false,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::Unknown,
                volumes: vec![],
            },
            // 9. PhysicalDrive9 will be used for mutation/replacement tests
            PhysicalDevice {
                device_id: r"\\.\PhysicalDrive9".to_string(),
                display_name: "Pre-Mutation External Drive 250GB".to_string(),
                vendor: Some("WD".to_string()),
                model: Some("My Passport".to_string()),
                serial_number: Some("SERIAL-ORIGINAL-123".to_string()),
                device_type: DeviceType::ExternalStorage,
                capacity_bytes: 250_000_000_000,
                removable: true,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::ExternalDevice,
                volumes: vec![],
            },
        ];

        Self {
            devices: Arc::new(RwLock::new(devices)),
        }
    }

    /// Simulates hardware mutation or replacement of a target device.
    pub fn mutate_device(&self, device_id: &str, mutator: impl FnOnce(&mut PhysicalDevice)) {
        let mut list = self.devices.write().unwrap();
        if let Some(dev) = list
            .iter_mut()
            .find(|d| d.device_id.eq_ignore_ascii_case(device_id))
        {
            mutator(dev);
        }
    }

    /// Simulates disconnecting a device.
    pub fn disconnect_device(&self, device_id: &str) {
        let mut list = self.devices.write().unwrap();
        list.retain(|d| !d.device_id.eq_ignore_ascii_case(device_id));
    }
}

impl DeviceDiscoveryProvider for MockDeviceRegistry {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        let list = self.devices.read().unwrap();
        Ok(list.clone())
    }
}
