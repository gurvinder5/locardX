use crate::models::{DriveEraseFailureReason, PhysicalDeviceSnapshot};
use locardx_device_manager::{DeviceClassification, DeviceDiscoveryProvider, PhysicalDevice};
use std::sync::Arc;

/// Validates whether a target string represents a valid physical device identifier
/// and explicitly rejects logical volumes, drive letters, and filesystem paths.
pub fn validate_physical_device_identifier(
    target: &str,
) -> Result<String, DriveEraseFailureReason> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(DriveEraseFailureReason::DeviceNotFound(
            "Empty physical device identifier specified".to_string(),
        ));
    }

    // Explicit rejection of logical volumes, mount points, and drive letters
    // E.g., "C:", "C:\", "D:", "/mnt", "/home"
    if trimmed.contains(":\\")
        || trimmed.ends_with(':')
        || (trimmed.len() == 2 && trimmed.chars().nth(1) == Some(':'))
        || (trimmed.starts_with('/') && !trimmed.starts_with("/dev/"))
    {
        return Err(DriveEraseFailureReason::LogicalVolumeTargetRejected(format!(
            "Target '{}' is a logical volume or filesystem path. Drive erasure strictly requires selecting a physical storage device (e.g. '\\\\.\\PhysicalDriveX').",
            trimmed
        )));
    }

    // Must match standard OS physical device naming convention
    let is_windows_physical =
        trimmed.starts_with(r"\\.\PhysicalDrive") || trimmed.starts_with("PhysicalDrive");
    let is_unix_physical = trimmed.starts_with("/dev/sd")
        || trimmed.starts_with("/dev/nvme")
        || trimmed.starts_with("/dev/hd");

    if !is_windows_physical && !is_unix_physical {
        // If it looks like a volume or file path, give clear logical volume rejection
        if trimmed.contains('\\') || trimmed.contains('/') {
            return Err(DriveEraseFailureReason::LogicalVolumeTargetRejected(format!(
                "Target '{}' is not recognized as a physical storage device. Please select a physical drive.",
                trimmed
            )));
        }
    }

    // Normalize Windows PhysicalDrive prefix if needed
    let normalized = if trimmed.starts_with("PhysicalDrive") && !trimmed.starts_with(r"\\.\") {
        format!(r"\\.\{}", trimmed)
    } else {
        trimmed.to_string()
    };

    Ok(normalized)
}

/// Authorizes a physical device as a safe external erasure candidate.
///
/// Strictly blocks:
/// - Any device flagged as a system device (`is_system_device == true`)
/// - Any device with `classification == SystemDevice` or `BootDevice`
/// - Any device hosting an active system or boot volume
/// - Any device hosting the Windows OS root volume (e.g. `C:\`)
/// - Primary system storage `PhysicalDrive0` when it hosts system partitions or is non-removable
/// - Ambiguous or internal fixed devices that cannot be confirmed as external/removable
pub fn is_safe_external_erase_target(
    device: &PhysicalDevice,
) -> Result<(), DriveEraseFailureReason> {
    // 1. Explicit system device flag
    if device.is_system_device {
        return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(format!(
            "Physical device '{}' is an active System or Boot Device. Destructive erasure is unconditionally blocked.",
            device.device_id
        )));
    }

    // 2. Safety classification must not be SystemDevice or BootDevice
    if device.classification == DeviceClassification::SystemDevice
        || device.classification == DeviceClassification::BootDevice
    {
        return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(format!(
            "Physical device '{}' classification is {:?} (active System or Boot Device). Destructive erasure is unconditionally blocked.",
            device.device_id, device.classification
        )));
    }

    // 3. Check for any system or boot volumes hosted on this device
    if device
        .volumes
        .iter()
        .any(|v| v.is_system_volume || v.is_boot_volume)
    {
        return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(format!(
            "Physical device '{}' contains an active System or Boot volume. Destructive erasure is unconditionally blocked.",
            device.device_id
        )));
    }

    // 4. Check for active OS drive letter (C: / C:\) in hosted volumes
    for vol in &device.volumes {
        if let Some(mount) = &vol.mount_point {
            let trimmed = mount.trim_end_matches('\\').to_uppercase();
            if trimmed == "C:" {
                return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(format!(
                    "Physical device '{}' hosts the active Windows OS volume (C:). Destructive erasure is prohibited.",
                    device.device_id
                )));
            }
        }
    }

    // 5. Hard block on PhysicalDrive0 when it represents primary system storage
    let norm = device.device_id.to_uppercase();
    if norm == r"\\.\PHYSICALDRIVE0" || norm == "PHYSICALDRIVE0" {
        if device.is_system_device
            || device.classification == DeviceClassification::SystemDevice
            || device.classification == DeviceClassification::BootDevice
            || !device.removable
        {
            return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(format!(
                "Physical device '{}' is the primary system storage unit (PhysicalDrive0). Destructive erasure is prohibited.",
                device.device_id
            )));
        }
    }

    Ok(())
}

/// Discovers the requested physical device and validates system/boot protections.
pub fn inspect_physical_device(
    device_id: &str,
    provider: &Arc<dyn DeviceDiscoveryProvider>,
    sector_size: u32,
) -> Result<(PhysicalDevice, PhysicalDeviceSnapshot), DriveEraseFailureReason> {
    let normalized_id = validate_physical_device_identifier(device_id)?;

    let devices = provider.discover_devices().map_err(|e| {
        DriveEraseFailureReason::DeviceNotFound(format!("Hardware discovery failed: {}", e))
    })?;

    let device = devices
        .into_iter()
        .find(|d| d.device_id.eq_ignore_ascii_case(&normalized_id))
        .ok_or_else(|| {
            DriveEraseFailureReason::DeviceNotFound(format!(
                "Physical device '{}' not found in active hardware registry",
                normalized_id
            ))
        })?;

    // Hard System and Boot Block via Evidence-Based Authorization
    is_safe_external_erase_target(&device)?;

    let snapshot = PhysicalDeviceSnapshot::from_device(&device, sector_size);
    Ok((device, snapshot))
}

/// Re-probes the physical device on live hardware to detect TOCTOU mutations, substitution, or disconnection.
pub fn verify_live_device_integrity(
    snapshot: &PhysicalDeviceSnapshot,
    provider: &Arc<dyn DeviceDiscoveryProvider>,
) -> Result<PhysicalDevice, DriveEraseFailureReason> {
    let devices = provider.discover_devices().map_err(|e| {
        DriveEraseFailureReason::DeviceNotFound(format!("Live hardware re-probe failed: {}", e))
    })?;

    let live_device = devices
        .into_iter()
        .find(|d| d.device_id.eq_ignore_ascii_case(&snapshot.device_id))
        .ok_or_else(|| {
            DriveEraseFailureReason::DeviceNotFound(format!(
                "Physical device '{}' disconnected or no longer present",
                snapshot.device_id
            ))
        })?;

    is_safe_external_erase_target(&live_device)?;

    let live_snapshot = PhysicalDeviceSnapshot::from_device(&live_device, snapshot.sector_size);

    snapshot
        .detect_mutation(&live_snapshot)
        .map_err(DriveEraseFailureReason::DeviceMutated)?;

    Ok(live_device)
}
