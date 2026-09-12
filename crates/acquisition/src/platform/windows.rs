use super::traits::{AcquisitionSourceReader, ReadOnlyDeviceStream};
use crate::models::{AcquisitionDeviceSnapshot, AcquisitionFailureReason};
use std::ffi::c_void;
use std::path::Path;

// Win32 Constants
const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
const GENERIC_READ: u32 = 0x80000000;
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x08000000;

// IOCTL Codes
const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x00560000;
const IOCTL_DISK_GET_DRIVE_GEOMETRY_EX: u32 = 0x000700A0;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;

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

extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut c_void,
    ) -> *mut c_void;

    fn ReadFile(
        hFile: *mut c_void,
        lpBuffer: *mut c_void,
        nNumberOfBytesToRead: u32,
        lpNumberOfBytesRead: *mut u32,
        lpOverlapped: *mut c_void,
    ) -> i32;

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
    fn GetLastError() -> u32;
}

/// Windows implementation of the read-only forensic acquisition reader.
#[derive(Default)]
pub struct WindowsAcquisitionReader;

impl WindowsAcquisitionReader {
    pub fn new() -> Self {
        Self
    }
}

impl AcquisitionSourceReader for WindowsAcquisitionReader {
    fn open_read_only(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ReadOnlyDeviceStream>, AcquisitionFailureReason> {
        // Enforce physical device naming format
        let canonical_id = if !device_id.starts_with(r"\\.\") {
            format!(r"\\.\{}", device_id)
        } else {
            device_id.to_string()
        };

        let wide_path: Vec<u16> = canonical_id
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // Open strictly with GENERIC_READ. Zero write flags.
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_SEQUENTIAL_SCAN,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let err = unsafe { GetLastError() };
            return Err(AcquisitionFailureReason::ReadError(format!(
                "Failed to open source physical device '{}' for read-only access: Win32 error {}",
                device_id, err
            )));
        }

        // Query capacity from the opened handle
        let mut geom_ex: DiskGeometryEx = unsafe { std::mem::zeroed() };
        let mut bytes_returned = 0u32;
        let ok = unsafe {
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

        let total_size = if ok != 0 && geom_ex.disk_size > 0 {
            geom_ex.disk_size as u64
        } else {
            0
        };

        Ok(Box::new(WindowsReadOnlyStream {
            handle,
            total_size,
            current_offset: 0,
        }))
    }

    fn query_live_snapshot(
        &self,
        device_id: &str,
    ) -> Result<AcquisitionDeviceSnapshot, AcquisitionFailureReason> {
        let canonical_id = if !device_id.starts_with(r"\\.\") {
            format!(r"\\.\{}", device_id)
        } else {
            device_id.to_string()
        };

        let wide_path: Vec<u16> = canonical_id
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // Open query-only handle (dwDesiredAccess = 0)
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let _err = unsafe { GetLastError() };
            return Err(AcquisitionFailureReason::SourceDisconnected);
        }

        // 1. Query Geometry
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

        let (capacity_bytes, sector_size) = if geom_ok != 0 && geom_ex.disk_size > 0 {
            (geom_ex.disk_size as u64, geom_ex.geometry.bytes_per_sector)
        } else {
            (0, 512)
        };

        // 2. Query Device Properties
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

        let mut vendor = None;
        let mut product = None;
        let mut serial = None;
        let mut bus_type = None;
        let mut is_removable = false;

        if desc_ok != 0 && bytes_returned >= 24 {
            let vendor_offset =
                u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]) as usize;
            let product_offset =
                u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]) as usize;
            let serial_offset = if bytes_returned >= 28 {
                u32::from_le_bytes([buffer[24], buffer[25], buffer[26], buffer[27]]) as usize
            } else {
                0
            };
            let bus_type_id = if bytes_returned >= 32 { buffer[28] } else { 0 };

            is_removable = buffer[12] != 0;

            if vendor_offset > 0 && vendor_offset < buffer.len() {
                if let Some(s) = parse_null_string(&buffer[vendor_offset..]) {
                    vendor = Some(s);
                }
            }
            if product_offset > 0 && product_offset < buffer.len() {
                if let Some(s) = parse_null_string(&buffer[product_offset..]) {
                    product = Some(s);
                }
            }
            if serial_offset > 0 && serial_offset < buffer.len() {
                if let Some(s) = parse_null_string(&buffer[serial_offset..]) {
                    serial = Some(s);
                }
            }

            bus_type = Some(match bus_type_id {
                7 => "USB".to_string(),
                11 => "SD".to_string(),
                12 => "MMC".to_string(),
                17 => "NVMe".to_string(),
                3 => "ATA".to_string(),
                8 => "SATA".to_string(),
                _ => "SCSI".to_string(),
            });
        }

        unsafe { CloseHandle(handle) };

        let display_name = format!(
            "{} {}",
            vendor.as_deref().unwrap_or(""),
            product.as_deref().unwrap_or("Storage Device")
        )
        .trim()
        .to_string();

        let is_system = canonical_id.to_lowercase().contains("physicaldrive0");

        Ok(AcquisitionDeviceSnapshot {
            device_id: canonical_id,
            display_name: if display_name.is_empty() {
                "Physical Drive".to_string()
            } else {
                display_name
            },
            vendor,
            model: product,
            serial_number: serial,
            media_type: if is_removable {
                "Removable".to_string()
            } else {
                "Fixed".to_string()
            },
            capacity_bytes,
            sector_size: if sector_size == 0 { 512 } else { sector_size },
            bus_type,
            is_removable,
            is_system,
            snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn resolve_device_for_path(&self, file_path: &str) -> Option<String> {
        let p = Path::new(file_path);
        let root = p.components().next()?;
        let root_str = root.as_os_str().to_string_lossy();

        if let Some(colon_pos) = root_str.find(':') {
            let drive_letter = &root_str[..colon_pos + 1];
            let vol_device_path = format!(r"\\.\{}", drive_letter);
            let wide: Vec<u16> = vol_device_path
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    0, // query only
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
            let ok = unsafe {
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

            if ok != 0 && extents.number_of_disk_extents > 0 {
                let disk_num = extents.extents[0].disk_number;
                return Some(format!(r"\\.\PhysicalDrive{}", disk_num));
            }
        }

        None
    }
}

/// Windows native read-only device stream.
pub struct WindowsReadOnlyStream {
    handle: *mut c_void,
    total_size: u64,
    current_offset: u64,
}

impl ReadOnlyDeviceStream for WindowsReadOnlyStream {
    fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, AcquisitionFailureReason> {
        let mut bytes_read = 0u32;
        let to_read = buffer.len().min(u32::MAX as usize) as u32;

        let ok = unsafe {
            ReadFile(
                self.handle,
                buffer.as_mut_ptr() as *mut c_void,
                to_read,
                &mut bytes_read,
                std::ptr::null_mut(),
            )
        };

        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == 38 {
                // ERROR_HANDLE_EOF
                return Ok(0);
            }
            return Err(AcquisitionFailureReason::ReadError(format!(
                "ReadFile failed at offset {}: Win32 error {}",
                self.current_offset, err
            )));
        }

        self.current_offset += bytes_read as u64;
        Ok(bytes_read as usize)
    }

    fn total_bytes(&self) -> u64 {
        self.total_size
    }
}

impl Drop for WindowsReadOnlyStream {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE && !self.handle.is_null() {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

unsafe impl Send for WindowsReadOnlyStream {}
unsafe impl Sync for WindowsReadOnlyStream {}

fn parse_null_string(slice: &[u8]) -> Option<String> {
    let len = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    let s = String::from_utf8_lossy(&slice[..len]).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
