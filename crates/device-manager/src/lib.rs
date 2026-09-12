//! LocardX Device Manager
//!
//! Provides read-only discovery and enumeration of physical storage devices,
//! logical volumes, and filesystems without modifying partitions, files, or disk metadata.

pub mod mock;
pub mod models;
pub mod service;
pub mod traits;

#[cfg(target_os = "windows")]
pub mod windows;

pub use mock::MockDeviceProvider;
pub use models::*;
pub use service::DeviceManagerService;
pub use traits::DeviceDiscoveryProvider;

#[cfg(target_os = "windows")]
pub use windows::WindowsDeviceProvider;
