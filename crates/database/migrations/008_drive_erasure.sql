-- Migration 008: Secure Drive Erasure Records Schema
-- Records physical storage device sanitization operations, media-aware plans, simulation telemetry, and verification results.

CREATE TABLE IF NOT EXISTS drive_erasure_records (
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
    execution_mode         TEXT NOT NULL, -- 'Simulation' or 'RealHardware'
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
