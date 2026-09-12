use crate::models::{ReasonCode, RiskLevel};
use locardx_common::OperationType;
use locardx_device_manager::{DeviceClassification, DeviceType, FilesystemType};

/// Evaluates risk levels and hardware/filesystem method compatibility.
pub struct SafetyPolicy;

impl SafetyPolicy {
    /// Assesses the risk level of an operation on a target.
    ///
    /// Risk Classification Rules:
    /// - Critical: Any destructive attempt against System/Boot devices, or unknown targets.
    /// - High: Destructive operations targeting removable or external storage.
    /// - Medium: Read-only operations on system devices or raw physical drives.
    /// - Low: Read-only integrity operations on standard non-system targets.
    pub fn assess_risk(
        op_type: OperationType,
        classification: Option<DeviceClassification>,
        is_system_or_boot: bool,
    ) -> RiskLevel {
        if is_system_or_boot {
            if op_type.is_destructive() {
                return RiskLevel::Critical;
            } else {
                return RiskLevel::Medium;
            }
        }

        match classification {
            Some(DeviceClassification::SystemDevice) | Some(DeviceClassification::BootDevice) => {
                if op_type.is_destructive() {
                    RiskLevel::Critical
                } else {
                    RiskLevel::Medium
                }
            }
            Some(DeviceClassification::Unknown) | None => {
                if op_type.is_destructive() {
                    RiskLevel::Critical
                } else {
                    RiskLevel::Medium
                }
            }
            Some(DeviceClassification::RemovableDevice)
            | Some(DeviceClassification::ExternalDevice)
            | Some(DeviceClassification::FixedDataDevice) => {
                if op_type.is_destructive() {
                    RiskLevel::High
                } else {
                    RiskLevel::Low
                }
            }
        }
    }

    /// Evaluates whether an operation type is compatible with target hardware and filesystem.
    ///
    /// INVARIANT: At this stage, all destructive methods return OperationDisabled.
    pub fn evaluate_compatibility(
        op_type: OperationType,
        _device_type: Option<DeviceType>,
        _fs_type: Option<&FilesystemType>,
    ) -> Result<(), ReasonCode> {
        match op_type {
            OperationType::IntegrityHash
            | OperationType::IntegrityVerify
            | OperationType::SanitizationPlanEvaluation
            | OperationType::FileErasure
            | OperationType::FolderErasure
            | OperationType::ForensicAcquisition
            | OperationType::Recovery => {
                // Supported operations
                Ok(())
            }
            OperationType::DriveErasure => {
                // Invariant: Real destructive drive erasure remains disabled in Step 10A (only simulation supported)
                Err(ReasonCode::OperationDisabled)
            }
            OperationType::Unknown => Err(ReasonCode::UnsupportedOperation),
        }
    }
}
