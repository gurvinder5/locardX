use crate::models::{
    CapabilityState, DriveCapabilities, DriveCapabilitiesAssessment, DriveCapability,
    DriveSanitizationMethod,
};
use locardx_device_manager::{DeviceType, PhysicalDevice};
use std::collections::HashMap;

/// Derives storage capabilities based on physical device hardware inspection.
/// In Step 10A, hardware metadata is evaluated without executing hardware commands.
pub fn detect_device_capabilities(device: &PhysicalDevice) -> DriveCapabilities {
    let dev_id_lower = device.device_id.to_lowercase();
    let display_lower = device.display_name.to_lowercase();
    let model_lower = device.model.as_deref().unwrap_or("").to_lowercase();

    let is_nvme = dev_id_lower.contains("nvme")
        || display_lower.contains("nvme")
        || model_lower.contains("nvme")
        || display_lower.contains("samsung 980")
        || display_lower.contains("samsung 970")
        || display_lower.contains("samsung 990")
        || display_lower.contains("wd black");

    match device.device_type {
        DeviceType::Hdd => DriveCapabilities {
            supported_capabilities: vec![
                DriveCapability::SequentialWrite,
                DriveCapability::FullDeviceRead,
                DriveCapability::SectorAccess,
            ],
            interface_bus: "SATA".to_string(),
            sector_size: 512,
            is_rotational: true,
            supports_crypto_erase: false,
            supports_firmware_sanitize: false,
            supports_overwrite: true,
        },
        DeviceType::Ssd => {
            if is_nvme {
                DriveCapabilities {
                    supported_capabilities: vec![
                        DriveCapability::NvmeSanitize,
                        DriveCapability::NvmeCryptoErase,
                        DriveCapability::NvmeFormat,
                        DriveCapability::SequentialWrite,
                        DriveCapability::FullDeviceRead,
                        DriveCapability::SectorAccess,
                    ],
                    interface_bus: "NVMe".to_string(),
                    sector_size: 4096,
                    is_rotational: false,
                    supports_crypto_erase: true,
                    supports_firmware_sanitize: true,
                    supports_overwrite: true,
                }
            } else {
                DriveCapabilities {
                    supported_capabilities: vec![
                        DriveCapability::AtaSecureErase,
                        DriveCapability::AtaSanitize,
                        DriveCapability::SequentialWrite,
                        DriveCapability::FullDeviceRead,
                        DriveCapability::SectorAccess,
                    ],
                    interface_bus: "SATA".to_string(),
                    sector_size: 512,
                    is_rotational: false,
                    supports_crypto_erase: false,
                    supports_firmware_sanitize: true,
                    supports_overwrite: true,
                }
            }
        }
        DeviceType::Usb => DriveCapabilities {
            supported_capabilities: vec![
                DriveCapability::RemovableMedia,
                DriveCapability::SequentialWrite,
                DriveCapability::FullDeviceRead,
                DriveCapability::SectorAccess,
            ],
            interface_bus: "USB".to_string(),
            sector_size: 512,
            is_rotational: false,
            supports_crypto_erase: false,
            supports_firmware_sanitize: false,
            supports_overwrite: true,
        },
        DeviceType::MemoryCard => DriveCapabilities {
            supported_capabilities: vec![
                DriveCapability::RemovableMedia,
                DriveCapability::SequentialWrite,
                DriveCapability::FullDeviceRead,
                DriveCapability::SectorAccess,
            ],
            interface_bus: "SD/MMC".to_string(),
            sector_size: 512,
            is_rotational: false,
            supports_crypto_erase: false,
            supports_firmware_sanitize: false,
            supports_overwrite: true,
        },
        DeviceType::ExternalStorage => DriveCapabilities {
            supported_capabilities: vec![
                DriveCapability::RemovableMedia,
                DriveCapability::SequentialWrite,
                DriveCapability::FullDeviceRead,
                DriveCapability::SectorAccess,
            ],
            interface_bus: "External".to_string(),
            sector_size: 512,
            is_rotational: false,
            supports_crypto_erase: false,
            supports_firmware_sanitize: false,
            supports_overwrite: true,
        },
        DeviceType::Unknown => DriveCapabilities {
            supported_capabilities: vec![],
            interface_bus: "Unknown".to_string(),
            sector_size: 512,
            is_rotational: false,
            supports_crypto_erase: false,
            supports_firmware_sanitize: false,
            supports_overwrite: false,
        },
    }
}

/// Formally assesses physical device capabilities with explicit fail-closed states.
///
/// In Step 10B.1, Unknown or DetectionFailed states cause planning and execution
/// to fail closed immediately.
pub fn assess_device_capabilities(device: &PhysicalDevice) -> DriveCapabilitiesAssessment {
    let capabilities = detect_device_capabilities(device);
    let mut notes = Vec::new();

    // 1. Determine overall capability state
    let overall_state = if device.capacity_bytes == 0 || device.device_id.trim().is_empty() {
        notes
            .push("Hardware probe failure: Device capacity is 0 or identifier missing".to_string());
        CapabilityState::DetectionFailed
    } else if device.device_type == DeviceType::Unknown {
        notes.push("Unknown media mechanism: Device media type cannot be identified".to_string());
        CapabilityState::Unknown
    } else if capabilities.supported_capabilities.is_empty() {
        notes.push("No supported sanitization capabilities detected on device".to_string());
        CapabilityState::Unsupported
    } else {
        notes.push(format!(
            "Identified {} supported capabilities over {} bus",
            capabilities.supported_capabilities.len(),
            capabilities.interface_bus
        ));
        CapabilityState::Supported
    };

    // 2. Map capability states
    let all_caps = [
        DriveCapability::SequentialWrite,
        DriveCapability::FullDeviceRead,
        DriveCapability::SectorAccess,
        DriveCapability::AtaSecureErase,
        DriveCapability::AtaSanitize,
        DriveCapability::NvmeFormat,
        DriveCapability::NvmeSanitize,
        DriveCapability::NvmeCryptoErase,
        DriveCapability::RemovableMedia,
    ];

    let mut capability_states = HashMap::new();
    for cap in all_caps {
        let state = match overall_state {
            CapabilityState::DetectionFailed => CapabilityState::DetectionFailed,
            CapabilityState::Unknown => CapabilityState::Unknown,
            _ => {
                if capabilities.has_capability(cap) {
                    CapabilityState::Supported
                } else {
                    CapabilityState::Unsupported
                }
            }
        };
        capability_states.insert(cap, state);
    }

    // 3. Map sanitization method support
    let all_methods = [
        DriveSanitizationMethod::Nist80088ClearZero,
        DriveSanitizationMethod::Dod522022M,
        DriveSanitizationMethod::AtaSecureErase,
        DriveSanitizationMethod::NvmeFormatSanitize,
        DriveSanitizationMethod::NvmeCryptoErase,
        DriveSanitizationMethod::BlockZeroOverwrite,
    ];

    let mut method_support = HashMap::new();
    for method in all_methods {
        let state = match overall_state {
            CapabilityState::DetectionFailed => CapabilityState::DetectionFailed,
            CapabilityState::Unknown => CapabilityState::Unknown,
            _ => match method {
                DriveSanitizationMethod::Nist80088ClearZero
                | DriveSanitizationMethod::BlockZeroOverwrite => {
                    if capabilities.supports_overwrite {
                        CapabilityState::Supported
                    } else {
                        CapabilityState::Unsupported
                    }
                }
                DriveSanitizationMethod::Dod522022M => {
                    if capabilities.is_rotational && capabilities.supports_overwrite {
                        CapabilityState::Supported
                    } else {
                        CapabilityState::Unsupported
                    }
                }
                DriveSanitizationMethod::AtaSecureErase => {
                    if capabilities.has_capability(DriveCapability::AtaSecureErase) {
                        CapabilityState::Supported
                    } else {
                        CapabilityState::Unsupported
                    }
                }
                DriveSanitizationMethod::NvmeFormatSanitize => {
                    if capabilities.has_capability(DriveCapability::NvmeSanitize)
                        || capabilities.has_capability(DriveCapability::NvmeFormat)
                    {
                        CapabilityState::Supported
                    } else {
                        CapabilityState::Unsupported
                    }
                }
                DriveSanitizationMethod::NvmeCryptoErase => {
                    if capabilities.supports_crypto_erase {
                        CapabilityState::Supported
                    } else {
                        CapabilityState::Unsupported
                    }
                }
            },
        };
        method_support.insert(method, state);
    }

    DriveCapabilitiesAssessment {
        overall_state,
        capabilities,
        capability_states,
        method_support,
        assessment_notes: notes,
    }
}
