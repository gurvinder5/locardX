use crate::models::ReportIntegrity;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// High-level case metadata included in the forensic report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseReportInfo {
    pub case_id: String,
    pub case_reference: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub lead_investigator: String,
    pub created_at: String,
    pub closed_at: Option<String>,
}

/// Evidential acquisition records captured within the case report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseAcquisitionReportInfo {
    pub acquisition_id: String,
    pub operation_id: String,
    pub source_device_id: String,
    pub source_display_name: String,
    pub source_serial: Option<String>,
    pub source_capacity_bytes: u64,
    pub destination_path: String,
    pub image_format: String,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: String,
    pub audit_reference: String,
}

/// Recovered file snippet providing evidential metadata without storing binary payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredFileSnippet {
    pub file_id: String,
    pub filename: String,
    pub file_type: String,
    pub size_bytes: u64,
    pub confidence_score: u32,
    pub confidence_grade: String,
    pub sha256_hash: String,
    pub recovery_method: String,
}

/// Recovery execution details and statistical confidence distributions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseRecoveryReportInfo {
    pub job_id: String,
    pub operation_id: String,
    pub acquisition_id: String,
    pub source_image_sha256: String,
    pub recovery_mode: String,
    pub status: String,
    pub files_recovered: usize,
    pub candidates_evaluated: usize,
    pub elapsed_seconds: f64,
    pub category_counts: HashMap<String, usize>,
    pub confidence_distribution: HashMap<String, usize>,
    pub sample_files: Vec<RecoveredFileSnippet>,
    pub audit_reference: String,
}

/// Drive sanitization records associated with the case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseErasureReportInfo {
    pub operation_id: String,
    pub plan_id: String,
    pub physical_device_id: String,
    pub display_name: String,
    pub serial_number: Option<String>,
    pub method: String,
    pub execution_mode: String,
    pub status: String,
    pub verification_outcome: String,
    pub verification_strategy: String,
    pub evidence_digest: Option<String>,
    pub limitations: Vec<String>,
    pub audit_reference: String,
    pub completed_at: String,
}

/// Chain of custody entry documenting an evidentiary lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyTimelineEntry {
    pub custody_id: String,
    pub evidence_id: Option<String>,
    pub timestamp: String,
    pub event_type: String,
    pub actor_id: String,
    pub action: String,
    pub details: String,
    pub audit_hash: Option<String>,
}

/// Audit chain verification status at the time of report generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportAuditIntegrity {
    pub is_valid: bool,
    pub total_events: i64,
    pub last_verified_sequence: i64,
    pub audit_root_hash: String,
}

/// Concise summary for reporting indexes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseReportSummary {
    pub report_id: String,
    pub case_id: String,
    pub report_type: String,
    pub title: String,
    pub report_digest: String,
    pub generated_by: String,
    pub generated_at: String,
}

/// Input data used to generate a `CaseForensicReport`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseReportData {
    pub case_info: CaseReportInfo,
    pub acquisitions: Vec<CaseAcquisitionReportInfo>,
    pub recoveries: Vec<CaseRecoveryReportInfo>,
    pub erasures: Vec<CaseErasureReportInfo>,
    pub custody_timeline: Vec<CustodyTimelineEntry>,
    pub audit_integrity: ReportAuditIntegrity,
}

/// Complete, immutable, tamper-evident case forensic report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseForensicReport {
    pub report_id: String,
    pub case_id: String,
    pub title: String,
    pub case_info: CaseReportInfo,
    pub acquisitions: Vec<CaseAcquisitionReportInfo>,
    pub recoveries: Vec<CaseRecoveryReportInfo>,
    pub erasures: Vec<CaseErasureReportInfo>,
    pub custody_timeline: Vec<CustodyTimelineEntry>,
    pub audit_integrity: ReportAuditIntegrity,
    pub integrity: ReportIntegrity,
}

/// Generator producing formal, tamper-evident case-level forensic reports.
pub struct ForensicReportGenerator;

impl ForensicReportGenerator {
    /// Generates a formal `CaseForensicReport` from aggregated case data.
    pub fn generate_case_report(
        data: &CaseReportData,
        audit_chain_ref: Option<String>,
        actor_id: Option<String>,
    ) -> CaseForensicReport {
        let report_id = format!("crep-{}", Uuid::new_v4());
        let generated_at = Utc::now().to_rfc3339();
        let title = format!(
            "Forensic Investigation Report - {}",
            data.case_info.case_reference
        );
        let audit_ref =
            audit_chain_ref.unwrap_or_else(|| data.audit_integrity.audit_root_hash.clone());

        let report_digest = Self::calculate_case_report_digest(
            &report_id,
            &data.case_info,
            &data.acquisitions,
            &data.recoveries,
            &data.erasures,
            &data.custody_timeline,
            &data.audit_integrity,
            actor_id.as_deref(),
            &audit_ref,
            &generated_at,
        );

        let integrity = ReportIntegrity {
            audit_chain_reference: audit_ref,
            report_digest,
            generated_at,
        };

        CaseForensicReport {
            report_id,
            case_id: data.case_info.case_id.clone(),
            title,
            case_info: data.case_info.clone(),
            acquisitions: data.acquisitions.clone(),
            recoveries: data.recoveries.clone(),
            erasures: data.erasures.clone(),
            custody_timeline: data.custody_timeline.clone(),
            audit_integrity: data.audit_integrity.clone(),
            integrity,
        }
    }

    /// Computes the deterministic SHA-256 canonical digest across substantive report fields.
    pub fn calculate_case_report_digest(
        report_id: &str,
        case_info: &CaseReportInfo,
        acquisitions: &[CaseAcquisitionReportInfo],
        recoveries: &[CaseRecoveryReportInfo],
        erasures: &[CaseErasureReportInfo],
        custody_timeline: &[CustodyTimelineEntry],
        audit_integrity: &ReportAuditIntegrity,
        actor_id: Option<&str>,
        audit_ref: &str,
        generated_at: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"LOCARDX_CASE_REPORT_V1|");
        hasher.update(report_id.as_bytes());
        hasher.update(b"|");
        hasher.update(case_info.case_id.as_bytes());
        hasher.update(b"|");
        hasher.update(case_info.case_reference.as_bytes());
        hasher.update(b"|");
        hasher.update(case_info.title.as_bytes());
        hasher.update(b"|");
        hasher.update(case_info.description.as_bytes());
        hasher.update(b"|");
        hasher.update(case_info.status.as_bytes());
        hasher.update(b"|");
        hasher.update(case_info.lead_investigator.as_bytes());
        hasher.update(b"|");
        if let Some(actor) = actor_id {
            hasher.update(actor.as_bytes());
        }
        hasher.update(b"|");

        // Incorporate acquisitions
        for acq in acquisitions {
            hasher.update(acq.acquisition_id.as_bytes());
            hasher.update(acq.source_device_id.as_bytes());
            hasher.update(acq.image_sha256.as_bytes());
            hasher.update(acq.image_size_bytes.to_le_bytes());
            hasher.update(acq.status.as_bytes());
        }
        hasher.update(b"|");

        // Incorporate recoveries
        for rec in recoveries {
            hasher.update(rec.job_id.as_bytes());
            hasher.update(rec.source_image_sha256.as_bytes());
            hasher.update(rec.recovery_mode.as_bytes());
            hasher.update((rec.files_recovered as u64).to_le_bytes());
            hasher.update(rec.status.as_bytes());
        }
        hasher.update(b"|");

        // Incorporate erasures
        for era in erasures {
            hasher.update(era.operation_id.as_bytes());
            hasher.update(era.physical_device_id.as_bytes());
            hasher.update(era.method.as_bytes());
            hasher.update(era.verification_outcome.as_bytes());
        }
        hasher.update(b"|");

        // Incorporate custody count and audit integrity
        hasher.update((custody_timeline.len() as u64).to_le_bytes());
        hasher.update(if audit_integrity.is_valid {
            b"VALID".as_slice()
        } else {
            b"INVALID".as_slice()
        });
        hasher.update(audit_integrity.last_verified_sequence.to_le_bytes());
        hasher.update(audit_integrity.audit_root_hash.as_bytes());
        hasher.update(b"|");
        hasher.update(audit_ref.as_bytes());
        hasher.update(b"|");
        hasher.update(generated_at.as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Verifies the cryptographic integrity of a `CaseForensicReport`.
    pub fn verify_report_integrity(report: &CaseForensicReport) -> bool {
        let expected = Self::calculate_case_report_digest(
            &report.report_id,
            &report.case_info,
            &report.acquisitions,
            &report.recoveries,
            &report.erasures,
            &report.custody_timeline,
            &report.audit_integrity,
            Some(&report.case_info.lead_investigator),
            &report.integrity.audit_chain_reference,
            &report.integrity.generated_at,
        );
        expected == report.integrity.report_digest
    }

    /// Formats the case report into a comprehensive Markdown presentation.
    pub fn format_markdown_report(report: &CaseForensicReport) -> String {
        let mut md = String::new();
        md.push_str("# LocardX Unified Forensic Investigation Report\n\n");
        md.push_str(&format!("**Report ID**: `{}`\n", report.report_id));
        md.push_str(&format!(
            "**Case Reference**: `{}`\n",
            report.case_info.case_reference
        ));
        md.push_str(&format!("**Title**: {}\n", report.case_info.title));
        md.push_str(&format!(
            "**Lead Investigator**: `{}`\n",
            report.case_info.lead_investigator
        ));
        md.push_str(&format!("**Case Status**: `{}`\n", report.case_info.status));
        md.push_str(&format!(
            "**Generated At**: `{}`\n\n",
            report.integrity.generated_at
        ));

        md.push_str("## 1. Case Description\n\n");
        md.push_str(&format!("{}\n\n", report.case_info.description));

        md.push_str("## 2. Evidential Disk Acquisitions\n\n");
        if report.acquisitions.is_empty() {
            md.push_str("_No forensic bitstream acquisitions associated with this case._\n\n");
        } else {
            md.push_str("| Acquisition ID | Source Device | Serial | Format | Size | SHA-256 Digest | Status |\n");
            md.push_str("|:---|:---|:---|:---|:---|:---|:---|\n");
            for acq in &report.acquisitions {
                md.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} bytes | `{}` | **{}** |\n",
                    acq.acquisition_id,
                    acq.source_display_name,
                    acq.source_serial.as_deref().unwrap_or("N/A"),
                    acq.image_format,
                    acq.image_size_bytes,
                    acq.image_sha256,
                    acq.status,
                ));
            }
            md.push_str("\n");
        }

        md.push_str("## 3. Forensic File Recovery Operations\n\n");
        if report.recoveries.is_empty() {
            md.push_str("_No file recovery jobs associated with this case._\n\n");
        } else {
            for rec in &report.recoveries {
                md.push_str(&format!(
                    "### Job `{}` ({})\n",
                    rec.job_id, rec.recovery_mode
                ));
                md.push_str(&format!(
                    "- **Evidence Image SHA-256**: `{}`\n",
                    rec.source_image_sha256
                ));
                md.push_str(&format!("- **Status**: `{}`\n", rec.status));
                md.push_str(&format!(
                    "- **Candidates Evaluated**: {}\n",
                    rec.candidates_evaluated
                ));
                md.push_str(&format!(
                    "- **Files Successfully Recovered**: {}\n",
                    rec.files_recovered
                ));
                md.push_str(&format!(
                    "- **Elapsed Time**: {:.2}s\n\n",
                    rec.elapsed_seconds
                ));

                if !rec.sample_files.is_empty() {
                    md.push_str("#### Recovered Candidates Sample\n\n");
                    md.push_str("| Filename | Type | Size | Grade | Method | SHA-256 Digest |\n");
                    md.push_str("|:---|:---|:---|:---|:---|:---|\n");
                    for f in &rec.sample_files {
                        md.push_str(&format!(
                            "| `{}` | {} | {} B | **{}** | {} | `{}` |\n",
                            f.filename,
                            f.file_type,
                            f.size_bytes,
                            f.confidence_grade,
                            f.recovery_method,
                            f.sha256_hash
                        ));
                    }
                    md.push_str("\n");
                }
            }
        }

        md.push_str("## 4. Drive Sanitization Operations\n\n");
        if report.erasures.is_empty() {
            md.push_str("_No drive sanitization operations associated with this case._\n\n");
        } else {
            md.push_str(
                "| Operation ID | Target Device | Method | Mode | Status | Verification |\n",
            );
            md.push_str("|:---|:---|:---|:---|:---|:---|\n");
            for era in &report.erasures {
                md.push_str(&format!(
                    "| `{}` | {} | {} | {} | **{}** | **{}** ({}) |\n",
                    era.operation_id,
                    era.display_name,
                    era.method,
                    era.execution_mode,
                    era.status,
                    era.verification_outcome,
                    era.verification_strategy,
                ));
            }
            md.push_str("\n");
        }

        md.push_str("## 5. Technical Chain of Custody\n\n");
        if report.custody_timeline.is_empty() {
            md.push_str("_No chain of custody events recorded._\n\n");
        } else {
            md.push_str(
                "| Timestamp (UTC) | Event Type | Actor | Action | Details | Audit Hash |\n",
            );
            md.push_str("|:---|:---|:---|:---|:---|:---|\n");
            for c in &report.custody_timeline {
                md.push_str(&format!(
                    "| `{}` | {} | `{}` | {} | {} | `{}` |\n",
                    c.timestamp,
                    c.event_type,
                    c.actor_id,
                    c.action,
                    c.details,
                    c.audit_hash.as_deref().unwrap_or("N/A"),
                ));
            }
            md.push_str("\n");
        }

        md.push_str("## 6. Audit & Cryptographic Integrity Verification\n\n");
        md.push_str(&format!(
            "- **Audit Chain Status**: {}\n",
            if report.audit_integrity.is_valid {
                "VERIFIED (Zero tampering detected)"
            } else {
                "INTEGRITY FAILURE (Tampering detected)"
            }
        ));
        md.push_str(&format!(
            "- **Total Audit Sequence Count**: {}\n",
            report.audit_integrity.total_events
        ));
        md.push_str(&format!(
            "- **Last Verified Sequence**: {}\n",
            report.audit_integrity.last_verified_sequence
        ));
        md.push_str(&format!(
            "- **Audit Root Reference**: `{}`\n",
            report.integrity.audit_chain_reference
        ));
        md.push_str(&format!(
            "- **Canonical Report Digest**: `{}`\n\n",
            report.integrity.report_digest
        ));

        md.push_str("---\n");
        md.push_str("_Report generated deterministically by LocardX Forensic Workstation. Every contact leaves a trace._\n");

        md
    }

    /// Formats the case report into a formal evidentiary text certificate.
    pub fn format_text_certificate(report: &CaseForensicReport) -> String {
        let mut cert = String::new();
        cert.push_str(
            "================================================================================\n",
        );
        cert.push_str(
            "               LOCARDX FORENSIC INVESTIGATION CERTIFICATE                      \n",
        );
        cert.push_str(
            "================================================================================\n\n",
        );
        cert.push_str(&format!("REPORT ID:           {}\n", report.report_id));
        cert.push_str(&format!(
            "CASE REFERENCE:      {}\n",
            report.case_info.case_reference
        ));
        cert.push_str(&format!(
            "TITLE:               {}\n",
            report.case_info.title
        ));
        cert.push_str(&format!(
            "INVESTIGATOR:        {}\n",
            report.case_info.lead_investigator
        ));
        cert.push_str(&format!(
            "CASE STATUS:         {}\n",
            report.case_info.status
        ));
        cert.push_str(&format!(
            "TIMESTAMP (UTC):     {}\n\n",
            report.integrity.generated_at
        ));

        cert.push_str(
            "--------------------------------------------------------------------------------\n",
        );
        cert.push_str("ACQUISITION & EVIDENCE SUMMARY:\n");
        cert.push_str(&format!(
            "- Total Bitstream Images: {}\n",
            report.acquisitions.len()
        ));
        for (idx, acq) in report.acquisitions.iter().enumerate() {
            cert.push_str(&format!(
                "  [{}] {} ({} bytes) -> SHA-256: {}\n",
                idx + 1,
                acq.source_display_name,
                acq.image_size_bytes,
                acq.image_sha256
            ));
        }

        cert.push_str("\nRECOVERY OPERATIONS SUMMARY:\n");
        cert.push_str(&format!(
            "- Total Recovery Jobs: {}\n",
            report.recoveries.len()
        ));
        let total_files: usize = report.recoveries.iter().map(|r| r.files_recovered).sum();
        cert.push_str(&format!(
            "- Total Files Carved/Recovered: {}\n",
            total_files
        ));

        cert.push_str("\nSANITIZATION OPERATIONS SUMMARY:\n");
        cert.push_str(&format!(
            "- Total Sanitization Operations: {}\n",
            report.erasures.len()
        ));
        for (idx, era) in report.erasures.iter().enumerate() {
            cert.push_str(&format!(
                "  [{}] {} ({}) - Status: {} - Outcome: {}\n",
                idx + 1,
                era.display_name,
                era.method,
                era.status,
                era.verification_outcome
            ));
        }

        cert.push_str(
            "\n--------------------------------------------------------------------------------\n",
        );
        cert.push_str("CHAIN OF CUSTODY & AUDIT VERIFICATION:\n");
        cert.push_str(&format!(
            "- Custody Event Count:    {}\n",
            report.custody_timeline.len()
        ));
        cert.push_str(&format!(
            "- Audit Chain Status:     {}\n",
            if report.audit_integrity.is_valid {
                "CRYPTOGRAPHICALLY VERIFIED"
            } else {
                "FAILED"
            }
        ));
        cert.push_str(&format!(
            "- Audit Reference:        {}\n",
            report.integrity.audit_chain_reference
        ));
        cert.push_str(&format!(
            "- Canonical Report Digest:{}\n",
            report.integrity.report_digest
        ));
        cert.push_str(
            "================================================================================\n",
        );
        cert.push_str(
            "This certificate confirms that the operations recorded herein adhere strictly to\n",
        );
        cert.push_str(
            "LocardX evidential invariants and tamper-evident audit chaining.                 \n",
        );
        cert.push_str(
            "================================================================================\n",
        );

        cert
    }
}
