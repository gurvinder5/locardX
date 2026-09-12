//! Safe, read-only Windows storage device discovery provider.
//!
//! SAFETY GUARANTEES:
//! 1. All handles are opened with `dwDesiredAccess = 0` (query-only access) or read-only flags.
//! 2. Zero write permissions (`GENERIC_WRITE`, `GENERIC_ALL`) are ever requested.
//! 3. Zero destructive system commands (`diskpart`, `format`, `cipher`, `fsutil`, `del`) are invoked.
//! 4. Non-elevated execution degrades gracefully without panicking.

use crate::models::{
    DeviceClassification, DeviceType, Filesystem, FilesystemType, LogicalVolume, PhysicalDevice,
};
use crate::traits::DeviceDiscoveryProvider;
use locardx_common::LocardError;
use std::collections::HashMap;
use std::ffi::c_void;
use tracing::debug;

// Win32 Constants
const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;

// IOCTL Codes
const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x00560000;
const IOCTL_DISK_GET_DRIVE_GEOMETRY_EX: u32 = 0x000700A0;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;

// Drive Type Constants
const DRIVE_CDROM: u32 = 5;

// Storage Bus Types
const BUS_TYPE_USB: u32 = 7;
const BUS_TYPE_SD: u32 = 11;
const BUS_TYPE_MMC: u32 = 12;
const BUS_TYPE_NVME: u32 = 17;

// Volume Flags
const FILE_READ_ONLY_VOLUME: u32 = 0x00080000;

#[repr(C)]
struct DiskExtent {
    disk_number: u32,
    starting_offset: i64,
    extent_length: i64,
}

#[repr(C)]
struct VolumeDiskExtents {
    number_of_disk_extents: u32,
    extents: [DiskExtent; 8],
}

#[repr(C)]
struct DiskGeometry {
    cylinders: i64,
    media_type: u32,
    tracks_per_cylinder: u32,
    sectors_per_track: u32,
    bytes_per_sector: u32,
}

#[repr(C)]
struct DiskGeometryEx {
    geometry: DiskGeometry,
    disk_size: i64,
    data: [u8; 1],
}

#[repr(C)]
struct StoragePropertyQuery {
    property_id: u32,
    query_type: u32,
    additional_parameters: [u8; 1],
}

#[repr(C)]
struct DeviceSeekPenaltyDescriptor {
    version: u32,
    size: u32,
    incurs_seek_penalty: u8,
}

extern "system" {
    fn GetSystemDirectoryW(lpBuffer: *mut u16, uSize: u32) -> u32;
    fn GetLogicalDriveStringsW(nBufferLength: u32, lpBuffer: *mut u16) -> u32;
    fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
    fn GetVolumeInformationW(
        lpRootPathName: *const u16,
        lpVolumeNameBuffer: *mut u16,
        nVolumeNameSize: u32,
        lpVolumeSerialNumber: *mut u32,
        lpMaximumComponentLength: *mut u32,
        lpFileSystemFlags: *mut u32,
        lpFileSystemNameBuffer: *mut u16,
        nFileSystemNameSize: u32,
    ) -> i32;
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut c_void,
    ) -> *mut c_void;
    fn DeviceIoControl(
        hDevice: *mut c_void,
        dwIoControlCode: u32,
        lpInBuffer: *const c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> i32;
    fn CloseHandle(hObject: *mut c_void) -> i32;
}

/// Real Windows implementation of `DeviceDiscoveryProvider`.
#[derive(Default)]
pub struct WindowsDeviceProvider;

impl WindowsDeviceProvider {
    pub fn new() -> Self {
        Self
    }

    /// Gets the drive letter hosting the active Windows system directory (e.g., "C:").
    fn get_system_drive_letter(&self) -> Option<String> {
        let mut buffer = [0u16; 260];
        let len = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        if len > 0 && len < buffer.len() as u32 {
            let sys_path = String::from_utf16_lossy(&buffer[..len as usize]);
            if let Some(colon_pos) = sys_path.find(':') {
                if colon_pos > 0 {
                    return Some(sys_path[..colon_pos + 1].to_uppercase());
                }
            }
        }
        None
    }

    /// Enumerates all mounted logical drive roots (e.g. `["C:\\", "D:\\"]`).
    fn get_logical_drive_roots(&self) -> Vec<String> {
        let mut buffer = [0u16; 512];
        let len = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };
        if len == 0 || len >= buffer.len() as u32 {
            return Vec::new();
        }

        let mut roots = Vec::new();
        let mut start = 0;
        for i in 0..len as usize {
            if buffer[i] == 0 {
                if i > start {
                    let root = String::from_utf16_lossy(&buffer[start..i]);
                    roots.push(root);
                }
                start = i + 1;
            }
        }
        roots
    }

    /// Resolves which physical disk number (if any) hosts the specified volume.
    /// Uses read-only handle (`dwDesiredAccess = 0`).
    fn get_volume_disk_number(&self, drive_letter: &str) -> Option<u32> {
        // Prepare device path: `\\.\C:`
        let trimmed = drive_letter.trim_end_matches('\\');
        let device_path = format!("\\\\.\\{}", trimmed);
        let wide_path: Vec<u16> = device_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0, // Query access only - no read or write payload permissions
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut extents: VolumeDiskExtents = unsafe { std::mem::zeroed() };
        let mut bytes_returned = 0u32;

        let success = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                std::ptr::null(),
                0,
                &mut extents as *mut _ as *mut c_void,
                std::mem::size_of::<VolumeDiskExtents>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        unsafe { CloseHandle(handle) };

        if success != 0 && extents.number_of_disk_extents > 0 {
            Some(extents.extents[0].disk_number)
        } else {
            None
        }
    }

    /// Queries physical disk hardware metadata using read-only access.
    fn query_physical_disk_info(&self, disk_number: u32) -> PhysicalDiskMetadata {
        let device_path = format!("\\\\.\\PhysicalDrive{}", disk_number);
        let wide_path: Vec<u16> = device_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0, // Query access only
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            debug!(
                disk = disk_number,
                "Direct PhysicalDrive access unavailable; using volume aggregation"
            );
            return PhysicalDiskMetadata::default();
        }

        let mut meta = PhysicalDiskMetadata::default();

        // 1. Query Disk Geometry Ex (Capacity)
        let mut geom_ex: DiskGeometryEx = unsafe { std::mem::zeroed() };
        let mut bytes_returned = 0u32;
        let geom_ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                std::ptr::null(),
                0,
                &mut geom_ex as *mut _ as *mut c_void,
                std::mem::size_of::<DiskGeometryEx>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if geom_ok != 0 && geom_ex.disk_size > 0 {
            meta.capacity_bytes = geom_ex.disk_size as u64;
        }

        // 2. Query Storage Device Descriptor
        let mut query = StoragePropertyQuery {
            property_id: 0, // StorageDeviceProperty
            query_type: 0,  // PropertyStandardQuery
            additional_parameters: [0],
        };

        let mut buffer = [0u8; 1024];
        let desc_ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut query as *mut _ as *const c_void,
                std::mem::size_of::<StoragePropertyQuery>() as u32,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if desc_ok != 0 && bytes_returned >= 24 {
            let vendor_offset =
                u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]) as usize;
            let product_offset =
                u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]) as usize;
            let bus_type = if bytes_returned >= 32 {
                u32::from_le_bytes([buffer[28], buffer[29], buffer[30], buffer[31]])
            } else {
                0
            };
            let removable_flag = buffer[6] != 0;

            meta.removable = removable_flag;
            meta.bus_type = bus_type;

            if vendor_offset > 0 && vendor_offset < buffer.len() {
                meta.vendor = read_null_terminated_ascii(&buffer[vendor_offset..]);
            }
            if product_offset > 0 && product_offset < buffer.len() {
                meta.model = read_null_terminated_ascii(&buffer[product_offset..]);
            }
        }

        // 3. Query Seek Penalty to distinguish SSD vs HDD
        let mut seek_query = StoragePropertyQuery {
            property_id: 7, // StorageDeviceSeekPenaltyProperty
            query_type: 0,
            additional_parameters: [0],
        };
        let mut seek_desc: DeviceSeekPenaltyDescriptor = unsafe { std::mem::zeroed() };
        let seek_ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut seek_query as *mut _ as *const c_void,
                std::mem::size_of::<StoragePropertyQuery>() as u32,
                &mut seek_desc as *mut _ as *mut c_void,
                std::mem::size_of::<DeviceSeekPenaltyDescriptor>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if seek_ok != 0 {
            meta.incurs_seek_penalty = Some(seek_desc.incurs_seek_penalty != 0);
        }

        unsafe { CloseHandle(handle) };
        meta
    }
}

#[derive(Default)]
struct PhysicalDiskMetadata {
    capacity_bytes: u64,
    vendor: Option<String>,
    model: Option<String>,
    removable: bool,
    bus_type: u32,
    incurs_seek_penalty: Option<bool>,
}

fn read_null_terminated_ascii(slice: &[u8]) -> Option<String> {
    let mut end = 0;
    while end < slice.len() && slice[end] != 0 {
        end += 1;
    }
    if end > 0 {
        let s = String::from_utf8_lossy(&slice[..end]).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

impl DeviceDiscoveryProvider for WindowsDeviceProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        let system_drive = self
            .get_system_drive_letter()
            .unwrap_or_else(|| "C:".to_string());
        let drive_roots = self.get_logical_drive_roots();

        debug!(system_drive = %system_drive, found_roots = drive_roots.len(), "Discovering Windows storage devices");

        // Map: disk_number -> Vec<LogicalVolume>
        let mut disk_to_volumes: HashMap<u32, Vec<LogicalVolume>> = HashMap::new();
        let mut unassigned_volumes: Vec<LogicalVolume> = Vec::new();

        for root in drive_roots {
            let drive_type = {
                let wide_root: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
                unsafe { GetDriveTypeW(wide_root.as_ptr()) }
            };

            // Skip optical discs and remote network shares
            if drive_type == DRIVE_CDROM || drive_type == 4 {
                continue;
            }

            let drive_letter = root.trim_end_matches('\\').to_uppercase();
            let is_system = drive_letter == system_drive;

            // Query volume information
            let mut volume_name_buf = [0u16; 260];
            let mut fs_name_buf = [0u16; 260];
            let mut serial_number = 0u32;
            let mut max_component_len = 0u32;
            let mut fs_flags = 0u32;

            let wide_root: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            let vol_ok = unsafe {
                GetVolumeInformationW(
                    wide_root.as_ptr(),
                    volume_name_buf.as_mut_ptr(),
                    volume_name_buf.len() as u32,
                    &mut serial_number,
                    &mut max_component_len,
                    &mut fs_flags,
                    fs_name_buf.as_mut_ptr(),
                    fs_name_buf.len() as u32,
                )
            };

            let label = if vol_ok != 0 {
                let len = volume_name_buf.iter().position(|&c| c == 0).unwrap_or(0);
                if len > 0 {
                    Some(String::from_utf16_lossy(&volume_name_buf[..len]))
                } else {
                    None
                }
            } else {
                None
            };

            let fs_type = if vol_ok != 0 {
                let len = fs_name_buf.iter().position(|&c| c == 0).unwrap_or(0);
                let name = String::from_utf16_lossy(&fs_name_buf[..len]);
                FilesystemType::from_os_str(&name)
            } else {
                FilesystemType::Unknown("Unknown".to_string())
            };

            let is_read_only = (fs_flags & FILE_READ_ONLY_VOLUME) != 0;

            // Query volume capacity & free space
            let mut free_caller = 0u64;
            let mut total_bytes = 0u64;
            let mut free_total = 0u64;

            unsafe {
                GetDiskFreeSpaceExW(
                    wide_root.as_ptr(),
                    &mut free_caller,
                    &mut total_bytes,
                    &mut free_total,
                );
            };

            let volume = LogicalVolume {
                volume_id: format!("volume-{}", drive_letter.to_lowercase()),
                mount_point: Some(drive_letter.clone()),
                label: label.clone(),
                filesystem: Filesystem {
                    fs_type,
                    label,
                    read_only: is_read_only,
                },
                capacity_bytes: total_bytes,
                free_bytes: free_caller,
                is_system_volume: is_system,
                is_boot_volume: is_system,
                read_only: is_read_only,
            };

            if let Some(disk_num) = self.get_volume_disk_number(&drive_letter) {
                disk_to_volumes.entry(disk_num).or_default().push(volume);
            } else {
                unassigned_volumes.push(volume);
            }
        }

        let mut physical_devices = Vec::new();

        // Build PhysicalDevice entries for mapped disks
        for (disk_num, volumes) in disk_to_volumes {
            let meta = self.query_physical_disk_info(disk_num);
            let has_system_volume = volumes.iter().any(|v| v.is_system_volume);

            // Compute capacity: direct geometry size if available, otherwise sum of volumes
            let total_vol_capacity: u64 = volumes.iter().map(|v| v.capacity_bytes).sum();
            let capacity_bytes = if meta.capacity_bytes > 0 {
                meta.capacity_bytes
            } else {
                total_vol_capacity
            };

            // Deduce Device Type
            let device_type = if meta.bus_type == BUS_TYPE_USB {
                DeviceType::Usb
            } else if meta.bus_type == BUS_TYPE_SD || meta.bus_type == BUS_TYPE_MMC {
                DeviceType::MemoryCard
            } else if meta.bus_type == BUS_TYPE_NVME {
                DeviceType::Ssd
            } else if let Some(incurs_penalty) = meta.incurs_seek_penalty {
                if incurs_penalty {
                    DeviceType::Hdd
                } else {
                    DeviceType::Ssd
                }
            } else if meta.removable {
                DeviceType::Usb
            } else {
                DeviceType::Unknown
            };

            // Deduce Safety Classification
            let classification = if has_system_volume {
                DeviceClassification::SystemDevice
            } else if meta.removable || device_type == DeviceType::Usb {
                DeviceClassification::RemovableDevice
            } else if meta.bus_type == BUS_TYPE_USB {
                DeviceClassification::ExternalDevice
            } else {
                DeviceClassification::FixedDataDevice
            };

            let display_name = match (&meta.vendor, &meta.model) {
                (Some(v), Some(m)) => format!("{} {}", v, m),
                (None, Some(m)) => m.clone(),
                _ => format!("Physical Drive {}", disk_num),
            };

            physical_devices.push(PhysicalDevice {
                device_id: format!("\\\\.\\PhysicalDrive{}", disk_num),
                display_name,
                vendor: meta.vendor,
                model: meta.model,
                serial_number: None, // Omitted to avoid unmasked serial exposure
                device_type,
                capacity_bytes,
                removable: meta.removable,
                read_only: false,
                is_system_device: has_system_volume,
                classification,
                volumes,
            });
        }

        // Handle unassigned volumes by grouping into virtual physical devices
        if !unassigned_volumes.is_empty() {
            for (idx, vol) in unassigned_volumes.into_iter().enumerate() {
                let is_sys = vol.is_system_volume;
                let cap = vol.capacity_bytes;
                let mount = vol
                    .mount_point
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());

                physical_devices.push(PhysicalDevice {
                    device_id: format!("\\\\.\\VolumeHost{}", idx),
                    display_name: format!("Storage Device ({})", mount),
                    vendor: None,
                    model: None,
                    serial_number: None,
                    device_type: if is_sys {
                        DeviceType::Ssd
                    } else {
                        DeviceType::Unknown
                    },
                    capacity_bytes: cap,
                    removable: false,
                    read_only: vol.read_only,
                    is_system_device: is_sys,
                    classification: if is_sys {
                        DeviceClassification::SystemDevice
                    } else {
                        DeviceClassification::FixedDataDevice
                    },
                    volumes: vec![vol],
                });
            }
        }

        // Sort: System devices first, then fixed, then removable
        physical_devices.sort_by(|a, b| {
            b.is_system_device
                .cmp(&a.is_system_device)
                .then_with(|| a.device_id.cmp(&b.device_id))
        });

        Ok(physical_devices)
    }
}
