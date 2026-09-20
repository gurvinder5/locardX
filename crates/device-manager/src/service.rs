use crate::models::{PhysicalDevice, StorageDeviceDto};
use crate::traits::DeviceDiscoveryProvider;
use locardx_common::LocardError;
use std::sync::Arc;
use tracing::info;

/// Core service managing storage device and filesystem discovery.
pub struct DeviceManagerService {
    provider: Arc<dyn DeviceDiscoveryProvider>,
}

impl DeviceManagerService {
    /// Creates a new `DeviceManagerService` with the specified discovery provider.
    pub fn new(provider: Arc<dyn DeviceDiscoveryProvider>) -> Self {
        Self { provider }
    }

    /// Initializes a default system service using platform-native discovery.
    pub fn new_system() -> Self {
        #[cfg(target_os = "windows")]
        {
            use crate::windows::WindowsDeviceProvider;
            info!("Initializing Windows platform device discovery provider");
            Self::new(Arc::new(WindowsDeviceProvider::new()))
        }
        #[cfg(not(target_os = "windows"))]
        {
            use crate::mock::MockDeviceProvider;
            info!("Initializing mock device discovery provider on non-Windows platform");
            Self::new(Arc::new(MockDeviceProvider::new_default()))
        }
    }

    /// Discovers all storage devices and volumes without modification.
    pub fn list_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        self.provider.discover_devices()
    }

    /// Discovers all storage devices and returns sanitized DTOs safe for Tauri IPC.
    pub fn list_devices_dto(&self) -> Result<Vec<StorageDeviceDto>, LocardError> {
        let devices = self.provider.discover_devices()?;
        let dtos = devices.iter().map(StorageDeviceDto::from).collect();
        Ok(dtos)
    }

    /// Refreshes device state on demand.
    pub fn refresh(&self) -> Result<Vec<StorageDeviceDto>, LocardError> {
        self.list_devices_dto()
    }

    /// Returns a reference to the underlying discovery provider.
    pub fn provider(&self) -> &Arc<dyn DeviceDiscoveryProvider> {
        &self.provider
    }
}

impl DeviceDiscoveryProvider for DeviceManagerService {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        self.provider.discover_devices()
    }
}
