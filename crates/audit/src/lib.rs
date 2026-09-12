use chrono::Utc;
use locardx_common::LocardError;
use locardx_database::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Record representing a single tamper-evident audit event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub sequence_number: i64,
    pub event_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub actor_id: Option<String>,
    pub target_ref: Option<String>,
    pub details: String,
    pub prev_hash: String,
    pub current_hash: String,
}

/// Verification summary for the audit hash chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditChainVerification {
    pub is_valid: bool,
    pub total_events: i64,
    pub last_verified_sequence: i64,
    pub broken_sequence: Option<i64>,
    pub details: String,
}

/// Audit logging service providing tamper-evident operational traceability.
/// Uses a cryptographic SHA-256 hash chain to ensure log entries cannot be modified or reordered undetected.
pub struct AuditService {
    db: Arc<Database>,
}

impl AuditService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Computes the SHA-256 hash for an audit entry given its fields and previous hash.
    pub fn compute_event_hash(
        sequence_number: i64,
        event_id: &str,
        event_type: &str,
        timestamp: &str,
        actor_id: Option<&str>,
        target_ref: Option<&str>,
        details: &str,
        prev_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(sequence_number.to_string().as_bytes());
        hasher.update(b"|");
        hasher.update(event_id.as_bytes());
        hasher.update(b"|");
        hasher.update(event_type.as_bytes());
        hasher.update(b"|");
        hasher.update(timestamp.as_bytes());
        hasher.update(b"|");
        hasher.update(actor_id.unwrap_or("").as_bytes());
        hasher.update(b"|");
        hasher.update(target_ref.unwrap_or("").as_bytes());
        hasher.update(b"|");
        hasher.update(details.as_bytes());
        hasher.update(b"|");
        hasher.update(prev_hash.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Records an audit event (convenience method preserving existing signature).
    pub fn log_event(&self, event_type: &str, details: &str) -> Result<(), LocardError> {
        self.log_structured_event(event_type, None, None, details)?;
        Ok(())
    }

    /// Records a structured audit event into SQLite with tamper-evident hash chaining.
    pub fn log_structured_event(
        &self,
        event_type: &str,
        actor_id: Option<&str>,
        target_ref: Option<&str>,
        details: &str,
    ) -> Result<AuditEvent, LocardError> {
        let event_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        let event = self.db.with_conn(|conn| {
            // Fetch last event to determine sequence_number and prev_hash
            let mut stmt = conn.prepare(
                "SELECT sequence_number, current_hash FROM audit_events ORDER BY sequence_number DESC LIMIT 1",
            )?;
            let mut rows = stmt.query([])?;
            let (next_seq, prev_hash) = if let Some(row) = rows.next()? {
                let last_seq: i64 = row.get(0)?;
                let last_hash: String = row.get(1)?;
                (last_seq + 1, last_hash)
            } else {
                (1, GENESIS_HASH.to_string())
            };

            let current_hash = Self::compute_event_hash(
                next_seq,
                &event_id,
                event_type,
                &timestamp,
                actor_id,
                target_ref,
                details,
                &prev_hash,
            );

            conn.execute(
                "INSERT INTO audit_events (sequence_number, event_id, event_type, timestamp, actor_id, target_ref, details, prev_hash, current_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    next_seq,
                    event_id,
                    event_type,
                    timestamp,
                    actor_id,
                    target_ref,
                    details,
                    prev_hash,
                    current_hash,
                ],
            )?;

            Ok(AuditEvent {
                sequence_number: next_seq,
                event_id,
                event_type: event_type.to_string(),
                timestamp,
                actor_id: actor_id.map(String::from),
                target_ref: target_ref.map(String::from),
                details: details.to_string(),
                prev_hash,
                current_hash,
            })
        })?;

        info!(
            target: "audit",
            event_type = %event.event_type,
            seq = %event.sequence_number,
            actor = ?event.actor_id,
            target_ref = ?event.target_ref,
            "Tamper-evident audit event registered"
        );

        Ok(event)
    }

    /// Verifies the cryptographic integrity of the entire audit chain from genesis to head.
    /// Detects missing, modified, inserted, or reordered entries.
    pub fn verify_chain(&self) -> Result<AuditChainVerification, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT sequence_number, event_id, event_type, timestamp, actor_id, target_ref, details, prev_hash, current_hash
                 FROM audit_events ORDER BY sequence_number ASC",
            )?;
            let mut rows = stmt.query([])?;

            let mut expected_seq = 1;
            let mut expected_prev_hash = GENESIS_HASH.to_string();
            let mut total_events = 0;

            while let Some(row) = rows.next()? {
                let seq: i64 = row.get(0)?;
                let event_id: String = row.get(1)?;
                let event_type: String = row.get(2)?;
                let timestamp: String = row.get(3)?;
                let actor_id: Option<String> = row.get(4)?;
                let target_ref: Option<String> = row.get(5)?;
                let details: String = row.get(6)?;
                let prev_hash: String = row.get(7)?;
                let current_hash: String = row.get(8)?;

                total_events += 1;

                if seq != expected_seq {
                    return Ok(AuditChainVerification {
                        is_valid: false,
                        total_events,
                        last_verified_sequence: expected_seq - 1,
                        broken_sequence: Some(seq),
                        details: format!("Sequence broken: expected {}, found {}", expected_seq, seq),
                    });
                }

                if prev_hash != expected_prev_hash {
                    return Ok(AuditChainVerification {
                        is_valid: false,
                        total_events,
                        last_verified_sequence: seq - 1,
                        broken_sequence: Some(seq),
                        details: format!("Previous hash mismatch at sequence {}", seq),
                    });
                }

                let computed_hash = Self::compute_event_hash(
                    seq,
                    &event_id,
                    &event_type,
                    &timestamp,
                    actor_id.as_deref(),
                    target_ref.as_deref(),
                    &details,
                    &prev_hash,
                );

                if computed_hash != current_hash {
                    return Ok(AuditChainVerification {
                        is_valid: false,
                        total_events,
                        last_verified_sequence: seq - 1,
                        broken_sequence: Some(seq),
                        details: format!("Record content tampered at sequence {}", seq),
                    });
                }

                expected_prev_hash = current_hash;
                expected_seq += 1;
            }

            Ok(AuditChainVerification {
                is_valid: true,
                total_events,
                last_verified_sequence: expected_seq - 1,
                broken_sequence: None,
                details: "Audit log cryptographic hash chain verified. Zero tampering detected.".to_string(),
            })
        })
    }

    /// Queries the most recent audit events.
    pub fn list_events(&self, limit: usize) -> Result<Vec<AuditEvent>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT sequence_number, event_id, event_type, timestamp, actor_id, target_ref, details, prev_hash, current_hash
                 FROM audit_events ORDER BY sequence_number DESC LIMIT ?1",
            )?;
            let mut rows = stmt.query(params![limit as i64])?;
            let mut events = Vec::new();

            while let Some(row) = rows.next()? {
                events.push(AuditEvent {
                    sequence_number: row.get(0)?,
                    event_id: row.get(1)?,
                    event_type: row.get(2)?,
                    timestamp: row.get(3)?,
                    actor_id: row.get(4)?,
                    target_ref: row.get(5)?,
                    details: row.get(6)?,
                    prev_hash: row.get(7)?,
                    current_hash: row.get(8)?,
                });
            }

            Ok(events)
        })
    }
}
