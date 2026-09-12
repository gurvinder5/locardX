-- Migration 011: Forensic File Recovery Schema
-- Stores recovery jobs, validated sources, recovered files, fragment information, and forensic recovery reports.

CREATE TABLE IF NOT EXISTS recovery_jobs (
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
