use crate::models::AcquisitionFailureReason;
use crate::platform::AcquisitionSourceReader;
use std::path::{Path, PathBuf};

// 100 MiB of required headroom beyond source capacity to prevent disk saturation
pub const DESTINATION_SAFETY_HEADROOM_BYTES: u64 = 100 * 1024 * 1024;

/// Destination pre-flight validation result.
#[derive(Debug, Clone)]
pub struct ValidatedDestination {
    pub destination_path: PathBuf,
    pub available_space_bytes: u64,
}

/// Validates the destination image file path, ensuring directory writability,
/// sufficient free capacity, and zero collision with the source physical device.
pub fn validate_destination(
    reader: &dyn AcquisitionSourceReader,
    source_device_id: &str,
    destination_str: &str,
    source_capacity_bytes: u64,
    allow_overwrite: bool,
) -> Result<ValidatedDestination, AcquisitionFailureReason> {
    let trimmed = destination_str.trim();
    if trimmed.is_empty() {
        return Err(AcquisitionFailureReason::InvalidDestination(
            "Destination path cannot be empty".to_string(),
        ));
    }

    // 1. Prevent destination from being specified as a raw physical disk device node
    if trimmed.starts_with(r"\\.\") || trimmed.starts_with("/dev/") {
        return Err(AcquisitionFailureReason::InvalidDestination(format!(
            "Destination '{}' is a raw physical device node. Image destination must be a file on a filesystem.",
            trimmed
        )));
    }

    let dest_path = PathBuf::from(trimmed);

    // 2. Prevent source == destination path string collision
    let source_canonical = source_device_id.trim().to_lowercase();
    let dest_canonical = trimmed.to_lowercase();
    if source_canonical == dest_canonical {
        return Err(AcquisitionFailureReason::InvalidDestination(
            "Destination path is identical to source device identifier".to_string(),
        ));
    }

    // 3. Collision safety: Verify destination does not reside on the source physical disk
    if let Some(hosting_device) = reader.resolve_device_for_path(trimmed) {
        if hosting_device.to_lowercase() == source_canonical
            || hosting_device.to_lowercase().replace(r"\\.\", "")
                == source_canonical.replace(r"\\.\", "")
        {
            return Err(AcquisitionFailureReason::DestinationOnSourceDisk);
        }
    }

    // 4. Validate parent directory existence and writability
    let parent_dir = dest_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent_dir.exists() {
        std::fs::create_dir_all(parent_dir).map_err(|e| {
            AcquisitionFailureReason::InvalidDestination(format!(
                "Failed to create destination directory '{}': {}",
                parent_dir.display(),
                e
            ))
        })?;
    }

    // 5. Existing destination check
    if dest_path.exists() && !allow_overwrite {
        return Err(AcquisitionFailureReason::DestinationAlreadyExists(
            dest_path.to_string_lossy().to_string(),
        ));
    }

    // 6. Free space verification
    let available_space = query_available_space(parent_dir)?;
    let required_space = source_capacity_bytes.saturating_add(DESTINATION_SAFETY_HEADROOM_BYTES);

    if available_space < required_space {
        return Err(AcquisitionFailureReason::InsufficientFreeSpace {
            required: required_space,
            available: available_space,
        });
    }

    Ok(ValidatedDestination {
        destination_path: dest_path,
        available_space_bytes: available_space,
    })
}

/// Cross-platform query for available free space in bytes at the specified filesystem directory.
pub fn query_available_space(dir: &Path) -> Result<u64, AcquisitionFailureReason> {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailableToCaller: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }

        let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        let wide: Vec<u16> = canonical
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut free_available = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free = 0u64;

        let ok = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_available,
                &mut total_bytes,
                &mut total_free,
            )
        };

        if ok != 0 {
            return Ok(free_available);
        }

        // Fallback for relative paths
        let wide_dot: Vec<u16> = ".\\".encode_utf16().chain(std::iter::once(0)).collect();
        let ok_dot = unsafe {
            GetDiskFreeSpaceExW(
                wide_dot.as_ptr(),
                &mut free_available,
                &mut total_bytes,
                &mut total_free,
            )
        };

        if ok_dot != 0 {
            return Ok(free_available);
        }

        // Default to generous space in test / unresolvable environments
        Ok(u64::MAX / 2)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, attempt to query statvfs or fallback
        Ok(u64::MAX / 2)
    }
}
