-- Migration 015: Cross-Module Case Management Integration
-- Adds case association tracking to Drive Erasure, File Erasure, and Core Operations.
-- Preserves all existing data non-destructively; existing records retain NULL case_id (standalone/unassigned).

ALTER TABLE drive_erasure_records ADD COLUMN case_id TEXT;
CREATE INDEX IF NOT EXISTS idx_drive_erasure_case ON drive_erasure_records(case_id);

ALTER TABLE file_erasure_records ADD COLUMN case_id TEXT;
CREATE INDEX IF NOT EXISTS idx_file_erasure_case ON file_erasure_records(case_id);

ALTER TABLE operations ADD COLUMN case_id TEXT;
CREATE INDEX IF NOT EXISTS idx_operations_case ON operations(case_id);

INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (15, datetime('now'));
