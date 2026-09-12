-- Migration 007: Secure File & Folder Erasure Records Schema
CREATE TABLE IF NOT EXISTS file_erasure_records (
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
