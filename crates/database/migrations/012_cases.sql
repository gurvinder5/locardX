-- Migration 012: Unified Case Management, Cross-Module Evidence, Chain of Custody, and Reports
-- Strictly additive; preserves all previous schema versions and existing data.

CREATE TABLE IF NOT EXISTS cases (
    case_id           TEXT PRIMARY KEY,
    case_reference    TEXT NOT NULL UNIQUE COLLATE NOCASE,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'Open', -- 'Open', 'InProgress', 'Completed', 'Archived'
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

-- Case to Operation associations (preserves independent operations)
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

-- Evidential items introduced, acquired, or carved under this case
CREATE TABLE IF NOT EXISTS case_evidence (
    evidence_id       TEXT PRIMARY KEY,
    case_id           TEXT NOT NULL,
    evidence_type     TEXT NOT NULL, -- 'PhysicalStorage', 'AcquisitionImage', 'RecoveredDataset', 'LogicalFile'
    identifier        TEXT NOT NULL, -- device id, image path, job id, or relative output path
    label             TEXT NOT NULL,
    sha256            TEXT,
    size_bytes        INTEGER,
    introduced_by     TEXT NOT NULL,
    introduced_at     TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'Active', -- 'Active', 'Analyzed', 'Archived', 'Exported'
    notes             TEXT,
    FOREIGN KEY (case_id) REFERENCES cases(case_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_case_evidence_case ON case_evidence(case_id);
CREATE INDEX IF NOT EXISTS idx_case_evidence_type ON case_evidence(evidence_type);
CREATE INDEX IF NOT EXISTS idx_case_evidence_id ON case_evidence(identifier);
CREATE INDEX IF NOT EXISTS idx_case_evidence_sha256 ON case_evidence(sha256);

-- Technical chain of custody ledger linked with cryptographic audit hashes
CREATE TABLE IF NOT EXISTS case_custody (
    custody_id        TEXT PRIMARY KEY,
    case_id           TEXT NOT NULL,
    evidence_id       TEXT,
    event_type        TEXT NOT NULL, -- 'EvidenceIntroduced', 'EvidenceAcquired', 'EvidenceVerified', 'EvidenceAnalyzed', 'EvidenceExported', 'ReportGenerated', 'CaseStatusChanged'
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

-- Case-level forensic and sanitization reports
CREATE TABLE IF NOT EXISTS case_reports (
    report_id             TEXT PRIMARY KEY,
    case_id               TEXT NOT NULL,
    report_type           TEXT NOT NULL, -- 'CaseForensicReport', 'SanitizationCertificate', 'RecoveryReport'
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

-- Additional index on audit_events for high-speed target_ref lookups
CREATE INDEX IF NOT EXISTS idx_audit_events_target ON audit_events(target_ref);

INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (12, datetime('now'));
