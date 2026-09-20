use super::traits::{AcquisitionSourceReader, ReadOnlyDeviceStream};
use crate::models::{AcquisitionDeviceSnapshot, AcquisitionFailureReason};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Linux implementation of the read-only forensic acquisition reader.
#[derive(Default)]
pub struct LinuxAcquisitionReader;

impl LinuxAcquisitionReader {
    pub fn new() -> Self {
        Self
    }
}

impl AcquisitionSourceReader for LinuxAcquisitionReader {
    fn open_read_only(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ReadOnlyDeviceStream>, AcquisitionFailureReason> {
        let file = File::open(device_id).map_err(|e| {
            AcquisitionFailureReason::ReadError(format!(
                "Failed to open device '{}' read-only: {}",
                device_id, e
            ))
        })?;

        let total_size = match std::fs::metadata(device_id) {
            Ok(m) => m.len(),
            Err(_) => 0,
        };

        Ok(Box::new(LinuxReadOnlyStream { file, total_size }))
    }

    fn query_live_snapshot(
        &self,
        device_id: &str,
    ) -> Result<AcquisitionDeviceSnapshot, AcquisitionFailureReason> {
        let p = Path::new(device_id);
        if !p.exists() {
            return Err(AcquisitionFailureReason::SourceDisconnected);
        }

        let dev_name = p.file_name().and_then(|f| f.to_str()).unwrap_or("unknown");
        let sysfs_base = format!("/sys/block/{}", dev_name);
        let sysfs = Path::new(&sysfs_base);

        let size_sectors = std::fs::read_to_string(sysfs.join("size"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        let capacity_bytes = size_sectors.saturating_mul(512);

        let model = std::fs::read_to_string(sysfs.join("device/model"))
            .ok()
            .map(|s| s.trim().to_string());
        let vendor = std::fs::read_to_string(sysfs.join("device/vendor"))
            .ok()
            .map(|s| s.trim().to_string());
        let serial = std::fs::read_to_string(sysfs.join("device/serial"))
            .ok()
            .map(|s| s.trim().to_string());
        let removable = std::fs::read_to_string(sysfs.join("removable"))
            .ok()
            .map(|s| s.trim() == "1")
            .unwrap_or(false);

        Ok(AcquisitionDeviceSnapshot {
            device_id: device_id.to_string(),
            display_name: format!(
                "{} {}",
                vendor.as_deref().unwrap_or(""),
                model.as_deref().unwrap_or(dev_name)
            )
            .trim()
            .to_string(),
            vendor,
            model,
            serial_number: serial,
            media_type: if removable {
                "Removable".to_string()
            } else {
                "Fixed".to_string()
            },
            capacity_bytes,
            sector_size: 512,
            bus_type: None,
            is_removable: removable,
            is_system: false,
            snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn resolve_device_for_path(&self, file_path: &str) -> Option<String> {
        if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
            let target_p = Path::new(file_path);
            let mut longest_mount: Option<(String, String)> = None;

            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let dev = parts[0];
                    let mount = parts[1];
                    if target_p.starts_with(mount) {
                        if longest_mount
                            .as_ref()
                            .map_or(true, |m| mount.len() > m.1.len())
                        {
                            longest_mount = Some((dev.to_string(), mount.to_string()));
                        }
                    }
                }
            }

            return longest_mount.map(|(dev, _)| dev);
        }
        None
    }
}

pub struct LinuxReadOnlyStream {
    file: File,
    total_size: u64,
}

impl ReadOnlyDeviceStream for LinuxReadOnlyStream {
    fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, AcquisitionFailureReason> {
        self.file
            .read(buffer)
            .map_err(|e| AcquisitionFailureReason::ReadError(format!("Read failure: {}", e)))
    }

    fn total_bytes(&self) -> u64 {
        self.total_size
    }
}
