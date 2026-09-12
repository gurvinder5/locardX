use super::models::{
    MediaType, SanitizationMethod, SanitizationScope, VerificationLimitations, VerificationStrategy,
};
use serde::{Deserialize, Serialize};

/// Detailed metadata describing a formal sanitization standard or reference methodology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizationStandardDefinition {
    pub standard_id: String,
    pub standard_name: String,
    pub method_id: String,
    pub method_name: String,
    pub method: SanitizationMethod,
    pub applicable_scopes: Vec<SanitizationScope>,
    pub applicable_media: Vec<MediaType>,
    pub recommended_verification: VerificationStrategy,
    pub verification_requirements: String,
    pub limitations: Vec<String>,
    pub description: String,
}

pub struct SanitizationStandardsRegistry;

impl SanitizationStandardsRegistry {
    /// Returns all supported standard methodologies.
    pub fn get_all_standards() -> Vec<SanitizationStandardDefinition> {
        vec![
            SanitizationStandardDefinition {
                standard_id: "NIST_SP_800_88_REV1_CLEAR".to_string(),
                standard_name: "NIST SP 800-88 Rev. 1 (Clear)".to_string(),
                method_id: "nist_800_88_clear_zero".to_string(),
                method_name: "Single-Pass Overwrite (Logical Zero)".to_string(),
                method: SanitizationMethod::Nist80088ClearZero,
                applicable_scopes: vec![
                    SanitizationScope::LogicalVolume,
                    SanitizationScope::PhysicalDevice,
                ],
                applicable_media: vec![
                    MediaType::Hdd,
                    MediaType::UsbRemovable,
                    MediaType::ExternalStorage,
                ],
                recommended_verification: VerificationStrategy::SampledRandomSectors,
                verification_requirements:
                    "Sequential sample verification across 10% of logical block addresses confirming zero bytes."
                        .to_string(),
                limitations: vec![
                    VerificationLimitations::FTL_WEAR_LEVELING.to_string(),
                    VerificationLimitations::BAD_SECTOR_REMAPPING.to_string(),
                    VerificationLimitations::HOST_BUFFER_CACHE.to_string(),
                ],
                description:
                    "Overwrites user-addressable storage space with a single pass of fixed zero bytes to prevent simple non-invasive recovery."
                        .to_string(),
            },
            SanitizationStandardDefinition {
                standard_id: "NIST_SP_800_88_REV1_PURGE".to_string(),
                standard_name: "NIST SP 800-88 Rev. 1 (Purge)".to_string(),
                method_id: "nist_800_88_purge_crypto".to_string(),
                method_name: "Cryptographic Erase (Purge)".to_string(),
                method: SanitizationMethod::Nist80088PurgeCrypto,
                applicable_scopes: vec![SanitizationScope::PhysicalDevice],
                applicable_media: vec![MediaType::Ssd, MediaType::NvmeSsd],
                recommended_verification: VerificationStrategy::CryptoKeyDestructionCheck,
                verification_requirements:
                    "Verify controller cryptographic master key generation and invalidation of previous encryption key."
                        .to_string(),
                limitations: vec![
                    VerificationLimitations::CONTROLLER_FIRMWARE_BYPASS.to_string(),
                    "Only applicable if the drive was continuously hardware-encrypted using AES-128 or AES-256 before erasure."
                        .to_string(),
                ],
                description:
                    "Leverages drive hardware controller encryption to cryptographically discard the Media Encryption Key (MEK)."
                        .to_string(),
            },
            SanitizationStandardDefinition {
                standard_id: "DOD_5220_22_M".to_string(),
                standard_name: "DoD 5220.22-M (NISPOM)".to_string(),
                method_id: "dod_5220_22_m_3pass".to_string(),
                method_name: "3-Pass Overwrite (0x00, 0xFF, Pseudo-Random)".to_string(),
                method: SanitizationMethod::Dod522022M,
                applicable_scopes: vec![
                    SanitizationScope::LogicalVolume,
                    SanitizationScope::PhysicalDevice,
                ],
                applicable_media: vec![MediaType::Hdd, MediaType::ExternalStorage],
                recommended_verification: VerificationStrategy::FullReadBack,
                verification_requirements:
                    "100% full sequential read-back verifying all sectors match the final pseudo-random character mask."
                        .to_string(),
                limitations: vec![
                    VerificationLimitations::BAD_SECTOR_REMAPPING.to_string(),
                    "Not recommended for SSDs due to high write amplification and FTL wear-leveling remanence."
                        .to_string(),
                ],
                description:
                    "Three-pass overwrite using a fixed character, its complement, and pseudo-random bytes followed by verification."
                        .to_string(),
            },
            SanitizationStandardDefinition {
                standard_id: "NVME_SPEC_CRYPTO".to_string(),
                standard_name: "NVM Express Base Specification (Crypto Erase)".to_string(),
                method_id: "nvme_format_crypto".to_string(),
                method_name: "NVMe Format Cryptographic Erase".to_string(),
                method: SanitizationMethod::NvmeCryptoErase,
                applicable_scopes: vec![SanitizationScope::PhysicalDevice],
                applicable_media: vec![MediaType::NvmeSsd],
                recommended_verification: VerificationStrategy::CryptoKeyDestructionCheck,
                verification_requirements:
                    "Validate completion of NVMe Format command with Cryptographic Erase (SES=2) bit set."
                        .to_string(),
                limitations: vec![
                    VerificationLimitations::CONTROLLER_FIRMWARE_BYPASS.to_string(),
                    "Target drive must support NVMe Cryptographic Erase command in controller capabilities."
                        .to_string(),
                ],
                description:
                    "Issues low-level NVMe Format command requesting controller-level cryptographic key regeneration."
                        .to_string(),
            },
            SanitizationStandardDefinition {
                standard_id: "LOGICAL_FILE_SHRED".to_string(),
                standard_name: "LocardX Forensic File Sanitization Standard".to_string(),
                method_id: "logical_file_shred_unlink".to_string(),
                method_name: "Logical File Payload Overwrite & Unlink".to_string(),
                method: SanitizationMethod::LogicalFileShred,
                applicable_scopes: vec![SanitizationScope::File, SanitizationScope::Folder],
                applicable_media: vec![
                    MediaType::Hdd,
                    MediaType::Ssd,
                    MediaType::NvmeSsd,
                    MediaType::UsbRemovable,
                    MediaType::ExternalStorage,
                ],
                recommended_verification: VerificationStrategy::MetadataUnlinkCheck,
                verification_requirements:
                    "Verify file cluster zeroing, truncation to 0 bytes, and directory entry unlinking."
                        .to_string(),
                limitations: vec![
                    VerificationLimitations::JOURNAL_REMANENCE.to_string(),
                    VerificationLimitations::SHADOW_COPIES_VSS.to_string(),
                    VerificationLimitations::FTL_WEAR_LEVELING.to_string(),
                ],
                description:
                    "Overwrites file contents with pseudo-random and zero patterns, flushes caches, truncates to 0 bytes, renames with randomized characters, and unlinks from the filesystem."
                        .to_string(),
            },
        ]
    }

    /// Finds the best matching standard definition for a method.
    pub fn find_by_method(method: SanitizationMethod) -> Option<SanitizationStandardDefinition> {
        Self::get_all_standards()
            .into_iter()
            .find(|s| s.method == method)
    }
}
