use chrono::Utc;
use locardx_device_manager::{DeviceClassification, DeviceType, PhysicalDevice};
use locardx_security::RiskLevel;
use locardx_verification::sanitization::VerificationOutcome;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Mode of drive erasure execution.
///
/// In Step 10A, only `Simulation` is active and permitted.
/// Real hardware execution remains permanently disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Simulation,
    RealHardware,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simulation => write!(f, "Simulation"),
            Self::RealHardware => write!(f, "RealHardware"),
        }
    }
}

/// Specific functional capabilities detectable or known for a physical storage drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveCapability {
    SequentialWrite,
    FullDeviceRead,
    SectorAccess,
    AtaSecureErase,
    AtaSanitize,
    NvmeFormat,
    NvmeSanitize,
    NvmeCryptoErase,
    RemovableMedia,
}

impl std::fmt::Display for DriveCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SequentialWrite => write!(f, "SequentialWrite"),
            Self::FullDeviceRead => write!(f, "FullDeviceRead"),
            Self::SectorAccess => write!(f, "SectorAccess"),
            Self::AtaSecureErase => write!(f, "AtaSecureErase"),
            Self::AtaSanitize => write!(f, "AtaSanitize"),
            Self::NvmeFormat => write!(f, "NvmeFormat"),
            Self::NvmeSanitize => write!(f, "NvmeSanitize"),
            Self::NvmeCryptoErase => write!(f, "NvmeCryptoErase"),
            Self::RemovableMedia => write!(f, "RemovableMedia"),
        }
    }
}

/// Aggregated hardware capabilities identified for a physical storage device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveCapabilities {
    pub supported_capabilities: Vec<DriveCapability>,
    pub interface_bus: String, // "SATA", "NVMe", "USB", "SD", "Unknown"
    pub sector_size: u32,      // Typically 512 or 4096 bytes
    pub is_rotational: bool,
    pub supports_crypto_erase: bool,
    pub supports_firmware_sanitize: bool,
    pub supports_overwrite: bool,
}

impl DriveCapabilities {
    pub fn has_capability(&self, cap: DriveCapability) -> bool {
        self.supported_capabilities.contains(&cap)
    }

    pub fn is_empty(&self) -> bool {
        self.supported_capabilities.is_empty()
    }
}

/// Explicit capability detection states for physical drives.
///
/// In Step 10B.1, any capability evaluated as `Unknown` or `DetectionFailed`
/// MUST cause planning and execution to fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityState {
    Supported,
    Unsupported,
    Unknown,
    DetectionFailed,
}

impl CapabilityState {
    pub fn is_supported(&self) -> bool {
        matches!(self, Self::Supported)
    }

    pub fn should_fail_closed(&self) -> bool {
        matches!(
            self,
            Self::Unknown | Self::DetectionFailed | Self::Unsupported
        )
    }
}

impl std::fmt::Display for CapabilityState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Supported => write!(f, "Supported"),
            Self::Unsupported => write!(f, "Unsupported"),
            Self::Unknown => write!(f, "Unknown"),
            Self::DetectionFailed => write!(f, "DetectionFailed"),
        }
    }
}

/// Complete assessed capabilities of a physical drive with explicit fail-closed states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveCapabilitiesAssessment {
    pub overall_state: CapabilityState,
    pub capabilities: DriveCapabilities,
    pub capability_states: HashMap<DriveCapability, CapabilityState>,
    pub method_support: HashMap<DriveSanitizationMethod, CapabilityState>,
    pub assessment_notes: Vec<String>,
}

impl DriveCapabilitiesAssessment {
    pub fn is_method_supported(&self, method: DriveSanitizationMethod) -> bool {
        self.method_support
            .get(&method)
            .map(|s| s.is_supported())
            .unwrap_or(false)
    }

    pub fn method_state(&self, method: DriveSanitizationMethod) -> CapabilityState {
        self.method_support
            .get(&method)
            .copied()
            .unwrap_or(CapabilityState::Unknown)
    }
}

/// Supported whole-disk sanitization methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveSanitizationMethod {
    Nist80088ClearZero,
    Dod522022M,
    AtaSecureErase,
    NvmeFormatSanitize,
    NvmeCryptoErase,
    BlockZeroOverwrite,
}

impl std::fmt::Display for DriveSanitizationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nist80088ClearZero => write!(f, "Nist80088ClearZero"),
            Self::Dod522022M => write!(f, "Dod522022M"),
            Self::AtaSecureErase => write!(f, "AtaSecureErase"),
            Self::NvmeFormatSanitize => write!(f, "NvmeFormatSanitize"),
            Self::NvmeCryptoErase => write!(f, "NvmeCryptoErase"),
            Self::BlockZeroOverwrite => write!(f, "BlockZeroOverwrite"),
        }
    }
}

/// Verification strategies appropriate for drive-level sanitization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveVerificationStrategy {
    FullDeviceReadVerify,
    FirmwareStatusVerify,
    CryptoKeyDestructionCheck,
    SampledSectorVerification,
    NotApplicable,
}

impl std::fmt::Display for DriveVerificationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullDeviceReadVerify => write!(f, "FullDeviceReadVerify"),
            Self::FirmwareStatusVerify => write!(f, "FirmwareStatusVerify"),
            Self::CryptoKeyDestructionCheck => write!(f, "CryptoKeyDestructionCheck"),
            Self::SampledSectorVerification => write!(f, "SampledSectorVerification"),
            Self::NotApplicable => write!(f, "NotApplicable"),
        }
    }
}

/// Post-erasure verification plan tailored to device media and capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveVerificationPlan {
    pub strategy: DriveVerificationStrategy,
    pub sample_percentage: Option<f64>,
    pub requirements: String,
    pub limitations: Vec<String>,
}

/// Explicit execution lifecycle states for physical drive sanitization.
///
/// Mandated in Step 10B.1:
/// PLANNED -> AUTHORIZED -> PRE_EXECUTION_CHECK -> EXECUTION_READY ->
/// EXECUTING -> EXECUTION_COMPLETE -> VERIFYING -> COMPLETED / FAILED / CANCELLED / UNKNOWN / VERIFICATION_FAILED / UNABLE_TO_VERIFY.
/// Interrupted real hardware operations must NEVER automatically become COMPLETED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveExecutionState {
    Planned,
    Authorized,
    PreExecutionCheck,
    ExecutionReady,
    Executing,
    ExecutionComplete,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Unknown,
    VerificationFailed,
    UnableToVerify,
}

impl DriveExecutionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "PLANNED",
            Self::Authorized => "AUTHORIZED",
            Self::PreExecutionCheck => "PRE_EXECUTION_CHECK",
            Self::ExecutionReady => "EXECUTION_READY",
            Self::Executing => "EXECUTING",
            Self::ExecutionComplete => "EXECUTION_COMPLETE",
            Self::Verifying => "VERIFYING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::Unknown => "UNKNOWN",
            Self::VerificationFailed => "VERIFICATION_FAILED",
            Self::UnableToVerify => "UNABLE_TO_VERIFY",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Failed
                | Self::Cancelled
                | Self::Unknown
                | Self::VerificationFailed
                | Self::UnableToVerify
        )
    }

    /// Enforces strictly valid state transitions for physical drive erasure.
    pub fn can_transition_to(&self, next: DriveExecutionState) -> bool {
        // Any non-terminal state can transition to failure/cancellation/unknown
        if matches!(next, Self::Failed | Self::Cancelled | Self::Unknown) {
            return !self.is_terminal();
        }

        match self {
            Self::Planned => matches!(next, Self::Authorized),
            Self::Authorized => matches!(next, Self::PreExecutionCheck),
            Self::PreExecutionCheck => matches!(next, Self::ExecutionReady),
            Self::ExecutionReady => matches!(next, Self::Executing),
            Self::Executing => matches!(next, Self::ExecutionComplete),
            Self::ExecutionComplete => matches!(next, Self::Verifying),
            Self::Verifying => matches!(
                next,
                Self::Completed | Self::VerificationFailed | Self::UnableToVerify
            ),
            _ => false,
        }
    }
}

impl std::fmt::Display for DriveExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Execution status for a drive erasure operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveEraseStatus {
    Planned,
    Authorized,
    PreExecutionCheck,
    ExecutionReady,
    Executing,
    ExecutionComplete,
    Verifying,
    Simulating,
    Completed,
    Failed,
    Cancelled,
    Unknown,
    VerificationFailed,
    UnableToVerify,
}

impl std::fmt::Display for DriveEraseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Planned => write!(f, "Planned"),
            Self::Authorized => write!(f, "Authorized"),
            Self::PreExecutionCheck => write!(f, "PreExecutionCheck"),
            Self::ExecutionReady => write!(f, "ExecutionReady"),
            Self::Executing => write!(f, "Executing"),
            Self::ExecutionComplete => write!(f, "ExecutionComplete"),
            Self::Verifying => write!(f, "Verifying"),
            Self::Simulating => write!(f, "Simulating"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Unknown => write!(f, "Unknown"),
            Self::VerificationFailed => write!(f, "VerificationFailed"),
            Self::UnableToVerify => write!(f, "UnableToVerify"),
        }
    }
}

impl From<DriveExecutionState> for DriveEraseStatus {
    fn from(state: DriveExecutionState) -> Self {
        match state {
            DriveExecutionState::Planned => Self::Planned,
            DriveExecutionState::Authorized => Self::Authorized,
            DriveExecutionState::PreExecutionCheck => Self::PreExecutionCheck,
            DriveExecutionState::ExecutionReady => Self::ExecutionReady,
            DriveExecutionState::Executing => Self::Executing,
            DriveExecutionState::ExecutionComplete => Self::ExecutionComplete,
            DriveExecutionState::Verifying => Self::Verifying,
            DriveExecutionState::Completed => Self::Completed,
            DriveExecutionState::Failed => Self::Failed,
            DriveExecutionState::Cancelled => Self::Cancelled,
            DriveExecutionState::Unknown => Self::Unknown,
            DriveExecutionState::VerificationFailed => Self::VerificationFailed,
            DriveExecutionState::UnableToVerify => Self::UnableToVerify,
        }
    }
}

impl From<DriveEraseStatus> for DriveExecutionState {
    fn from(status: DriveEraseStatus) -> Self {
        match status {
            DriveEraseStatus::Planned => Self::Planned,
            DriveEraseStatus::Authorized => Self::Authorized,
            DriveEraseStatus::PreExecutionCheck => Self::PreExecutionCheck,
            DriveEraseStatus::ExecutionReady => Self::ExecutionReady,
            DriveEraseStatus::Executing => Self::Executing,
            DriveEraseStatus::ExecutionComplete => Self::ExecutionComplete,
            DriveEraseStatus::Verifying => Self::Verifying,
            DriveEraseStatus::Simulating => Self::Executing,
            DriveEraseStatus::Completed => Self::Completed,
            DriveEraseStatus::Failed => Self::Failed,
            DriveEraseStatus::Cancelled => Self::Cancelled,
            DriveEraseStatus::Unknown => Self::Unknown,
            DriveEraseStatus::VerificationFailed => Self::VerificationFailed,
            DriveEraseStatus::UnableToVerify => Self::UnableToVerify,
        }
    }
}

/// Detailed categorised failure reasons for drive erasure operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveEraseFailureReason {
    LogicalVolumeTargetRejected(String),
    SystemOrBootDeviceProtected(String),
    DeviceNotFound(String),
    DeviceMutated(String),
    UnsupportedMedia(String),
    NoSupportedCapabilities(String),
    ConfirmationExpired(String),
    Cancelled,
    SimulationError(String),
    RealHardwareExecutionDisabled(String),
    RealHardwareExecutionNotEnabled(String),
    CapabilityUnknown(String),
    CapabilityDetectionFailed(String),
    CapabilityUnsupported(String),
    PermitAlreadyConsumed(String),
    PermitBindingMismatch(String),
    PermitExpired(String),
    ExclusiveAccessFailed(String),
    PreExecutionCheckFailed(String),
    DeviceDisconnected(String),
    IoError(String),
    ExecutionGateClosed(String),
    SecurityViolation(String),
    VerificationFailed(String),
    UnableToVerify(String),
}

impl std::fmt::Display for DriveEraseFailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LogicalVolumeTargetRejected(msg) => {
                write!(f, "Logical volume target rejected: {}", msg)
            }
            Self::SystemOrBootDeviceProtected(msg) => {
                write!(f, "System or boot device protected: {}", msg)
            }
            Self::DeviceNotFound(msg) => write!(f, "Device not found: {}", msg),
            Self::DeviceMutated(msg) => write!(
                f,
                "Device mutated since plan generation (TOCTOU violation): {}",
                msg
            ),
            Self::UnsupportedMedia(msg) => write!(f, "Unsupported media type: {}", msg),
            Self::NoSupportedCapabilities(msg) => write!(
                f,
                "Device possesses no supported sanitization capabilities: {}",
                msg
            ),
            Self::ConfirmationExpired(msg) => write!(f, "Confirmation challenge expired: {}", msg),
            Self::Cancelled => write!(f, "Operation cancelled cooperatively by operator"),
            Self::SimulationError(msg) => write!(f, "Simulation error: {}", msg),
            Self::RealHardwareExecutionDisabled(msg) => write!(
                f,
                "Real hardware execution is disabled in Step 10A: {}",
                msg
            ),
            Self::RealHardwareExecutionNotEnabled(msg) => write!(
                f,
                "Real hardware execution is not enabled in Step 10B.1: {}",
                msg
            ),
            Self::CapabilityUnknown(msg) => {
                write!(f, "Device capability is unknown (failing closed): {}", msg)
            }
            Self::CapabilityDetectionFailed(msg) => write!(
                f,
                "Device capability detection failed (failing closed): {}",
                msg
            ),
            Self::CapabilityUnsupported(msg) => write!(
                f,
                "Device capability is unsupported for requested method: {}",
                msg
            ),
            Self::PermitAlreadyConsumed(id) => write!(
                f,
                "Hardware execution permit '{}' has already been consumed (single-use)",
                id
            ),
            Self::PermitBindingMismatch(msg) => {
                write!(f, "Hardware execution permit binding mismatch: {}", msg)
            }
            Self::PermitExpired(msg) => {
                write!(
                    f,
                    "Hardware execution permit expired (TTL exceeded): {}",
                    msg
                )
            }
            Self::ExclusiveAccessFailed(msg) => {
                write!(f, "Exclusive device access failed: {}", msg)
            }
            Self::PreExecutionCheckFailed(msg) => {
                write!(f, "Pre-execution validation failed: {}", msg)
            }
            Self::DeviceDisconnected(msg) => {
                write!(
                    f,
                    "Target device disconnected or vanished during execution: {}",
                    msg
                )
            }
            Self::IoError(msg) => {
                write!(f, "Hardware I/O error during sanitization: {}", msg)
            }
            Self::ExecutionGateClosed(msg) => {
                write!(f, "Real hardware execution gate closed: {}", msg)
            }
            Self::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            Self::UnableToVerify(msg) => {
                write!(f, "Unable to verify physical sanitization: {}", msg)
            }
        }
    }
}

fn default_exists() -> bool {
    true
}

/// Pre-erasure snapshot capturing physical device identity and topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalDeviceSnapshot {
    pub device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub media_type: DeviceType,
    pub capacity_bytes: u64,
    pub sector_size: u32,
    #[serde(default)]
    pub physical_sector_size: Option<u32>,
    #[serde(default)]
    pub bus_type: Option<String>,
    pub is_system: bool,
    pub is_boot: bool,
    pub is_removable: bool,
    #[serde(default)]
    pub is_read_only: bool,
    #[serde(default = "default_exists")]
    pub exists: bool,
    pub classification: DeviceClassification,
    pub partition_count: usize,
    pub volume_labels: Vec<String>,
    pub snapshot_timestamp: String,
}

impl PhysicalDeviceSnapshot {
    pub fn from_device(device: &PhysicalDevice, sector_size: u32) -> Self {
        let is_boot = device.is_system_device
            || device.classification == DeviceClassification::BootDevice
            || device.volumes.iter().any(|v| v.is_boot_volume);

        let is_sys = device.is_system_device
            || device.classification == DeviceClassification::SystemDevice
            || device.volumes.iter().any(|v| v.is_system_volume);

        let labels = device
            .volumes
            .iter()
            .filter_map(|v| v.label.clone())
            .collect();

        Self {
            device_id: device.device_id.clone(),
            display_name: device.display_name.clone(),
            vendor: device.vendor.clone(),
            model: device.model.clone(),
            serial_number: device.serial_number.clone(),
            media_type: device.device_type,
            capacity_bytes: device.capacity_bytes,
            sector_size,
            physical_sector_size: None,
            bus_type: None,
            is_system: is_sys,
            is_boot,
            is_removable: device.removable,
            is_read_only: device.read_only,
            exists: true,
            classification: device.classification,
            partition_count: device.volumes.len(),
            volume_labels: labels,
            snapshot_timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub fn with_observations(
        mut self,
        physical_sector_size: Option<u32>,
        bus_type: Option<String>,
        is_read_only: bool,
    ) -> Self {
        self.physical_sector_size = physical_sector_size;
        self.bus_type = bus_type;
        self.is_read_only = is_read_only;
        self
    }

    /// Verifies whether live hardware attributes match the recorded snapshot.
    /// Detects drive substitution, capacity changes, sector size shifts, and partition changes.
    pub fn detect_mutation(&self, live: &PhysicalDeviceSnapshot) -> Result<(), String> {
        if !live.exists {
            return Err(format!(
                "Target device '{}' no longer exists or was disconnected",
                self.device_id
            ));
        }

        if self.device_id != live.device_id {
            return Err(format!(
                "Device ID mismatch: planned '{}', live '{}'",
                self.device_id, live.device_id
            ));
        }

        if self.serial_number != live.serial_number
            && (self.serial_number.is_some() || live.serial_number.is_some())
        {
            return Err(format!(
                "Device serial number changed: planned {:?}, live {:?}",
                self.serial_number, live.serial_number
            ));
        }

        if self.capacity_bytes != live.capacity_bytes {
            return Err(format!(
                "Device capacity changed: planned {} bytes, live {} bytes",
                self.capacity_bytes, live.capacity_bytes
            ));
        }

        if self.sector_size != live.sector_size {
            return Err(format!(
                "Sector size mutated: planned {} bytes, live {} bytes",
                self.sector_size, live.sector_size
            ));
        }

        if self.physical_sector_size != live.physical_sector_size
            && (self.physical_sector_size.is_some() || live.physical_sector_size.is_some())
        {
            return Err(format!(
                "Physical sector size mutated: planned {:?}, live {:?}",
                self.physical_sector_size, live.physical_sector_size
            ));
        }

        if self.bus_type != live.bus_type && (self.bus_type.is_some() || live.bus_type.is_some()) {
            return Err(format!(
                "Bus type mutated: planned {:?}, live {:?}",
                self.bus_type, live.bus_type
            ));
        }

        if self.media_type != live.media_type {
            return Err(format!(
                "Media type mutated: planned '{:?}', live '{:?}'",
                self.media_type, live.media_type
            ));
        }

        if self.model != live.model && (self.model.is_some() && live.model.is_some()) {
            return Err(format!(
                "Hardware model identity mutated: planned {:?}, live {:?}",
                self.model, live.model
            ));
        }

        if self.partition_count != live.partition_count {
            return Err(format!(
                "Partition topology mutated: planned {} partitions, live {} partitions",
                self.partition_count, live.partition_count
            ));
        }

        if !self.volume_labels.is_empty() && self.volume_labels != live.volume_labels {
            return Err(format!(
                "Volume labels mutated: planned {:?}, live {:?}",
                self.volume_labels, live.volume_labels
            ));
        }

        if live.is_system || live.is_boot {
            return Err(
                "Target device is now recognized as an active system or boot device".to_string(),
            );
        }

        if !self.is_read_only && live.is_read_only {
            return Err(
                "Target device write status changed: device is now read-only / write-protected"
                    .to_string(),
            );
        }

        Ok(())
    }
}

/// Formally generated plan for whole-disk sanitization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveErasePlan {
    pub plan_id: String,
    pub physical_device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub media_type: DeviceType,
    pub capacity_bytes: u64,
    pub sector_size: u32,
    pub method: DriveSanitizationMethod,
    pub passes: usize,
    pub risk_level: RiskLevel,
    pub capabilities: DriveCapabilities,
    pub verification_plan: DriveVerificationPlan,
    pub limitations: Vec<String>,
    pub device_snapshot: PhysicalDeviceSnapshot,
    pub execution_mode: ExecutionMode,
    pub created_at: String,
}

fn default_consumed_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

fn default_permit_ttl() -> u64 {
    300
}

/// Cryptographic and topological permit authorizing a single execution attempt.
///
/// Crucial Step 10B.1 Security Invariants:
/// - Cannot be created by any hardware backend or unprivileged caller (constructor is pub(crate))
/// - Generated only after mandatory safety checks pass in RealHardwareExecutionGate
/// - Bound to operation ID
/// - Bound to plan ID
/// - Bound to physical device identity
/// - Bound to validated PhysicalDeviceSnapshot
/// - Bound to sanitization method and execution mode
/// - Single-use: Calling `consume()` flags the permit and subsequent attempts fail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareExecutionPermit {
    permit_id: String,
    operation_id: String,
    plan_id: String,
    physical_device_id: String,
    device_snapshot: PhysicalDeviceSnapshot,
    method: DriveSanitizationMethod,
    execution_mode: ExecutionMode,
    issued_at: String,
    #[serde(default = "default_permit_ttl")]
    ttl_seconds: u64,
    #[serde(skip, default = "default_consumed_flag")]
    consumed: Arc<AtomicBool>,
}

impl HardwareExecutionPermit {
    /// Issues a new single-use execution permit.
    /// Restricted to `pub(crate)` so hardware backends cannot forge permits.
    pub(crate) fn issue(
        operation_id: String,
        plan_id: String,
        physical_device_id: String,
        device_snapshot: PhysicalDeviceSnapshot,
        method: DriveSanitizationMethod,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            permit_id: uuid::Uuid::new_v4().to_string(),
            operation_id,
            plan_id,
            physical_device_id,
            device_snapshot,
            method,
            execution_mode,
            issued_at: Utc::now().to_rfc3339(),
            ttl_seconds: 300,
            consumed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn ttl_seconds(&self) -> u64 {
        self.ttl_seconds
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }

    /// Verifies that the permit has not exceeded its authorized TTL.
    pub fn validate_not_expired(&self) -> Result<(), DriveEraseFailureReason> {
        if let Ok(issued) = chrono::DateTime::parse_from_rfc3339(&self.issued_at) {
            let issued_utc = issued.with_timezone(&Utc);
            let elapsed = Utc::now().signed_duration_since(issued_utc);
            if elapsed.num_milliseconds() > (self.ttl_seconds * 1000) as i64 {
                return Err(DriveEraseFailureReason::PermitExpired(format!(
                    "Permit '{}' expired: elapsed {}ms exceeds TTL {}s",
                    self.permit_id,
                    elapsed.num_milliseconds(),
                    self.ttl_seconds
                )));
            }
        }
        Ok(())
    }

    pub fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn physical_device_id(&self) -> &str {
        &self.physical_device_id
    }

    pub fn device_snapshot(&self) -> &PhysicalDeviceSnapshot {
        &self.device_snapshot
    }

    pub fn method(&self) -> DriveSanitizationMethod {
        self.method
    }

    pub fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }

    pub fn issued_at(&self) -> &str {
        &self.issued_at
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed.load(Ordering::SeqCst)
    }

    /// Single-use consumption check. If already consumed, returns Err(PermitAlreadyConsumed).
    pub fn consume(&self) -> Result<(), DriveEraseFailureReason> {
        if self.consumed.swap(true, Ordering::SeqCst) {
            return Err(DriveEraseFailureReason::PermitAlreadyConsumed(
                self.permit_id.clone(),
            ));
        }
        Ok(())
    }

    /// Validates that the permit matches the caller's target context.
    pub fn validate_binding(
        &self,
        operation_id: &str,
        plan_id: &str,
        physical_device_id: &str,
    ) -> Result<(), DriveEraseFailureReason> {
        if self.operation_id != operation_id {
            return Err(DriveEraseFailureReason::PermitBindingMismatch(format!(
                "Operation ID mismatch: permit '{}' bound to '{}', caller provided '{}'",
                self.permit_id, self.operation_id, operation_id
            )));
        }
        if self.plan_id != plan_id {
            return Err(DriveEraseFailureReason::PermitBindingMismatch(format!(
                "Plan ID mismatch: permit '{}' bound to '{}', caller provided '{}'",
                self.permit_id, self.plan_id, plan_id
            )));
        }
        if self.physical_device_id != physical_device_id {
            return Err(DriveEraseFailureReason::PermitBindingMismatch(format!(
                "Device ID mismatch: permit '{}' bound to '{}', caller provided '{}'",
                self.permit_id, self.physical_device_id, physical_device_id
            )));
        }
        Ok(())
    }
}

/// Live telemetry during drive sanitization simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveEraseProgress {
    pub operation_id: String,
    pub percentage: f32,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub current_pass: usize,
    pub total_passes: usize,
    pub current_stage: String,
    pub elapsed_seconds: f64,
    pub eta_seconds: Option<f64>,
}

/// Post-operation verification outcome for drive sanitization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveVerificationResult {
    pub outcome: VerificationOutcome,
    pub strategy: DriveVerificationStrategy,
    pub details: String,
    pub verified_at: String,
}

/// Final result record for a drive erasure operation (simulation in Step 10A).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveEraseResult {
    pub operation_id: String,
    pub plan_id: String,
    pub physical_device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub media_type: DeviceType,
    pub capacity_bytes: u64,
    pub sector_size: u32,
    pub method: DriveSanitizationMethod,
    pub execution_mode: ExecutionMode,
    pub status: DriveEraseStatus,
    pub bytes_processed: u64,
    pub elapsed_seconds: f64,
    pub verification: DriveVerificationResult,
    pub failure_reason: Option<DriveEraseFailureReason>,
    pub audit_references: Vec<String>,
    pub started_at: String,
    pub completed_at: String,
    pub limitations: Vec<String>,
}

/// Request payload to plan a drive erasure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveEraseRequest {
    pub target_device_id: String,
    pub requested_method: Option<String>,
    pub execution_mode: Option<ExecutionMode>,
    pub session_token: Option<String>,
}
