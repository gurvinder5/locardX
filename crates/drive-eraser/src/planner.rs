use crate::models::{
    DriveCapabilities, DriveEraseFailureReason, DriveErasePlan, DriveSanitizationMethod,
    DriveVerificationPlan, DriveVerificationStrategy, ExecutionMode, PhysicalDeviceSnapshot,
};
use chrono::Utc;
use locardx_device_manager::DeviceType;
use locardx_security::RiskLevel;
use uuid::Uuid;

/// Generates a deterministic, media-aware sanitization and verification plan for a physical storage drive.
pub fn generate_drive_erase_plan(
    snapshot: PhysicalDeviceSnapshot,
    capabilities: DriveCapabilities,
    requested_method: Option<&str>,
    execution_mode: ExecutionMode,
) -> Result<DriveErasePlan, DriveEraseFailureReason> {
    // Invariant: Real hardware execution is strictly forbidden in Step 10A
    if execution_mode == ExecutionMode::RealHardware {
        return Err(DriveEraseFailureReason::RealHardwareExecutionDisabled(
            "CRITICAL INVARIANT: Real hardware execution is permanently disabled in Step 10A; only Simulation is permitted.".to_string(),
        ));
    }

    // 1. Unknown media rejection
    if snapshot.media_type == DeviceType::Unknown || capabilities.is_empty() {
        return Err(DriveEraseFailureReason::UnsupportedMedia(format!(
            "Device '{}' has unknown media type or undetectable hardware capabilities. Erasure is rejected.",
            snapshot.device_id
        )));
    }

    // 2. Select sanitization method & passes based on media type and capabilities
    let (method, passes, verification_plan, limitations) = match snapshot.media_type {
        DeviceType::Hdd => {
            let requested_dod = requested_method
                .map(|m| m.eq_ignore_ascii_case("dod522022m") || m.eq_ignore_ascii_case("dod"))
                .unwrap_or(false);

            let (method, passes) = if requested_dod {
                (DriveSanitizationMethod::Dod522022M, 3)
            } else {
                (DriveSanitizationMethod::Nist80088ClearZero, 1)
            };

            let v_plan = DriveVerificationPlan {
                strategy: DriveVerificationStrategy::FullDeviceReadVerify,
                sample_percentage: Some(100.0),
                requirements: "Sequential readback of all logical block addresses verifying 0x00 null bytes".to_string(),
                limitations: vec![
                    "Reallocated bad sectors (G-List / P-List) cannot be verified via standard ATA read commands.".to_string(),
                    "Host controller write cache must be flushed prior to verification pass.".to_string(),
                ],
            };

            let lims = vec![
                "Logical sector overwrite targets addressable LBA range only.".to_string(),
                "Host Protected Area (HPA) and Device Configuration Overlay (DCO) require explicit unlocking.".to_string(),
                "Reallocated defect sectors remain physically preserved on platter surfaces.".to_string(),
            ];

            (method, passes, v_plan, lims)
        }
        DeviceType::Ssd => {
            if capabilities.interface_bus == "NVMe" {
                let prefer_crypto = requested_method
                    .map(|m| {
                        m.eq_ignore_ascii_case("nvmecryptoerase")
                            || m.eq_ignore_ascii_case("crypto")
                    })
                    .unwrap_or(false)
                    || capabilities.supports_crypto_erase;

                let (method, v_strat, v_reqs) = if prefer_crypto {
                    (
                        DriveSanitizationMethod::NvmeCryptoErase,
                        DriveVerificationStrategy::CryptoKeyDestructionCheck,
                        "Verify media encryption key destruction via NVMe controller log page 0x15"
                            .to_string(),
                    )
                } else {
                    (
                        DriveSanitizationMethod::NvmeFormatSanitize,
                        DriveVerificationStrategy::FirmwareStatusVerify,
                        "Verify sanitize status completion in NVMe Sanitize Progress log page"
                            .to_string(),
                    )
                };

                let v_plan = DriveVerificationPlan {
                    strategy: v_strat,
                    sample_percentage: None,
                    requirements: v_reqs,
                    limitations: vec![
                        "Controller firmware log status is authoritative; raw NAND flash cells are inaccessible across NVMe bus.".to_string(),
                    ],
                };

                let lims = vec![
                    "NVMe Sanitize / Crypto Erase executes at firmware level across all namespaces.".to_string(),
                    "Flash Translation Layer (FTL) over-provisioned blocks and wear-leveling reserves are sanitized internally.".to_string(),
                    "DoD 5220.22-M overwrite is prohibited on NVMe due to flash wear amplification and FTL evasion.".to_string(),
                ];

                (method, 1, v_plan, lims)
            } else {
                // SATA SSD
                let method = DriveSanitizationMethod::AtaSecureErase;
                let v_plan = DriveVerificationPlan {
                    strategy: DriveVerificationStrategy::FirmwareStatusVerify,
                    sample_percentage: None,
                    requirements: "Verify ATA command completion register and IDENTIFY DEVICE security status".to_string(),
                    limitations: vec![
                        "ATA controller firmware status is authoritative; physical NAND flash blocks cannot be addressed individually.".to_string(),
                    ],
                };

                let lims = vec![
                    "ATA Secure Erase instructs drive controller to purge internal flash translation tables and erase NAND blocks.".to_string(),
                    "Wear leveling spare area and retired blocks are purged by controller internal firmware routine.".to_string(),
                    "Multi-pass pattern overwriting is avoided on SSDs to prevent unnecessary write amplification.".to_string(),
                ];

                (method, 1, v_plan, lims)
            }
        }
        DeviceType::Usb | DeviceType::MemoryCard | DeviceType::ExternalStorage => {
            let method = DriveSanitizationMethod::Nist80088ClearZero;
            let v_plan = DriveVerificationPlan {
                strategy: DriveVerificationStrategy::SampledSectorVerification,
                sample_percentage: Some(10.0),
                requirements: "Sampled pseudorandom sector read verification across start, middle, and end LBA ranges".to_string(),
                limitations: vec![
                    "USB mass storage bridge chips may translate commands and prevent direct controller access.".to_string(),
                    "Consumer USB flash media lacks standardized firmware sanitize logging.".to_string(),
                ],
            };

            let lims = vec![
                "Removable USB and memory card controllers perform low-cost wear leveling.".to_string(),
                "Physical flash cells outside logical addressing may retain residual charge patterns.".to_string(),
            ];

            (method, 1, v_plan, lims)
        }
        DeviceType::Unknown => unreachable!(),
    };

    // 3. Risk Assessment
    let risk_level = if snapshot.is_removable {
        RiskLevel::High
    } else {
        RiskLevel::High
    };

    let plan_id = format!("dplan-{}", Uuid::new_v4());

    Ok(DriveErasePlan {
        plan_id,
        physical_device_id: snapshot.device_id.clone(),
        display_name: snapshot.display_name.clone(),
        vendor: snapshot.vendor.clone(),
        model: snapshot.model.clone(),
        serial_number: snapshot.serial_number.clone(),
        media_type: snapshot.media_type,
        capacity_bytes: snapshot.capacity_bytes,
        sector_size: capabilities.sector_size,
        method,
        passes,
        risk_level,
        capabilities,
        verification_plan,
        limitations,
        device_snapshot: snapshot,
        execution_mode,
        created_at: Utc::now().to_rfc3339(),
    })
}

/// Generates a validated, media-aware plan for real hardware physical device sanitization.
pub fn generate_real_hardware_drive_erase_plan(
    snapshot: PhysicalDeviceSnapshot,
    capabilities: DriveCapabilities,
    requested_method: Option<&str>,
) -> Result<DriveErasePlan, DriveEraseFailureReason> {
    // 1. Target checks: Unknown media rejection
    if snapshot.media_type == DeviceType::Unknown || capabilities.is_empty() {
        return Err(DriveEraseFailureReason::UnsupportedMedia(format!(
            "Device '{}' has unknown media type or undetectable hardware capabilities. Erasure is rejected.",
            snapshot.device_id
        )));
    }

    // 2. System and boot hard blocks
    if snapshot.is_system || snapshot.is_boot {
        return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
            format!(
            "Device '{}' is an active system or boot device; sanitization plan cannot be generated",
            snapshot.device_id
        ),
        ));
    }

    if snapshot.classification == locardx_device_manager::DeviceClassification::SystemDevice
        || snapshot.classification == locardx_device_manager::DeviceClassification::BootDevice
    {
        return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
            format!(
                "Device '{}' classification is {:?}; sanitization plan cannot be generated",
                snapshot.device_id, snapshot.classification
            ),
        ));
    }

    // 3. Write-protection check
    if snapshot.is_read_only {
        return Err(DriveEraseFailureReason::PreExecutionCheckFailed(format!(
            "Device '{}' is write-protected or read-only",
            snapshot.device_id
        )));
    }

    let (method, passes, verification_plan, limitations) = match snapshot.media_type {
        DeviceType::Hdd => {
            let requested_dod = requested_method
                .map(|m| m.eq_ignore_ascii_case("dod522022m") || m.eq_ignore_ascii_case("dod"))
                .unwrap_or(false);

            let (method, passes) = if requested_dod {
                (DriveSanitizationMethod::Dod522022M, 3)
            } else {
                (DriveSanitizationMethod::Nist80088ClearZero, 1)
            };

            let v_plan = DriveVerificationPlan {
                strategy: DriveVerificationStrategy::FullDeviceReadVerify,
                sample_percentage: Some(100.0),
                requirements: "Sequential readback of all logical block addresses verifying 0x00 null bytes".to_string(),
                limitations: vec![
                    "Reallocated bad sectors (G-List / P-List) cannot be verified via standard ATA read commands.".to_string(),
                    "Host controller write cache must be flushed prior to verification pass.".to_string(),
                ],
            };

            let lims = vec![
                "Logical sector overwrite targets addressable LBA range only.".to_string(),
                "Host Protected Area (HPA) and Device Configuration Overlay (DCO) require explicit unlocking.".to_string(),
                "Reallocated defect sectors remain physically preserved on platter surfaces.".to_string(),
            ];

            (method, passes, v_plan, lims)
        }
        DeviceType::Ssd => {
            if capabilities.interface_bus == "NVMe" {
                let prefer_crypto = requested_method
                    .map(|m| {
                        m.eq_ignore_ascii_case("nvmecryptoerase")
                            || m.eq_ignore_ascii_case("crypto")
                    })
                    .unwrap_or(false)
                    || capabilities.supports_crypto_erase;

                let (method, v_strat, v_reqs) = if prefer_crypto {
                    (
                        DriveSanitizationMethod::NvmeCryptoErase,
                        DriveVerificationStrategy::CryptoKeyDestructionCheck,
                        "Verify media encryption key destruction via NVMe controller log page 0x15"
                            .to_string(),
                    )
                } else {
                    (
                        DriveSanitizationMethod::NvmeFormatSanitize,
                        DriveVerificationStrategy::FirmwareStatusVerify,
                        "Verify sanitize status completion in NVMe Sanitize Progress log page"
                            .to_string(),
                    )
                };

                let v_plan = DriveVerificationPlan {
                    strategy: v_strat,
                    sample_percentage: None,
                    requirements: v_reqs,
                    limitations: vec![
                        "Controller firmware log status is authoritative; raw NAND flash cells are inaccessible across NVMe bus.".to_string(),
                    ],
                };

                let lims = vec![
                    "NVMe Sanitize / Crypto Erase executes at firmware level across all namespaces.".to_string(),
                    "Flash Translation Layer (FTL) over-provisioned blocks and wear-leveling reserves are sanitized internally.".to_string(),
                    "DoD 5220.22-M overwrite is prohibited on NVMe due to flash wear amplification and FTL evasion.".to_string(),
                ];

                (method, 1, v_plan, lims)
            } else {
                let method = DriveSanitizationMethod::AtaSecureErase;
                let v_plan = DriveVerificationPlan {
                    strategy: DriveVerificationStrategy::FirmwareStatusVerify,
                    sample_percentage: None,
                    requirements: "Verify ATA command completion register and IDENTIFY DEVICE security status".to_string(),
                    limitations: vec![
                        "ATA controller firmware status is authoritative; physical NAND flash blocks cannot be addressed individually.".to_string(),
                    ],
                };

                let lims = vec![
                    "ATA Secure Erase instructs drive controller to purge internal flash translation tables and erase NAND blocks.".to_string(),
                    "Wear leveling spare area and retired blocks are purged by controller internal firmware routine.".to_string(),
                    "Multi-pass pattern overwriting is avoided on SSDs to prevent unnecessary write amplification.".to_string(),
                ];

                (method, 1, v_plan, lims)
            }
        }
        DeviceType::Usb | DeviceType::MemoryCard | DeviceType::ExternalStorage => {
            let method = DriveSanitizationMethod::Nist80088ClearZero;
            let v_plan = DriveVerificationPlan {
                strategy: DriveVerificationStrategy::SampledSectorVerification,
                sample_percentage: Some(10.0),
                requirements: "Sampled pseudorandom sector read verification across start, middle, and end LBA ranges".to_string(),
                limitations: vec![
                    "USB mass storage bridge chips may translate commands and prevent direct controller access.".to_string(),
                    "Consumer USB flash media lacks standardized firmware sanitize logging.".to_string(),
                ],
            };

            let lims = vec![
                "Removable USB and memory card controllers perform low-cost wear leveling.".to_string(),
                "Physical flash cells outside logical addressing may retain residual charge patterns.".to_string(),
            ];

            (method, 1, v_plan, lims)
        }
        DeviceType::Unknown => unreachable!(),
    };

    let plan_id = format!("dplan-real-{}", Uuid::new_v4());

    Ok(DriveErasePlan {
        plan_id,
        physical_device_id: snapshot.device_id.clone(),
        display_name: snapshot.display_name.clone(),
        vendor: snapshot.vendor.clone(),
        model: snapshot.model.clone(),
        serial_number: snapshot.serial_number.clone(),
        media_type: snapshot.media_type,
        capacity_bytes: snapshot.capacity_bytes,
        sector_size: capabilities.sector_size,
        method,
        passes,
        risk_level: RiskLevel::Critical,
        capabilities,
        verification_plan,
        limitations,
        device_snapshot: snapshot,
        execution_mode: ExecutionMode::RealHardware,
        created_at: Utc::now().to_rfc3339(),
    })
}
