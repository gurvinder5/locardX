-- Migration 009: Drive Erasure Forensics & Sanitization Reports
-- Adds forensic metadata fields (plan_id, bus_type, evidence_digest) to drive_erasure_records
-- and creates the sanitization_reports table for tamper-evident certificates.

CREATE TABLE IF NOT EXISTS sanitization_reports (
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
CREATE INDEX IF NOT EXISTS idx_sanitization_reports_device ON sanitization_reports(target_identifier);
