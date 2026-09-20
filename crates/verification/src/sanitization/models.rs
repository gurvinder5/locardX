use locardx_common::TargetIdentity;
use locardx_device_manager::DeviceType;
use locardx_security::{ReasonCode, RiskLevel, TargetSnapshot};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Conceptual storage media categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Hdd,
    Ssd,
    NvmeSsd,
    UsbRemovable,
    MemoryCard,
    ExternalStorage,
    Unknown,
}

impl MediaType {
    /// Infers MediaType from device type and model/vendor string telemetry.
    pub fn from_device_type(dt: DeviceType, model_or_name: Option<&str>) -> Self {
        let name_lower = model_or_name.unwrap_or("").to_lowercase();
        match dt {
            DeviceType::Hdd => Self::Hdd,
            DeviceType::Ssd => {
                if name_lower.contains("nvme") {
                    Self::NvmeSsd
                } else {
                    Self::Ssd
                }
            }
            DeviceType::Usb => Self::UsbRemovable,
            DeviceType::MemoryCard => Self::MemoryCard,
            DeviceType::ExternalStorage => Self::ExternalStorage,
            DeviceType::Unknown => {
                if name_lower.contains("nvme") {
                    Self::NvmeSsd
                } else if name_lower.contains("ssd") {
                    Self::Ssd
                } else if name_lower.contains("hdd") || name_lower.contains("hard disk") {
                    Self::Hdd
                } else {
                    Self::Unknown
                }
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hdd => "HDD (Magnetic Rotational)",
            Self::Ssd => "SSD (Solid State Flash)",
            Self::NvmeSsd => "NVMe SSD (High Performance Flash)",
            Self::UsbRemovable => "USB / Removable Flash",
            Self::MemoryCard => "Memory Card (SD/MMC)",
            Self::ExternalStorage => "External Storage Enclosure",
            Self::Unknown => "Unknown Media Category",
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Target scope of sanitization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanitizationScope {
    File,
    Folder,
    LogicalVolume,
    PhysicalDevice,
}

impl SanitizationScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Folder => "Folder",
            Self::LogicalVolume => "LogicalVolume",
            Self::PhysicalDevice => "PhysicalDevice",
        }
    }
}

impl fmt::Display for SanitizationScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Concrete sanitization methods across physical and logical domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanitizationMethod {
    /// NIST SP 800-88 Rev. 1 Clear: Single pass zero-byte overwrite.
    Nist80088ClearZero,
    /// NIST SP 800-88 Rev. 1 Purge: Cryptographic Erase (key destruction).
    Nist80088PurgeCrypto,
    /// DoD 5220.22-M: 3-pass overwrite (zeros, ones, pseudo-random) + verify.
    Dod522022M,
    /// NVMe Format Cryptographic Erase command.
    NvmeCryptoErase,
    /// ATA Secure Erase / Sanitize command.
    AtaSecureErase,
    /// Generic block zero overwrite.
    BlockZeroOverwrite,
    /// Logical multi-pass file overwrite, truncation, and unlinking.
    LogicalFileShred,
    /// Recursive directory traversal and logical file shredding.
    DirectoryRecursiveShred,
    /// Unsupported or invalid method.
    Unsupported,
}

impl SanitizationMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nist80088ClearZero => "NIST SP 800-88 Clear (Single-Pass Zero)",
            Self::Nist80088PurgeCrypto => "NIST SP 800-88 Purge (Cryptographic Erase)",
            Self::Dod522022M => "DoD 5220.22-M (3-Pass Overwrite & Verify)",
            Self::NvmeCryptoErase => "NVMe Cryptographic Erase (Format Command)",
            Self::AtaSecureErase => "ATA Firmware Secure Erase",
            Self::BlockZeroOverwrite => "Block-Level Zero Overwrite",
            Self::LogicalFileShred => "Logical File Shred & Unlink",
            Self::DirectoryRecursiveShred => "Recursive Directory File Shred",
            Self::Unsupported => "Unsupported Method",
        }
    }
}

impl fmt::Display for SanitizationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Verification strategies applied post-sanitization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStrategy {
    /// 100% full sequential read scan verifying pattern.
    FullReadBack,
    /// Statistical sampling of pseudo-random sectors (e.g. 5-10%).
    SampledRandomSectors,
    /// Verification that hardware cryptographic master key was zeroized/rotated.
    CryptoKeyDestructionCheck,
    /// Verification that filesystem MFT records / directory entries are unlinked.
    MetadataUnlinkCheck,
    /// Verification is not applicable or bypassed.
    NoVerification,
}

impl VerificationStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FullReadBack => "Full Sequential Read-Back (100% Sectors)",
            Self::SampledRandomSectors => "Sampled Pseudo-Random Sectors (10%)",
            Self::CryptoKeyDestructionCheck => "Cryptographic Key Destruction Verification",
            Self::MetadataUnlinkCheck => "Filesystem Metadata & Unlink Verification",
            Self::NoVerification => "No Post-Sanitization Verification",
        }
    }
}

impl fmt::Display for VerificationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Structured outcome of post-erasure verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationOutcome {
    Verified,
    VerificationFailed,
    PartiallyVerified,
    UnableToVerify,
    NotApplicable,
}

impl fmt::Display for VerificationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verified => write!(f, "Verified"),
            Self::VerificationFailed => write!(f, "Verification Failed"),
            Self::PartiallyVerified => write!(f, "Partially Verified"),
            Self::UnableToVerify => write!(f, "Unable To Verify"),
            Self::NotApplicable => write!(f, "Not Applicable"),
        }
    }
}

/// Well-known technical limitations documented for transparency.
pub struct VerificationLimitations;

impl VerificationLimitations {
    pub const FTL_WEAR_LEVELING: &'static str =
        "Flash Translation Layer (FTL) wear leveling and over-provisioning prevent guaranteed software overwrite of remapped or retired physical NAND blocks.";
    pub const BAD_SECTOR_REMAPPING: &'static str =
        "Hardware-reallocated bad sectors are hidden by the disk controller and cannot be overwritten or verified from host user space.";
    pub const JOURNAL_REMANENCE: &'static str =
        "Filesystem journaling (NTFS $LogFile, ext4 journal) may retain metadata and filename fragments outside the overwritten file clusters.";
    pub const SHADOW_COPIES_VSS: &'static str =
        "Volume Shadow Copies (VSS) or filesystem snapshots may retain historical file contents unless explicitly purged.";
    pub const HOST_BUFFER_CACHE: &'static str =
        "Host operating system page cache and drive write caches may acknowledge writes before physical media commit unless cache flushing is enforced.";
    pub const CONTROLLER_FIRMWARE_BYPASS: &'static str =
        "Firmware cryptographic erasure depends on drive controller hardware compliance and cannot be verified via raw sector reads without key re-generation testing.";
    pub const WEAR_OUT_RISK: &'static str =
        "Multi-pass overwrite on consumer flash media (USB / SD cards) incurs significant write amplification and premature NAND wear-out.";
}

/// Safe failure states for sanitization workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    Cancelled,
    DeviceDisconnected,
    TargetChanged,
    PermissionDenied,
    IoFailure,
    UnsupportedMedia,
    UnsupportedMethod,
    VerificationFailed,
    Unexpected(String),
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Operation was cancelled by user"),
            Self::DeviceDisconnected => {
                write!(f, "Storage device was disconnected during execution")
            }
            Self::TargetChanged => write!(
                f,
                "Target identity mutated between plan and execution (TOCTOU)"
            ),
            Self::PermissionDenied => write!(f, "Permission denied accessing target"),
            Self::IoFailure => write!(f, "Hardware I/O failure communicating with device"),
            Self::UnsupportedMedia => write!(
                f,
                "Target media category is not supported for requested scope"
            ),
            Self::UnsupportedMethod => write!(
                f,
                "Requested sanitization method is not applicable to media"
            ),
            Self::VerificationFailed => {
                write!(f, "Post-sanitization verification detected data remnants")
            }
            Self::Unexpected(msg) => write!(f, "Unexpected execution failure: {}", msg),
        }
    }
}

/// Complete evaluated sanitization plan produced without modifying storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizationPlan {
    pub plan_id: String,
    pub target: TargetIdentity,
    pub media_type: MediaType,
    pub scope: SanitizationScope,
    pub recommended_method: SanitizationMethod,
    pub is_applicable: bool,
    pub risk_level: RiskLevel,
    pub verification_strategy: VerificationStrategy,
    pub applicable_standard: Option<String>,
    pub standard_method_id: Option<String>,
    pub limitations: Vec<String>,
    pub reason_codes: Vec<ReasonCode>,
    pub target_snapshot: TargetSnapshot,
    pub created_at: String,
    pub actor_id: Option<String>,
}

/// Pre-erasure evidence record permanently capturing pre-operation target state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreErasureEvidence {
    pub evidence_id: String,
    pub operation_id: String,
    pub actor_id: Option<String>,
    pub target: TargetIdentity,
    pub target_snapshot: TargetSnapshot,
    pub sanitization_method: SanitizationMethod,
    pub sanitization_scope: SanitizationScope,
    pub verification_strategy: VerificationStrategy,
    pub timestamp: String,
    pub applicable_limitations: Vec<String>,
    pub safety_evaluation_ref: Option<String>,
}

/// Post-erasure verification result structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizationVerificationResult {
    pub result_id: String,
    pub plan_id: String,
    pub operation_id: Option<String>,
    pub target_identifier: String,
    pub method_used: SanitizationMethod,
    pub verification_strategy: VerificationStrategy,
    pub outcome: VerificationOutcome,
    pub evidence_produced: Option<String>,
    pub limitations: Vec<String>,
    pub failure_reason: Option<String>,
    pub timestamp: String,
}

/// Comparison result between two snapshots to detect TOCTOU changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotComparisonResult {
    pub matches: bool,
    pub reason: Option<String>,
    pub differences: Vec<String>,
    pub plan_snapshot: TargetSnapshot,
    pub current_snapshot: Option<TargetSnapshot>,
}
