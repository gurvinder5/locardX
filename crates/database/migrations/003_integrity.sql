-- LocardX Database Migration 003: Tamper-Evident Audit Events & Integrity Records
CREATE TABLE IF NOT EXISTS audit_events (
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
