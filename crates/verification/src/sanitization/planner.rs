use super::models::{
    MediaType, SanitizationMethod, SanitizationPlan, SanitizationScope, VerificationLimitations,
    VerificationStrategy,
};
use super::standards::SanitizationStandardsRegistry;
use chrono::Utc;
use locardx_common::{TargetIdentity, TargetType};
use locardx_security::{ReasonCode, RiskLevel, TargetSnapshot};
use uuid::Uuid;

pub struct SanitizationPlanner;

impl SanitizationPlanner {
    /// Deterministically evaluates a target and produces a complete non-destructive sanitization plan.
    pub fn evaluate_plan(
        target: &TargetIdentity,
        snapshot: &TargetSnapshot,
        media_type: MediaType,
        scope: SanitizationScope,
        requested_method: Option<SanitizationMethod>,
        requested_strategy: Option<VerificationStrategy>,
        actor_id: Option<String>,
    ) -> SanitizationPlan {
        let plan_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        let mut limitations = Vec::new();
        let mut reason_codes = Vec::new();

        // 1. Hard System & Boot Device Protection
        if snapshot.is_system || snapshot.is_boot {
            let reason = if snapshot.is_boot && !snapshot.is_system {
                ReasonCode::BootDevice
            } else {
                ReasonCode::SystemDevice
            };
            reason_codes.push(reason);

            return SanitizationPlan {
                plan_id,
                target: target.clone(),
                media_type,
                scope,
                recommended_method: SanitizationMethod::Unsupported,
                is_applicable: false,
                risk_level: RiskLevel::Critical,
                verification_strategy: VerificationStrategy::NoVerification,
                applicable_standard: None,
                standard_method_id: None,
                limitations: vec![
                    "Active system disk and boot partitions are hard-blocked from sanitization by software safety policy."
                        .to_string(),
                ],
                reason_codes,
                target_snapshot: snapshot.clone(),
                created_at,
                actor_id,
            };
        }

        // 2. Scope vs TargetType Compatibility Check
        let scope_compatible = match scope {
            SanitizationScope::File => target.target_type == TargetType::File,
            SanitizationScope::Folder => target.target_type == TargetType::Directory,
            SanitizationScope::LogicalVolume => target.target_type == TargetType::LogicalVolume,
            SanitizationScope::PhysicalDevice => target.target_type == TargetType::PhysicalDevice,
        };

        if !scope_compatible {
            reason_codes.push(ReasonCode::InvalidTarget);
            return SanitizationPlan {
                plan_id,
                target: target.clone(),
                media_type,
                scope,
                recommended_method: SanitizationMethod::Unsupported,
                is_applicable: false,
                risk_level: RiskLevel::Critical,
                verification_strategy: VerificationStrategy::NoVerification,
                applicable_standard: None,
                standard_method_id: None,
                limitations: vec![format!(
                    "Requested scope '{}' is incompatible with target type '{:?}'.",
                    scope, target.target_type
                )],
                reason_codes,
                target_snapshot: snapshot.clone(),
                created_at,
                actor_id,
            };
        }

        // 3. Unknown Media Handling
        if media_type == MediaType::Unknown
            && (scope == SanitizationScope::PhysicalDevice
                || scope == SanitizationScope::LogicalVolume)
        {
            reason_codes.push(ReasonCode::UnsupportedOperation);
            return SanitizationPlan {
                plan_id,
                target: target.clone(),
                media_type,
                scope,
                recommended_method: SanitizationMethod::Unsupported,
                is_applicable: false,
                risk_level: RiskLevel::Critical,
                verification_strategy: VerificationStrategy::NoVerification,
                applicable_standard: None,
                standard_method_id: None,
                limitations: vec![
                    "Target hardware media category is indeterminate. Cannot safely select sanitization algorithm."
                        .to_string(),
                ],
                reason_codes,
                target_snapshot: snapshot.clone(),
                created_at,
                actor_id,
            };
        }

        // 4. Method Selection Based on Scope and Media Type
        let (recommended_method, default_strategy, is_applicable) = match scope {
            SanitizationScope::File | SanitizationScope::Folder => {
                limitations.push(VerificationLimitations::JOURNAL_REMANENCE.to_string());
                limitations.push(VerificationLimitations::SHADOW_COPIES_VSS.to_string());
                if matches!(media_type, MediaType::Ssd | MediaType::NvmeSsd) {
                    limitations.push(VerificationLimitations::FTL_WEAR_LEVELING.to_string());
                }
                (
                    SanitizationMethod::LogicalFileShred,
                    VerificationStrategy::MetadataUnlinkCheck,
                    true,
                )
            }
            SanitizationScope::PhysicalDevice | SanitizationScope::LogicalVolume => {
                match media_type {
                    MediaType::Hdd => {
                        limitations.push(VerificationLimitations::BAD_SECTOR_REMAPPING.to_string());
                        limitations.push(VerificationLimitations::HOST_BUFFER_CACHE.to_string());
                        (
                            SanitizationMethod::Nist80088ClearZero,
                            VerificationStrategy::SampledRandomSectors,
                            true,
                        )
                    }
                    MediaType::NvmeSsd => {
                        limitations
                            .push(VerificationLimitations::CONTROLLER_FIRMWARE_BYPASS.to_string());
                        limitations.push(VerificationLimitations::FTL_WEAR_LEVELING.to_string());
                        (
                            SanitizationMethod::NvmeCryptoErase,
                            VerificationStrategy::CryptoKeyDestructionCheck,
                            true,
                        )
                    }
                    MediaType::Ssd => {
                        limitations.push(VerificationLimitations::FTL_WEAR_LEVELING.to_string());
                        limitations
                            .push(VerificationLimitations::CONTROLLER_FIRMWARE_BYPASS.to_string());
                        (
                            SanitizationMethod::Nist80088PurgeCrypto,
                            VerificationStrategy::CryptoKeyDestructionCheck,
                            true,
                        )
                    }
                    MediaType::UsbRemovable | MediaType::MemoryCard => {
                        limitations.push(VerificationLimitations::FTL_WEAR_LEVELING.to_string());
                        limitations.push(VerificationLimitations::WEAR_OUT_RISK.to_string());
                        (
                            SanitizationMethod::Nist80088ClearZero,
                            VerificationStrategy::SampledRandomSectors,
                            true,
                        )
                    }
                    MediaType::ExternalStorage => {
                        limitations.push(VerificationLimitations::BAD_SECTOR_REMAPPING.to_string());
                        limitations.push(VerificationLimitations::HOST_BUFFER_CACHE.to_string());
                        (
                            SanitizationMethod::Nist80088ClearZero,
                            VerificationStrategy::SampledRandomSectors,
                            true,
                        )
                    }
                    MediaType::Unknown => (
                        SanitizationMethod::Unsupported,
                        VerificationStrategy::NoVerification,
                        false,
                    ),
                }
            }
        };

        // 5. Evaluate Method Overrides if Requested
        let (final_method, method_applicable) = if let Some(req) = requested_method {
            let applicable = match (req, media_type, scope) {
                (
                    SanitizationMethod::Dod522022M,
                    MediaType::Hdd | MediaType::ExternalStorage,
                    _,
                ) => true,
                (SanitizationMethod::Dod522022M, MediaType::Ssd | MediaType::NvmeSsd, _) => {
                    limitations.push("WARNING: DoD 5220.22-M is not recommended for SSD/NVMe flash media due to severe write amplification and FTL remanence.".to_string());
                    false
                }
                (
                    SanitizationMethod::NvmeCryptoErase,
                    MediaType::NvmeSsd,
                    SanitizationScope::PhysicalDevice,
                ) => true,
                (SanitizationMethod::NvmeCryptoErase, _, _) => false,
                (
                    SanitizationMethod::Nist80088PurgeCrypto,
                    MediaType::Ssd | MediaType::NvmeSsd,
                    SanitizationScope::PhysicalDevice,
                ) => true,
                (SanitizationMethod::Nist80088PurgeCrypto, _, _) => false,
                (
                    SanitizationMethod::LogicalFileShred,
                    _,
                    SanitizationScope::File | SanitizationScope::Folder,
                ) => true,
                (SanitizationMethod::LogicalFileShred, _, _) => false,
                (
                    SanitizationMethod::Nist80088ClearZero,
                    MediaType::Hdd | MediaType::UsbRemovable | MediaType::ExternalStorage,
                    _,
                ) => true,
                (
                    SanitizationMethod::Nist80088ClearZero,
                    MediaType::Ssd | MediaType::NvmeSsd,
                    _,
                ) => {
                    limitations.push("WARNING: Single-pass zero overwrite on flash media leaves inaccessible remanence in over-provisioned blocks.".to_string());
                    true
                }
                (SanitizationMethod::Unsupported, _, _) => false,
                _ => true,
            };
            (req, applicable && is_applicable)
        } else {
            (recommended_method, is_applicable)
        };

        if !method_applicable {
            reason_codes.push(ReasonCode::UnsupportedOperation);
        } else {
            reason_codes.push(ReasonCode::ValidTarget);
        }

        // 6. Strategy Selection
        let final_strategy = requested_strategy.unwrap_or(default_strategy);

        // 7. Standards Mapping
        let standard_def = SanitizationStandardsRegistry::find_by_method(final_method);
        let applicable_standard = standard_def.as_ref().map(|s| s.standard_name.to_string());
        let standard_method_id = standard_def.as_ref().map(|s| s.method_id.to_string());

        // 8. Risk Level
        let risk_level = if !method_applicable {
            RiskLevel::Critical
        } else {
            match scope {
                SanitizationScope::PhysicalDevice => RiskLevel::High,
                SanitizationScope::LogicalVolume => RiskLevel::High,
                SanitizationScope::Folder => RiskLevel::Medium,
                SanitizationScope::File => RiskLevel::Low,
            }
        };

        SanitizationPlan {
            plan_id,
            target: target.clone(),
            media_type,
            scope,
            recommended_method: final_method,
            is_applicable: method_applicable,
            risk_level,
            verification_strategy: final_strategy,
            applicable_standard,
            standard_method_id,
            limitations,
            reason_codes,
            target_snapshot: snapshot.clone(),
            created_at,
            actor_id,
        }
    }
}
