use crate::models::DriveSanitizationReport;
use crate::sanitization_report::SanitizationReportGenerator;
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use locardx_drive_eraser::models::DriveEraseResult;
use rusqlite::params;
use std::sync::Arc;
use tracing::info;

/// Core service orchestrating report generation, persistence, and forensic verification.
pub struct ReportingService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
}

impl ReportingService {
    pub fn new(db: Arc<Database>, audit: Arc<AuditService>) -> Self {
        Self { db, audit }
    }

    pub fn database(&self) -> &Arc<Database> {
        &self.db
    }

    pub fn audit(&self) -> &Arc<AuditService> {
        &self.audit
    }

    /// Generates, signs, and persists a formal `DriveSanitizationReport` for a drive erasure operation.
    pub fn generate_drive_erasure_report(
        &self,
        result: &DriveEraseResult,
        actor_id: Option<&str>,
    ) -> Result<DriveSanitizationReport, LocardError> {
        // Query current audit-chain state for cryptographic correlation
        let audit_ref = match self.audit.verify_chain() {
            Ok(_) => {
                // Fetch latest audit record hash from DB if possible
                self.db
                    .with_conn(|conn| {
                        let mut stmt = conn.prepare(
                            "SELECT current_hash FROM audit_events ORDER BY sequence_number DESC LIMIT 1",
                        )?;
                        let mut rows = stmt.query([])?;
                        if let Some(row) = rows.next()? {
                            let h: String = row.get(0)?;
                            Ok(Some(h))
                        } else {
                            Ok(None)
                        }
                    })
                    .unwrap_or(None)
            }
            Err(_) => None,
        };

        let report = SanitizationReportGenerator::generate_from_result(
            result,
            audit_ref,
            actor_id.map(|s| s.to_string()),
        );

        // Persist to database
        self.persist_report(&report)?;

        // Audit report generation
        let _ = self.audit.log_structured_event(
            "SANITIZATION_REPORT_GENERATED",
            actor_id,
            Some(&report.device_info.physical_device_id),
            &format!(
                "Generated sanitization report '{}' for operation '{}' (Digest: {})",
                report.report_id, report.operation_id, report.integrity.report_digest
            ),
        );

        info!(
            report_id = %report.report_id,
            operation_id = %report.operation_id,
            digest = %report.integrity.report_digest,
            "Sanitization report generated and persisted"
        );

        Ok(report)
    }

    /// Persists a report to SQLite.
    pub fn persist_report(&self, report: &DriveSanitizationReport) -> Result<(), LocardError> {
        let report_json = serde_json::to_string(report).map_err(|e| {
            LocardError::Database(format!("Failed to serialize report to JSON: {}", e))
        })?;

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO sanitization_reports (
                    report_id, operation_id, plan_id, actor_id, target_identifier,
                    execution_mode, is_simulation, status, verification_outcome,
                    report_digest, audit_chain_reference, report_json, generated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    report.report_id,
                    report.operation_id,
                    report.plan_id,
                    report.actor_id,
                    report.device_info.physical_device_id,
                    report.operation_info.execution_mode,
                    if report.is_simulation() { 1 } else { 0 },
                    report.execution_metrics.status,
                    report.verification.outcome,
                    report.integrity.report_digest,
                    report.integrity.audit_chain_reference,
                    report_json,
                    report.integrity.generated_at,
                ],
            )?;
            Ok(())
        })
    }

    /// Retrieves a persisted report by operation ID.
    pub fn get_report_by_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<DriveSanitizationReport>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT report_json FROM sanitization_reports WHERE operation_id = ?1 ORDER BY generated_at DESC LIMIT 1",
            )?;
            let mut rows = stmt.query(params![operation_id])?;
            if let Some(row) = rows.next()? {
                let json_str: String = row.get(0)?;
                let report: DriveSanitizationReport = serde_json::from_str(&json_str)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?;
                Ok(Some(report))
            } else {
                Ok(None)
            }
        })
    }

    /// Retrieves a persisted report by report ID.
    pub fn get_report(
        &self,
        report_id: &str,
    ) -> Result<Option<DriveSanitizationReport>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT report_json FROM sanitization_reports WHERE report_id = ?1 LIMIT 1",
            )?;
            let mut rows = stmt.query(params![report_id])?;
            if let Some(row) = rows.next()? {
                let json_str: String = row.get(0)?;
                let report: DriveSanitizationReport =
                    serde_json::from_str(&json_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                Ok(Some(report))
            } else {
                Ok(None)
            }
        })
    }

    /// Verifies the cryptographic integrity of a given report.
    pub fn verify_report(&self, report: &DriveSanitizationReport) -> bool {
        SanitizationReportGenerator::verify_report_integrity(report)
    }
}
