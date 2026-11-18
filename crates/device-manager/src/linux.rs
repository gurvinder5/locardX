//! Linux storage device discovery provider.
//!
//! SAFETY GUARANTEES:
//! 1. All device information is read from `/sys/block`, `/proc/mounts`, and `lsblk`.
//! 2. Zero write operations are performed — strictly read-only sysfs/procfs queries.
//! 3. No destructive system commands are invoked.
//! 4. Graceful degradation when sysfs entries or permissions are unavailable.

use crate::models::{
    DeviceClassification, DeviceType, Filesystem, FilesystemType, LogicalVolume, PhysicalDevice,
};
use crate::traits::DeviceDiscoveryProvider;
use locardx_common::LocardError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::debug;

/// Real Linux implementation of `DeviceDiscoveryProvider`.
pub struct LinuxDeviceProvider;

impl LinuxDeviceProvider {
    pub fn new() -> Self {
        Self
    }

    /// Read a sysfs attribute file, returning trimmed content or None.
    fn read_sysfs_attr(base: &Path, attr: &str) -> Option<String> {
        let path = base.join(attr);
        fs::read_to_string(&path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Read a numeric sysfs attribute.
    fn read_sysfs_u64(base: &Path, attr: &str) -> Option<u64> {
        Self::read_sysfs_attr(base, attr).and_then(|s| s.parse::<u64>().ok())
    }

    /// Read a numeric sysfs attribute as u32.
    fn read_sysfs_u32(base: &Path, attr: &str) -> Option<u32> {
        Self::read_sysfs_attr(base, attr).and_then(|s| s.parse::<u32>().ok())
    }

    /// Parse /proc/mounts to build a map: device_path -> (mount_point, fs_type)
    fn parse_mounts() -> HashMap<String, (String, String)> {
        let mut mounts = HashMap::new();
        if let Ok(content) = fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let dev = parts[0].to_string();
                    let mount = parts[1].to_string();
                    let fstype = parts[2].to_string();
                    mounts.insert(dev, (mount, fstype));
                }
            }
        }
        mounts
    }

    /// Determine if a block device is a physical disk (not a partition, loop, ram, etc.)
    fn is_physical_disk(name: &str) -> bool {
        // Include sd*, hd*, nvme*n* (but not nvme*n*p*), vd*, xvd*, mmcblk* (not p*)
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("dm-")
            || name.starts_with("sr")
            || name.starts_with("fd")
            || name.starts_with("md")
            || name.starts_with("nbd")
        {
            return false;
        }

        // sd*, hd*, vd*, xvd* — anything without trailing digits after initial letters is a whole disk
        if name.starts_with("sd")
            || name.starts_with("hd")
            || name.starts_with("vd")
            || name.starts_with("xvd")
        {
            // A partition would be like sda1, sdb2; a whole disk is sda, sdb
            // Check: if last char is a digit, it's a partition
            return !name.chars().last().map_or(false, |c| c.is_ascii_digit());
        }

        // nvme: nvme0n1 is a disk, nvme0n1p1 is a partition
        if name.starts_with("nvme") {
            return !name.contains('p');
        }

        // mmcblk: mmcblk0 is a disk, mmcblk0p1 is a partition
        if name.starts_with("mmcblk") {
            return !name.contains('p');
        }

        false
    }

    /// List partitions for a given disk under /sys/block/<disk>/
    fn get_partitions(disk_name: &str) -> Vec<String> {
        let disk_path = Path::new("/sys/block").join(disk_name);
        let mut partitions = Vec::new();

        if let Ok(entries) = fs::read_dir(&disk_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Partitions are subdirectories that start with the disk name
                if name.starts_with(disk_name) && entry.path().join("partition").exists() {
                    partitions.push(name);
                }
            }
        }

        partitions.sort();
        partitions
    }

    /// Get the root filesystem device to identify the system disk
    fn get_root_device() -> Option<String> {
        if let Ok(content) = fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "/" {
                    return Some(parts[0].to_string());
                }
            }
        }
        None
    }
}

impl Default for LinuxDeviceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceDiscoveryProvider for LinuxDeviceProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        let sys_block = Path::new("/sys/block");

        if !sys_block.exists() {
            return Err(LocardError::Device(
                "/sys/block not available — cannot enumerate block devices".to_string(),
            ));
        }

        let mounts = Self::parse_mounts();
        let root_device = Self::get_root_device();

        let entries = fs::read_dir(sys_block)
            .map_err(|e| LocardError::Device(format!("Failed to read /sys/block: {}", e)))?;

        let mut physical_devices = Vec::new();

        for entry in entries.flatten() {
            let disk_name = entry.file_name().to_string_lossy().to_string();

            if !Self::is_physical_disk(&disk_name) {
                continue;
            }

            let disk_sys_path = sys_block.join(&disk_name);
            let device_path = format!("/dev/{}", disk_name);

            debug!(disk = %disk_name, "Inspecting block device");

            // Read device metadata from sysfs
            let device_dir = disk_sys_path.join("device");
            let vendor = Self::read_sysfs_attr(&device_dir, "vendor");
            let model = Self::read_sysfs_attr(&device_dir, "model");
            let serial = Self::read_sysfs_attr(&device_dir, "serial")
                .or_else(|| Self::read_sysfs_attr(&disk_sys_path, "serial"));

            // Capacity: /sys/block/<dev>/size gives 512-byte sectors (always 512 in sysfs)
            let size_sectors = Self::read_sysfs_u64(&disk_sys_path, "size").unwrap_or(0);
            let capacity_bytes = size_sectors * 512;

            // Actual hardware sector size for partition size calculations
            let hw_sector_size =
                Self::read_sysfs_u64(&disk_sys_path.join("queue"), "hw_sector_size").unwrap_or(512);

            // Removable flag
            let removable = Self::read_sysfs_u32(&disk_sys_path, "removable").unwrap_or(0) == 1;

            // Rotational flag (0 = SSD, 1 = HDD)
            let rotational = Self::read_sysfs_u32(&disk_sys_path.join("queue"), "rotational");

            // Determine device type
            let device_type = if disk_name.starts_with("mmcblk") {
                DeviceType::MemoryCard
            } else if disk_name.starts_with("nvme") {
                DeviceType::Ssd
            } else if removable {
                DeviceType::Usb
            } else {
                match rotational {
                    Some(0) => DeviceType::Ssd,
                    Some(1) => DeviceType::Hdd,
                    _ => DeviceType::Unknown,
                }
            };

            // Check if this disk hosts the root filesystem
            let is_system_device = root_device
                .as_ref()
                .map_or(false, |rd| rd.starts_with(&device_path));

            // Classification
            let classification = if is_system_device {
                DeviceClassification::SystemDevice
            } else if removable || device_type == DeviceType::Usb {
                DeviceClassification::RemovableDevice
            } else {
                DeviceClassification::FixedDataDevice
            };

            // Display name
            let display_name = match (&vendor, &model) {
                (Some(v), Some(m)) => format!("{} {}", v.trim(), m.trim()),
                (None, Some(m)) => m.trim().to_string(),
                _ => format!("/dev/{}", disk_name),
            };

            // Enumerate partitions as logical volumes
            let partitions = Self::get_partitions(&disk_name);
            let mut volumes = Vec::new();

            for part_name in &partitions {
                let part_dev = format!("/dev/{}", part_name);

                // Look up mount info
                let (mount_point, fs_type_str) = mounts
                    .get(&part_dev)
                    .cloned()
                    .unwrap_or_else(|| ("".to_string(), "".to_string()));

                let fs_type = if fs_type_str.is_empty() {
                    FilesystemType::Unknown("Unmounted".to_string())
                } else {
                    FilesystemType::from_os_str(&fs_type_str)
                };

                let is_system_volume = mount_point == "/";
                let is_boot_volume = mount_point == "/boot" || mount_point == "/boot/efi";

                // Read partition size from sysfs (sysfs always uses 512-byte sectors)
                let part_sys = disk_sys_path.join(part_name);
                let part_sectors = Self::read_sysfs_u64(&part_sys, "size").unwrap_or(0);
                let part_capacity = part_sectors * 512;

                // Attempt to get free space from statvfs-style info
                let free_bytes = if !mount_point.is_empty() {
                    get_free_space(&mount_point).unwrap_or(0)
                } else {
                    0
                };

                let label = Self::read_sysfs_attr(&part_sys, "label");

                volumes.push(LogicalVolume {
                    volume_id: part_dev.clone(),
                    mount_point: if mount_point.is_empty() {
                        None
                    } else {
                        Some(mount_point.clone())
                    },
                    label: label.clone(),
                    filesystem: Filesystem {
                        fs_type,
                        label,
                        read_only: false,
                    },
                    capacity_bytes: part_capacity,
                    free_bytes,
                    is_system_volume,
                    is_boot_volume,
                    read_only: false,
                });
            }

            // If no partitions but the whole disk is mounted (common for USB sticks)
            if volumes.is_empty() {
                if let Some((mount_point, fs_type_str)) = mounts.get(&device_path) {
                    let fs_type = FilesystemType::from_os_str(fs_type_str);
                    let is_system_volume = mount_point == "/";
                    let free_bytes = get_free_space(mount_point).unwrap_or(0);

                    volumes.push(LogicalVolume {
                        volume_id: device_path.clone(),
                        mount_point: Some(mount_point.clone()),
                        label: None,
                        filesystem: Filesystem {
                            fs_type,
                            label: None,
                            read_only: false,
                        },
                        capacity_bytes: capacity_bytes,
                        free_bytes,
                        is_system_volume,
                        is_boot_volume: false,
                        read_only: false,
                    });
                }
            }

            physical_devices.push(PhysicalDevice {
                device_id: device_path,
                display_name,
                vendor,
                model,
                serial_number: serial,
                device_type,
                capacity_bytes,
                removable,
                read_only: false,
                is_system_device,
                classification,
                volumes,
            });
        }

        // Sort: system devices first, then by device_id
        physical_devices.sort_by(|a, b| {
            b.is_system_device
                .cmp(&a.is_system_device)
                .then_with(|| a.device_id.cmp(&b.device_id))
        });

        Ok(physical_devices)
    }
}

/// Get free space for a mount point using libc::statvfs.
#[cfg(target_os = "linux")]
fn get_free_space(mount_point: &str) -> Option<u64> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let c_path = CString::new(mount_point).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();

    let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };

    if result == 0 {
        let stat = unsafe { stat.assume_init() };
        Some(stat.f_bavail as u64 * stat.f_frsize as u64)
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn get_free_space(_mount_point: &str) -> Option<u64> {
    None
}
