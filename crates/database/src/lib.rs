use locardx_common::LocardError;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

/// Thread-safe SQLite database manager.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Opens or connects to a SQLite database.
    /// Uses `:memory:` for ephemeral testing or a filesystem path for persistence.
    pub fn open(path: &str) -> Result<Self, LocardError> {
        let connection = if path == ":memory:" {
            info!("Initializing in-memory SQLite database");
            Connection::open_in_memory().map_err(|e| {
                LocardError::Database(format!("Failed to open in-memory SQLite: {}", e))
            })?
        } else {
            let p = Path::new(path);
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        LocardError::Database(format!("Failed to create DB directory: {}", e))
                    })?;
                }
            }
            info!("Connecting to SQLite database at: {}", path);
            Connection::open(path).map_err(|e| {
                LocardError::Database(format!("Failed to open SQLite database: {}", e))
            })?
        };

        let db = Self {
            conn: Mutex::new(connection),
        };

        db.init_schema()?;
        Ok(db)
    }

    /// Initializes core tables and runs migrations non-destructively.
    /// Preserves existing data and migrations.
    pub fn init_schema(&self) -> Result<(), LocardError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LocardError::Database(format!("Database mutex poisoned: {}", e)))?;

        // Baseline schema version table and metadata
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
            INSERT OR IGNORE INTO app_metadata (key, value, updated_at)
            VALUES ('schema_version', '1', datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to initialize baseline metadata schema: {}",
                e
            ))
        })?;

        // Migration 002: Closed Authentication & User Management
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                user_id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE COLLATE NOCASE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_login_at TEXT
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS login_attempts (
                username TEXT PRIMARY KEY COLLATE NOCASE,
                failed_count INTEGER NOT NULL DEFAULT 0,
                locked_until INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (2, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!("Failed to execute migration 002 (users): {}", e))
        })?;

        // Migration 003: Tamper-Evident Audit Events & Integrity Records
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_events (
                sequence_number INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL UNIQUE,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                actor_id TEXT,
                target_ref TEXT,
                details TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                current_hash TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_events_type ON audit_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp);

            CREATE TABLE IF NOT EXISTS integrity_records (
                record_id TEXT PRIMARY KEY,
                operation_id TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_path TEXT NOT NULL,
                target_name TEXT NOT NULL,
                algorithm TEXT NOT NULL,
                digest TEXT NOT NULL,
                bytes_processed INTEGER NOT NULL,
                verification_status TEXT NOT NULL,
                expected_digest TEXT,
                actor_id TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_integrity_records_target ON integrity_records(target_path);
            CREATE INDEX IF NOT EXISTS idx_integrity_records_created ON integrity_records(created_at);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (3, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!("Failed to execute migration 003 (integrity & audit): {}", e))
        })?;

        // Migration 004: Operations Orchestration & Lifecycle Tracking
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS operations (
                operation_id TEXT PRIMARY KEY,
                operation_type TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_identifier TEXT NOT NULL,
                target_display_name TEXT NOT NULL,
                target_size_bytes INTEGER,
                actor_id TEXT,
                current_state TEXT NOT NULL,
                progress_percentage REAL,
                progress_bytes_processed INTEGER,
                progress_total_bytes INTEGER,
                progress_stage TEXT,
                progress_message TEXT,
                result_summary TEXT,
                failure_reason TEXT,
                cancellation_requested INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_operations_state ON operations(current_state);
            CREATE INDEX IF NOT EXISTS idx_operations_created ON operations(created_at);
            CREATE INDEX IF NOT EXISTS idx_operations_type ON operations(operation_type);
            CREATE INDEX IF NOT EXISTS idx_operations_actor ON operations(actor_id);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (4, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 004 (operations): {}",
                e
            ))
        })?;

        // Migration 005: Safety & Authorization Interlocks
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS safety_evaluations (
                evaluation_id       TEXT PRIMARY KEY,
                operation_id        TEXT,
                target_identifier   TEXT NOT NULL,
                target_type         TEXT NOT NULL,
                operation_type      TEXT NOT NULL,
                actor_id            TEXT,
                decision            TEXT NOT NULL,
                reason_code         TEXT NOT NULL,
                risk_level          TEXT NOT NULL,
                reason_message      TEXT NOT NULL,
                evaluated_at        TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_safety_eval_target ON safety_evaluations(target_identifier);
            CREATE INDEX IF NOT EXISTS idx_safety_eval_time ON safety_evaluations(evaluated_at);
            CREATE INDEX IF NOT EXISTS idx_safety_eval_decision ON safety_evaluations(decision);

            CREATE TABLE IF NOT EXISTS operation_confirmations (
                confirmation_id     TEXT PRIMARY KEY,
                operation_id        TEXT NOT NULL,
                actor_id            TEXT NOT NULL,
                target_identifier   TEXT NOT NULL,
                target_type         TEXT NOT NULL,
                target_snapshot     TEXT NOT NULL,
                operation_type      TEXT NOT NULL,
                risk_level          TEXT NOT NULL,
                warning_acknowledged INTEGER NOT NULL DEFAULT 0,
                status              TEXT NOT NULL,
                created_at          TEXT NOT NULL,
                expires_at          TEXT NOT NULL,
                confirmed_at        TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_op_conf_op_id ON operation_confirmations(operation_id);
            CREATE INDEX IF NOT EXISTS idx_op_conf_actor ON operation_confirmations(actor_id);
            CREATE INDEX IF NOT EXISTS idx_op_conf_status ON operation_confirmations(status);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (5, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 005 (safety interlocks): {}",
                e
            ))
        })?;

        // Migration 006: Sanitization Planning & Pre-Erasure Evidence Schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sanitization_plans (
                plan_id             TEXT PRIMARY KEY,
                target_type         TEXT NOT NULL,
                target_identifier   TEXT NOT NULL,
                media_type          TEXT NOT NULL,
                sanitization_scope  TEXT NOT NULL,
                recommended_method  TEXT NOT NULL,
                is_applicable       INTEGER NOT NULL,
                risk_level          TEXT NOT NULL,
                verification_strategy TEXT NOT NULL,
                applicable_standard TEXT,
                standard_method_id  TEXT,
                limitations         TEXT NOT NULL,
                reason_codes        TEXT NOT NULL,
                target_snapshot     TEXT NOT NULL,
                actor_id            TEXT,
                created_at          TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sanitization_plans_target ON sanitization_plans(target_identifier);
            CREATE INDEX IF NOT EXISTS idx_sanitization_plans_created ON sanitization_plans(created_at);

            CREATE TABLE IF NOT EXISTS pre_erasure_evidence (
                evidence_id         TEXT PRIMARY KEY,
                operation_id        TEXT NOT NULL,
                actor_id            TEXT,
                target_type         TEXT NOT NULL,
                target_identifier   TEXT NOT NULL,
                target_display_name TEXT NOT NULL,
                target_snapshot     TEXT NOT NULL,
                sanitization_method TEXT NOT NULL,
                sanitization_scope  TEXT NOT NULL,
                verification_strategy TEXT NOT NULL,
                applicable_limitations TEXT NOT NULL,
                safety_evaluation_ref TEXT,
                created_at          TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_pre_erasure_evidence_op_id ON pre_erasure_evidence(operation_id);
            CREATE INDEX IF NOT EXISTS idx_pre_erasure_evidence_target ON pre_erasure_evidence(target_identifier);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (6, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 006 (sanitization plans): {}",
                e
            ))
        })?;

        // Migration 007: Secure File & Folder Erasure Records Schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_erasure_records (
                record_id           TEXT PRIMARY KEY,
                operation_id        TEXT NOT NULL,
                actor_id            TEXT,
                target_path         TEXT NOT NULL,
                canonical_path      TEXT NOT NULL,
                scope               TEXT NOT NULL,
                sanitization_method TEXT NOT NULL,
                status              TEXT NOT NULL,
                verification_outcome TEXT NOT NULL,
                bytes_processed     INTEGER NOT NULL,
                total_files         INTEGER NOT NULL DEFAULT 1,
                files_sanitized     INTEGER NOT NULL DEFAULT 0,
                files_failed        INTEGER NOT NULL DEFAULT 0,
                directories_removed INTEGER NOT NULL DEFAULT 0,
                pre_metadata_json   TEXT NOT NULL,
                failure_reason      TEXT,
                limitations         TEXT NOT NULL,
                started_at          TEXT NOT NULL,
                completed_at        TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_file_erasure_op_id ON file_erasure_records(operation_id);
            CREATE INDEX IF NOT EXISTS idx_file_erasure_target ON file_erasure_records(target_path);
            CREATE INDEX IF NOT EXISTS idx_file_erasure_status ON file_erasure_records(status);
            CREATE INDEX IF NOT EXISTS idx_file_erasure_completed ON file_erasure_records(completed_at);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (7, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 007 (file erasure records): {}",
                e
            ))
        })?;

        // Migration 008: Secure Drive Erasure Records Schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS drive_erasure_records (
                record_id              TEXT PRIMARY KEY,
                operation_id           TEXT NOT NULL UNIQUE,
                actor_id               TEXT,
                physical_device_id     TEXT NOT NULL,
                display_name           TEXT NOT NULL,
                vendor                 TEXT,
                model                  TEXT,
                serial_number          TEXT,
                media_type             TEXT NOT NULL,
                capacity_bytes         INTEGER NOT NULL,
                sector_size            INTEGER NOT NULL,
                device_snapshot_json   TEXT NOT NULL,
                capabilities_json      TEXT NOT NULL,
                sanitization_method    TEXT NOT NULL,
                execution_mode         TEXT NOT NULL,
                verification_strategy  TEXT NOT NULL,
                verification_outcome   TEXT NOT NULL,
                status                 TEXT NOT NULL,
                bytes_processed        INTEGER NOT NULL,
                elapsed_seconds        REAL NOT NULL,
                failure_reason         TEXT,
                audit_references       TEXT,
                started_at             TEXT NOT NULL,
                completed_at           TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_drive_erasure_op_id ON drive_erasure_records(operation_id);
            CREATE INDEX IF NOT EXISTS idx_drive_erasure_device ON drive_erasure_records(physical_device_id);
            CREATE INDEX IF NOT EXISTS idx_drive_erasure_status ON drive_erasure_records(status);
            CREATE INDEX IF NOT EXISTS idx_drive_erasure_completed ON drive_erasure_records(completed_at);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at)
            VALUES (8, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 008 (drive erasure records): {}",
                e
            ))
        })?;

        // Migration 009: Drive Erasure Forensics & Sanitization Reports
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sanitization_reports (
                report_id              TEXT PRIMARY KEY,
                operation_id           TEXT NOT NULL UNIQUE,
                plan_id                TEXT NOT NULL,
                actor_id               TEXT,
                target_identifier      TEXT NOT NULL,
                execution_mode         TEXT NOT NULL,
                is_simulation          INTEGER NOT NULL,
                status                 TEXT NOT NULL,
                verification_outcome   TEXT NOT NULL,
                report_digest          TEXT NOT NULL,
                audit_chain_reference  TEXT NOT NULL,
                report_json            TEXT NOT NULL,
                generated_at           TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sanitization_reports_op ON sanitization_reports(operation_id);
            CREATE INDEX IF NOT EXISTS idx_sanitization_reports_device ON sanitization_reports(target_identifier);",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 009 (sanitization reports): {}",
                e
            ))
        })?;

        let migration_9_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 9",
                [],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count > 0)
                },
            )
            .unwrap_or(false);

        if !migration_9_applied {
            let _ = conn.execute(
                "ALTER TABLE drive_erasure_records ADD COLUMN plan_id TEXT;",
                [],
            );
            let _ = conn.execute(
                "ALTER TABLE drive_erasure_records ADD COLUMN bus_type TEXT;",
                [],
            );
            let _ = conn.execute(
                "ALTER TABLE drive_erasure_records ADD COLUMN evidence_digest TEXT;",
                [],
            );
            let _ = conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (9, datetime('now'));",
                [],
            );
        }

        // Migration 010: Forensic Acquisition & Raw/DD Imaging Records
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS acquisition_records (
                acquisition_id        TEXT PRIMARY KEY,
                operation_id          TEXT NOT NULL UNIQUE,
                actor_id              TEXT,
                source_device_id      TEXT NOT NULL,
                source_display_name   TEXT NOT NULL,
                source_vendor         TEXT,
                source_model          TEXT,
                source_serial         TEXT,
                source_media_type     TEXT NOT NULL,
                source_capacity_bytes INTEGER NOT NULL,
                source_sector_size    INTEGER NOT NULL,
                source_bus_type       TEXT,
                source_snapshot_json  TEXT NOT NULL,
                destination_path      TEXT NOT NULL,
                image_format          TEXT NOT NULL,
                image_size_bytes      INTEGER NOT NULL,
                image_sha256          TEXT NOT NULL,
                status                TEXT NOT NULL,
                bytes_acquired        INTEGER NOT NULL,
                elapsed_seconds       REAL NOT NULL,
                failure_reason        TEXT,
                audit_reference       TEXT NOT NULL,
                started_at            TEXT NOT NULL,
                completed_at          TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_acq_records_op_id ON acquisition_records(operation_id);
            CREATE INDEX IF NOT EXISTS idx_acq_records_source ON acquisition_records(source_device_id);
            CREATE INDEX IF NOT EXISTS idx_acq_records_status ON acquisition_records(status);
            CREATE INDEX IF NOT EXISTS idx_acq_records_completed ON acquisition_records(completed_at);
            INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (10, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 010 (acquisition records): {}",
                e
            ))
        })?;

        // Migration 011: Forensic File Recovery Schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS recovery_jobs (
                job_id               TEXT PRIMARY KEY,
                operation_id         TEXT NOT NULL UNIQUE,
                actor_id             TEXT,
                acquisition_id       TEXT NOT NULL,
                source_image_path    TEXT NOT NULL,
                source_image_sha256  TEXT NOT NULL,
                recovery_mode        TEXT NOT NULL,
                status               TEXT NOT NULL,
                bytes_scanned        INTEGER NOT NULL DEFAULT 0,
                files_recovered      INTEGER NOT NULL DEFAULT 0,
                candidates_evaluated INTEGER NOT NULL DEFAULT 0,
                elapsed_seconds      REAL NOT NULL DEFAULT 0.0,
                failure_reason       TEXT,
                audit_reference      TEXT NOT NULL,
                started_at           TEXT NOT NULL,
                completed_at         TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS recovery_sources (
                source_id            TEXT PRIMARY KEY,
                acquisition_id       TEXT NOT NULL,
                image_path           TEXT NOT NULL,
                image_size_bytes     INTEGER NOT NULL,
                image_sha256         TEXT NOT NULL,
                original_device_id   TEXT NOT NULL,
                original_serial      TEXT,
                verified_at          TEXT NOT NULL,
                is_trusted           INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS recovered_files (
                file_id              TEXT PRIMARY KEY,
                job_id               TEXT NOT NULL,
                source_offset        INTEGER NOT NULL,
                size_bytes           INTEGER NOT NULL,
                file_type            TEXT NOT NULL,
                mime_type            TEXT NOT NULL,
                suggested_filename   TEXT NOT NULL,
                recovery_method      TEXT NOT NULL,
                validation_status    TEXT NOT NULL,
                confidence_score     INTEGER NOT NULL,
                confidence_grade     TEXT NOT NULL,
                is_fragmented        INTEGER NOT NULL DEFAULT 0,
                sha256_hash          TEXT NOT NULL,
                output_relative_path TEXT,
                evidence_factors_json TEXT NOT NULL,
                created_at           TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS recovery_fragments (
                fragment_id          TEXT PRIMARY KEY,
                file_id              TEXT NOT NULL,
                fragment_index       INTEGER NOT NULL,
                source_offset        INTEGER NOT NULL,
                size_bytes           INTEGER NOT NULL,
                fragment_type        TEXT NOT NULL,
                confidence           REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS recovery_reports (
                report_id            TEXT PRIMARY KEY,
                job_id               TEXT NOT NULL UNIQUE,
                report_digest        TEXT NOT NULL,
                audit_reference      TEXT NOT NULL,
                report_json          TEXT NOT NULL,
                generated_at         TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_recovery_jobs_op_id ON recovery_jobs(operation_id);
            CREATE INDEX IF NOT EXISTS idx_recovery_jobs_acq_id ON recovery_jobs(acquisition_id);
            CREATE INDEX IF NOT EXISTS idx_recovery_jobs_status ON recovery_jobs(status);
            CREATE INDEX IF NOT EXISTS idx_recovered_files_job ON recovered_files(job_id);
            CREATE INDEX IF NOT EXISTS idx_recovered_files_type ON recovered_files(file_type);
            CREATE INDEX IF NOT EXISTS idx_recovered_files_method ON recovered_files(recovery_method);
            CREATE INDEX IF NOT EXISTS idx_recovered_files_validation ON recovered_files(validation_status);
            CREATE INDEX IF NOT EXISTS idx_recovered_files_confidence ON recovered_files(confidence_score);
            CREATE INDEX IF NOT EXISTS idx_recovered_files_created ON recovered_files(created_at);
            CREATE INDEX IF NOT EXISTS idx_recovery_fragments_file ON recovery_fragments(file_id);
            INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (11, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 011 (recovery schema): {}",
                e
            ))
        })?;

        // Migration 012: Unified Case Management, Cross-Module Evidence, Chain of Custody, and Reports
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cases (
                case_id           TEXT PRIMARY KEY,
                case_reference    TEXT NOT NULL UNIQUE COLLATE NOCASE,
                title             TEXT NOT NULL,
                description       TEXT NOT NULL,
                status            TEXT NOT NULL DEFAULT 'Open',
                lead_investigator TEXT NOT NULL,
                created_at        TEXT NOT NULL,
                updated_at        TEXT NOT NULL,
                closed_at         TEXT,
                metadata_json     TEXT NOT NULL DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_cases_ref ON cases(case_reference);
            CREATE INDEX IF NOT EXISTS idx_cases_status ON cases(status);
            CREATE INDEX IF NOT EXISTS idx_cases_actor ON cases(lead_investigator);
            CREATE INDEX IF NOT EXISTS idx_cases_created ON cases(created_at);

            CREATE TABLE IF NOT EXISTS case_operations (
                id                TEXT PRIMARY KEY,
                case_id           TEXT NOT NULL,
                operation_id      TEXT NOT NULL,
                operation_type    TEXT NOT NULL,
                associated_by     TEXT NOT NULL,
                associated_at     TEXT NOT NULL,
                notes             TEXT,
                FOREIGN KEY (case_id) REFERENCES cases(case_id) ON DELETE RESTRICT,
                UNIQUE(case_id, operation_id)
            );
            CREATE INDEX IF NOT EXISTS idx_case_ops_case ON case_operations(case_id);
            CREATE INDEX IF NOT EXISTS idx_case_ops_op ON case_operations(operation_id);
            CREATE INDEX IF NOT EXISTS idx_case_ops_type ON case_operations(operation_type);

            CREATE TABLE IF NOT EXISTS case_evidence (
                evidence_id       TEXT PRIMARY KEY,
                case_id           TEXT NOT NULL,
                evidence_type     TEXT NOT NULL,
                identifier        TEXT NOT NULL,
                label             TEXT NOT NULL,
                sha256            TEXT,
                size_bytes        INTEGER,
                introduced_by     TEXT NOT NULL,
                introduced_at     TEXT NOT NULL,
                status            TEXT NOT NULL DEFAULT 'Active',
                notes             TEXT,
                FOREIGN KEY (case_id) REFERENCES cases(case_id) ON DELETE RESTRICT
            );
            CREATE INDEX IF NOT EXISTS idx_case_evidence_case ON case_evidence(case_id);
            CREATE INDEX IF NOT EXISTS idx_case_evidence_type ON case_evidence(evidence_type);
            CREATE INDEX IF NOT EXISTS idx_case_evidence_id ON case_evidence(identifier);
            CREATE INDEX IF NOT EXISTS idx_case_evidence_sha256 ON case_evidence(sha256);

            CREATE TABLE IF NOT EXISTS case_custody (
                custody_id        TEXT PRIMARY KEY,
                case_id           TEXT NOT NULL,
                evidence_id       TEXT,
                event_type        TEXT NOT NULL,
                actor_id          TEXT NOT NULL,
                timestamp         TEXT NOT NULL,
                action            TEXT NOT NULL,
                details           TEXT NOT NULL,
                audit_event_id    TEXT,
                audit_hash        TEXT,
                FOREIGN KEY (case_id) REFERENCES cases(case_id) ON DELETE RESTRICT,
                FOREIGN KEY (evidence_id) REFERENCES case_evidence(evidence_id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_case_custody_case ON case_custody(case_id);
            CREATE INDEX IF NOT EXISTS idx_case_custody_evidence ON case_custody(evidence_id);
            CREATE INDEX IF NOT EXISTS idx_case_custody_time ON case_custody(timestamp);
            CREATE INDEX IF NOT EXISTS idx_case_custody_type ON case_custody(event_type);

            CREATE TABLE IF NOT EXISTS case_reports (
                report_id             TEXT PRIMARY KEY,
                case_id               TEXT NOT NULL,
                report_type           TEXT NOT NULL,
                title                 TEXT NOT NULL,
                report_digest         TEXT NOT NULL,
                audit_chain_reference TEXT NOT NULL,
                generated_by          TEXT NOT NULL,
                generated_at          TEXT NOT NULL,
                report_json           TEXT NOT NULL,
                report_markdown       TEXT NOT NULL,
                FOREIGN KEY (case_id) REFERENCES cases(case_id) ON DELETE RESTRICT
            );
            CREATE INDEX IF NOT EXISTS idx_case_reports_case ON case_reports(case_id);
            CREATE INDEX IF NOT EXISTS idx_case_reports_type ON case_reports(report_type);
            CREATE INDEX IF NOT EXISTS idx_case_reports_created ON case_reports(generated_at);

            CREATE INDEX IF NOT EXISTS idx_audit_events_target ON audit_events(target_ref);

            INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (12, datetime('now'));",
        )
        .map_err(|e| {
            LocardError::Database(format!(
                "Failed to execute migration 012 (cases schema): {}",
                e
            ))
        })?;

        // Migration 013: Authentication & Role-Based Access Control Enhancements
        let migration_13_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 13",
                [],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count > 0)
                },
            )
            .unwrap_or(false);

        if !migration_13_applied {
            let _ = conn.execute("ALTER TABLE users ADD COLUMN display_name TEXT;", []);
            let _ = conn.execute(
                "ALTER TABLE users ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';",
                [],
            );
            let _ = conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);",
                [],
            );
            let _ = conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);",
                [],
            );
            let _ = conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);",
                [],
            );
            let _ = conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_users_enabled ON users(enabled);",
                [],
            );
            let _ = conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (13, datetime('now'));",
                [],
            );
        }

        info!("SQLite schema and migrations verified.");
        Ok(())
    }

    /// Safely executes a closure with read/write access to the inner connection.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, LocardError>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| LocardError::Database(format!("Database mutex poisoned: {}", e)))?;

        f(&mut conn).map_err(|e| LocardError::Database(format!("Database query error: {}", e)))
    }

    /// Health check verifying connectivity without side effects.
    pub fn is_healthy(&self) -> bool {
        match self.conn.lock() {
            Ok(conn) => conn.query_row("SELECT 1", [], |_| Ok(())).is_ok(),
            Err(_) => false,
        }
    }
}
