-- Migration 006: Sanitization Planning & Pre-Erasure Evidence Schema
-- Establishes non-destructive planning, media-aware selection persistence, and tamper-evident pre-erasure evidence.

CREATE TABLE IF NOT EXISTS sanitization_plans (
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
VALUES (6, datetime('now'));
