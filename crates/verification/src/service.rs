use crate::hasher::StreamHasher;
use crate::models::{
    HashAlgorithm, HashResult, IntegrityRecord, TargetType, VerificationResult, VerificationStatus,
};
use crate::verifier::HashVerifier;
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use rusqlite::params;
use std::path::Path;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Core service for cryptographic integrity verification and evidence hashing.
pub struct IntegrityService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
}

impl IntegrityService {
    pub fn new(db: Arc<Database>, audit: Arc<AuditService>) -> Self {
        Self { db, audit }
    }

    /// Calculates cryptographic hash of a file, logs to tamper-evident audit log, and persists integrity record.
    pub fn calculate_file_hash<P: AsRef<Path>>(
        &self,
        path: P,
        actor_id: Option<&str>,
    ) -> Result<HashResult, LocardError> {
        let path_ref = path.as_ref();
        let result = StreamHasher::hash_file(path_ref, HashAlgorithm::Sha256)?;

        let operation_id = Uuid::new_v4().to_string();
        let record_id = Uuid::new_v4().to_string();

        // 1. Audit event registration
        self.audit.log_structured_event(
            "EVIDENCE_HASH_CALCULATED",
            actor_id,
            Some(&result.target.identifier),
            &format!(
                "Calculated {} digest '{}' for file '{}' ({} bytes, {} ms)",
                result.algorithm,
                result.digest,
                result.target.display_name,
                result.bytes_processed,
                result.duration_ms
            ),
        )?;

        // 2. Persist record to SQLite
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO integrity_records (
                    record_id, operation_id, target_type, target_path, target_name,
                    algorithm, digest, bytes_processed, verification_status,
                    expected_digest, actor_id, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    record_id,
                    operation_id,
                    result.target.target_type.to_string(),
                    result.target.identifier,
                    result.target.display_name,
                    result.algorithm.to_string(),
                    result.digest,
                    result.bytes_processed as i64,
                    "Calculated",
                    None::<String>,
                    actor_id,
                    result.completed_at,
                ],
            )?;
            Ok(())
        })?;

        info!(
            target = %result.target.display_name,
            algorithm = %result.algorithm,
            digest = %result.digest,
            "Cryptographic file hash computed and recorded"
        );

        Ok(result)
    }

    /// Verifies cryptographic hash of a file against expected value, records in audit chain and persistence.
    pub fn verify_file_hash<P: AsRef<Path>>(
        &self,
        path: P,
        expected_digest: &str,
        actor_id: Option<&str>,
    ) -> Result<VerificationResult, LocardError> {
        let path_ref = path.as_ref();
        let result = HashVerifier::verify_file(path_ref, expected_digest, HashAlgorithm::Sha256);

        let operation_id = Uuid::new_v4().to_string();
        let record_id = Uuid::new_v4().to_string();
        let (event_type, status_str) = match &result.status {
            VerificationStatus::Verified => ("INTEGRITY_VERIFICATION_PASS", "Verified"),
            VerificationStatus::Mismatch { .. } => ("INTEGRITY_VERIFICATION_FAIL", "Mismatch"),
            VerificationStatus::UnableToVerify { .. } => {
                ("INTEGRITY_VERIFICATION_ERROR", "UnableToVerify")
            }
        };

        // 1. Audit event registration
        self.audit.log_structured_event(
            event_type,
            actor_id,
            Some(&result.target.identifier),
            &format!(
                "Integrity check for '{}': {} (expected: '{}', calculated: '{}')",
                result.target.display_name,
                result.status,
                result.expected_digest,
                result.calculated_digest.as_deref().unwrap_or("N/A")
            ),
        )?;

        // 2. Persist record to SQLite
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO integrity_records (
                    record_id, operation_id, target_type, target_path, target_name,
                    algorithm, digest, bytes_processed, verification_status,
                    expected_digest, actor_id, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    record_id,
                    operation_id,
                    result.target.target_type.to_string(),
                    result.target.identifier,
                    result.target.display_name,
                    result.algorithm.to_string(),
                    result.calculated_digest.clone().unwrap_or_default(),
                    result.bytes_processed as i64,
                    status_str,
                    Some(result.expected_digest.clone()),
                    actor_id,
                    result.timestamp,
                ],
            )?;
            Ok(())
        })?;

        info!(
            target = %result.target.display_name,
            status = %result.status,
            "Cryptographic integrity verification completed and recorded"
        );

        Ok(result)
    }

    /// Lists recent persistent integrity verification records from SQLite.
    pub fn list_integrity_records(
        &self,
        limit: usize,
    ) -> Result<Vec<IntegrityRecord>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT record_id, operation_id, target_type, target_path, target_name,
                        algorithm, digest, bytes_processed, verification_status,
                        expected_digest, actor_id, created_at
                 FROM integrity_records ORDER BY created_at DESC LIMIT ?1",
            )?;
            let mut rows = stmt.query(params![limit as i64])?;
            let mut records = Vec::new();

            while let Some(row) = rows.next()? {
                let target_type_str: String = row.get(2)?;
                let target_type = match target_type_str.as_str() {
                    "File" => TargetType::File,
                    "Directory" => TargetType::Directory,
                    "LogicalVolume" => TargetType::LogicalVolume,
                    "PhysicalDevice" => TargetType::PhysicalDevice,
                    "EvidenceObject" => TargetType::EvidenceObject,
                    _ => TargetType::Unknown,
                };

                let algorithm_str: String = row.get(5)?;
                let algorithm = match algorithm_str.as_str() {
                    _ => HashAlgorithm::Sha256,
                };

                let bytes: i64 = row.get(7)?;

                records.push(IntegrityRecord {
                    record_id: row.get(0)?,
                    operation_id: row.get(1)?,
                    target_type,
                    target_path: row.get(3)?,
                    target_name: row.get(4)?,
                    algorithm,
                    digest: row.get(6)?,
                    bytes_processed: bytes.max(0) as u64,
                    verification_status: row.get(8)?,
                    expected_digest: row.get(9)?,
                    actor_id: row.get(10)?,
                    created_at: row.get(11)?,
                });
            }

            Ok(records)
        })
    }
}
