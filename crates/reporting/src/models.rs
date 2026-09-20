use serde::{Deserialize, Serialize};

/// Detailed device hardware attributes captured in the sanitization report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDeviceInfo {
    pub physical_device_id: String,
    pub display_name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub media_type: String,
    pub capacity_bytes: u64,
    pub sector_size: u32,
    pub bus_type: Option<String>,
}

/// Operational parameters, timing, and execution mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportOperationInfo {
    pub sanitization_method: String,
    pub execution_mode: String,
    pub is_simulation: bool,
    pub started_at: String,
    pub completed_at: String,
    pub elapsed_seconds: f64,
}

/// Execution outcome, metrics, and any applicable limitations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportExecutionMetrics {
    pub status: String,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub failure_reason: Option<String>,
    pub limitations: Vec<String>,
}

/// Forensic post-sanitization verification results and evidence digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportVerificationInfo {
    pub strategy: String,
    pub outcome: String,
    pub details: String,
    pub evidence_digest: String,
    pub verified_at: String,
}

/// Tamper-evident report metadata, cryptographic hash, and audit chain correlation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportIntegrity {
    pub audit_chain_reference: String,
    pub report_digest: String,
    pub generated_at: String,
}

/// A formal, tamper-evident physical drive sanitization certificate / report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveSanitizationReport {
    pub report_id: String,
    pub operation_id: String,
    pub plan_id: String,
    pub actor_id: Option<String>,
    pub device_info: ReportDeviceInfo,
    pub operation_info: ReportOperationInfo,
    pub execution_metrics: ReportExecutionMetrics,
    pub verification: ReportVerificationInfo,
    pub integrity: ReportIntegrity,
}

impl DriveSanitizationReport {
    /// Returns true if the operation was performed as a simulation.
    pub fn is_simulation(&self) -> bool {
        self.operation_info.is_simulation
    }

    /// Returns true if the report documents a real, verified physical erasure.
    pub fn is_verified_physical_erasure(&self) -> bool {
        !self.operation_info.is_simulation
            && self.execution_metrics.status == "Completed"
            && self.verification.outcome == "Verified"
    }
}
