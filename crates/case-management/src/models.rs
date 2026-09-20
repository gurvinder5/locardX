use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Permitted lifecycle states for an investigation case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    InProgress,
    Completed,
    Archived,
}

impl fmt::Display for CaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::InProgress => write!(f, "InProgress"),
            Self::Completed => write!(f, "Completed"),
            Self::Archived => write!(f, "Archived"),
        }
    }
}

impl FromStr for CaseStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "").replace('-', "").as_str() {
            "open" => Ok(Self::Open),
            "inprogress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("Unknown case status: '{}'", s)),
        }
    }
}

/// Core investigation case record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Case {
    pub case_id: String,
    pub case_reference: String,
    pub title: String,
    pub description: String,
    pub status: CaseStatus,
    pub lead_investigator: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub metadata_json: String,
}

/// Request payload to instantiate a new case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCaseRequest {
    pub case_reference: String,
    pub title: String,
    pub description: String,
    pub metadata_json: Option<String>,
}

/// Request payload to update metadata of an existing case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaseRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub metadata_json: Option<String>,
}

/// Cross-module association linking an operation to an investigation case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseOperation {
    pub id: String,
    pub case_id: String,
    pub operation_id: String,
    pub operation_type: String,
    pub associated_by: String,
    pub associated_at: String,
    pub notes: Option<String>,
}

/// Category of evidential material tracked under a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    PhysicalStorage,
    AcquisitionImage,
    RecoveredDataset,
    LogicalFile,
}

impl fmt::Display for EvidenceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicalStorage => write!(f, "PhysicalStorage"),
            Self::AcquisitionImage => write!(f, "AcquisitionImage"),
            Self::RecoveredDataset => write!(f, "RecoveredDataset"),
            Self::LogicalFile => write!(f, "LogicalFile"),
        }
    }
}

impl FromStr for EvidenceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "").as_str() {
            "physicalstorage" => Ok(Self::PhysicalStorage),
            "acquisitionimage" => Ok(Self::AcquisitionImage),
            "recovereddataset" => Ok(Self::RecoveredDataset),
            "logicalfile" => Ok(Self::LogicalFile),
            _ => Err(format!("Unknown evidence type: '{}'", s)),
        }
    }
}

/// Evidential asset tracked under an investigation case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseEvidence {
    pub evidence_id: String,
    pub case_id: String,
    pub evidence_type: EvidenceType,
    pub identifier: String,
    pub label: String,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub introduced_by: String,
    pub introduced_at: String,
    pub status: String,
    pub notes: Option<String>,
}

/// Request payload to introduce evidence into a case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEvidenceRequest {
    pub evidence_type: EvidenceType,
    pub identifier: String,
    pub label: String,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub notes: Option<String>,
}

/// Category of technical chain of custody event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyEventType {
    EvidenceIntroduced,
    EvidenceAcquired,
    EvidenceVerified,
    EvidenceAnalyzed,
    EvidenceExported,
    ReportGenerated,
    CaseStatusChanged,
}

impl fmt::Display for CustodyEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceIntroduced => write!(f, "EvidenceIntroduced"),
            Self::EvidenceAcquired => write!(f, "EvidenceAcquired"),
            Self::EvidenceVerified => write!(f, "EvidenceVerified"),
            Self::EvidenceAnalyzed => write!(f, "EvidenceAnalyzed"),
            Self::EvidenceExported => write!(f, "EvidenceExported"),
            Self::ReportGenerated => write!(f, "ReportGenerated"),
            Self::CaseStatusChanged => write!(f, "CaseStatusChanged"),
        }
    }
}

impl FromStr for CustodyEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "").as_str() {
            "evidenceintroduced" => Ok(Self::EvidenceIntroduced),
            "evidenceacquired" => Ok(Self::EvidenceAcquired),
            "evidenceverified" => Ok(Self::EvidenceVerified),
            "evidenceanalyzed" => Ok(Self::EvidenceAnalyzed),
            "evidenceexported" => Ok(Self::EvidenceExported),
            "reportgenerated" => Ok(Self::ReportGenerated),
            "casestatuschanged" => Ok(Self::CaseStatusChanged),
            _ => Err(format!("Unknown custody event type: '{}'", s)),
        }
    }
}

/// Technical chain of custody record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyEvent {
    pub custody_id: String,
    pub case_id: String,
    pub evidence_id: Option<String>,
    pub event_type: CustodyEventType,
    pub actor_id: String,
    pub timestamp: String,
    pub action: String,
    pub details: String,
    pub audit_event_id: Option<String>,
    pub audit_hash: Option<String>,
}

/// Request payload to log a chain of custody action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCustodyRequest {
    pub evidence_id: Option<String>,
    pub event_type: CustodyEventType,
    pub action: String,
    pub details: String,
}

/// Chronological unified timeline item combining custody and audit hash-chain events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseTimelineItem {
    pub id: String,
    pub timestamp: String,
    pub item_type: String, // "Custody" or "Audit"
    pub actor_id: String,
    pub title: String,
    pub details: String,
    pub audit_hash: Option<String>,
    pub is_verified: bool,
}

/// High-level case metrics and KPI summary for dashboard presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseSummary {
    pub case: Case,
    pub operation_count: usize,
    pub evidence_count: usize,
    pub acquisition_count: usize,
    pub recovery_job_count: usize,
    pub recovered_file_count: usize,
    pub erasure_count: usize,
    pub report_count: usize,
    pub custody_event_count: usize,
    pub audit_chain_status: String,
    pub last_activity_at: String,
}
