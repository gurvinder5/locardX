-- Migration 005: Safety & Authorization Interlocks
-- Tracks safety evaluations and structured two-stage confirmation challenges.

CREATE TABLE IF NOT EXISTS safety_evaluations (
    evaluation_id       TEXT PRIMARY KEY,
    operation_id        TEXT,
    target_identifier   TEXT NOT NULL,
    target_type         TEXT NOT NULL,
    operation_type      TEXT NOT NULL,
    actor_id            TEXT,
    decision            TEXT NOT NULL,
    reason_code         TEXT NOT NULL,
    risk_level          TEXT NOT NULL,
    reason_message      TEXT NOT NULL,
    evaluated_at        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_safety_eval_target ON safety_evaluations(target_identifier);
CREATE INDEX IF NOT EXISTS idx_safety_eval_time ON safety_evaluations(evaluated_at);
CREATE INDEX IF NOT EXISTS idx_safety_eval_decision ON safety_evaluations(decision);

CREATE TABLE IF NOT EXISTS operation_confirmations (
    confirmation_id     TEXT PRIMARY KEY,
    operation_id        TEXT NOT NULL,
    actor_id            TEXT NOT NULL,
    target_identifier   TEXT NOT NULL,
    target_type         TEXT NOT NULL,
    target_snapshot     TEXT NOT NULL,
    operation_type      TEXT NOT NULL,
    risk_level          TEXT NOT NULL,
    warning_acknowledged INTEGER NOT NULL DEFAULT 0,
    status              TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    expires_at          TEXT NOT NULL,
    confirmed_at        TEXT
);

CREATE INDEX IF NOT EXISTS idx_op_conf_op_id ON operation_confirmations(operation_id);
CREATE INDEX IF NOT EXISTS idx_op_conf_actor ON operation_confirmations(actor_id);
CREATE INDEX IF NOT EXISTS idx_op_conf_status ON operation_confirmations(status);
