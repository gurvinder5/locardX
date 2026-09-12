use crate::models::{
    AddEvidenceRequest, Case, CaseEvidence, CaseOperation, CaseStatus, CaseSummary,
    CaseTimelineItem, CreateCaseRequest, CustodyEvent, CustodyEventType, EvidenceType,
    RecordCustodyRequest, UpdateCaseRequest,
};
use chrono::Utc;
use locardx_audit::AuditService;
use locardx_auth::PublicUser;
use locardx_common::LocardError;
use locardx_database::Database;
use locardx_reporting::forensic_report::{
    CaseAcquisitionReportInfo, CaseErasureReportInfo, CaseForensicReport, CaseRecoveryReportInfo,
    CaseReportData, CaseReportInfo, CaseReportSummary, CustodyTimelineEntry, RecoveredFileSnippet,
    ReportAuditIntegrity,
};
use locardx_reporting::ReportingService;
use rusqlite::params;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Core service managing investigation cases, cross-module associations, custody ledgers, and reports.
pub struct CaseService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    reporting: Arc<ReportingService>,
}

impl CaseService {
    pub fn new(
        db: Arc<Database>,
        audit: Arc<AuditService>,
        reporting: Arc<ReportingService>,
    ) -> Self {
        Self {
            db,
            audit,
            reporting,
        }
    }

    pub fn database(&self) -> &Arc<Database> {
        &self.db
    }

    pub fn audit(&self) -> &Arc<AuditService> {
        &self.audit
    }

    pub fn reporting(&self) -> &Arc<ReportingService> {
        &self.reporting
    }

    /// Creates a new investigation case, recording an audit event and initial custody record.
    pub fn create_case(
        &self,
        req: CreateCaseRequest,
        actor: &PublicUser,
    ) -> Result<Case, LocardError> {
        if req.title.trim().is_empty() {
            return Err(LocardError::Operation(
                "Case title cannot be empty".to_string(),
            ));
        }
        if req.case_reference.trim().is_empty() {
            return Err(LocardError::Operation(
                "Case reference cannot be empty".to_string(),
            ));
        }

        let case_id = format!("case-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        let metadata = req.metadata_json.unwrap_or_else(|| "{}".to_string());

        let case = Case {
            case_id: case_id.clone(),
            case_reference: req.case_reference.trim().to_string(),
            title: req.title.trim().to_string(),
            description: req.description.trim().to_string(),
            status: CaseStatus::Open,
            lead_investigator: actor.username.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            closed_at: None,
            metadata_json: metadata,
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO cases (case_id, case_reference, title, description, status, lead_investigator, created_at, updated_at, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    case.case_id,
                    case.case_reference,
                    case.title,
                    case.description,
                    case.status.to_string(),
                    case.lead_investigator,
                    case.created_at,
                    case.updated_at,
                    case.metadata_json,
                ],
            )?;
            Ok(())
        })?;

        // Audit logging
        let audit_event = self.audit.log_structured_event(
            "CASE_CREATED",
            Some(&actor.username),
            Some(&case_id),
            &format!(
                "Created investigation case '{}' with reference '{}'",
                case.title, case.case_reference
            ),
        )?;

        // Initial chain-of-custody event
        let _ = self.record_custody_internal(
            &case_id,
            None,
            CustodyEventType::CaseStatusChanged,
            &actor.username,
            "Investigation case created and opened",
            &format!("Opened by lead investigator '{}'", actor.username),
            Some(&audit_event.event_id),
            Some(&audit_event.current_hash),
        );

        info!(
            case_id = %case.case_id,
            case_ref = %case.case_reference,
            lead = %case.lead_investigator,
            "Investigation case created"
        );

        Ok(case)
    }

    /// Retrieves an investigation case by its ID.
    pub fn get_case(&self, case_id: &str) -> Result<Option<Case>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT case_id, case_reference, title, description, status, lead_investigator,
                        created_at, updated_at, closed_at, metadata_json
                 FROM cases WHERE case_id = ?1 LIMIT 1",
            )?;
            let mut rows = stmt.query(params![case_id])?;
            if let Some(row) = rows.next()? {
                let status_str: String = row.get(4)?;
                let status = CaseStatus::from_str(&status_str).unwrap_or(CaseStatus::Open);
                Ok(Some(Case {
                    case_id: row.get(0)?,
                    case_reference: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status,
                    lead_investigator: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    closed_at: row.get(8)?,
                    metadata_json: row.get(9)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    /// Retrieves an investigation case by its unique reference string.
    pub fn get_case_by_reference(&self, case_ref: &str) -> Result<Option<Case>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT case_id, case_reference, title, description, status, lead_investigator,
                        created_at, updated_at, closed_at, metadata_json
                 FROM cases WHERE case_reference = ?1 COLLATE NOCASE LIMIT 1",
            )?;
            let mut rows = stmt.query(params![case_ref])?;
            if let Some(row) = rows.next()? {
                let status_str: String = row.get(4)?;
                let status = CaseStatus::from_str(&status_str).unwrap_or(CaseStatus::Open);
                Ok(Some(Case {
                    case_id: row.get(0)?,
                    case_reference: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status,
                    lead_investigator: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    closed_at: row.get(8)?,
                    metadata_json: row.get(9)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    /// Lists investigation cases with optional status filter and pagination.
    pub fn list_cases(
        &self,
        status_filter: Option<CaseStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Case>, usize), LocardError> {
        self.db.with_conn(|conn| {
            let total: usize = if let Some(ref s) = status_filter {
                conn.query_row(
                    "SELECT COUNT(*) FROM cases WHERE status = ?1",
                    params![s.to_string()],
                    |row| row.get(0),
                )?
            } else {
                conn.query_row("SELECT COUNT(*) FROM cases", [], |row| row.get(0))?
            };

            let mut cases = Vec::new();
            if let Some(ref s) = status_filter {
                let mut stmt = conn.prepare(
                    "SELECT case_id, case_reference, title, description, status, lead_investigator,
                            created_at, updated_at, closed_at, metadata_json
                     FROM cases WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
                )?;
                let mut rows = stmt.query(params![s.to_string(), limit as i64, offset as i64])?;
                while let Some(row) = rows.next()? {
                    let st_str: String = row.get(4)?;
                    cases.push(Case {
                        case_id: row.get(0)?,
                        case_reference: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        status: CaseStatus::from_str(&st_str).unwrap_or(CaseStatus::Open),
                        lead_investigator: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        closed_at: row.get(8)?,
                        metadata_json: row.get(9)?,
                    });
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT case_id, case_reference, title, description, status, lead_investigator,
                            created_at, updated_at, closed_at, metadata_json
                     FROM cases ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                )?;
                let mut rows = stmt.query(params![limit as i64, offset as i64])?;
                while let Some(row) = rows.next()? {
                    let st_str: String = row.get(4)?;
                    cases.push(Case {
                        case_id: row.get(0)?,
                        case_reference: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        status: CaseStatus::from_str(&st_str).unwrap_or(CaseStatus::Open),
                        lead_investigator: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        closed_at: row.get(8)?,
                        metadata_json: row.get(9)?,
                    });
                }
            }

            Ok((cases, total))
        })
    }

    /// Updates metadata of an existing investigation case.
    pub fn update_case(
        &self,
        case_id: &str,
        req: UpdateCaseRequest,
        actor: &PublicUser,
    ) -> Result<Case, LocardError> {
        let mut existing = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let now = Utc::now().to_rfc3339();
        if let Some(t) = req.title {
            if !t.trim().is_empty() {
                existing.title = t.trim().to_string();
            }
        }
        if let Some(d) = req.description {
            existing.description = d.trim().to_string();
        }
        if let Some(m) = req.metadata_json {
            existing.metadata_json = m;
        }
        existing.updated_at = now;

        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE cases SET title = ?1, description = ?2, metadata_json = ?3, updated_at = ?4 WHERE case_id = ?5",
                params![
                    existing.title,
                    existing.description,
                    existing.metadata_json,
                    existing.updated_at,
                    existing.case_id,
                ],
            )?;
            Ok(())
        })?;

        let _ = self.audit.log_structured_event(
            "CASE_UPDATED",
            Some(&actor.username),
            Some(case_id),
            &format!("Updated metadata for case '{}'", existing.title),
        );

        Ok(existing)
    }

    /// Updates the lifecycle status of an investigation case.
    /// STRICT PRESERVATION INVARIANT: Closing or archiving a case NEVER deletes evidence or records.
    pub fn update_case_status(
        &self,
        case_id: &str,
        new_status: CaseStatus,
        actor: &PublicUser,
    ) -> Result<Case, LocardError> {
        let mut existing = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let now = Utc::now().to_rfc3339();
        let previous_status = existing.status;
        existing.status = new_status;
        existing.updated_at = now.clone();

        if matches!(new_status, CaseStatus::Completed | CaseStatus::Archived) {
            if existing.closed_at.is_none() {
                existing.closed_at = Some(now.clone());
            }
        } else {
            existing.closed_at = None;
        }

        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE cases SET status = ?1, updated_at = ?2, closed_at = ?3 WHERE case_id = ?4",
                params![
                    existing.status.to_string(),
                    existing.updated_at,
                    existing.closed_at,
                    existing.case_id,
                ],
            )?;
            Ok(())
        })?;

        let audit_event = self.audit.log_structured_event(
            "CASE_STATUS_CHANGED",
            Some(&actor.username),
            Some(case_id),
            &format!(
                "Status transitioned from '{}' to '{}'",
                previous_status, new_status
            ),
        )?;

        let _ = self.record_custody_internal(
            case_id,
            None,
            CustodyEventType::CaseStatusChanged,
            &actor.username,
            &format!("Case status changed to {}", new_status),
            &format!(
                "Investigator '{}' changed status from {} to {}",
                actor.username, previous_status, new_status
            ),
            Some(&audit_event.event_id),
            Some(&audit_event.current_hash),
        );

        info!(
            case_id = %case_id,
            from = %previous_status,
            to = %new_status,
            "Case status transitioned"
        );

        Ok(existing)
    }

    /// Associates an operation with an investigation case.
    /// Auto-links evidential artifacts produced by the operation.
    pub fn associate_operation(
        &self,
        case_id: &str,
        operation_id: &str,
        op_type: &str,
        actor: &PublicUser,
        notes: Option<&str>,
    ) -> Result<CaseOperation, LocardError> {
        let case = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let assoc_id = format!("c-op-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        let case_op = CaseOperation {
            id: assoc_id,
            case_id: case.case_id.clone(),
            operation_id: operation_id.to_string(),
            operation_type: op_type.to_string(),
            associated_by: actor.username.clone(),
            associated_at: now.clone(),
            notes: notes.map(|n| n.to_string()),
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO case_operations (id, case_id, operation_id, operation_type, associated_by, associated_at, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    case_op.id,
                    case_op.case_id,
                    case_op.operation_id,
                    case_op.operation_type,
                    case_op.associated_by,
                    case_op.associated_at,
                    case_op.notes,
                ],
            )?;
            Ok(())
        })?;

        let audit_event = self.audit.log_structured_event(
            "CASE_OPERATION_ASSOCIATED",
            Some(&actor.username),
            Some(case_id),
            &format!(
                "Associated operation '{}' ({}) with case '{}'",
                operation_id, op_type, case.case_reference
            ),
        )?;

        // Auto-discover and associate evidence from completed operations
        self.auto_link_operation_evidence(
            case_id,
            operation_id,
            op_type,
            actor,
            &audit_event.current_hash,
        )?;

        Ok(case_op)
    }

    /// Internal helper to auto-link acquisition and recovery artifacts.
    fn auto_link_operation_evidence(
        &self,
        case_id: &str,
        operation_id: &str,
        op_type: &str,
        actor: &PublicUser,
        audit_hash: &str,
    ) -> Result<(), LocardError> {
        match op_type {
            "ForensicAcquisition" => {
                // Check if acquisition record exists
                let acq_info = self.db.with_conn(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT destination_path, source_display_name, image_sha256, image_size_bytes
                         FROM acquisition_records WHERE operation_id = ?1 LIMIT 1",
                    )?;
                    let mut rows = stmt.query(params![operation_id])?;
                    if let Some(row) = rows.next()? {
                        Ok(Some((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, u64>(3)?,
                        )))
                    } else {
                        Ok(None)
                    }
                })?;

                if let Some((path, name, sha256, size)) = acq_info {
                    let ev_req = AddEvidenceRequest {
                        evidence_type: EvidenceType::AcquisitionImage,
                        identifier: path.clone(),
                        label: format!("Bitstream DD Image - {}", name),
                        sha256: Some(sha256.clone()),
                        size_bytes: Some(size),
                        notes: Some(format!("Acquired via operation '{}'", operation_id)),
                    };
                    let ev = self.add_case_evidence(case_id, ev_req, actor)?;
                    let _ = self.record_custody_internal(
                        case_id,
                        Some(&ev.evidence_id),
                        CustodyEventType::EvidenceAcquired,
                        &actor.username,
                        "Forensic image acquired and linked",
                        &format!("Image '{}' (SHA-256: {})", path, sha256),
                        None,
                        Some(audit_hash),
                    );
                }
            }
            "Recovery" => {
                let rec_info = self.db.with_conn(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT job_id, source_image_path, files_recovered
                         FROM recovery_jobs WHERE operation_id = ?1 LIMIT 1",
                    )?;
                    let mut rows = stmt.query(params![operation_id])?;
                    if let Some(row) = rows.next()? {
                        Ok(Some((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, usize>(2)?,
                        )))
                    } else {
                        Ok(None)
                    }
                })?;

                if let Some((job_id, path, files)) = rec_info {
                    let ev_req = AddEvidenceRequest {
                        evidence_type: EvidenceType::RecoveredDataset,
                        identifier: job_id.clone(),
                        label: format!("Carved Files Dataset ({} files)", files),
                        sha256: None,
                        size_bytes: None,
                        notes: Some(format!("Recovered from source '{}'", path)),
                    };
                    let ev = self.add_case_evidence(case_id, ev_req, actor)?;
                    let _ = self.record_custody_internal(
                        case_id,
                        Some(&ev.evidence_id),
                        CustodyEventType::EvidenceAnalyzed,
                        &actor.username,
                        "Evidence analyzed via recovery engine",
                        &format!("Job '{}' recovered {} files from '{}'", job_id, files, path),
                        None,
                        Some(audit_hash),
                    );
                }
            }
            "DriveErasure" => {
                let _ = self.record_custody_internal(
                    case_id,
                    None,
                    CustodyEventType::EvidenceAnalyzed,
                    &actor.username,
                    "Drive erasure linked to case",
                    &format!("Sanitization operation '{}' linked to case", operation_id),
                    None,
                    Some(audit_hash),
                );
            }
            _ => {}
        }
        Ok(())
    }

    /// Lists all operations associated with a case.
    pub fn list_case_operations(&self, case_id: &str) -> Result<Vec<CaseOperation>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, case_id, operation_id, operation_type, associated_by, associated_at, notes
                 FROM case_operations WHERE case_id = ?1 ORDER BY associated_at ASC",
            )?;
            let mut rows = stmt.query(params![case_id])?;
            let mut ops = Vec::new();
            while let Some(row) = rows.next()? {
                ops.push(CaseOperation {
                    id: row.get(0)?,
                    case_id: row.get(1)?,
                    operation_id: row.get(2)?,
                    operation_type: row.get(3)?,
                    associated_by: row.get(4)?,
                    associated_at: row.get(5)?,
                    notes: row.get(6)?,
                });
            }
            Ok(ops)
        })
    }

    /// Introduces an evidentiary item into a case.
    pub fn add_case_evidence(
        &self,
        case_id: &str,
        req: AddEvidenceRequest,
        actor: &PublicUser,
    ) -> Result<CaseEvidence, LocardError> {
        let case = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let evidence_id = format!("ev-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        let evidence = CaseEvidence {
            evidence_id: evidence_id.clone(),
            case_id: case.case_id.clone(),
            evidence_type: req.evidence_type,
            identifier: req.identifier,
            label: req.label,
            sha256: req.sha256,
            size_bytes: req.size_bytes,
            introduced_by: actor.username.clone(),
            introduced_at: now.clone(),
            status: "Active".to_string(),
            notes: req.notes,
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO case_evidence (evidence_id, case_id, evidence_type, identifier, label, sha256, size_bytes, introduced_by, introduced_at, status, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    evidence.evidence_id,
                    evidence.case_id,
                    evidence.evidence_type.to_string(),
                    evidence.identifier,
                    evidence.label,
                    evidence.sha256,
                    evidence.size_bytes,
                    evidence.introduced_by,
                    evidence.introduced_at,
                    evidence.status,
                    evidence.notes,
                ],
            )?;
            Ok(())
        })?;

        let audit_event = self.audit.log_structured_event(
            "CASE_EVIDENCE_INTRODUCED",
            Some(&actor.username),
            Some(case_id),
            &format!(
                "Introduced evidence '{}' ({}) with ID '{}'",
                evidence.label, evidence.evidence_type, evidence.evidence_id
            ),
        )?;

        let _ = self.record_custody_internal(
            case_id,
            Some(&evidence.evidence_id),
            CustodyEventType::EvidenceIntroduced,
            &actor.username,
            &format!("Introduced evidence: {}", evidence.label),
            &format!(
                "Evidence identifier '{}' (SHA-256: {})",
                evidence.identifier,
                evidence.sha256.as_deref().unwrap_or("N/A")
            ),
            Some(&audit_event.event_id),
            Some(&audit_event.current_hash),
        );

        Ok(evidence)
    }

    /// Lists all evidence items associated with a case.
    pub fn list_case_evidence(&self, case_id: &str) -> Result<Vec<CaseEvidence>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT evidence_id, case_id, evidence_type, identifier, label, sha256, size_bytes,
                        introduced_by, introduced_at, status, notes
                 FROM case_evidence WHERE case_id = ?1 ORDER BY introduced_at ASC",
            )?;
            let mut rows = stmt.query(params![case_id])?;
            let mut ev_list = Vec::new();
            while let Some(row) = rows.next()? {
                let ev_type_str: String = row.get(2)?;
                let ev_type =
                    EvidenceType::from_str(&ev_type_str).unwrap_or(EvidenceType::LogicalFile);
                ev_list.push(CaseEvidence {
                    evidence_id: row.get(0)?,
                    case_id: row.get(1)?,
                    evidence_type: ev_type,
                    identifier: row.get(3)?,
                    label: row.get(4)?,
                    sha256: row.get(5)?,
                    size_bytes: row.get(6)?,
                    introduced_by: row.get(7)?,
                    introduced_at: row.get(8)?,
                    status: row.get(9)?,
                    notes: row.get(10)?,
                });
            }
            Ok(ev_list)
        })
    }

    /// Records a formal chain-of-custody technical ledger entry.
    pub fn record_custody_event(
        &self,
        case_id: &str,
        req: RecordCustodyRequest,
        actor: &PublicUser,
    ) -> Result<CustodyEvent, LocardError> {
        let _case = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let audit_event = self.audit.log_structured_event(
            "CUSTODY_EVENT_RECORDED",
            Some(&actor.username),
            Some(case_id),
            &format!("Custody event: {} - {}", req.action, req.details),
        )?;

        self.record_custody_internal(
            case_id,
            req.evidence_id.as_deref(),
            req.event_type,
            &actor.username,
            &req.action,
            &req.details,
            Some(&audit_event.event_id),
            Some(&audit_event.current_hash),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn record_custody_internal(
        &self,
        case_id: &str,
        evidence_id: Option<&str>,
        event_type: CustodyEventType,
        actor_id: &str,
        action: &str,
        details: &str,
        audit_event_id: Option<&str>,
        audit_hash: Option<&str>,
    ) -> Result<CustodyEvent, LocardError> {
        let custody_id = format!("cust-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        let event = CustodyEvent {
            custody_id: custody_id.clone(),
            case_id: case_id.to_string(),
            evidence_id: evidence_id.map(|s| s.to_string()),
            event_type,
            actor_id: actor_id.to_string(),
            timestamp: now.clone(),
            action: action.to_string(),
            details: details.to_string(),
            audit_event_id: audit_event_id.map(|s| s.to_string()),
            audit_hash: audit_hash.map(|s| s.to_string()),
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO case_custody (custody_id, case_id, evidence_id, event_type, actor_id, timestamp, action, details, audit_event_id, audit_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.custody_id,
                    event.case_id,
                    event.evidence_id,
                    event.event_type.to_string(),
                    event.actor_id,
                    event.timestamp,
                    event.action,
                    event.details,
                    event.audit_event_id,
                    event.audit_hash,
                ],
            )?;
            Ok(())
        })?;

        Ok(event)
    }

    /// Aggregates custody events and audit records into a unified chronological timeline.
    pub fn get_case_timeline(&self, case_id: &str) -> Result<Vec<CaseTimelineItem>, LocardError> {
        let is_audit_valid = self
            .audit
            .verify_chain()
            .map(|v| v.is_valid)
            .unwrap_or(false);

        // Fetch associated operation IDs
        let ops = self.list_case_operations(case_id)?;
        let mut target_refs = vec![case_id.to_string()];
        for op in &ops {
            target_refs.push(op.operation_id.clone());
        }

        let mut items = Vec::new();

        // 1. Fetch custody events
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT custody_id, timestamp, actor_id, action, details, audit_hash
                 FROM case_custody WHERE case_id = ?1 ORDER BY timestamp ASC",
            )?;
            let mut rows = stmt.query(params![case_id])?;
            while let Some(row) = rows.next()? {
                items.push(CaseTimelineItem {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    item_type: "Custody".to_string(),
                    actor_id: row.get(2)?,
                    title: row.get(3)?,
                    details: row.get(4)?,
                    audit_hash: row.get(5)?,
                    is_verified: is_audit_valid,
                });
            }
            Ok(())
        })?;

        // 2. Fetch audit events matching case or associated operations
        self.db.with_conn(|conn| {
            for target in &target_refs {
                let mut stmt = conn.prepare(
                    "SELECT event_id, timestamp, actor_id, event_type, details, current_hash
                     FROM audit_events WHERE target_ref = ?1 ORDER BY sequence_number ASC",
                )?;
                let mut rows = stmt.query(params![target])?;
                while let Some(row) = rows.next()? {
                    let event_id: String = row.get(0)?;
                    // Avoid duplicating custody records
                    if !items.iter().any(|it| it.id == event_id) {
                        items.push(CaseTimelineItem {
                            id: event_id,
                            timestamp: row.get(1)?,
                            item_type: "Audit".to_string(),
                            actor_id: row
                                .get::<_, Option<String>>(2)?
                                .unwrap_or_else(|| "System".to_string()),
                            title: row.get(3)?,
                            details: row.get(4)?,
                            audit_hash: Some(row.get(5)?),
                            is_verified: is_audit_valid,
                        });
                    }
                }
            }
            Ok(())
        })?;

        // Sort chronologically ascending
        items.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(items)
    }

    /// Aggregates case metrics into a high-level summary.
    pub fn get_case_summary(&self, case_id: &str) -> Result<CaseSummary, LocardError> {
        let case = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let ops = self.list_case_operations(case_id)?;
        let evidence = self.list_case_evidence(case_id)?;
        let timeline = self.get_case_timeline(case_id)?;
        let reports = self.reporting.list_case_reports(case_id)?;

        let mut acq_count = 0;
        let mut rec_count = 0;
        let mut recovered_files = 0;
        let mut era_count = 0;

        for op in &ops {
            match op.operation_type.as_str() {
                "ForensicAcquisition" => acq_count += 1,
                "Recovery" => {
                    rec_count += 1;
                    // Query recovered file count from DB
                    let count: usize = self
                        .db
                        .with_conn(|conn| {
                            let mut stmt = conn.prepare(
                                "SELECT files_recovered FROM recovery_jobs WHERE operation_id = ?1",
                            )?;
                            let mut rows = stmt.query(params![op.operation_id])?;
                            if let Some(r) = rows.next()? {
                                Ok(r.get(0)?)
                            } else {
                                Ok(0)
                            }
                        })
                        .unwrap_or(0);
                    recovered_files += count;
                }
                "DriveErasure" => era_count += 1,
                _ => {}
            }
        }

        let audit_status = match self.audit.verify_chain() {
            Ok(v) if v.is_valid => "Verified".to_string(),
            Ok(_) => "Integrity Compromised".to_string(),
            Err(e) => format!("Error: {}", e),
        };

        let last_activity = timeline
            .last()
            .map(|t| t.timestamp.clone())
            .unwrap_or_else(|| case.updated_at.clone());

        Ok(CaseSummary {
            case,
            operation_count: ops.len(),
            evidence_count: evidence.len(),
            acquisition_count: acq_count,
            recovery_job_count: rec_count,
            recovered_file_count: recovered_files,
            erasure_count: era_count,
            report_count: reports.len(),
            custody_event_count: timeline.len(),
            audit_chain_status: audit_status,
            last_activity_at: last_activity,
        })
    }

    /// Compiles all case data and generates a formal `CaseForensicReport`.
    pub fn generate_case_report(
        &self,
        case_id: &str,
        actor: &PublicUser,
    ) -> Result<CaseForensicReport, LocardError> {
        let case = self
            .get_case(case_id)?
            .ok_or_else(|| LocardError::Operation(format!("Case '{}' not found", case_id)))?;

        let ops = self.list_case_operations(case_id)?;

        // 1. Gather acquisitions
        let mut acquisitions = Vec::new();
        for op in &ops {
            if op.operation_type == "ForensicAcquisition" {
                let acq = self.db.with_conn(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT acquisition_id, operation_id, source_device_id, source_display_name,
                                source_serial, source_capacity_bytes, destination_path, image_format,
                                image_size_bytes, image_sha256, status, started_at, completed_at, audit_reference
                         FROM acquisition_records WHERE operation_id = ?1 LIMIT 1",
                    )?;
                    let mut rows = stmt.query(params![op.operation_id])?;
                    if let Some(row) = rows.next()? {
                        Ok(Some(CaseAcquisitionReportInfo {
                            acquisition_id: row.get(0)?,
                            operation_id: row.get(1)?,
                            source_device_id: row.get(2)?,
                            source_display_name: row.get(3)?,
                            source_serial: row.get(4)?,
                            source_capacity_bytes: row.get(5)?,
                            destination_path: row.get(6)?,
                            image_format: row.get(7)?,
                            image_size_bytes: row.get(8)?,
                            image_sha256: row.get(9)?,
                            status: row.get(10)?,
                            started_at: row.get(11)?,
                            completed_at: row.get(12)?,
                            audit_reference: row.get(13)?,
                        }))
                    } else {
                        Ok(None)
                    }
                })?;
                if let Some(a) = acq {
                    acquisitions.push(a);
                }
            }
        }

        // 2. Gather recoveries
        let mut recoveries = Vec::new();
        for op in &ops {
            if op.operation_type == "Recovery" {
                let rec = self.db.with_conn(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT job_id, operation_id, acquisition_id, source_image_sha256, recovery_mode,
                                status, files_recovered, candidates_evaluated, elapsed_seconds, audit_reference
                         FROM recovery_jobs WHERE operation_id = ?1 LIMIT 1",
                    )?;
                    let mut rows = stmt.query(params![op.operation_id])?;
                    if let Some(row) = rows.next()? {
                        let job_id: String = row.get(0)?;

                        // Query categories and confidence
                        let mut cat_counts = HashMap::new();
                        let mut conf_dist = HashMap::new();
                        let mut samples = Vec::new();

                        let mut file_stmt = conn.prepare(
                            "SELECT file_id, suggested_filename, file_type, size_bytes, confidence_score,
                                    confidence_grade, sha256_hash, recovery_method
                             FROM recovered_files WHERE job_id = ?1 LIMIT 10",
                        )?;
                        let mut file_rows = file_stmt.query(params![job_id])?;
                        while let Some(fr) = file_rows.next()? {
                            let f_type: String = fr.get(2)?;
                            let f_grade: String = fr.get(5)?;
                            *cat_counts.entry(f_type.clone()).or_insert(0) += 1;
                            *conf_dist.entry(f_grade.clone()).or_insert(0) += 1;

                            samples.push(RecoveredFileSnippet {
                                file_id: fr.get(0)?,
                                filename: fr.get(1)?,
                                file_type: f_type,
                                size_bytes: fr.get(3)?,
                                confidence_score: fr.get(4)?,
                                confidence_grade: f_grade,
                                sha256_hash: fr.get(6)?,
                                recovery_method: fr.get(7)?,
                            });
                        }

                        Ok(Some(CaseRecoveryReportInfo {
                            job_id,
                            operation_id: row.get(1)?,
                            acquisition_id: row.get(2)?,
                            source_image_sha256: row.get(3)?,
                            recovery_mode: row.get(4)?,
                            status: row.get(5)?,
                            files_recovered: row.get(6)?,
                            candidates_evaluated: row.get(7)?,
                            elapsed_seconds: row.get(8)?,
                            category_counts: cat_counts,
                            confidence_distribution: conf_dist,
                            sample_files: samples,
                            audit_reference: row.get(9)?,
                        }))
                    } else {
                        Ok(None)
                    }
                })?;
                if let Some(r) = rec {
                    recoveries.push(r);
                }
            }
        }

        // 3. Gather erasures
        let mut erasures = Vec::new();
        for op in &ops {
            if op.operation_type == "DriveErasure" {
                let era = self.db.with_conn(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT operation_id, plan_id, physical_device_id, display_name, serial_number,
                                sanitization_method, execution_mode, status, verification_outcome,
                                verification_strategy, evidence_digest, audit_references, completed_at
                         FROM drive_erasure_records WHERE operation_id = ?1 LIMIT 1",
                    )?;
                    let mut rows = stmt.query(params![op.operation_id])?;
                    if let Some(row) = rows.next()? {
                        let audit_refs: Option<String> = row.get(11)?;
                        let primary_audit_ref = audit_refs
                            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
                            .and_then(|v| v.first().cloned())
                            .unwrap_or_else(|| "N/A".to_string());

                        Ok(Some(CaseErasureReportInfo {
                            operation_id: row.get(0)?,
                            plan_id: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                            physical_device_id: row.get(2)?,
                            display_name: row.get(3)?,
                            serial_number: row.get(4)?,
                            method: row.get(5)?,
                            execution_mode: row.get(6)?,
                            status: row.get(7)?,
                            verification_outcome: row.get(8)?,
                            verification_strategy: row.get(9)?,
                            evidence_digest: row.get(10)?,
                            limitations: vec![],
                            audit_reference: primary_audit_ref,
                            completed_at: row.get(12)?,
                        }))
                    } else {
                        Ok(None)
                    }
                })?;
                if let Some(e) = era {
                    erasures.push(e);
                }
            }
        }

        // 4. Gather custody timeline
        let mut custody_entries = Vec::new();
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT custody_id, evidence_id, timestamp, event_type, actor_id, action, details, audit_hash
                 FROM case_custody WHERE case_id = ?1 ORDER BY timestamp ASC",
            )?;
            let mut rows = stmt.query(params![case_id])?;
            while let Some(row) = rows.next()? {
                custody_entries.push(CustodyTimelineEntry {
                    custody_id: row.get(0)?,
                    evidence_id: row.get(1)?,
                    timestamp: row.get(2)?,
                    event_type: row.get(3)?,
                    actor_id: row.get(4)?,
                    action: row.get(5)?,
                    details: row.get(6)?,
                    audit_hash: row.get(7)?,
                });
            }
            Ok(())
        })?;

        // 5. Gather audit integrity
        let chain_ver = self.audit.verify_chain().map_err(|e| {
            LocardError::AuditIntegrity(format!("Failed to verify audit hash chain: {}", e))
        })?;

        let root_hash = self
            .db
            .with_conn(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT current_hash FROM audit_events ORDER BY sequence_number DESC LIMIT 1",
                )?;
                let mut rows = stmt.query([])?;
                if let Some(r) = rows.next()? {
                    Ok(r.get::<_, String>(0)?)
                } else {
                    Ok("GENESIS".to_string())
                }
            })
            .unwrap_or_else(|_| "UNKNOWN".to_string());

        let audit_integrity = ReportAuditIntegrity {
            is_valid: chain_ver.is_valid,
            total_events: chain_ver.total_events,
            last_verified_sequence: chain_ver.last_verified_sequence,
            audit_root_hash: root_hash,
        };

        let report_data = CaseReportData {
            case_info: CaseReportInfo {
                case_id: case.case_id.clone(),
                case_reference: case.case_reference.clone(),
                title: case.title.clone(),
                description: case.description.clone(),
                status: case.status.to_string(),
                lead_investigator: case.lead_investigator.clone(),
                created_at: case.created_at.clone(),
                closed_at: case.closed_at.clone(),
            },
            acquisitions,
            recoveries,
            erasures,
            custody_timeline: custody_entries,
            audit_integrity,
        };

        let report = self
            .reporting
            .generate_case_report(&report_data, Some(&actor.username))?;

        let _ = self.record_custody_internal(
            case_id,
            None,
            CustodyEventType::ReportGenerated,
            &actor.username,
            "Forensic case report generated",
            &format!(
                "Report ID: '{}' (Digest: {})",
                report.report_id, report.integrity.report_digest
            ),
            None,
            Some(&report.integrity.audit_chain_reference),
        );

        Ok(report)
    }

    /// Retrieves a persisted `CaseForensicReport` by report ID.
    pub fn get_case_report(
        &self,
        report_id: &str,
    ) -> Result<Option<CaseForensicReport>, LocardError> {
        self.reporting.get_case_report(report_id)
    }

    /// Lists summaries of reports generated for a case.
    pub fn list_case_reports(&self, case_id: &str) -> Result<Vec<CaseReportSummary>, LocardError> {
        self.reporting.list_case_reports(case_id)
    }

    /// Cryptographically validates the canonical integrity digest of a case report.
    pub fn verify_case_report(&self, report: &CaseForensicReport) -> bool {
        self.reporting.verify_case_report(report)
    }
}
