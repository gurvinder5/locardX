use crate::models::PhysicalDevice;
use locardx_common::LocardError;

/// Trait defining the contract for storage device discovery providers.
///
/// Implementations must adhere to the read-only invariant: no write handles,
/// no partition modifications, and no destructive commands.
pub trait DeviceDiscoveryProvider: Send + Sync {
    /// Discovers and enumerates all attached physical storage devices and their logical volumes.
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError>;
}
