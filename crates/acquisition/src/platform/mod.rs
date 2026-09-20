pub mod test_reader;
pub mod traits;

pub use test_reader::{FaultInjectableAcquisitionReader, FaultInjectionMode};
pub use traits::{AcquisitionSourceReader, ReadOnlyDeviceStream};

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub type RealAcquisitionReader = windows::WindowsAcquisitionReader;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub type RealAcquisitionReader = linux::LinuxAcquisitionReader;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub type RealAcquisitionReader = test_reader::FaultInjectableAcquisitionReader;

use crate::models::{AcquisitionDeviceSnapshot, AcquisitionFailureReason};
use std::sync::Arc;

/// Composite reader that delegates to a mock reader for registered simulated devices,
/// and falls back to the native OS `RealAcquisitionReader` for real physical devices.
pub struct CompositeAcquisitionReader {
    pub real: RealAcquisitionReader,
    pub mock: Option<Arc<FaultInjectableAcquisitionReader>>,
}

impl CompositeAcquisitionReader {
    pub fn new(
        real: RealAcquisitionReader,
        mock: Option<Arc<FaultInjectableAcquisitionReader>>,
    ) -> Self {
        Self { real, mock }
    }
}

impl AcquisitionSourceReader for CompositeAcquisitionReader {
    fn open_read_only(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ReadOnlyDeviceStream>, AcquisitionFailureReason> {
        if let Some(m) = &self.mock {
            if m.has_device(device_id) {
                return m.open_read_only(device_id);
            }
        }
        self.real.open_read_only(device_id)
    }

    fn query_live_snapshot(
        &self,
        device_id: &str,
    ) -> Result<AcquisitionDeviceSnapshot, AcquisitionFailureReason> {
        if let Some(m) = &self.mock {
            if m.has_device(device_id) {
                return m.query_live_snapshot(device_id);
            }
        }
        self.real.query_live_snapshot(device_id)
    }

    fn resolve_device_for_path(&self, file_path: &str) -> Option<String> {
        if let Some(m) = &self.mock {
            if let Some(id) = m.resolve_device_for_path(file_path) {
                return Some(id);
            }
        }
        self.real.resolve_device_for_path(file_path)
    }
}
