use serde::{Deserialize, Serialize};
use std::fmt;

/// Immutable forensic snapshot of a physical storage device prior to acquisition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionDeviceSnapshot {
    pub device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub media_type: String,
    pub capacity_bytes: u64,
    pub sector_size: u32,
    pub bus_type: Option<String>,
    pub is_removable: bool,
    pub is_system: bool,
    pub snapshot_timestamp: String,
}

impl AcquisitionDeviceSnapshot {
    /// Compares the baseline snapshot with a live probe to prevent TOCTOU device mutation.
    /// Fails closed if any identity, capacity, or geometry attribute has changed.
    pub fn detect_mutation(
        &self,
        live: &AcquisitionDeviceSnapshot,
    ) -> Result<(), AcquisitionFailureReason> {
        if self.device_id != live.device_id {
            return Err(AcquisitionFailureReason::SourceMutated(format!(
                "Device ID changed from '{}' to '{}'",
                self.device_id, live.device_id
            )));
        }

        if self.capacity_bytes != live.capacity_bytes {
            return Err(AcquisitionFailureReason::SourceMutated(format!(
                "Capacity mutated from {} to {} bytes",
                self.capacity_bytes, live.capacity_bytes
            )));
        }

        if self.sector_size != live.sector_size {
            return Err(AcquisitionFailureReason::SourceMutated(format!(
                "Sector size mutated from {} to {} bytes",
                self.sector_size, live.sector_size
            )));
        }

        if let (Some(orig_s), Some(live_s)) = (&self.serial_number, &live.serial_number) {
            if orig_s != live_s {
                return Err(AcquisitionFailureReason::SourceMutated(format!(
                    "Serial number mismatch: expected '{}', found '{}'",
                    orig_s, live_s
                )));
            }
        }

        if let (Some(orig_b), Some(live_b)) = (&self.bus_type, &live.bus_type) {
            if orig_b != live_b {
                return Err(AcquisitionFailureReason::SourceMutated(format!(
                    "Bus type mutated from '{}' to '{}'",
                    orig_b, live_b
                )));
            }
        }

        Ok(())
    }
}

/// Pre-acquisition plan specifying the source, destination, format, and space requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionPlan {
    pub plan_id: String,
    pub source: AcquisitionDeviceSnapshot,
    pub destination_path: String,
    pub image_format: String,
    pub chunk_size_bytes: usize,
    pub required_space_bytes: u64,
    pub created_at: String,
}

/// Real-time progress telemetry emitted during acquisition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionProgress {
    pub operation_id: String,
    pub bytes_acquired: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub throughput_mbps: f64,
    pub elapsed_seconds: f64,
    pub eta_seconds: Option<f64>,
    pub stage: String,
}

/// Verified filesystem outcome of partial-artifact cleanup after a failed or cancelled acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactCleanupStatus {
    CleanupConfirmed,
    CleanupFailed,
    CleanupNotRequired,
}

impl fmt::Display for ArtifactCleanupStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CleanupConfirmed => write!(f, "CLEANUP_CONFIRMED"),
            Self::CleanupFailed => write!(f, "CLEANUP_FAILED"),
            Self::CleanupNotRequired => write!(f, "CLEANUP_NOT_REQUIRED"),
        }
    }
}

/// Forensic acquisition lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionStatus {
    Planned,
    Validating,
    Acquiring,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}

impl AcquisitionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Unknown
        )
    }

    pub fn can_transition_to(&self, next: AcquisitionStatus) -> bool {
        match (self, next) {
            (Self::Planned, Self::Validating) => true,
            (Self::Planned, Self::Cancelled) => true,
            (Self::Planned, Self::Failed) => true,

            (Self::Validating, Self::Acquiring) => true,
            (Self::Validating, Self::Cancelled) => true,
            (Self::Validating, Self::Failed) => true,

            (Self::Acquiring, Self::Completed) => true,
            (Self::Acquiring, Self::Failed) => true,
            (Self::Acquiring, Self::Cancelled) => true,
            (Self::Acquiring, Self::Unknown) => true,

            // Terminal states cannot transition further
            _ => false,
        }
    }
}

impl fmt::Display for AcquisitionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planned => write!(f, "Planned"),
            Self::Validating => write!(f, "Validating"),
            Self::Acquiring => write!(f, "Acquiring"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for AcquisitionStatus {
    type Err = locardx_common::LocardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Planned" => Ok(Self::Planned),
            "Validating" => Ok(Self::Validating),
            "Acquiring" => Ok(Self::Acquiring),
            "Completed" => Ok(Self::Completed),
            "Failed" => Ok(Self::Failed),
            "Cancelled" => Ok(Self::Cancelled),
            "Unknown" => Ok(Self::Unknown),
            other => Err(locardx_common::LocardError::Operation(format!(
                "Unrecognized acquisition status: {}",
                other
            ))),
        }
    }
}

/// Structured diagnostic telemetry recorded on acquisition completion or failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionDiagnostics {
    pub source_device_id: String,
    pub source_size_bytes: u64,
    pub destination_path: String,
    pub current_byte_offset: u64,
    pub requested_read_size: usize,
    pub bytes_actually_read: usize,
    pub current_stage: String,
    pub underlying_error: Option<String>,
    pub win32_error_code: Option<u32>,
    pub final_status: String,
}

/// Explicit forensic acquisition failure reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", content = "details")]
pub enum AcquisitionFailureReason {
    InvalidSource(String),
    SourceDisconnected,
    SourceMutated(String),
    InvalidDestination(String),
    DestinationOnSourceDisk,
    InsufficientFreeSpace {
        required: u64,
        available: u64,
    },
    DestinationAlreadyExists(String),
    PermissionDenied {
        device_id: String,
        win32_error: u32,
        message: String,
    },
    ReadError(String),
    DeviceIoError {
        device_id: String,
        offset: u64,
        requested_bytes: usize,
        bytes_read: usize,
        win32_error: Option<u32>,
        message: String,
    },
    ShortRead {
        expected: u64,
        actual: u64,
    },
    WriteError(String),
    HashMismatch {
        computed: String,
        expected: String,
    },
    Cancelled,
    Interrupted,
    Unknown(String),
}

impl AcquisitionFailureReason {
    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for AcquisitionFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource(msg) => write!(f, "Invalid acquisition source: {}", msg),
            Self::SourceDisconnected => write!(
                f,
                "Source physical device was disconnected during acquisition"
            ),
            Self::SourceMutated(msg) => write!(f, "Source physical device mutated: {}", msg),
            Self::InvalidDestination(msg) => write!(f, "Invalid acquisition destination: {}", msg),
            Self::DestinationOnSourceDisk => write!(
                f,
                "Destination path resides on the source physical disk (collision risk)"
            ),
            Self::InsufficientFreeSpace {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient destination free space: required {} bytes, available {} bytes",
                    required, available
                )
            }
            Self::DestinationAlreadyExists(path) => {
                write!(f, "Destination file already exists: {}", path)
            }
            Self::PermissionDenied {
                device_id,
                win32_error,
                message,
            } => {
                write!(
                    f,
                    "Access Denied on device '{}' (Win32 error {}): {}",
                    device_id, win32_error, message
                )
            }
            Self::ReadError(msg) => write!(f, "Read failure from source: {}", msg),
            Self::DeviceIoError {
                device_id,
                offset,
                message,
                win32_error,
                ..
            } => {
                if let Some(code) = win32_error {
                    write!(
                        f,
                        "Device I/O read failure on '{}' at offset {} (Win32 error {}): {}",
                        device_id, offset, code, message
                    )
                } else {
                    write!(
                        f,
                        "Device I/O read failure on '{}' at offset {}: {}",
                        device_id, offset, message
                    )
                }
            }
            Self::ShortRead { expected, actual } => {
                write!(
                    f,
                    "Unexpected short read from source: expected {} bytes, read {}",
                    expected, actual
                )
            }
            Self::WriteError(msg) => write!(f, "Write failure to destination image: {}", msg),
            Self::HashMismatch { computed, expected } => {
                write!(
                    f,
                    "Forensic hash mismatch: computed {}, expected {}",
                    computed, expected
                )
            }
            Self::Cancelled => write!(f, "Acquisition was cancelled by operator"),
            Self::Interrupted => write!(
                f,
                "Acquisition was interrupted by system termination or crash"
            ),
            Self::Unknown(msg) => write!(f, "Unknown acquisition error: {}", msg),
        }
    }
}

/// Completed or finalized forensic acquisition record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionResult {
    pub acquisition_id: String,
    pub operation_id: String,
    pub source: AcquisitionDeviceSnapshot,
    pub destination_path: String,
    pub image_format: String,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub status: AcquisitionStatus,
    pub bytes_acquired: u64,
    pub elapsed_seconds: f64,
    pub average_throughput_mbps: f64,
    pub failure_reason: Option<AcquisitionFailureReason>,
    #[serde(default)]
    pub diagnostics: Option<AcquisitionDiagnostics>,
    /// Filesystem-verified cleanup outcome. Never inferred from a delete attempt alone.
    #[serde(default)]
    pub cleanup_status: Option<ArtifactCleanupStatus>,
    /// OS/filesystem error when cleanup_status is CLEANUP_FAILED.
    #[serde(default)]
    pub cleanup_error: Option<String>,
    /// Path of a leftover partial artifact when deletion could not be confirmed.
    #[serde(default)]
    pub leftover_artifact_path: Option<String>,
    pub audit_reference: String,
    pub started_at: String,
    pub completed_at: String,
}

/// Clean evidential handoff artifact ready for consumption by Recovery Engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionArtifact {
    pub acquisition_id: String,
    pub image_path: String,
    pub image_format: String,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub source_device_snapshot: AcquisitionDeviceSnapshot,
    pub acquisition_timestamp: String,
    pub is_verified: bool,
    pub audit_reference: String,
}

/// DTO representing an acquisition record in history and audit reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionRecordDto {
    pub acquisition_id: String,
    pub operation_id: String,
    pub actor_id: Option<String>,
    pub source_device_id: String,
    pub source_display_name: String,
    pub source_serial: Option<String>,
    pub source_capacity_bytes: u64,
    pub destination_path: String,
    pub image_format: String,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub status: String,
    pub case_id: Option<String>,
    pub started_at: String,
    pub completed_at: String,
}
