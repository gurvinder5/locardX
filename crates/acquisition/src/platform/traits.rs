use crate::models::{AcquisitionDeviceSnapshot, AcquisitionFailureReason};

/// Read-only streaming interface for acquiring raw bytes from a source physical storage device.
/// INVARIANT: Read-only streams provide no write, seek-overwrite, or modifying capabilities.
pub trait ReadOnlyDeviceStream: Send + Sync {
    /// Reads up to `buffer.len()` bytes into `buffer`.
    /// Returns the number of bytes read (0 indicates EOF).
    fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, AcquisitionFailureReason>;

    /// Returns the total expected byte capacity of the underlying physical storage media.
    fn total_bytes(&self) -> u64;
}

/// Abstract hardware reader interface for physical storage device discovery and stream opening.
pub trait AcquisitionSourceReader: Send + Sync {
    /// Opens the specified physical device strictly with read-only permissions.
    /// Fails closed if write access would be required or if device is unreadable.
    fn open_read_only(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ReadOnlyDeviceStream>, AcquisitionFailureReason>;

    /// Queries live hardware state to construct a fresh snapshot for TOCTOU validation.
    fn query_live_snapshot(
        &self,
        device_id: &str,
    ) -> Result<AcquisitionDeviceSnapshot, AcquisitionFailureReason>;

    /// Resolves which physical storage device identifier (e.g. `\\.\PhysicalDrive0`) hosts the given file/directory path.
    /// Used to enforce destination collision safety (destination must not reside on the source physical disk).
    fn resolve_device_for_path(&self, file_path: &str) -> Option<String>;
}
