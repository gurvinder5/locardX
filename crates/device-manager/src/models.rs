use serde::{Deserialize, Serialize};

/// Identifies the underlying physical storage hardware category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Hdd,
    Ssd,
    Usb,
    MemoryCard,
    ExternalStorage,
    Unknown,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hdd => write!(f, "HDD"),
            Self::Ssd => write!(f, "SSD"),
            Self::Usb => write!(f, "USB"),
            Self::MemoryCard => write!(f, "Memory Card"),
            Self::ExternalStorage => write!(f, "External Storage"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Read-only safety classification for discovered storage devices.
///
/// INVARIANT: Device classification is informational only.
/// Device classification is NOT authorization for erasure, sanitization, or modification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClassification {
    /// Device housing the active operating system root / system directory.
    SystemDevice,
    /// Device housing the active boot loader partition.
    BootDevice,
    /// Removable media (e.g., USB flash drive, SD card).
    RemovableDevice,
    /// External storage enclosure or drive.
    ExternalDevice,
    /// Fixed internal data storage device without system/boot partitions.
    FixedDataDevice,
    /// Unclassified or indeterminate storage device.
    Unknown,
}

impl std::fmt::Display for DeviceClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemDevice => write!(f, "System Device"),
            Self::BootDevice => write!(f, "Boot Device"),
            Self::RemovableDevice => write!(f, "Removable Media"),
            Self::ExternalDevice => write!(f, "External Storage"),
            Self::FixedDataDevice => write!(f, "Fixed Data Device"),
            Self::Unknown => write!(f, "Unknown Classification"),
        }
    }
}

/// Supported and recognized filesystem types.
/// Extensible for modern and forensic filesystem signatures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemType {
    Ntfs,
    Fat32,
    ExFat,
    Ext4,
    Apfs,
    HfsPlus,
    #[serde(untagged)]
    Unknown(String),
}

impl FilesystemType {
    /// Parses a filesystem identifier string from the OS into a typed variant.
    pub fn from_os_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "NTFS" => Self::Ntfs,
            "FAT32" => Self::Fat32,
            "EXFAT" => Self::ExFat,
            "EXT4" => Self::Ext4,
            "APFS" => Self::Apfs,
            "HFS+" | "HFSPLUS" => Self::HfsPlus,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Ntfs => "NTFS",
            Self::Fat32 => "FAT32",
            Self::ExFat => "exFAT",
            Self::Ext4 => "ext4",
            Self::Apfs => "APFS",
            Self::HfsPlus => "HFS+",
            Self::Unknown(raw) => raw.as_str(),
        }
    }
}

/// Metadata describing a filesystem instance on a volume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filesystem {
    pub fs_type: FilesystemType,
    pub label: Option<String>,
    pub read_only: bool,
}

/// A logical volume (partition or filesystem container) hosted on a physical device.
///
/// NOTE: A logical volume (such as `C:`) is distinct from the underlying physical disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalVolume {
    pub volume_id: String,
    pub mount_point: Option<String>,
    pub label: Option<String>,
    pub filesystem: Filesystem,
    pub capacity_bytes: u64,
    pub free_bytes: u64,
    pub is_system_volume: bool,
    pub is_boot_volume: bool,
    pub read_only: bool,
}

/// Physical storage hardware device (e.g. `PhysicalDrive0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalDevice {
    pub device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub device_type: DeviceType,
    pub capacity_bytes: u64,
    pub removable: bool,
    pub read_only: bool,
    pub is_system_device: bool,
    pub classification: DeviceClassification,
    pub volumes: Vec<LogicalVolume>,
}

/// Sanitized Logical Volume DTO safe for Tauri IPC transmission to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalVolumeDto {
    pub volume_id: String,
    pub mount_point: Option<String>,
    pub label: Option<String>,
    pub filesystem_type: String,
    pub capacity_bytes: u64,
    pub free_bytes: u64,
    pub is_system_volume: bool,
    pub is_boot_volume: bool,
    pub read_only: bool,
}

impl From<&LogicalVolume> for LogicalVolumeDto {
    fn from(v: &LogicalVolume) -> Self {
        Self {
            volume_id: v.volume_id.clone(),
            mount_point: v.mount_point.clone(),
            label: v.label.clone(),
            filesystem_type: v.filesystem.fs_type.as_str().to_string(),
            capacity_bytes: v.capacity_bytes,
            free_bytes: v.free_bytes,
            is_system_volume: v.is_system_volume,
            is_boot_volume: v.is_boot_volume,
            read_only: v.read_only,
        }
    }
}

/// Sanitized Storage Device DTO safe for Tauri IPC transmission to the frontend.
///
/// NOTE: Raw OS handles, kernel pointers, and sensitive unmasked serial numbers
/// are strictly excluded from IPC to maintain memory safety and privacy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDeviceDto {
    pub device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub device_type: DeviceType,
    pub capacity_bytes: u64,
    pub removable: bool,
    pub read_only: bool,
    pub is_system_device: bool,
    pub classification: DeviceClassification,
    pub classification_note: String,
    pub volumes: Vec<LogicalVolumeDto>,
}

impl From<&PhysicalDevice> for StorageDeviceDto {
    fn from(p: &PhysicalDevice) -> Self {
        Self {
            device_id: p.device_id.clone(),
            display_name: p.display_name.clone(),
            vendor: p.vendor.clone(),
            model: p.model.clone(),
            device_type: p.device_type,
            capacity_bytes: p.capacity_bytes,
            removable: p.removable,
            read_only: p.read_only,
            is_system_device: p.is_system_device,
            classification: p.classification,
            classification_note: "Device classification is not authorization.".to_string(),
            volumes: p.volumes.iter().map(LogicalVolumeDto::from).collect(),
        }
    }
}
