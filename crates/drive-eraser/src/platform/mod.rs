//! Platform-specific physical storage device discovery and hardware capability probing implementations.
//!
//! ISOLATION INVARIANT:
//! All low-level OS storage handles (Win32 IOCTLs, Linux sysfs) are strictly isolated
//! behind the `DriveHardwareProvider` trait. Core domain logic never imports platform APIs directly.

pub mod executor;
pub mod privileges;
pub mod test_executor;

pub use executor::{DriveHardwareExecutor, ExclusiveDriveHandle};
pub use privileges::{is_elevated_admin, verify_administrative_privileges};
pub use test_executor::{FaultInjectableExecutor, FaultInjectionMode};

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub mod fallback {
    use super::executor::{DriveHardwareExecutor, ExclusiveDriveHandle};
    use crate::hardware::DriveHardwareProvider;
    use crate::models::{
        CapabilityState, DriveCapabilities, DriveCapabilitiesAssessment, DriveEraseFailureReason,
        DriveEraseProgress, DriveVerificationResult, HardwareExecutionPermit,
        PhysicalDeviceSnapshot,
    };
    use locardx_common::LocardError;
    use locardx_device_manager::PhysicalDevice;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct FallbackHardwareProvider;

    impl DriveHardwareProvider for FallbackHardwareProvider {
        fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
            Ok(Vec::new())
        }

        fn probe_capabilities(
            &self,
            _device: &PhysicalDevice,
        ) -> Result<DriveCapabilitiesAssessment, LocardError> {
            Ok(DriveCapabilitiesAssessment {
                overall_state: CapabilityState::Unknown,
                capabilities: DriveCapabilities {
                    supported_capabilities: vec![],
                    interface_bus: "UnsupportedPlatform".to_string(),
                    sector_size: 512,
                    is_rotational: false,
                    supports_crypto_erase: false,
                    supports_firmware_sanitize: false,
                    supports_overwrite: false,
                },
                capability_states: HashMap::new(),
                method_support: HashMap::new(),
                assessment_notes: vec![
                    "Platform not supported for real hardware probing".to_string()
                ],
            })
        }

        fn probe_device_snapshot(
            &self,
            _device_id: &str,
        ) -> Result<Option<PhysicalDeviceSnapshot>, LocardError> {
            Ok(None)
        }
    }

    #[derive(Default)]
    pub struct FallbackDriveHardwareExecutor;

    impl FallbackDriveHardwareExecutor {
        pub fn new() -> Self {
            Self
        }
    }

    impl DriveHardwareExecutor for FallbackDriveHardwareExecutor {
        fn acquire_exclusive_access(
            &self,
            device_id: &str,
        ) -> Result<Box<dyn ExclusiveDriveHandle>, DriveEraseFailureReason> {
            Err(DriveEraseFailureReason::PreExecutionCheckFailed(format!(
                "Platform not supported for hardware execution on '{}'",
                device_id
            )))
        }

        fn execute_sanitization(
            &self,
            _handle: &mut Box<dyn ExclusiveDriveHandle>,
            permit: &HardwareExecutionPermit,
            _is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
            _on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
        ) -> Result<(u64, f64), DriveEraseFailureReason> {
            Err(DriveEraseFailureReason::PreExecutionCheckFailed(format!(
                "Platform not supported for hardware execution on '{}'",
                permit.physical_device_id()
            )))
        }

        fn verify_sanitization(
            &self,
            _handle: &mut Box<dyn ExclusiveDriveHandle>,
            permit: &HardwareExecutionPermit,
        ) -> Result<DriveVerificationResult, DriveEraseFailureReason> {
            Err(DriveEraseFailureReason::PreExecutionCheckFailed(format!(
                "Platform not supported for hardware execution on '{}'",
                permit.physical_device_id()
            )))
        }
    }
}

#[cfg(target_os = "windows")]
pub type RealDriveHardwareProvider = windows::WindowsHardwareProvider;

#[cfg(target_os = "windows")]
pub type RealDriveHardwareExecutor = windows::WindowsDriveHardwareExecutor;

#[cfg(target_os = "linux")]
pub type RealDriveHardwareProvider = linux::LinuxHardwareProvider;

#[cfg(target_os = "linux")]
pub type RealDriveHardwareExecutor = linux::LinuxDriveHardwareExecutor;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub type RealDriveHardwareProvider = fallback::FallbackHardwareProvider;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub type RealDriveHardwareExecutor = fallback::FallbackDriveHardwareExecutor;
