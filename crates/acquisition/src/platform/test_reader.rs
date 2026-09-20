use super::traits::{AcquisitionSourceReader, ReadOnlyDeviceStream};
use crate::models::{AcquisitionDeviceSnapshot, AcquisitionFailureReason};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Fault injection modes for deterministic testing without physical storage devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultInjectionMode {
    Normal,
    FailOpen(String),
    FailReadAtOffset(u64),
    ShortReadAtOffset {
        trigger_offset: u64,
        return_bytes: usize,
    },
    SimulateDisconnect,
    MutateCapacity(u64),
    MutateSerial(String),
    Disappear,
}

/// In-memory fault-injectable acquisition reader for comprehensive non-destructive testing.
pub struct FaultInjectableAcquisitionReader {
    mode: Mutex<FaultInjectionMode>,
    devices: Mutex<HashMap<String, (AcquisitionDeviceSnapshot, Vec<u8>)>>,
    destination_mappings: Mutex<HashMap<String, String>>,
}

impl FaultInjectableAcquisitionReader {
    pub fn new() -> Self {
        Self {
            mode: Mutex::new(FaultInjectionMode::Normal),
            devices: Mutex::new(HashMap::new()),
            destination_mappings: Mutex::new(HashMap::new()),
        }
    }

    /// Creates a reader pre-loaded with a standard synthetic test device.
    pub fn new_with_test_device(device_id: &str, capacity_bytes: u64) -> Self {
        let reader = Self::new();
        let mut data = Vec::with_capacity(capacity_bytes as usize);
        // Generate deterministic test pattern
        for i in 0..capacity_bytes {
            data.push(((i % 251) ^ 0x5A) as u8);
        }

        let snapshot = AcquisitionDeviceSnapshot {
            device_id: device_id.to_string(),
            display_name: format!("Synthetic Test Disk ({})", device_id),
            vendor: Some("LocardX".to_string()),
            model: Some("TestDrive-V1".to_string()),
            serial_number: Some("TEST-SER-001".to_string()),
            media_type: "Ssd".to_string(),
            capacity_bytes,
            sector_size: 512,
            bus_type: Some("SATA".to_string()),
            is_removable: false,
            is_system: false,
            snapshot_timestamp: "2026-09-12T00:00:00Z".to_string(),
        };

        reader.register_device(snapshot, data);
        reader
    }

    pub fn set_mode(&self, mode: FaultInjectionMode) {
        *self.mode.lock().unwrap() = mode;
    }

    pub fn register_device(&self, snapshot: AcquisitionDeviceSnapshot, data: Vec<u8>) {
        let mut devs = self.devices.lock().unwrap();
        devs.insert(snapshot.device_id.clone(), (snapshot, data));
    }

    pub fn register_destination_mapping(&self, prefix: &str, physical_device_id: &str) {
        let mut mappings = self.destination_mappings.lock().unwrap();
        mappings.insert(prefix.to_lowercase(), physical_device_id.to_string());
    }

    pub fn has_device(&self, device_id: &str) -> bool {
        self.devices.lock().unwrap().contains_key(device_id)
    }
}

impl AcquisitionSourceReader for FaultInjectableAcquisitionReader {
    fn open_read_only(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ReadOnlyDeviceStream>, AcquisitionFailureReason> {
        let mode = self.mode.lock().unwrap().clone();
        if let FaultInjectionMode::FailOpen(msg) = mode {
            return Err(AcquisitionFailureReason::ReadError(msg));
        }

        let devs = self.devices.lock().unwrap();
        let (snapshot, data) = devs.get(device_id).ok_or_else(|| {
            AcquisitionFailureReason::InvalidSource(format!(
                "Device '{}' not found in test registry",
                device_id
            ))
        })?;

        Ok(Box::new(MockReadOnlyStream {
            data: data.clone(),
            total_size: snapshot.capacity_bytes,
            current_offset: 0,
            short_read_triggered: false,
            mode: Arc::new(Mutex::new(mode)),
        }))
    }

    fn query_live_snapshot(
        &self,
        device_id: &str,
    ) -> Result<AcquisitionDeviceSnapshot, AcquisitionFailureReason> {
        let mode = self.mode.lock().unwrap().clone();
        if matches!(mode, FaultInjectionMode::Disappear) {
            return Err(AcquisitionFailureReason::SourceDisconnected);
        }

        let devs = self.devices.lock().unwrap();
        let (snapshot, _) = devs.get(device_id).ok_or_else(|| {
            AcquisitionFailureReason::InvalidSource(format!(
                "Device '{}' not found in test registry",
                device_id
            ))
        })?;

        let mut live = snapshot.clone();
        match mode {
            FaultInjectionMode::MutateCapacity(new_cap) => {
                live.capacity_bytes = new_cap;
            }
            FaultInjectionMode::MutateSerial(new_ser) => {
                live.serial_number = Some(new_ser);
            }
            _ => {}
        }

        Ok(live)
    }

    fn resolve_device_for_path(&self, file_path: &str) -> Option<String> {
        let mappings = self.destination_mappings.lock().unwrap();
        let path_lower = file_path.to_lowercase();
        for (prefix, dev_id) in mappings.iter() {
            if path_lower.starts_with(prefix) {
                return Some(dev_id.clone());
            }
        }
        None
    }
}

/// In-memory mock stream backing test acquisitions.
pub struct MockReadOnlyStream {
    data: Vec<u8>,
    total_size: u64,
    current_offset: u64,
    short_read_triggered: bool,
    mode: Arc<Mutex<FaultInjectionMode>>,
}

impl ReadOnlyDeviceStream for MockReadOnlyStream {
    fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, AcquisitionFailureReason> {
        let mode = self.mode.lock().unwrap().clone();

        match mode {
            FaultInjectionMode::SimulateDisconnect => {
                return Err(AcquisitionFailureReason::SourceDisconnected);
            }
            FaultInjectionMode::FailReadAtOffset(err_off) if self.current_offset >= err_off => {
                return Err(AcquisitionFailureReason::ReadError(format!(
                    "Injected I/O error at offset {}",
                    self.current_offset
                )));
            }
            FaultInjectionMode::ShortReadAtOffset {
                trigger_offset,
                return_bytes,
            } if self.current_offset >= trigger_offset => {
                if self.short_read_triggered {
                    return Ok(0); // Premature EOF
                }
                self.short_read_triggered = true;
                let remaining =
                    (self.data.len() as u64).saturating_sub(self.current_offset) as usize;
                let to_copy = remaining.min(buffer.len()).min(return_bytes);
                if to_copy > 0 {
                    buffer[..to_copy].copy_from_slice(
                        &self.data
                            [self.current_offset as usize..self.current_offset as usize + to_copy],
                    );
                    self.current_offset += to_copy as u64;
                    return Ok(to_copy);
                }
                return Ok(0);
            }
            _ => {}
        }

        if self.current_offset >= self.data.len() as u64 {
            return Ok(0); // EOF
        }

        let remaining = (self.data.len() as u64 - self.current_offset) as usize;
        let to_copy = remaining.min(buffer.len());

        buffer[..to_copy].copy_from_slice(
            &self.data[self.current_offset as usize..self.current_offset as usize + to_copy],
        );
        self.current_offset += to_copy as u64;

        Ok(to_copy)
    }

    fn total_bytes(&self) -> u64 {
        self.total_size
    }
}
