use crate::models::{AcquisitionDeviceSnapshot, AcquisitionFailureReason};
use crate::platform::AcquisitionSourceReader;

/// Validates that the requested target is an acceptable physical storage device,
/// strictly rejecting logical volumes, drive letters, and mount paths.
pub fn validate_source_device_id(target: &str) -> Result<String, AcquisitionFailureReason> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(AcquisitionFailureReason::InvalidSource(
            "Device target cannot be empty".to_string(),
        ));
    }

    // Reject logical drive letters and volume paths
    if trimmed.contains(r":\")
        || trimmed.ends_with(':')
        || (trimmed.len() == 2 && trimmed.chars().nth(1) == Some(':'))
    {
        return Err(AcquisitionFailureReason::InvalidSource(format!(
            "Target '{}' is a logical drive letter or volume. Forensic acquisition operates exclusively on physical disks (e.g. \\\\.\\PhysicalDrive1 or /dev/sdb).",
            trimmed
        )));
    }

    // Reject logical filesystem paths on Unix
    if trimmed.starts_with('/') && !trimmed.starts_with("/dev/") {
        return Err(AcquisitionFailureReason::InvalidSource(format!(
            "Target '{}' is a filesystem mount path. Physical disk device node required (e.g. /dev/sdb).",
            trimmed
        )));
    }

    Ok(trimmed.to_string())
}

/// Queries the hardware reader to construct an initial immutable snapshot of the source device.
pub fn create_source_snapshot(
    reader: &dyn AcquisitionSourceReader,
    device_id: &str,
) -> Result<AcquisitionDeviceSnapshot, AcquisitionFailureReason> {
    let canonical_id = validate_source_device_id(device_id)?;
    let snapshot = reader.query_live_snapshot(&canonical_id)?;

    if snapshot.capacity_bytes == 0 {
        return Err(AcquisitionFailureReason::InvalidSource(format!(
            "Device '{}' reported 0 bytes capacity or media is unreadable",
            canonical_id
        )));
    }

    Ok(snapshot)
}

/// Revalidates the live physical device against the pre-acquisition baseline snapshot.
/// If any identity or geometry mutation has occurred (TOCTOU defense), fails closed.
pub fn revalidate_source_snapshot(
    reader: &dyn AcquisitionSourceReader,
    baseline: &AcquisitionDeviceSnapshot,
) -> Result<(), AcquisitionFailureReason> {
    let live = reader.query_live_snapshot(&baseline.device_id)?;
    baseline.detect_mutation(&live)
}
