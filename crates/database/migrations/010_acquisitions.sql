-- Migration 010: Forensic Acquisition & Raw/DD Imaging Records
-- Tracks raw bitstream forensic disk acquisitions, source snapshots, destination images, and SHA-256 hashes.

CREATE TABLE IF NOT EXISTS acquisition_records (
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
