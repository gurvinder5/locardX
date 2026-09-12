-- Migration 004: Centralized Operation Orchestration & Lifecycle Tracking
CREATE TABLE IF NOT EXISTS operations (
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
